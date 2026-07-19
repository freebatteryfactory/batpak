use super::*;

impl PakVmNodeId {
    /// The authored docs/07 spelling. This is the projection surface and the
    /// audit key; it is not an encoding.
    pub const fn authored_name(self) -> &'static str {
        match self {
            PakVmNodeId::Literal => "literal",
            PakVmNodeId::Binding => "binding",
            PakVmNodeId::Field => "field",
            PakVmNodeId::Let => "let",
            PakVmNodeId::Compare => "compare",
            PakVmNodeId::All => "all",
            PakVmNodeId::Any => "any",
            PakVmNodeId::Not => "not",
            PakVmNodeId::Case => "case",
            PakVmNodeId::Allow => "allow",
            PakVmNodeId::Deny => "deny",
            PakVmNodeId::Defer => "defer",
            PakVmNodeId::Require => "require",
            PakVmNodeId::Margin => "margin",
            PakVmNodeId::Explain => "explain",
            PakVmNodeId::KernelCall => "built-in kernel call",
            PakVmNodeId::Scan => "scan",
            PakVmNodeId::Seek => "seek",
            PakVmNodeId::Select => "select",
            PakVmNodeId::Filter => "filter",
            PakVmNodeId::Project => "project",
            PakVmNodeId::Fold => "fold",
            PakVmNodeId::Group => "group",
            PakVmNodeId::Aggregate => "aggregate",
            PakVmNodeId::Join => "join",
            PakVmNodeId::SequenceMatch => "sequence match",
            PakVmNodeId::Sort => "sort",
            PakVmNodeId::Page => "page",
            PakVmNodeId::Limit => "limit",
            PakVmNodeId::MaterializeTile => "materialize tile",
            PakVmNodeId::Append => "append",
            PakVmNodeId::AppendBatch => "append batch",
            PakVmNodeId::RequestEffect => "request effect",
            PakVmNodeId::StageArtifact => "stage artifact",
            PakVmNodeId::EmitResult => "emit result",
            PakVmNodeId::AdvanceCheckpointIntent => "advance checkpoint intent",
        }
    }

    /// Authored algebra membership: docs/07 lists each node under exactly one of
    /// the three algebra headings. Total by construction — a new node cannot be
    /// added without stating which algebra authored it.
    pub const fn algebra(self) -> PakVmAlgebra {
        match self {
            PakVmNodeId::Literal
            | PakVmNodeId::Binding
            | PakVmNodeId::Field
            | PakVmNodeId::Let
            | PakVmNodeId::Compare
            | PakVmNodeId::All
            | PakVmNodeId::Any
            | PakVmNodeId::Not
            | PakVmNodeId::Case
            | PakVmNodeId::Allow
            | PakVmNodeId::Deny
            | PakVmNodeId::Defer
            | PakVmNodeId::Require
            | PakVmNodeId::Margin
            | PakVmNodeId::Explain
            | PakVmNodeId::KernelCall => PakVmAlgebra::FormulaDecision,
            PakVmNodeId::Scan
            | PakVmNodeId::Seek
            | PakVmNodeId::Select
            | PakVmNodeId::Filter
            | PakVmNodeId::Project
            | PakVmNodeId::Fold
            | PakVmNodeId::Group
            | PakVmNodeId::Aggregate
            | PakVmNodeId::Join
            | PakVmNodeId::SequenceMatch
            | PakVmNodeId::Sort
            | PakVmNodeId::Page
            | PakVmNodeId::Limit
            | PakVmNodeId::MaterializeTile => PakVmAlgebra::QueryDataflow,
            PakVmNodeId::Append
            | PakVmNodeId::AppendBatch
            | PakVmNodeId::RequestEffect
            | PakVmNodeId::StageArtifact
            | PakVmNodeId::EmitResult
            | PakVmNodeId::AdvanceCheckpointIntent => PakVmAlgebra::Effect,
        }
    }

    /// Authored producer surface. Total by construction: a new node cannot be
    /// added without stating who may lawfully author it, and deleting a node's
    /// arm refuses at compile time — "no legal producer" is unrepresentable.
    ///
    /// `Join` is the one V1 node the frozen BatQL grammar cannot express
    /// (single primary source per query; no JOIN syntax, and none is added to
    /// rescue an enum variant). It remains a public semantic node because a
    /// validated canonical ProgramImage is a real production door; it enters
    /// BatQL, if ever, through the full language-amendment law.
    pub const fn authoring_surface(self) -> NodeAuthoringSurface {
        match self {
            PakVmNodeId::Join => NodeAuthoringSurface::CanonicalProgramImageOnly,
            PakVmNodeId::Literal
            | PakVmNodeId::Binding
            | PakVmNodeId::Field
            | PakVmNodeId::Let
            | PakVmNodeId::Compare
            | PakVmNodeId::All
            | PakVmNodeId::Any
            | PakVmNodeId::Not
            | PakVmNodeId::Case
            | PakVmNodeId::Allow
            | PakVmNodeId::Deny
            | PakVmNodeId::Defer
            | PakVmNodeId::Require
            | PakVmNodeId::Margin
            | PakVmNodeId::Explain
            | PakVmNodeId::KernelCall
            | PakVmNodeId::Scan
            | PakVmNodeId::Seek
            | PakVmNodeId::Select
            | PakVmNodeId::Filter
            | PakVmNodeId::Project
            | PakVmNodeId::Fold
            | PakVmNodeId::Group
            | PakVmNodeId::Aggregate
            | PakVmNodeId::SequenceMatch
            | PakVmNodeId::Sort
            | PakVmNodeId::Page
            | PakVmNodeId::Limit
            | PakVmNodeId::MaterializeTile
            | PakVmNodeId::Append
            | PakVmNodeId::AppendBatch
            | PakVmNodeId::RequestEffect
            | PakVmNodeId::StageArtifact
            | PakVmNodeId::EmitResult
            | PakVmNodeId::AdvanceCheckpointIntent => NodeAuthoringSurface::BatQlLowered,
        }
    }

    /// Authored family membership. Total by construction.
    pub const fn class(self) -> PakVmNodeClass {
        match self {
            PakVmNodeId::Literal
            | PakVmNodeId::Binding
            | PakVmNodeId::Field
            | PakVmNodeId::Let
            | PakVmNodeId::Compare
            | PakVmNodeId::All
            | PakVmNodeId::Any
            | PakVmNodeId::Not
            | PakVmNodeId::Case
            | PakVmNodeId::Margin
            | PakVmNodeId::Explain => PakVmNodeClass::ScalarComputation,
            PakVmNodeId::Allow | PakVmNodeId::Deny | PakVmNodeId::Defer => {
                PakVmNodeClass::DecisionOutcome
            }
            PakVmNodeId::Require => PakVmNodeClass::DecisionGuard,
            PakVmNodeId::KernelCall => PakVmNodeClass::KernelInvocation,
            PakVmNodeId::Scan | PakVmNodeId::Seek => PakVmNodeClass::SourceTraversal,
            PakVmNodeId::Select | PakVmNodeId::Filter | PakVmNodeId::Project => {
                PakVmNodeClass::RowTransform
            }
            PakVmNodeId::Fold | PakVmNodeId::Aggregate => PakVmNodeClass::RowReduction,
            PakVmNodeId::Group => PakVmNodeClass::Grouping,
            PakVmNodeId::Join => PakVmNodeClass::Joining,
            PakVmNodeId::SequenceMatch => PakVmNodeClass::SequenceMatching,
            PakVmNodeId::Sort => PakVmNodeClass::Ordering,
            PakVmNodeId::Page | PakVmNodeId::Limit => PakVmNodeClass::Windowing,
            PakVmNodeId::MaterializeTile => PakVmNodeClass::TileMaterialization,
            PakVmNodeId::Append | PakVmNodeId::AppendBatch => PakVmNodeClass::DurableAppend,
            PakVmNodeId::RequestEffect => PakVmNodeClass::PortEffectRequest,
            PakVmNodeId::StageArtifact => PakVmNodeClass::ArtifactStaging,
            PakVmNodeId::EmitResult => PakVmNodeClass::ResultEmission,
            PakVmNodeId::AdvanceCheckpointIntent => PakVmNodeClass::CheckpointIntent,
        }
    }

    /// Operand sorts, in the closed value algebra docs/07 names. Genuinely
    /// node-specific: a class fixes a cost law, not a signature.
    pub const fn operand_sorts(self) -> &'static str {
        match self {
            PakVmNodeId::Literal => "none; a constant-pool reference",
            PakVmNodeId::Binding => "none; a validated frame slot reference",
            PakVmNodeId::Field => "record or enum value, plus a declared field reference",
            PakVmNodeId::Let => "value, plus a frame slot to bind",
            PakVmNodeId::Compare => "two values of one comparable sort",
            PakVmNodeId::All | PakVmNodeId::Any => "bounded list of Truth",
            PakVmNodeId::Not => "Truth",
            PakVmNodeId::Case => "scrutinee value, plus bounded declared arms",
            PakVmNodeId::Allow | PakVmNodeId::Deny => "none; a decision terminal",
            PakVmNodeId::Defer => "declared deferral reason",
            PakVmNodeId::Require => "Truth, plus a declared refusal reason",
            PakVmNodeId::Margin => "two comparable values",
            PakVmNodeId::Explain => "origin reference, plus explanation-dictionary reference",
            PakVmNodeId::KernelCall => "declared by the KernelInterfaceHash",
            PakVmNodeId::Scan => "declared source reference, selector, and bounds",
            PakVmNodeId::Seek => "declared source reference, key, and bounds",
            PakVmNodeId::Select | PakVmNodeId::Project => "row stream, plus declared field selection",
            PakVmNodeId::Filter => "row stream, plus a Truth-valued predicate",
            PakVmNodeId::Fold => "row stream, seed value, and a declared step",
            PakVmNodeId::Group => "row stream, plus a declared grouping key",
            PakVmNodeId::Aggregate => "row or group stream, plus a declared aggregation",
            PakVmNodeId::Join => "two row streams, plus a declared join key",
            PakVmNodeId::SequenceMatch => "row stream, plus a declared bounded pattern",
            PakVmNodeId::Sort => "row stream, plus a declared total sort key",
            PakVmNodeId::Page => "row stream, plus a declared page limit and cursor",
            PakVmNodeId::Limit => "row stream, plus a declared limit",
            PakVmNodeId::MaterializeTile => "row stream, plus a declared typed layout",
            PakVmNodeId::Append => "declared event value",
            PakVmNodeId::AppendBatch => "bounded list of declared event values",
            PakVmNodeId::RequestEffect => "declared PortRequest value",
            PakVmNodeId::StageArtifact => "bounded artifact content",
            PakVmNodeId::EmitResult => "semantic result value, plus its orthogonal axes",
            PakVmNodeId::AdvanceCheckpointIntent => "declared checkpoint position",
        }
    }

    /// Result sorts, in the closed value algebra. No generic host object or `Any`
    /// value crosses the boundary (docs/07).
    pub const fn result_sorts(self) -> &'static str {
        match self {
            PakVmNodeId::Literal | PakVmNodeId::Binding | PakVmNodeId::Field => "one value",
            PakVmNodeId::Let => "unit; the frame slot is bound",
            PakVmNodeId::Compare => "Truth with TypedMargin",
            PakVmNodeId::All | PakVmNodeId::Any | PakVmNodeId::Not => "Truth",
            PakVmNodeId::Case => "the selected arm's value",
            PakVmNodeId::Allow | PakVmNodeId::Deny | PakVmNodeId::Defer => "Decision",
            PakVmNodeId::Require => "Truth, or a typed refusal",
            PakVmNodeId::Margin => "TypedMargin",
            PakVmNodeId::Explain => "explanation reference",
            PakVmNodeId::KernelCall => "declared by the KernelInterfaceHash",
            PakVmNodeId::Scan | PakVmNodeId::Seek => "row stream, plus a source stamp",
            PakVmNodeId::Select
            | PakVmNodeId::Filter
            | PakVmNodeId::Project
            | PakVmNodeId::Join
            | PakVmNodeId::Sort
            | PakVmNodeId::Limit => "row stream",
            PakVmNodeId::Fold => "one accumulated value",
            PakVmNodeId::Group => "group stream",
            PakVmNodeId::Aggregate => "one aggregated value per group",
            PakVmNodeId::SequenceMatch => "match stream",
            PakVmNodeId::Page => "page: ordered entries, completeness, next cursor, source stamp, WorkObservation, proof result",
            PakVmNodeId::MaterializeTile => "tile reference",
            PakVmNodeId::Append | PakVmNodeId::AppendBatch => "durable append outcome",
            PakVmNodeId::RequestEffect => "suspension; resumed under the same AttemptId",
            PakVmNodeId::StageArtifact => "ArtifactRef, only on completion",
            PakVmNodeId::EmitResult => "ExecutedResult",
            PakVmNodeId::AdvanceCheckpointIntent => "checkpoint-advance outcome",
        }
    }

    /// The authoring document clause this node's identity and semantics come
    /// from. Lineage sufficient for an auditor to check the node against its
    /// source rather than against this file.
    pub const fn source_origin(self) -> &'static str {
        match self.algebra() {
            PakVmAlgebra::FormulaDecision => "docs/07 Three instruction algebras: Formula and decision",
            PakVmAlgebra::QueryDataflow => "docs/07 Three instruction algebras: Query and dataflow",
            PakVmAlgebra::Effect => "docs/07 Three instruction algebras: Effects",
        }
    }
}

