//! Cluster 7 IR — `#[derive(Error)]` semantic model (enum-only v1 parity).
//!
//! Absorbs, byte-for-byte, the model that today lives in
//! `bpk-lib/crates/macros/src/error.rs`: the per-variant `#[error("…")]` format
//! literal (with the positional→binding rewrite recorded in [`FormatSpec`]) and
//! the explicit `#[source]` / `#[from]` wiring. [`build`] reads the validated
//! derive input and produces an [`ErrorIr`]; the matching `lowering::error`
//! renders the exact `Display` / `Error::source` / `From` tokens the hand-rolled
//! derive rendered (see §E.1 — PreserveExact).
//!
//! v1 scope is TODAY's enum-only surface ONLY. The packet-02 extensions
//! (struct targets, `code` / HandlingClass, `PathBuf::display`, nested-Display
//! delegate) are DEFERRED to crate 2 and encode NO fields here.

use std::collections::BTreeSet;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{Data, Fields, Ident, LitStr, Type, Variant};

use crate::diagnostic::{Diagnostics, ExpectedForm, MacroDiagnostic, RequiredAction};
use crate::identity::{DiagnosticCode, KeyName, ShapeRule};
use crate::tokens::SynItem;
use crate::validation::ValidatedDeclaration;

/// The enum-only error contract: the target enum's identity + generic surface,
/// plus one modeled variant per source variant, in declaration order.
pub struct ErrorIr {
    /// The enum's own ident — the span/blame anchor for every emitted impl.
    /// `ContractId` is a local symbolic id and is NEVER string-parsed for this.
    pub ident: Ident,
    /// The enum's generic surface. The old derive calls `split_for_impl` on the
    /// input generics for every impl (error.rs:55, :249), so generic /
    /// lifetime-bearing enums are accepted and MUST keep byte-identical headers.
    pub generics: syn::Generics,
    pub variants: Vec<ErrorVariant>,
}

/// A single `#[derive(Error)]` variant: its ident, its field shape, its parsed
/// `#[error]` format literal, and any explicit source binding.
pub struct ErrorVariant {
    pub ident: Ident,
    pub fields: ErrorFields,
    pub display: FormatSpec,
    pub source: Option<SourceBinding>,
}

/// The parsed `#[error("…")]` format literal.
///
/// `rewritten` is the string fed to `write!` after positional/implicit refs
/// (`{0}`, `{}`) are rewritten to `{__arg_N}`; named refs (`{field}`) pass
/// through untouched. `original` is retained so a no-rewrite variant re-emits
/// its exact original token (preserving `\`-continued multi-line literals).
pub struct FormatSpec {
    pub original: LitStr,
    pub rewritten: String,
    pub used_names: BTreeSet<String>,
    pub used_positions: BTreeSet<usize>,
}

/// Which field of a variant carries the error source, and how.
pub struct SourceBinding {
    pub member: SourceMember,
    /// `true` when the field type's outermost path segment is `Box` — the
    /// source is then `Some(field.as_ref())` (the inner `dyn Error`).
    pub is_box: bool,
    pub ty: Type,
    /// `true` when annotated `#[from]` (also emit `impl From`).
    pub is_from: bool,
}

/// Named-field member or tuple index carrying the source.
pub enum SourceMember {
    Named(Ident),
    Indexed(usize),
}

/// A guaranteed-ident named field (cures the `Option<Ident>` `.expect` that the
/// hand-rolled derive carried at its display-arm site: the model is built once
/// from a guaranteed-`Some` `Fields::Named`, so downstream never re-checks).
pub struct NamedField {
    pub ident: Ident,
    pub ty: Type,
}

/// The variant field shape, modeled so lowering never re-touches raw `syn`.
pub enum ErrorFields {
    Named(Vec<NamedField>),
    Unnamed(usize),
    Unit,
}

impl FormatSpec {
    /// The literal to feed `write!`: the untouched original token when no
    /// positional rewrite happened (preserves `\`-continued literals
    /// byte-for-byte), otherwise a fresh literal carrying the rewritten string.
    pub fn literal(&self) -> LitStr {
        if self.rewritten == self.original.value() {
            self.original.clone()
        } else {
            LitStr::new(&self.rewritten, self.original.span())
        }
    }
}

