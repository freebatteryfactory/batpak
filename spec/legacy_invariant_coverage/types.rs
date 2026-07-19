#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageDisposition {
    Preserve,
    Supersede,
    Demote,
    Kill,
    Requalify,
}

impl CoverageDisposition {
    pub const ALL: &'static [CoverageDisposition] = &[
        CoverageDisposition::Preserve,
        CoverageDisposition::Supersede,
        CoverageDisposition::Demote,
        CoverageDisposition::Kill,
        CoverageDisposition::Requalify,
    ];

    /// The documentary spelling (5.5E4c): the one spelling the generated
    /// coverage matrix renders — no Python disposition map exists.
    pub const fn spelling(self) -> &'static str {
        match self {
            CoverageDisposition::Preserve => "PRESERVE",
            CoverageDisposition::Supersede => "SUPERSEDE",
            CoverageDisposition::Demote => "DEMOTE",
            CoverageDisposition::Kill => "KILL",
            CoverageDisposition::Requalify => "REQUALIFY",
        }
    }

    /// The documentary meaning — owned here, projected into the generated
    /// legend, never restated by hand.
    pub const fn meaning(self) -> &'static str {
        match self {
            CoverageDisposition::Preserve => {
                "semantic law survives through named successor"
            }
            CoverageDisposition::Supersede => {
                "law survives but old mechanism, API, or catalog does not"
            }
            CoverageDisposition::Demote => {
                "useful compatibility or reference evidence, not native authority"
            }
            CoverageDisposition::Kill => "intentionally absent from the target",
            CoverageDisposition::Requalify => {
                "valid only under an explicit target or profile proof scope"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyInvariantCoverage {
    pub legacy_id: &'static str,
    pub disposition: CoverageDisposition,
    pub successor: &'static str,
    pub rationale: &'static str,
}