/// The authored node inventory, in docs/07 reading order. Order is provenance,
/// not encoding.
pub const PAKVM_NODES: &[PakVmNodeId] = &[
    PakVmNodeId::Literal,
    PakVmNodeId::Binding,
    PakVmNodeId::Field,
    PakVmNodeId::Let,
    PakVmNodeId::Compare,
    PakVmNodeId::All,
    PakVmNodeId::Any,
    PakVmNodeId::Not,
    PakVmNodeId::Case,
    PakVmNodeId::Allow,
    PakVmNodeId::Deny,
    PakVmNodeId::Defer,
    PakVmNodeId::Require,
    PakVmNodeId::Margin,
    PakVmNodeId::Explain,
    PakVmNodeId::KernelCall,
    PakVmNodeId::Scan,
    PakVmNodeId::Seek,
    PakVmNodeId::Select,
    PakVmNodeId::Filter,
    PakVmNodeId::Project,
    PakVmNodeId::Fold,
    PakVmNodeId::Group,
    PakVmNodeId::Aggregate,
    PakVmNodeId::Join,
    PakVmNodeId::SequenceMatch,
    PakVmNodeId::Sort,
    PakVmNodeId::Page,
    PakVmNodeId::Limit,
    PakVmNodeId::MaterializeTile,
    PakVmNodeId::Append,
    PakVmNodeId::AppendBatch,
    PakVmNodeId::RequestEffect,
    PakVmNodeId::StageArtifact,
    PakVmNodeId::EmitResult,
    PakVmNodeId::AdvanceCheckpointIntent,
];

