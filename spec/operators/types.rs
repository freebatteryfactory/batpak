use crate::guarantees::{ContractId, DecisionId};

/// The closed typed operator identity. `ALL` is canonical order; the
/// `OPERATORS` rows must equal it exactly and in order, and every generated
/// projection derives its order from it — there is no second ranking table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorId {
    Multiply,
    Divide,
    Add,
    Subtract,
    Equal,
    NotEqual,
    LessThan,
    AtMost,
    MoreThan,
    AtLeast,
    Not,
    And,
    Or,
}

impl OperatorId {
    pub const ALL: &'static [OperatorId] = &[
        OperatorId::Multiply,
        OperatorId::Divide,
        OperatorId::Add,
        OperatorId::Subtract,
        OperatorId::Equal,
        OperatorId::NotEqual,
        OperatorId::LessThan,
        OperatorId::AtMost,
        OperatorId::MoreThan,
        OperatorId::AtLeast,
        OperatorId::Not,
        OperatorId::And,
        OperatorId::Or,
    ];

    pub const fn spelling(self) -> &'static str {
        match self {
            OperatorId::Multiply => "OP-MUL",
            OperatorId::Divide => "OP-DIV",
            OperatorId::Add => "OP-ADD",
            OperatorId::Subtract => "OP-SUB",
            OperatorId::Equal => "OP-EQ",
            OperatorId::NotEqual => "OP-NE",
            OperatorId::LessThan => "OP-LT",
            OperatorId::AtMost => "OP-LE",
            OperatorId::MoreThan => "OP-GT",
            OperatorId::AtLeast => "OP-GE",
            OperatorId::Not => "OP-NOT",
            OperatorId::And => "OP-AND",
            OperatorId::Or => "OP-OR",
        }
    }

    /// The BatQL companion owns operator identity semantics.
    pub const fn semantic_owner(self) -> ContractId {
        match self {
            OperatorId::Multiply => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Divide => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Add => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Subtract => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Equal => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::NotEqual => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::LessThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::AtMost => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::MoreThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Not => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::And => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorId::Or => ContractId("BP-BATQL-LANGUAGE-1"),
        }
    }

    /// The standing decision whose forward-policy fields name the typed
    /// operator identity, both surface inventories, the syntax shapes, and
    /// the exact V1 word/symbol mapping.
    pub const fn admission_basis(self) -> DecisionId {
        match self {
            OperatorId::Multiply => DecisionId("DEC-060"),
            OperatorId::Divide => DecisionId("DEC-060"),
            OperatorId::Add => DecisionId("DEC-060"),
            OperatorId::Subtract => DecisionId("DEC-060"),
            OperatorId::Equal => DecisionId("DEC-060"),
            OperatorId::NotEqual => DecisionId("DEC-060"),
            OperatorId::LessThan => DecisionId("DEC-060"),
            OperatorId::AtMost => DecisionId("DEC-060"),
            OperatorId::MoreThan => DecisionId("DEC-060"),
            OperatorId::AtLeast => DecisionId("DEC-060"),
            OperatorId::Not => DecisionId("DEC-060"),
            OperatorId::And => DecisionId("DEC-060"),
            OperatorId::Or => DecisionId("DEC-060"),
        }
    }
}

/// The closed word-surface inventory. Every token obeys the canonical
/// uppercase word grammar `[A-Z]+( [A-Z]+)*`; punctuation can never enter
/// this inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorWordSurface {
    Is,
    IsNot,
    IsLessThan,
    IsAtMost,
    IsMoreThan,
    IsAtLeast,
    Not,
    And,
    Or,
}

