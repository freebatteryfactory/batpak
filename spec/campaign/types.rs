use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::bootstrap_qualification::{HexParseError, Sha256Digest};
use crate::sprouting::{CandidateChangeClass, CandidateOriginKind, RealizationPosture};

// ===========================================================================
// Content-addressed campaign primitives (F5, DEC-079)
// ===========================================================================

/// Mints the campaign's content-addressed newtypes. Each wraps the ONE
/// bootstrap digest primitive (`Sha256Digest`, `spec/bootstrap_qualification/`)
/// rather than minting a second digest type, and each stays a DISTINCT type:
/// a candidate identity, an evidence address, a receipt address, and a reuse
/// key answer different questions and never typecheck for one another.
macro_rules! content_addressed {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(Sha256Digest);

        impl $name {
            /// Wrap the 32-byte content address an independent hasher computed.
            pub const fn from_digest(digest: Sha256Digest) -> Self {
                $name(digest)
            }
            /// Parse exactly 64 lowercase hex characters; anything else is
            /// refused with the exact `HexParseError` arm.
            pub fn from_hex(s: &str) -> Result<Self, HexParseError> {
                Ok($name(Sha256Digest::from_hex(s)?))
            }
            /// The canonical 64-character lowercase spelling the manifest
            /// grammar renders.
            pub fn render(&self) -> String {
                self.0.render()
            }
        }
    };
}

content_addressed! {
    /// The candidate lineage identity (DEC-079, docs/39 §3): one runtime-minted,
    /// content-addressed identity per sprouted candidate. This is the
    /// `IdentityKind::Candidate` OBJECT, owned by BP-SPROUTING-1.
    ///
    /// It is ORTHOGONAL and never a generalization: it does not replace the
    /// kernel identity quartet (`KernelContractId`, `KernelImplementationId`,
    /// `QualifiedKernelId`, `KernelQualificationReceiptId`), `ContentDigest`,
    /// `ProgramImageId`, or a git commit — a specialized kernel candidate
    /// carries BOTH a `CandidateId` and a `KernelImplementationId`. The
    /// candidate's CONTENT is bound independently by the record's content
    /// commitment; identity, content, and parents are three distinct facts
    /// carried by three distinct mechanisms, never one collapsed axis.
    CandidateId
}

content_addressed! {
    /// The content address of ONE generator or pilot evidence artifact in the
    /// append-oriented evidence root. Runtime evidence is append-only: a later
    /// observation appends or references new evidence and never edits prior
    /// law, history, or proof disposition (DEC-078). The referenced artifact
    /// binds the exact frozen judge digest it was produced under; this
    /// reference is only its address.
    EvidenceRef
}

content_addressed! {
    /// The content address of ONE campaign receipt — a qualification, holdout,
    /// promotion, refusal, or invalidation receipt in the evidence root. A
    /// receipt is evidence of a decision already taken; referencing one
    /// confers nothing the receipt itself does not prove.
    ReceiptRef
}

content_addressed! {
    /// The transfer-reuse key (DEC-081): the key under which a candidate
    /// unit's evidence may be reused by a later campaign step. Reuse is keyed
    /// and REQUALIFIED against upstream churn — a reuse key that no longer
    /// matches the current upstream frontier licenses nothing, so a reused
    /// candidate unit is never blessed by stale evidence.
    ReuseKey
}

/// One dependency the candidate was built against, bound to the EXACT content
/// commitment of that upstream candidate at build time (DEC-079/DEC-082).
/// After a trusted-frontier promotion the loop recomputes transitive
/// staleness: a descendant bound to an OLD dependency commitment is
/// invalidated, unaffected records are preserved, and the workspace is
/// rematerialized — stale whole-tree bytes earn no squatters' rights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DependencyCommitment {
    /// The upstream candidate this candidate depends on.
    pub dependency: CandidateId,
    /// The upstream content commitment the dependent was built against.
    pub commitment: Sha256Digest,
}