/// One admitted PakVM semantic node.
///
/// Every field is supplied by exactly one owner: a value authored per node, a
/// value declared once as algebra policy, a value declared once as class policy,
/// or one named lawful derivation. `bootstrap/project.py` serializes an admitted
/// spec into the docs/07 inventory projection; it may not complete one, and
/// neither may `bootstrap/audit.py`, which independently re-derives every field
/// and compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PakVmNodeSpec {
    pub id: PakVmNodeId,
    pub authored_name: &'static str,
    pub algebra: PakVmAlgebra,
    pub class: PakVmNodeClass,
    pub effect: EffectPosture,
    pub operand_sorts: &'static str,
    pub result_sorts: &'static str,
    pub boundedness: BoundednessPosture,
    pub capability: CapabilityRequirement,
    pub work_formula: WorkFormulaFamily,
    pub evidence: EvidenceClass,
    /// Positive proof of admission. Not a per-node value: every admitted node is
    /// a public semantic node, and nothing else can become one.
    pub lowering: AdmittedAsPublicSemanticNode,
    pub source_origin: &'static str,
}

/// The outcome of admitting one node. `Refused` carries why: an unresolved
/// field, two owners for one field, or a posture the ISA cannot hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PakVmAdmission {
    Admitted(PakVmNodeSpec),
    Refused(&'static str),
}