/// Build the [`ErrorIr`] from a validated `#[derive(Error)]` declaration.
///
/// # Errors
///
/// Returns [`Diagnostics`] when the target is not an enum (`BP-SHAPE-0010`),
/// a variant is missing / duplicating its `#[error]` attribute
/// (`BP-GRAMMAR-0010` / `BP-GRAMMAR-0011`), a variant declares more than one
/// `#[source]`/`#[from]` field or a `#[from]` on a multi-field variant
/// (`BP-RULE-0010` / `BP-RULE-0011`), or the format literal is malformed
/// (`BP-VALUE-0010`..`BP-VALUE-0013`).
pub fn build(decl: &ValidatedDeclaration) -> Result<ErrorIr, Diagnostics> {
    let SynItem::Derive(input) = &decl.item else {
        return Err(diagnostic(
            DiagnosticCode::ErrorNotEnum,
            Span::call_site(),
            RequiredAction::ChangeShape {
                expected: ShapeRule::Enum,
            },
            "#[derive(Error)] supports enums only; hand-write `Display`/`Error` for structs"
                .to_owned(),
        ));
    };
    let Data::Enum(data) = &input.data else {
        return Err(diagnostic(
            DiagnosticCode::ErrorNotEnum,
            input.ident.span(),
            RequiredAction::ChangeShape {
                expected: ShapeRule::Enum,
            },
            "#[derive(Error)] supports enums only; hand-write `Display`/`Error` for structs"
                .to_owned(),
        ));
    };

    let mut variants = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        variants.push(build_variant(variant)?);
    }
    Ok(ErrorIr {
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        variants,
    })
}

/// Model a single variant, folding its `#[error]`/`#[source]`/`#[from]` attrs.
fn build_variant(variant: &Variant) -> Result<ErrorVariant, Diagnostics> {
    let lit = variant_error_literal(variant)?;
    let display = rewrite_format(&lit)?;
    let source = parse_source_field(variant)?;
    Ok(ErrorVariant {
        ident: variant.ident.clone(),
        fields: model_fields(&variant.fields),
        display,
        source,
    })
}

/// Lower raw `syn::Fields` into the guaranteed-ident [`ErrorFields`] model. A
/// `Fields::Named` member always carries `Some(ident)` by `syn`'s invariants;
/// the (unreachable) `None` is skipped rather than `.expect`-ed, so the model
/// downstream is total.
fn model_fields(fields: &Fields) -> ErrorFields {
    match fields {
        Fields::Named(named) => {
            let mut modeled = Vec::with_capacity(named.named.len());
            for field in &named.named {
                let Some(ident) = field.ident.as_ref() else {
                    continue;
                };
                modeled.push(NamedField {
                    ident: ident.clone(),
                    ty: field.ty.clone(),
                });
            }
            ErrorFields::Named(modeled)
        }
        Fields::Unnamed(unnamed) => ErrorFields::Unnamed(unnamed.unnamed.len()),
        Fields::Unit => ErrorFields::Unit,
    }
}

/// Extract the single `#[error("...")]` literal from a variant.
fn variant_error_literal(variant: &Variant) -> Result<LitStr, Diagnostics> {
    let mut found: Option<LitStr> = None;
    for attr in &variant.attrs {
        if !attr.path().is_ident("error") {
            continue;
        }
        if found.is_some() {
            return Err(diagnostic(
                DiagnosticCode::ErrorVariantDuplicateErrorAttr,
                attr.span(),
                RequiredAction::RemoveKey {
                    key: KeyName::new("error"),
                },
                "duplicate `#[error(...)]` on this variant; exactly one is required".to_owned(),
            ));
        }
        // Ratified flag-5 mapping: a malformed `#[error(...)]` argument (not a
        // single string literal) has no dedicated Section-D code and reports as
        // the missing-error-attr condition.
        let lit = attr.parse_args::<LitStr>().map_err(|e| {
            diagnostic(
                DiagnosticCode::ErrorVariantMissingErrorAttr,
                attr.span(),
                RequiredAction::AddKey {
                    key: KeyName::new("error"),
                    form: ExpectedForm::StrLit,
                },
                e.to_string(),
            )
        })?;
        found = Some(lit);
    }
    match found {
        Some(lit) => Ok(lit),
        None => Err(diagnostic(
            DiagnosticCode::ErrorVariantMissingErrorAttr,
            variant.ident.span(),
            RequiredAction::AddKey {
                key: KeyName::new("error"),
                form: ExpectedForm::StrLit,
            },
            "every variant of a `#[derive(Error)]` enum needs an `#[error(\"...\")]` attribute; \
             if a variant's Display cannot be a single format literal, hand-write the whole type \
             instead"
                .to_owned(),
        )),
    }
}

/// Locate the `#[source]` / `#[from]` field of a variant (at most one).
fn parse_source_field(variant: &Variant) -> Result<Option<SourceBinding>, Diagnostics> {
    let mut result: Option<SourceBinding> = None;
    for (index, field) in variant.fields.iter().enumerate() {
        let is_source = field.attrs.iter().any(|a| a.path().is_ident("source"));
        let is_from = field.attrs.iter().any(|a| a.path().is_ident("from"));
        if !is_source && !is_from {
            continue;
        }
        if result.is_some() {
            return Err(diagnostic(
                DiagnosticCode::ErrorMultipleSourceFields,
                field.span(),
                RequiredAction::RemoveKey {
                    key: KeyName::new("source"),
                },
                "a variant may declare at most one `#[source]`/`#[from]` field".to_owned(),
            ));
        }
        if is_from && variant.fields.len() != 1 {
            // Section D pins ChangeShape for this rule; ShapeRule::Enum is the
            // error kind's declared item-shape law (the closest typed payload —
            // ShapeRule has no per-variant field-count vocabulary).
            return Err(diagnostic(
                DiagnosticCode::ErrorFromRequiresSingleField,
                field.span(),
                RequiredAction::ChangeShape {
                    expected: ShapeRule::Enum,
                },
                "`#[from]` requires the variant to have exactly one field".to_owned(),
            ));
        }
        let member = match &field.ident {
            Some(ident) => SourceMember::Named(ident.clone()),
            None => SourceMember::Indexed(index),
        };
        result = Some(SourceBinding {
            member,
            is_box: type_is_box(&field.ty),
            ty: field.ty.clone(),
            is_from,
        });
    }
    Ok(result)
}