// ===========================================================================
// Campaign terminals (DEC-080): every outcome stays in the denominator
// ===========================================================================

/// The closed vocabulary of campaign terminal dispositions. A terminal is a
/// RECEIPTED event, never a mutable lifecycle state: the nursery record is
/// immutable and the terminal arrives as a receipt reference beside it.
///
/// `ArchitectRequired` is the DEC-080 terminal disposition home: when the
/// judge itself is wrong — a drifted independent model, a mis-specified
/// fixture, a stale judge-root view — the campaign STOPS as ArchitectRequired;
/// the judge or proof-policy amendment proceeds SEPARATELY under DEC-074,
/// affected evidence becomes stale, and the campaign resumes from the new
/// frozen judge snapshot. There is no ordinary repair lane by which a
/// candidate modifies its own courtroom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignTerminal {
    /// The candidate satisfied the complete conjunctive promotion denominator
    /// and was promoted; the promotion receipt is the evidence.
    Promoted,
    /// The candidate was refused against the denominator; the refusal receipt
    /// names the failed requirement.
    Refused,
    /// The candidate's evidence was invalidated — by judge mutation or by an
    /// upstream dependency-commitment change — and the invalidation receipt
    /// records why. Distinct from `Refused`: nothing here judged the
    /// candidate's own content.
    Invalidated,
    /// The campaign stopped because the outcome requires the architect: a
    /// law-changing candidate, a wrong judge, a mis-specified fixture, or a
    /// proof-policy conflict. The campaign does not continue past this
    /// terminal on its own authority.
    ArchitectRequired,
}

impl CampaignTerminal {
    /// The frozen inventory of campaign terminals, in canonical order.
    pub const ALL: &'static [CampaignTerminal] = &[
        CampaignTerminal::Promoted,
        CampaignTerminal::Refused,
        CampaignTerminal::Invalidated,
        CampaignTerminal::ArchitectRequired,
    ];

    /// The documentary spelling: the variant name is the spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            CampaignTerminal::Promoted => "Promoted",
            CampaignTerminal::Refused => "Refused",
            CampaignTerminal::Invalidated => "Invalidated",
            CampaignTerminal::ArchitectRequired => "ArchitectRequired",
        }
    }

    /// The denominator-home law (DEC-080): NO terminal ever leaves the
    /// campaign denominator. A promoted candidate is counted as promoted, a
    /// refused one as refused, an invalidated one as invalidated — and an
    /// escalated campaign is accounted for as escalated, not quietly absent:
    /// `ArchitectRequired` stops the campaign and STAYS VISIBLE in the
    /// denominator. The match is exhaustive so a new terminal must decide its
    /// denominator posture explicitly, and every arm answers `true` because
    /// the law admits no silent drop.
    pub const fn remains_in_denominator(self) -> bool {
        match self {
            CampaignTerminal::Promoted => true,
            CampaignTerminal::Refused => true,
            CampaignTerminal::Invalidated => true,
            CampaignTerminal::ArchitectRequired => true,
        }
    }
}

/// A receipted terminal: WHICH terminal disposition, bound to the receipt that
/// proves it. A terminal without its receipt is an assertion, not a fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignTerminalReceipt {
    /// The terminal disposition the receipt records.
    pub terminal: CampaignTerminal,
    /// The promotion, refusal, or invalidation receipt; for
    /// `ArchitectRequired`, the escalation receipt that stopped the campaign.
    pub receipt: ReceiptRef,
}

// ===========================================================================
// The immutable lineage record (DEC-079)
// ===========================================================================