impl OperatorWordSurface {
    pub const ALL: &'static [OperatorWordSurface] = &[
        OperatorWordSurface::Is,
        OperatorWordSurface::IsNot,
        OperatorWordSurface::IsLessThan,
        OperatorWordSurface::IsAtMost,
        OperatorWordSurface::IsMoreThan,
        OperatorWordSurface::IsAtLeast,
        OperatorWordSurface::Not,
        OperatorWordSurface::And,
        OperatorWordSurface::Or,
    ];

    pub const fn token(self) -> &'static str {
        match self {
            OperatorWordSurface::Is => "IS",
            OperatorWordSurface::IsNot => "IS NOT",
            OperatorWordSurface::IsLessThan => "IS LESS THAN",
            OperatorWordSurface::IsAtMost => "IS AT MOST",
            OperatorWordSurface::IsMoreThan => "IS MORE THAN",
            OperatorWordSurface::IsAtLeast => "IS AT LEAST",
            OperatorWordSurface::Not => "NOT",
            OperatorWordSurface::And => "AND",
            OperatorWordSurface::Or => "OR",
        }
    }

    pub const fn semantic_owner(self) -> ContractId {
        match self {
            OperatorWordSurface::Is => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::IsNot => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::IsLessThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::IsAtMost => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::IsMoreThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::IsAtLeast => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::Not => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::And => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorWordSurface::Or => ContractId("BP-BATQL-LANGUAGE-1"),
        }
    }

    pub const fn admission_basis(self) -> DecisionId {
        match self {
            OperatorWordSurface::Is => DecisionId("DEC-060"),
            OperatorWordSurface::IsNot => DecisionId("DEC-060"),
            OperatorWordSurface::IsLessThan => DecisionId("DEC-060"),
            OperatorWordSurface::IsAtMost => DecisionId("DEC-060"),
            OperatorWordSurface::IsMoreThan => DecisionId("DEC-060"),
            OperatorWordSurface::IsAtLeast => DecisionId("DEC-060"),
            OperatorWordSurface::Not => DecisionId("DEC-060"),
            OperatorWordSurface::And => DecisionId("DEC-060"),
            OperatorWordSurface::Or => DecisionId("DEC-060"),
        }
    }
}

/// The closed symbol-surface inventory. Every token is nonempty ASCII
/// punctuation; a word can never enter this inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorSymbolSurface {
    Multiply,
    Divide,
    Add,
    Subtract,
    Equal,
    NotEqual,
    LessThan,
    AtMost,
    MoreThan,
    AtLeast,
}

impl OperatorSymbolSurface {
    pub const ALL: &'static [OperatorSymbolSurface] = &[
        OperatorSymbolSurface::Multiply,
        OperatorSymbolSurface::Divide,
        OperatorSymbolSurface::Add,
        OperatorSymbolSurface::Subtract,
        OperatorSymbolSurface::Equal,
        OperatorSymbolSurface::NotEqual,
        OperatorSymbolSurface::LessThan,
        OperatorSymbolSurface::AtMost,
        OperatorSymbolSurface::MoreThan,
        OperatorSymbolSurface::AtLeast,
    ];

    pub const fn token(self) -> &'static str {
        match self {
            OperatorSymbolSurface::Multiply => "*",
            OperatorSymbolSurface::Divide => "/",
            OperatorSymbolSurface::Add => "+",
            OperatorSymbolSurface::Subtract => "-",
            OperatorSymbolSurface::Equal => "=",
            OperatorSymbolSurface::NotEqual => "!=",
            OperatorSymbolSurface::LessThan => "<",
            OperatorSymbolSurface::AtMost => "<=",
            OperatorSymbolSurface::MoreThan => ">",
            OperatorSymbolSurface::AtLeast => ">=",
        }
    }

    pub const fn semantic_owner(self) -> ContractId {
        match self {
            OperatorSymbolSurface::Multiply => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::Divide => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::Add => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::Subtract => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::Equal => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::NotEqual => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::LessThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::AtMost => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::MoreThan => ContractId("BP-BATQL-LANGUAGE-1"),
            OperatorSymbolSurface::AtLeast => ContractId("BP-BATQL-LANGUAGE-1"),
        }
    }

    pub const fn admission_basis(self) -> DecisionId {
        match self {
            OperatorSymbolSurface::Multiply => DecisionId("DEC-060"),
            OperatorSymbolSurface::Divide => DecisionId("DEC-060"),
            OperatorSymbolSurface::Add => DecisionId("DEC-060"),
            OperatorSymbolSurface::Subtract => DecisionId("DEC-060"),
            OperatorSymbolSurface::Equal => DecisionId("DEC-060"),
            OperatorSymbolSurface::NotEqual => DecisionId("DEC-060"),
            OperatorSymbolSurface::LessThan => DecisionId("DEC-060"),
            OperatorSymbolSurface::AtMost => DecisionId("DEC-060"),
            OperatorSymbolSurface::MoreThan => DecisionId("DEC-060"),
            OperatorSymbolSurface::AtLeast => DecisionId("DEC-060"),
        }
    }
}