/// `true` when `ty`'s outermost path segment is `Box`.
fn type_is_box(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Box")
}

/// Rewrite a `#[error]` format string so positional/implicit references
/// (`{0}`, `{}`) become named binding references (`{__arg_0}`), while named
/// references (`{field}`) pass through untouched. Records which fields/positions
/// the string references so the caller can bind exactly those (and ignore the
/// rest, avoiding unused-binding warnings). The rendered output is unchanged —
/// only the *argument reference*, never the value or its format spec.
fn rewrite_format(lit: &LitStr) -> Result<FormatSpec, Diagnostics> {
    let input = lit.value();
    let mut out = String::with_capacity(input.len());
    let mut used_names = BTreeSet::new();
    let mut used_positions = BTreeSet::new();
    let mut implicit = 0usize;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push_str("{{");
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push_str("}}");
            }
            '}' => {
                return Err(diagnostic(
                    DiagnosticCode::ErrorFmtUnmatchedBrace,
                    lit.span(),
                    RequiredAction::ReplaceValue {
                        key: KeyName::new("error"),
                        form: ExpectedForm::StrLit,
                    },
                    "unmatched `}` in `#[error]` format string".to_owned(),
                ));
            }
            '{' => {
                let (arg, spec) = read_placeholder(lit, &mut chars)?;
                if spec.contains('$') {
                    return Err(diagnostic(
                        DiagnosticCode::ErrorFmtDollarArg,
                        lit.span(),
                        RequiredAction::ReplaceValue {
                            key: KeyName::new("error"),
                            form: ExpectedForm::StrLit,
                        },
                        "width/precision `$` arguments are not supported by #[derive(Error)]; \
                         inline the value or hand-write the type"
                            .to_owned(),
                    ));
                }
                let reference = if arg.is_empty() {
                    let index = implicit;
                    implicit += 1;
                    used_positions.insert(index);
                    format!("__arg_{index}")
                } else if arg.bytes().all(|b| b.is_ascii_digit()) {
                    let index: usize = arg.parse().map_err(|_| {
                        diagnostic(
                            DiagnosticCode::ErrorFmtInvalidPosition,
                            lit.span(),
                            RequiredAction::ReplaceValue {
                                key: KeyName::new("error"),
                                form: ExpectedForm::StrLit,
                            },
                            "invalid positional index in `#[error]`".to_owned(),
                        )
                    })?;
                    used_positions.insert(index);
                    format!("__arg_{index}")
                } else {
                    used_names.insert(arg.clone());
                    arg
                };
                out.push('{');
                out.push_str(&reference);
                out.push_str(&spec);
                out.push('}');
            }
            other => out.push(other),
        }
    }

    Ok(FormatSpec {
        original: lit.clone(),
        rewritten: out,
        used_names,
        used_positions,
    })
}

/// Read one `{...}` placeholder body (the leading `{` already consumed),
/// splitting it into the argument reference and its `:spec` suffix (spec
/// includes the leading `:`). Consumes through the closing `}`.
fn read_placeholder(
    lit: &LitStr,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<(String, String), Diagnostics> {
    let mut arg = String::new();
    let mut spec = String::new();
    let mut in_spec = false;
    loop {
        match chars.next() {
            Some('}') => return Ok((arg, spec)),
            Some(':') if !in_spec => {
                in_spec = true;
                spec.push(':');
            }
            Some(other) => {
                if in_spec {
                    spec.push(other);
                } else {
                    arg.push(other);
                }
            }
            None => {
                return Err(diagnostic(
                    DiagnosticCode::ErrorFmtUnterminatedBrace,
                    lit.span(),
                    RequiredAction::ReplaceValue {
                        key: KeyName::new("error"),
                        form: ExpectedForm::StrLit,
                    },
                    "unterminated `{` in `#[error]` format string".to_owned(),
                ));
            }
        }
    }
}

/// Wrap a single registry-backed diagnostic into an accumulator; the failure
/// site supplies the concrete [`RequiredAction`] payload.
fn diagnostic(
    code: DiagnosticCode,
    span: Span,
    action: RequiredAction,
    message: String,
) -> Diagnostics {
    Diagnostics::one(MacroDiagnostic::from_spec(code, span, action, message))
}
