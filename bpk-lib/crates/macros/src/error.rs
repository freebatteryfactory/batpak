//! `#[derive(Error)]` — a thiserror-lite error derive.
//!
//! Generates `impl core::fmt::Display` from a per-variant `#[error("...")]`
//! format literal and `impl std::error::Error` (with `source()` wiring from an
//! explicit `#[source]` / `#[from]` field). It exists so the batpak family can
//! delete hand-rolled `Display` + `Error` boilerplate on error enums whose
//! rendering is a straight per-variant format string — WITHOUT changing a single
//! output byte.
//!
//! Deliberate scope (kept small on purpose — the value is safe boilerplate
//! removal, not feature parity with thiserror):
//!
//! * Enums only. A struct/union is a compile error.
//! * All-or-nothing: EVERY variant must carry exactly one `#[error("...")]`.
//!   A variant whose `Display` cannot be a single format literal (dynamic
//!   branches, loops, custom number formatting) simply keeps its type
//!   hand-written — do not derive on it.
//! * The `source` field is opt-in and EXPLICIT: annotate it `#[source]` (source
//!   only) or `#[from]` (source + `impl From`). There is no implicit
//!   "field named `source`" magic — the wiring a reader sees is the wiring that
//!   exists.
//! * `{field}`, `{field:?}`, `{0}`, `{}` and format specs (`{n:02x}`) are
//!   supported. Width/precision args via `$` are rejected (none of the family's
//!   error strings use them; rejecting keeps codegen provably faithful).
//!
//! Byte-identity contract: the generated `write!` receives the SAME string the
//! hand-rolled `write!` used (positional `{0}`/`{}` are rewritten to a binding
//! name, which changes the argument reference but never the rendered bytes), and
//! the destructured field bindings carry the same values with the same format
//! trait. `source()` returns exactly the same reference the hand-rolled impl did.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, LitStr, Type, Variant};

/// Expand `#[derive(Error)]` for `input`.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(Error)] supports enums only; hand-write `Display`/`Error` for structs",
        ));
    };

    let variants = data
        .variants
        .iter()
        .map(VariantModel::parse)
        .collect::<syn::Result<Vec<_>>>()?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let display_arms = variants
        .iter()
        .map(VariantModel::display_arm)
        .collect::<Vec<_>>();

    let has_any_source = variants.iter().any(|v| v.source.is_some());
    let error_impl = if has_any_source {
        let source_arms = variants
            .iter()
            .map(VariantModel::source_arm)
            .collect::<Vec<_>>();
        quote! {
            impl #impl_generics ::std::error::Error for #ident #ty_generics #where_clause {
                fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                    match self {
                        #(#source_arms)*
                    }
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics ::std::error::Error for #ident #ty_generics #where_clause {}
        }
    };

    let from_impls = variants
        .iter()
        .filter_map(|v| v.emit_from_impl(ident, &input.generics))
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_generics ::core::fmt::Display for #ident #ty_generics #where_clause {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#display_arms)*
                }
            }
        }

        #error_impl

        #(#from_impls)*
    })
}

/// Which field of a variant carries the error source, and how.
struct SourceField {
    /// Named-field member (`Some`) or tuple index (`None` → position).
    member: SourceMember,
    /// `true` when the field type's outermost path segment is `Box` — the
    /// source is then `Some(field.as_ref())` (the inner `dyn Error`), matching
    /// the hand-rolled `Box<dyn Error>` wiring exactly.
    is_box: bool,
    /// The field type (used for the generated `impl From`).
    ty: Type,
    /// `true` when annotated `#[from]` (also emit `impl From`).
    is_from: bool,
}

enum SourceMember {
    Named(syn::Ident),
    Indexed(usize),
}

struct VariantModel {
    ident: syn::Ident,
    fields: Fields,
    /// The rewritten format literal (positional refs → binding names) plus the
    /// original, so a no-rewrite variant re-emits its exact original token.
    format: FormatSpec,
    source: Option<SourceField>,
}

struct FormatSpec {
    original: LitStr,
    rewritten: String,
    used_names: BTreeSet<String>,
    used_positions: BTreeSet<usize>,
}

impl VariantModel {
    fn parse(variant: &Variant) -> syn::Result<Self> {
        let lit = variant_error_literal(variant)?;
        let format = rewrite_format(&lit)?;
        let source = parse_source_field(variant)?;
        Ok(Self {
            ident: variant.ident.clone(),
            fields: variant.fields.clone(),
            format,
            source,
        })
    }