/// One immutable, content-addressed nursery lineage record (DEC-079). The
/// nursery persists RECORDS; full speculative workspaces are disposable
/// rematerializations, never persisted as authority.
///
/// Lineage is carried by these REQUIRED fields and every lineage RELATION is
/// DERIVED from them — a parent edge is the parent-`CandidateId` field, a
/// dependency edge is a dependency commitment, a reuse edge is the reuse key,
/// an invalidation edge is the invalidation receipt. No generic
/// candidate-evidence-relation vocabulary exists beside them.
///
/// Immutability is a NURSERY STORAGE law, not a Rust mutability property: a
/// persisted record is never rewritten, and a repair is a NEW record whose
/// origin is `RepairOfCandidate` and whose parent field names the failed
/// candidate — the parent record stays intact. Origin records HOW the
/// candidate was produced (truthfully, including machine-assisted synthesis);
/// it confers NO authority, and whether the candidate is admitted is decided
/// against the conjunctive promotion denominator, never inferred from origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateRecord {
    /// The candidate lineage identity.
    pub id: CandidateId,
    /// Explicit parent candidate identities. Empty for a first-generation
    /// candidate; a repair names its failed parent here.
    pub parents: Vec<CandidateId>,
    /// The commitment to the exact trusted source frontier the candidate was
    /// generated against (DEC-082): the frontier the campaign had qualified
    /// when this candidate was minted.
    pub source_frontier_commitment: Sha256Digest,
    /// The exact upstream commitments the candidate was built against; the
    /// staleness recomputation reads these after every frontier promotion.
    pub dependency_commitments: Vec<DependencyCommitment>,
    /// The commitment to this candidate's own patch or content.
    pub content_commitment: Sha256Digest,
    /// HOW the candidate was produced — truthful provenance, zero authority.
    pub origin: CandidateOriginKind,
    /// Whether the candidate preserves the realized law or changes it; a
    /// law-changing candidate routes to the architect (DEC-080).
    pub change_class: CandidateChangeClass,
    /// Missing, scaffold, or candidate: a compiling stub is a `Scaffold`,
    /// counted in the UNREALIZED denominator, and a realized graph can never
    /// go green over a tree of stubs (DEC-079).
    pub realization_posture: RealizationPosture,
    /// The generator or pilot evidence produced when the candidate was minted.
    pub generator_or_pilot_evidence: Vec<EvidenceRef>,
    /// The qualification receipts the candidate has earned.
    pub qualification_receipts: Vec<ReceiptRef>,
    /// The holdout receipts: holdout evidence for a campaign cannot reuse its
    /// search inputs (DEC-081), so these are earned on the disjoint set.
    pub holdout_receipts: Vec<ReceiptRef>,
    /// The transfer-reuse key this unit may later be reused under.
    pub reuse_key: ReuseKey,
    /// The receipted terminal, once one exists. `None` means the candidate is
    /// still in flight — NOT that it left the denominator; every terminal
    /// that ever arrives remains in the denominator.
    pub terminal: Option<CampaignTerminalReceipt>,
}

// ===========================================================================
// The versioned candidate manifest (DEC-079: version it NOW)
// ===========================================================================

/// The persisted, line-oriented serialized form of a `CandidateRecord`. A
/// persisted manifest gets its own distinct manifest-version identity and
/// canonical grammar FROM DAY ONE — no unversioned "temporary" format ever
/// exists. The version identity is `VersionIdentityKind::CandidateManifest`
/// (`CandidateManifestVersion`, `spec/identities/`), and the first line of
/// every manifest names its own version.
///
/// The canonical grammar is `key value` lines in EXACTLY this order, with an
/// explicit count line before every repeated section and an explicit
/// `terminal none` when no terminal exists — an empty set is stated, never
/// omitted:
///
/// ```text
/// BATPAK-CANDIDATE-MANIFEST/1
/// candidate-id <64 lowercase hex>
/// parent-count <n>
/// parent <64 hex>                                   (n lines)
/// source-frontier-commitment <64 hex>
/// dependency-count <n>
/// dependency <candidate 64 hex> <commitment 64 hex> (n lines)
/// content-commitment <64 hex>
/// origin <CandidateOriginKind variant name>
/// change-class <CandidateChangeClass variant name>
/// realization-posture <RealizationPosture variant name>
/// evidence-count <n>
/// evidence <64 hex>                                 (n lines)
/// qualification-receipt-count <n>
/// qualification-receipt <64 hex>                    (n lines)
/// holdout-receipt-count <n>
/// holdout-receipt <64 hex>                          (n lines)
/// reuse-key <64 hex>
/// terminal none | terminal <CampaignTerminal spelling> <receipt 64 hex>
/// ```
///
/// The axis values render the `spec/sprouting/` variant names verbatim:
/// sprouting owns the axes, this grammar owns only their serialization, and
/// the projection matches exhaustively so a new sprouting variant fails this
/// file's compilation until the grammar decides its spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateManifest {
    /// The record this manifest serializes.
    pub record: CandidateRecord,
}