/// The closed source-syntax shapes. This sum type replaces the retired
/// `word_surface`/`symbol_surface` string pair and its empty-string
/// "no symbol" sentinel: a shape that does not exist cannot be spelled.
/// The class law is total — Arithmetic is `SymbolOnly`, Comparison is
/// `WordWithSymbolAlias`, Logical is `WordOnly`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorSyntax {
    /// The canonical source spelling is the symbol; no word form exists.
    SymbolOnly(OperatorSymbolSurface),
    /// The canonical source spelling is the word; no symbol form exists.
    WordOnly(OperatorWordSurface),
    /// The canonical source spelling is the word; the symbol is an accepted
    /// exact compact alias that lowers to the same `OperatorId`.
    WordWithSymbolAlias(OperatorWordSurface, OperatorSymbolSurface),
}

impl OperatorSyntax {
    /// What the default formatter always emits. There is no separate
    /// `formatting` fact: it is fully derived from the canonical syntax.
    pub const fn canonical_token(self) -> &'static str {
        match self {
            OperatorSyntax::SymbolOnly(symbol) => symbol.token(),
            OperatorSyntax::WordOnly(word) => word.token(),
            OperatorSyntax::WordWithSymbolAlias(word, _) => word.token(),
        }
    }

    pub const fn canonical_word(self) -> Option<OperatorWordSurface> {
        match self {
            OperatorSyntax::SymbolOnly(_) => None,
            OperatorSyntax::WordOnly(word) => Some(word),
            OperatorSyntax::WordWithSymbolAlias(word, _) => Some(word),
        }
    }

    pub const fn canonical_symbol(self) -> Option<OperatorSymbolSurface> {
        match self {
            OperatorSyntax::SymbolOnly(symbol) => Some(symbol),
            OperatorSyntax::WordOnly(_) => None,
            OperatorSyntax::WordWithSymbolAlias(_, _) => None,
        }
    }

    pub const fn symbol_alias(self) -> Option<OperatorSymbolSurface> {
        match self {
            OperatorSyntax::SymbolOnly(_) => None,
            OperatorSyntax::WordOnly(_) => None,
            OperatorSyntax::WordWithSymbolAlias(_, symbol) => Some(symbol),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorClass {
    Arithmetic,
    Comparison,
    Logical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arity {
    Unary,
    Binary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixity {
    Prefix,
    Infix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Associativity {
    Left,
    Right,
    NonAssociative,
}

/// V1 numeric authority is exact. `NotApplicable` marks operators that do not
/// carry an exact/approximate distinction (the truth operators). No
/// `Approximate` variant exists: approximate numeric values exist as typed
/// values (DEC-069, docs/37), but a variant here would admit them into
/// ordinary operators, and that admission belongs only to a future qualified
/// operator profile with a language-change record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exactness {
    Exact,
    NotApplicable,
}

/// Per-operator numeric authority-support posture. Exact arithmetic is always
/// supported (DEC-060). Approximate numeric values already exist as typed
/// values under DEC-069 and docs/37; what does NOT exist is ordinary-operator
/// admission: no current ordinary OperatorSpec admits raw approximate
/// operands, and no current row uses `QualifiedProfileOnly`. A future
/// qualified operator profile requires explicit admission, sound error or
/// interval propagation, and a language-change record. The value vocabulary
/// exists; the operator admission does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericSupport {
    /// Exact operands admitted directly; approximate operands must Quantize first
    /// or use a qualified profile.
    ExactSupported,
    /// Admitted only inside a qualified numeric profile.
    QualifiedProfileOnly,
    /// Not admitted for numeric operands.
    Unsupported,
    /// Not an arithmetic/numeric operator (logical truth operators).
    NotApplicable,
}

/// One legal operand→result shape for an operator (5.5E1 ruling). The
/// vocabulary is closed: a combination not derivable from an operator's rules
/// is a compile-time type error, and the companion's §5.2a legality matrix is
/// a generated projection of these rules — a hand-authored exception hiding in
/// prose is exactly how `DecimalMoney` and `SignedDuration` were born, two
/// phantom sorts no authority owned, deleted by this ruling.
///
/// An operator's rules are matched in DECLARATION ORDER and the first rule
/// whose operand shape matches decides: `PercentDifference` precedes
/// `SameUnit` on subtraction, so `Percent - Percent` yields
/// `PercentagePoints`, never a bare same-unit `Percent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorTypingRule {
    /// Exact same-unit pair → the same unit (`Money<USD> + Money<USD>`;
    /// cross-unit operands are a type error).
    SameUnit,
    /// Exact dimensional × exact dimensionless → the dimensional operand's
    /// sort (`Money<USD> * Percent → Money<USD>`), exact by scale increase
    /// where representable; scale reduction requires an explicit named
    /// rounding mode (DEC-060). There is no `DecimalMoney` result sort.
    DimensionalByDimensionless,
    /// Exact like-dimension pair → `Ratio` (`Money<USD> / Money<USD>`,
    /// `Duration / Duration`; unlike dimensions are a type error).
    LikeDimensionRatio,
    /// `Percent - Percent → PercentagePoints`: the difference of two rates is
    /// a point difference, not a rate.
    PercentDifference,
    /// `Percent ± PercentagePoints → Percent` (addition in either order): a
    /// point offset applied to a rate yields a rate.
    PercentAdjustment,
    /// `ObservedWallTime - ObservedWallTime → TimeDelta`: a signed diagnostic
    /// difference between two physical observations. A negative value is
    /// lawful evidence of wall-clock regression. `TimeDelta` is never
    /// `Duration`, `MonotonicDeadline`, HLC, journal progress, stream
    /// position, causal authority, commit identity, cursor authority, or
    /// durability evidence, satisfies no budget or timeout law, and converts
    /// only through an explicitly named conversion (docs/16, DEC-061).
    WallObservationDifference,
    /// Exact same-sort pair → `Truth` with `TypedMargin` (cross-sort
    /// comparison is a type error).
    SameSortComparison,
    /// `Truth → Truth`, K3 total.
    TruthUnary,
    /// `Truth × Truth → Truth`, K3 total.
    TruthBinary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperatorSpec {
    pub id: OperatorId,
    pub class: OperatorClass,
    /// The typed source-surface shape. Canonical spelling, optional alias,
    /// and formatter output are all derived from this one value.
    pub syntax: OperatorSyntax,
    pub semantic_op: &'static str,
    pub arity: Arity,
    pub fixity: Fixity,
    /// Higher binds tighter.
    pub precedence: u8,
    pub associativity: Associativity,
    /// The closed legality rules for this operator, in match order. These are
    /// the operand→result AUTHORITY; `input_sorts`/`result_sort` remain the
    /// spoken/display description of the same law and may not disagree.
    pub typing: &'static [OperatorTypingRule],
    pub input_sorts: &'static str,
    pub result_sort: &'static str,
    pub exactness: Exactness,
    pub overflow: &'static str,
    pub exception: &'static str,
    /// Narration projection only — never accepted source syntax.
    pub spoken: &'static str,
    pub mutation_classes: &'static str,
    pub numeric_support: NumericSupport,
}