    /// The `write!`-bearing `Display` match arm for this variant.
    fn display_arm(&self) -> TokenStream {
        let vident = &self.ident;
        let lit = self.format.literal();
        match &self.fields {
            Fields::Named(named) => {
                let pats = named.named.iter().map(|f| {
                    let ident = f
                        .ident
                        .as_ref()
                        .expect("named field has an identifier by construction");
                    if self.format.used_names.contains(&ident.to_string()) {
                        quote!(#ident)
                    } else {
                        quote!(#ident: _)
                    }
                });
                quote! {
                    Self::#vident { #(#pats),* } => ::core::write!(__formatter, #lit),
                }
            }
            Fields::Unnamed(unnamed) => {
                let pats = (0..unnamed.unnamed.len()).map(|i| {
                    if self.format.used_positions.contains(&i) {
                        let binding = format_ident!("__arg_{}", i);
                        quote!(#binding)
                    } else {
                        quote!(_)
                    }
                });
                quote! {
                    Self::#vident( #(#pats),* ) => ::core::write!(__formatter, #lit),
                }
            }
            Fields::Unit => quote! {
                Self::#vident => ::core::write!(__formatter, #lit),
            },
        }
    }

    /// The `source()` match arm for this variant.
    fn source_arm(&self) -> TokenStream {
        let vident = &self.ident;
        match &self.source {
            Some(source) => {
                let expr = if source.is_box {
                    quote!(__source.as_ref())
                } else {
                    quote!(__source)
                };
                match &source.member {
                    SourceMember::Named(name) => quote! {
                        Self::#vident { #name: __source, .. } =>
                            ::core::option::Option::Some(#expr),
                    },
                    SourceMember::Indexed(index) => {
                        let count = match &self.fields {
                            Fields::Unnamed(u) => u.unnamed.len(),
                            Fields::Named(_) | Fields::Unit => 0,
                        };
                        let pats = (0..count).map(|i| {
                            if i == *index {
                                quote!(__source)
                            } else {
                                quote!(_)
                            }
                        });
                        quote! {
                            Self::#vident( #(#pats),* ) =>
                                ::core::option::Option::Some(#expr),
                        }
                    }
                }
            }
            None => match &self.fields {
                Fields::Named(_) => quote! {
                    Self::#vident { .. } => ::core::option::Option::None,
                },
                Fields::Unnamed(_) => quote! {
                    Self::#vident( .. ) => ::core::option::Option::None,
                },
                Fields::Unit => quote! {
                    Self::#vident => ::core::option::Option::None,
                },
            },
        }
    }

    /// `impl From<SourceTy> for Enum`, emitted only for a `#[from]` field.
    fn emit_from_impl(
        &self,
        enum_ident: &syn::Ident,
        generics: &syn::Generics,
    ) -> Option<TokenStream> {
        let source = self.source.as_ref()?;
        if !source.is_from {
            return None;
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let vident = &self.ident;
        let ty = &source.ty;
        let construct = match &source.member {
            SourceMember::Named(name) => quote!(Self::#vident { #name: __value }),
            SourceMember::Indexed(_) => quote!(Self::#vident(__value)),
        };
        Some(quote! {
            impl #impl_generics ::core::convert::From<#ty>
                for #enum_ident #ty_generics #where_clause
            {
                fn from(__value: #ty) -> Self {
                    #construct
                }
            }
        })
    }
}

impl FormatSpec {
    /// The literal to feed `write!`: the untouched original token when no
    /// positional rewrite happened (preserves multi-line `\`-continued literals
    /// byte-for-byte), otherwise a fresh literal carrying the rewritten string.
    fn literal(&self) -> LitStr {
        if self.rewritten == self.original.value() {
            self.original.clone()
        } else {
            LitStr::new(&self.rewritten, self.original.span())
        }
    }
}

/// Extract the single `#[error("...")]` literal from a variant.
fn variant_error_literal(variant: &Variant) -> syn::Result<LitStr> {
    let mut found: Option<LitStr> = None;
    for attr in &variant.attrs {
        if !attr.path().is_ident("error") {
            continue;
        }
        if found.is_some() {
            return Err(syn::Error::new(
                attr.span(),
                "duplicate `#[error(...)]` on this variant; exactly one is required",
            ));
        }
        found = Some(attr.parse_args::<LitStr>()?);
    }
    found.ok_or_else(|| {
        syn::Error::new(
            variant.ident.span(),
            "every variant of a `#[derive(Error)]` enum needs an `#[error(\"...\")]` \
             attribute; if a variant's Display cannot be a single format literal, \
             hand-write the whole type instead",
        )
    })
}

/// Locate the `#[source]` / `#[from]` field of a variant (at most one).
fn parse_source_field(variant: &Variant) -> syn::Result<Option<SourceField>> {
    let mut result: Option<SourceField> = None;
    for (index, field) in variant.fields.iter().enumerate() {
        let is_source = field.attrs.iter().any(|a| a.path().is_ident("source"));
        let is_from = field.attrs.iter().any(|a| a.path().is_ident("from"));
        if !is_source && !is_from {
            continue;
        }
        if result.is_some() {
            return Err(syn::Error::new(
                field.span(),
                "a variant may declare at most one `#[source]`/`#[from]` field",
            ));
        }
        if is_from && variant.fields.len() != 1 {
            return Err(syn::Error::new(
                field.span(),
                "`#[from]` requires the variant to have exactly one field",
            ));
        }
        let member = match &field.ident {
            Some(ident) => SourceMember::Named(ident.clone()),
            None => SourceMember::Indexed(index),
        };
        result = Some(SourceField {
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
fn rewrite_format(lit: &LitStr) -> syn::Result<FormatSpec> {
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
                return Err(syn::Error::new(
                    lit.span(),
                    "unmatched `}` in `#[error]` format string",
                ));
            }
            '{' => {
                let (arg, spec) = read_placeholder(lit, &mut chars)?;
                if spec.contains('$') {
                    return Err(syn::Error::new(
                        lit.span(),
                        "width/precision `$` arguments are not supported by #[derive(Error)]; \
                         inline the value or hand-write the type",
                    ));
                }
                let reference = if arg.is_empty() {
                    let index = implicit;
                    implicit += 1;
                    used_positions.insert(index);
                    format!("__arg_{index}")
                } else if arg.bytes().all(|b| b.is_ascii_digit()) {
                    let index: usize = arg.parse().map_err(|_| {
                        syn::Error::new(lit.span(), "invalid positional index in `#[error]`")
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
) -> syn::Result<(String, String)> {
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
                return Err(syn::Error::new(
                    lit.span(),
                    "unterminated `{` in `#[error]` format string",
                ));
            }
        }
    }
}