impl CandidateManifest {
    /// The versioned first line. A manifest whose first line is not exactly
    /// this string is not a version-1 candidate manifest.
    pub const MAGIC: &'static str = "BATPAK-CANDIDATE-MANIFEST/1";

    /// Render the canonical line-oriented form, in the documented field
    /// order, terminated by a trailing newline on every line.
    pub fn render(&self) -> String {
        let r = &self.record;
        let mut out = String::new();
        push_line(&mut out, Self::MAGIC);
        push_kv(&mut out, "candidate-id", &r.id.render());
        push_count(&mut out, "parent-count", r.parents.len());
        for parent in &r.parents {
            push_kv(&mut out, "parent", &parent.render());
        }
        push_kv(
            &mut out,
            "source-frontier-commitment",
            &r.source_frontier_commitment.render(),
        );
        push_count(&mut out, "dependency-count", r.dependency_commitments.len());
        for dep in &r.dependency_commitments {
            let mut value = dep.dependency.render();
            value.push(' ');
            value.push_str(&dep.commitment.render());
            push_kv(&mut out, "dependency", &value);
        }
        push_kv(&mut out, "content-commitment", &r.content_commitment.render());
        push_kv(&mut out, "origin", origin_token(r.origin));
        push_kv(&mut out, "change-class", change_class_token(r.change_class));
        push_kv(
            &mut out,
            "realization-posture",
            posture_token(r.realization_posture),
        );
        push_count(&mut out, "evidence-count", r.generator_or_pilot_evidence.len());
        for evidence in &r.generator_or_pilot_evidence {
            push_kv(&mut out, "evidence", &evidence.render());
        }
        push_count(
            &mut out,
            "qualification-receipt-count",
            r.qualification_receipts.len(),
        );
        for receipt in &r.qualification_receipts {
            push_kv(&mut out, "qualification-receipt", &receipt.render());
        }
        push_count(&mut out, "holdout-receipt-count", r.holdout_receipts.len());
        for receipt in &r.holdout_receipts {
            push_kv(&mut out, "holdout-receipt", &receipt.render());
        }
        push_kv(&mut out, "reuse-key", &r.reuse_key.render());
        match &r.terminal {
            None => push_line(&mut out, "terminal none"),
            Some(t) => {
                let mut value = String::from(t.terminal.spelling());
                value.push(' ');
                value.push_str(&t.receipt.render());
                push_kv(&mut out, "terminal", &value);
            }
        }
        out
    }
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push(' ');
    out.push_str(value);
    out.push('\n');
}

fn push_count(out: &mut String, key: &str, count: usize) {
    out.push_str(&format!("{key} {count}\n"));
}

/// The manifest spelling of a candidate origin: the variant name, projected
/// exhaustively. Sprouting owns the axis; this grammar owns the serialization.
const fn origin_token(origin: CandidateOriginKind) -> &'static str {
    match origin {
        CandidateOriginKind::DeterministicGeneration => "DeterministicGeneration",
        CandidateOriginKind::BoundedSearch => "BoundedSearch",
        CandidateOriginKind::TransferReuse => "TransferReuse",
        CandidateOriginKind::HumanAuthored => "HumanAuthored",
        CandidateOriginKind::MachineAssistedSynthesis => "MachineAssistedSynthesis",
        CandidateOriginKind::RepairOfCandidate => "RepairOfCandidate",
    }
}

