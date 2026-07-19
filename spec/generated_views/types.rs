use crate::guarantees::BootstrapToolId;

/// The three admitted surface forms a generated view may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedViewSurface {
    /// A marker-fenced block embedded inside an authored document. The block
    /// inherits no authority from its containing document.
    EmbeddedBlock,
    /// An entire generated file. A standalone generated file is a derived
    /// view, never an authored contract.
    StandaloneFile,
    /// Mechanical frontmatter convergence across the eligible Markdown
    /// corpus. Corpus epoch frontmatter states corpus membership, not
    /// semantic truth.
    CorpusFrontmatter,
}

/// Where a generated view lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedViewTarget {
    /// Exact tracked paths. One logical view may lawfully have multiple
    /// static targets (StaleVocabulary is the standing multi-target case).
    Static(&'static [&'static str]),
    /// Every eligible tracked Markdown document, discovered mechanically.
    EligibleMarkdownCorpus,
}

/// One view's generation metadata. `authority_sources` names the files whose
/// facts the view serializes; supporting typed identities used only for
/// validation or formatting do not become co-owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedViewSpec {
    pub authority_sources: &'static [&'static str],
    pub target: GeneratedViewTarget,
    pub surface: GeneratedViewSurface,
    pub marker: Option<&'static str>,
    pub generator: BootstrapToolId,
}