/// The manifest spelling of a change class: the variant name, projected
/// exhaustively.
const fn change_class_token(class: CandidateChangeClass) -> &'static str {
    match class {
        CandidateChangeClass::RealizationPreserving => "RealizationPreserving",
        CandidateChangeClass::LawChanging => "LawChanging",
    }
}

/// The manifest spelling of a realization posture: the variant name,
/// projected exhaustively.
const fn posture_token(posture: RealizationPosture) -> &'static str {
    match posture {
        RealizationPosture::Missing => "Missing",
        RealizationPosture::Scaffold => "Scaffold",
        RealizationPosture::Candidate => "Candidate",
    }
}

// ===========================================================================
// Frozen judge binding (DEC-082)
// ===========================================================================

/// The content-addressed frozen judge snapshot the campaign runs under
/// (DEC-082). The judge root is content-addressed BEFORE any candidate
/// executes, and every evidence artifact binds the exact judge digest it was
/// produced under. Digest binding DETECTS and INVALIDATES judge mutation: a
/// candidate that mutated its judge produces evidence that cannot verify —
/// the mutation is never quietly repaired from inside the campaign, and the
/// campaign resumes only from a NEW frozen snapshot after the separate
/// DEC-074 amendment lane runs. Types do not make filesystem writes
/// unrepresentable; isolation is the ruled combination of disjoint roots,
/// write containment, entry-aware snapshots, this exact digest binding,
/// root-containment checks, and evidence invalidation on judge change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrozenJudgeSnapshot {
    /// The content digest of the complete judge root at freeze time.
    pub judge_root_digest: Sha256Digest,
}

// ===========================================================================
// Frontier and freshness vocabulary (DEC-082, docs/39 §6)
// ===========================================================================

/// The dependency-first frontier status of ONE candidate (DEC-082, docs/39
/// §6). Authority advances only through dependency-ordered qualification: a
/// candidate is frontier-qualified only when every dependency is qualified,
/// and LATER GREEN CANNOT BLESS EARLIER RED — a locally green candidate above
/// an unresolved dependency is `BlockedByDependency`, not qualified; the
/// frontier has not reached it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierState {
    /// Every dependency is qualified and this candidate's own qualification
    /// receipts stand; the trusted frontier has reached and admitted it.
    Qualified,
    /// The candidate may be locally green, but at least one dependency is not
    /// qualified; no amount of downstream green changes this.
    BlockedByDependency,
    /// The candidate's standing was invalidated — its evidence is bound to a
    /// mutated judge or a superseded dependency commitment. This is the
    /// current frontier STATUS; the receipted `CampaignTerminal::Invalidated`
    /// event is the separate fact that put it here.
    Invalidated,
}

impl FrontierState {
    /// The frozen inventory of frontier states, in canonical order.
    pub const ALL: &'static [FrontierState] = &[
        FrontierState::Qualified,
        FrontierState::BlockedByDependency,
        FrontierState::Invalidated,
    ];

    /// The documentary spelling: the variant name is the spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            FrontierState::Qualified => "Qualified",
            FrontierState::BlockedByDependency => "BlockedByDependency",
            FrontierState::Invalidated => "Invalidated",
        }
    }
}

/// Whether a candidate's evidence is still fresh (DEC-082). Evidence binds
/// the exact judge digest and the exact dependency commitments it was
/// produced under; when either changes, the evidence is STALE and is never
/// quietly reused — the staleness recomputation runs after every frontier
/// promotion and after every judge amendment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceFreshness {
    /// The evidence binds the current frozen judge digest and the current
    /// upstream dependency commitments.
    Fresh,
    /// The frozen judge changed after this evidence was produced; the
    /// evidence cannot verify against the new judge snapshot.
    StaleByJudgeChange,
    /// An upstream dependency commitment changed after this evidence was
    /// produced; the descendant must requalify against the new frontier.
    StaleByDependencyChange,
}

impl EvidenceFreshness {
    /// The frozen inventory of freshness states, in canonical order.
    pub const ALL: &'static [EvidenceFreshness] = &[
        EvidenceFreshness::Fresh,
        EvidenceFreshness::StaleByJudgeChange,
        EvidenceFreshness::StaleByDependencyChange,
    ];

    /// The documentary spelling: the variant name is the spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            EvidenceFreshness::Fresh => "Fresh",
            EvidenceFreshness::StaleByJudgeChange => "StaleByJudgeChange",
            EvidenceFreshness::StaleByDependencyChange => "StaleByDependencyChange",
        }
    }
}

// ===========================================================================
// Campaign closure graph vocabulary (DEC-082: the typed closure owner)
// ===========================================================================

/// One candidate node of the campaign closure graph: the projection of one
/// lineage record into the generated closure view (DEC-082 requires the
/// closure graph to enter the closed generated-view registry before it may
/// land). The node carries the candidate's identity, its realization posture,
/// its per-candidate frontier status, its receipted terminal if one exists,
/// and its evidence freshness — the exact facts the closure view renders.
/// The view PROJECTS typed records; it is never a log parser and never a
/// competing authority over them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignClosureNode {
    /// The candidate this node projects.
    pub candidate: CandidateId,
    /// Missing, scaffold, or candidate — a scaffold stays visibly unrealized
    /// in the closure view.
    pub realization_posture: RealizationPosture,
    /// The dependency-first frontier status of this candidate.
    pub frontier: FrontierState,
    /// The receipted terminal disposition, if one exists. Every terminal —
    /// including `ArchitectRequired` — remains visible in the denominator.
    pub terminal: Option<CampaignTerminal>,
    /// Whether this candidate's evidence is still fresh.
    pub freshness: EvidenceFreshness,
}

/// WHICH required lineage field a closure edge is read back out of. Lineage
/// relations are DERIVED from the required record fields (DEC-079, docs/39
/// §2): a parent edge IS the parent-`CandidateId` field, a dependency edge IS
/// a dependency commitment, a reuse edge IS a reuse-key match, an
/// invalidation edge IS the invalidation receipt. This enum is the closure
/// projection's read-back vocabulary only — it authors no relation beside the
/// fields and is not a generic candidate-evidence-relation vocabulary; the
/// required fields remain the sole lineage authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignClosureEdgeKind {
    /// Read from the `parents` field: `from` names `to` as its parent.
    Parent,
    /// Read from a `dependency_commitments` entry: `from` was built against
    /// `to` at the committed content.
    Dependency,
    /// Read from a reuse-key match: `from` reused the evidence of `to` under
    /// a matching, requalified reuse key.
    Reuse,
    /// Read from an invalidation receipt: the promotion or judge change
    /// recorded at `to` invalidated `from`.
    Invalidation,
}

impl CampaignClosureEdgeKind {
    /// The frozen inventory of closure edge kinds, in canonical order.
    pub const ALL: &'static [CampaignClosureEdgeKind] = &[
        CampaignClosureEdgeKind::Parent,
        CampaignClosureEdgeKind::Dependency,
        CampaignClosureEdgeKind::Reuse,
        CampaignClosureEdgeKind::Invalidation,
    ];

    /// The documentary spelling: the variant name is the spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            CampaignClosureEdgeKind::Parent => "Parent",
            CampaignClosureEdgeKind::Dependency => "Dependency",
            CampaignClosureEdgeKind::Reuse => "Reuse",
            CampaignClosureEdgeKind::Invalidation => "Invalidation",
        }
    }
}

/// One derived edge of the campaign closure graph: `from`'s required lineage
/// field names `to`, and `kind` says WHICH field it was read out of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignClosureEdge {
    /// The candidate whose required field carries the relation.
    pub from: CandidateId,
    /// The candidate the field names.
    pub to: CandidateId,
    /// The required field the edge is read back out of.
    pub kind: CampaignClosureEdgeKind,
}
