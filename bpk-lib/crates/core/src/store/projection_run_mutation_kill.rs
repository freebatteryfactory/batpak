//! Mutation-kill tests for `projection_run.rs` (cluster G, projection).
//!
//! PROVES: the deterministic-evidence mapping surface — `append_common_findings`
//! (five independent finding predicates), the four `map_*` translators, the
//! `output_hash_for_state` availability split, and the domain-neutral
//! `ProjectionEvidenceRegistry` dispatch — pin their EXACT behaviour.
//! CATCHES: boolean/comparison swaps (`&& -> ||`, `!= -> ==`, `== -> !=`),
//! condition-to-`true`/`false` replacements, match-arm body swaps, and
//! return-value replacements (`contains -> true/false`, `run -> None`).
//! SEEDED: deterministic — the finding/mapper tests call the private functions
//! directly with hand-built inputs; the registry `run` proof drives an empty
//! entity so the outcome is fully determined by dispatch, not by timing.

use super::*;
use crate::event::{Event, EventKind, JsonValueInput, ProjectionStateContract, StateExtent};
use crate::store::StoreConfig;

// ---------------------------------------------------------------------------
// append_common_findings — the densest predicate surface in the file.
// ---------------------------------------------------------------------------

fn unavailable_freshness() -> ProjectionRunFreshnessStatus {
    ProjectionRunFreshnessStatus::Unavailable {
        reason: "projection_failed".to_owned(),
    }
}

fn unavailable_output() -> ProjectionRunOutputHash {
    ProjectionRunOutputHash::Unavailable {
        reason: "projection_failed".to_owned(),
    }
}

fn unavailable_cache() -> ProjectionRunCacheStatus {
    ProjectionRunCacheStatus::Unavailable {
        reason: "projection_failed".to_owned(),
    }
}

fn sample_frontier() -> ProjectionRunInputFrontier {
    ProjectionRunInputFrontier {
        kind: ProjectionRunFrontierKind::Visible,
        wall_ms: 5,
        global_sequence: 9,
    }
}

#[test]
fn append_common_findings_emits_every_unavailable_finding() {
    // Kills the condition-to-`false` mutant on each of the four guarded pushes
    // (ObservedFreshnessUnavailable / InputFrontierUnknown / OutputHashUnavailable
    // / CacheStatusUnavailable) and the deletion of the unconditional
    // PartialVisibilityNotApplicable push: every one of these must appear here.
    let mut findings = Vec::new();
    append_common_findings(
        &mut findings,
        &unavailable_freshness(),
        None,
        &unavailable_output(),
        &unavailable_cache(),
    );

    assert!(
        findings.contains(&ProjectionRunFinding::ObservedFreshnessUnavailable),
        "PROPERTY: an Unavailable observed-freshness must emit ObservedFreshnessUnavailable"
    );
    assert!(
        findings.contains(&ProjectionRunFinding::InputFrontierUnknown),
        "PROPERTY: a missing input frontier with a non-NotApplicable freshness must emit InputFrontierUnknown"
    );
    assert!(
        findings.contains(&ProjectionRunFinding::OutputHashUnavailable),
        "PROPERTY: an Unavailable output hash must emit OutputHashUnavailable"
    );
    assert!(
        findings.contains(&ProjectionRunFinding::CacheStatusUnavailable),
        "PROPERTY: an Unavailable cache status must emit CacheStatusUnavailable"
    );
    assert!(
        findings.contains(&ProjectionRunFinding::PartialVisibilityNotApplicable),
        "PROPERTY: PartialVisibilityNotApplicable is always appended"
    );
    assert!(
        !findings.contains(&ProjectionRunFinding::StaleUsed),
        "PROPERTY: an Unavailable (not StaleAllowed) freshness must NOT emit StaleUsed"
    );
}

#[test]
fn append_common_findings_stays_silent_on_a_fully_fresh_run() {
    // Kills the condition-to-`true` mutant on all four guarded pushes and the
    // `&& -> ||` mutant on the input-frontier guard: with a present frontier and
    // Fresh freshness, `is_none() (=false) && ...` is false, but `|| ...` would be
    // true and wrongly push InputFrontierUnknown. Only the unconditional finding
    // survives.
    let mut findings = Vec::new();
    append_common_findings(
        &mut findings,
        &ProjectionRunFreshnessStatus::Fresh,
        Some(sample_frontier()),
        &ProjectionRunOutputHash::NotApplicable,
        &ProjectionRunCacheStatus::Hit,
    );

    assert_eq!(
        findings,
        vec![ProjectionRunFinding::PartialVisibilityNotApplicable],
        "PROPERTY: a fresh run with a known frontier emits only PartialVisibilityNotApplicable"
    );
}

#[test]
fn append_common_findings_emits_stale_used_only_for_stale_allowed() {
    // Kills the `== -> !=` mutant on the StaleAllowed guard: StaleAllowed must
    // push StaleUsed. Ordering is exact — StaleUsed precedes the trailing
    // PartialVisibilityNotApplicable — so a flipped predicate (no StaleUsed) is
    // caught by the vec equality.
    let mut findings = Vec::new();
    append_common_findings(
        &mut findings,
        &ProjectionRunFreshnessStatus::StaleAllowed,
        Some(sample_frontier()),
        &ProjectionRunOutputHash::NotApplicable,
        &ProjectionRunCacheStatus::Hit,
    );

    assert_eq!(
        findings,
        vec![
            ProjectionRunFinding::StaleUsed,
            ProjectionRunFinding::PartialVisibilityNotApplicable,
        ],
        "PROPERTY: StaleAllowed freshness emits StaleUsed exactly once, before the trailing finding"
    );
}

#[test]
fn append_common_findings_suppresses_input_frontier_unknown_for_not_applicable() {
    // Kills the `!= -> ==` mutant on `observed_freshness != NotApplicable`: with
    // NotApplicable freshness and a missing frontier, the real `!=` term is false
    // so InputFrontierUnknown must NOT be pushed; the `==` mutant (and `&& -> ||`)
    // would push it.
    let mut findings = Vec::new();
    append_common_findings(
        &mut findings,
        &ProjectionRunFreshnessStatus::NotApplicable,
        None,
        &ProjectionRunOutputHash::NotApplicable,
        &ProjectionRunCacheStatus::Hit,
    );

    assert_eq!(
        findings,
        vec![ProjectionRunFinding::PartialVisibilityNotApplicable],
        "PROPERTY: NotApplicable freshness suppresses InputFrontierUnknown even with no frontier"
    );
}

#[test]
fn append_common_findings_reports_input_frontier_unknown_when_missing_and_applicable() {
    // Kills the condition-to-`false` mutant on the input-frontier guard: a missing
    // frontier under an applicable (Fresh) freshness MUST emit InputFrontierUnknown.
    let mut findings = Vec::new();
    append_common_findings(
        &mut findings,
        &ProjectionRunFreshnessStatus::Fresh,
        None,
        &ProjectionRunOutputHash::NotApplicable,
        &ProjectionRunCacheStatus::Hit,
    );

    assert_eq!(
        findings,
        vec![
            ProjectionRunFinding::InputFrontierUnknown,
            ProjectionRunFinding::PartialVisibilityNotApplicable,
        ],
        "PROPERTY: a missing frontier with applicable freshness emits InputFrontierUnknown"
    );
}

// ---------------------------------------------------------------------------
// map_requested_freshness / map_observed_freshness / map_cache_status /
// map_input_frontier — match-arm translators.
// ---------------------------------------------------------------------------

#[test]
fn map_requested_freshness_translates_each_variant_verbatim() {
    // Kills match-arm body swaps and the `*max_stale_ms -> 0` value mutant: the
    // stale bound (4242) must survive the translation exactly.
    assert_eq!(
        map_requested_freshness(&Freshness::Consistent),
        ProjectionRunRequestedFreshness::Consistent,
        "PROPERTY: Consistent maps to Consistent"
    );
    assert_eq!(
        map_requested_freshness(&Freshness::MaybeStale { max_stale_ms: 4242 }),
        ProjectionRunRequestedFreshness::MaybeStale { max_stale_ms: 4242 },
        "PROPERTY: MaybeStale carries its max_stale_ms bound verbatim"
    );
}

#[test]
fn map_observed_freshness_translates_each_variant() {
    // Kills the three match-arm body swaps.
    assert_eq!(
        map_observed_freshness(ProjectionObservedFreshness::Fresh),
        ProjectionRunFreshnessStatus::Fresh,
    );
    assert_eq!(
        map_observed_freshness(ProjectionObservedFreshness::StaleAllowed),
        ProjectionRunFreshnessStatus::StaleAllowed,
    );
    assert_eq!(
        map_observed_freshness(ProjectionObservedFreshness::NotApplicable),
        ProjectionRunFreshnessStatus::NotApplicable,
    );
}

#[test]
fn map_cache_status_translates_each_variant_and_preserves_reason() {
    // Kills the four match-arm body swaps and the `reason.to_owned()` drop.
    assert_eq!(
        map_cache_status(ProjectionCacheObservation::Hit),
        ProjectionRunCacheStatus::Hit,
    );
    assert_eq!(
        map_cache_status(ProjectionCacheObservation::Miss),
        ProjectionRunCacheStatus::Miss,
    );
    assert_eq!(
        map_cache_status(ProjectionCacheObservation::Bypassed),
        ProjectionRunCacheStatus::Bypassed,
    );
    assert_eq!(
        map_cache_status(ProjectionCacheObservation::Unavailable {
            reason: "cache_get_failed",
        }),
        ProjectionRunCacheStatus::Unavailable {
            reason: "cache_get_failed".to_owned(),
        },
        "PROPERTY: the Unavailable reason string is carried through, not dropped"
    );
}

#[test]
fn map_input_frontier_carries_hlc_fields_into_a_visible_boundary() {
    // Kills field-source swaps (wall_ms <-> global_sequence) via distinct values
    // and the `kind` constant: the boundary must be Visible with the HLC's own
    // wall/sequence.
    let frontier = map_input_frontier(HlcPoint {
        wall_ms: 77,
        global_sequence: 88,
    });
    assert_eq!(frontier.kind, ProjectionRunFrontierKind::Visible);
    assert_eq!(
        frontier.wall_ms, 77,
        "PROPERTY: wall_ms is taken from HlcPoint.wall_ms"
    );
    assert_eq!(
        frontier.global_sequence, 88,
        "PROPERTY: global_sequence is taken from HlcPoint.global_sequence"
    );
}

// ---------------------------------------------------------------------------
// output_hash_for_state — None -> NotApplicable, Some -> Known(exact hash).
// ---------------------------------------------------------------------------

#[test]
fn output_hash_for_state_is_not_applicable_for_absent_state() {
    // Kills the `let Some(..) else { return NotApplicable }` early-return removal:
    // an absent state must map to NotApplicable, never Known/Unavailable.
    assert_eq!(
        output_hash_for_state::<u32>(None),
        ProjectionRunOutputHash::NotApplicable,
        "PROPERTY: no state means the output hash is NotApplicable"
    );
}

#[test]
fn output_hash_for_state_hashes_present_state_canonically() {
    // Kills a Known-arm replacement (NotApplicable / wrong-constant): the hash must
    // equal content_hash over the canonical encoding of the value, byte-for-byte.
    let value = 12_345_u32;
    let bytes = crate::canonical::to_bytes(&value).expect("canonical encode of u32");
    let expected = crate::evidence::content_hash(&bytes);
    assert_eq!(
        output_hash_for_state(Some(&value)),
        ProjectionRunOutputHash::Known(expected),
        "PROPERTY: a present state hashes to content_hash(canonical(value))"
    );
}

// ---------------------------------------------------------------------------
// ProjectionEvidenceRegistry — domain-neutral dispatch.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RegistryProbe;

impl crate::event::EventSourced for RegistryProbe {
    type Input = JsonValueInput;
    const STATE_CONTRACT: ProjectionStateContract =
        ProjectionStateContract::single_entity("projection-run-registry-probe");

    fn from_events(events: &[Event<serde_json::Value>]) -> Option<Self> {
        (!events.is_empty()).then_some(Self)
    }

    fn apply_event(&mut self, _event: &Event<serde_json::Value>) {}

    fn relevant_event_kinds() -> &'static [EventKind] {
        &[]
    }

    fn state_extent(&self) -> StateExtent {
        StateExtent::single_entity()
    }
}

#[test]
fn registry_new_is_empty() {
    // Kills `contains -> true` (an empty registry contains nothing) and a
    // non-empty `projection_ids` mutant.
    let registry: ProjectionEvidenceRegistry = ProjectionEvidenceRegistry::new();
    assert!(
        !registry.contains("anything"),
        "PROPERTY: a fresh registry contains no projection ids"
    );
    assert_eq!(
        registry.projection_ids().count(),
        0,
        "PROPERTY: a fresh registry iterates zero projection ids"
    );
}

#[test]
fn registry_contains_reflects_registration() {
    // Kills `contains -> false` (registered id must be found) AND `contains -> true`
    // (an unregistered id must NOT be found). A no-op `register` mutant also dies
    // here: the registered id would then be absent.
    let mut registry: ProjectionEvidenceRegistry = ProjectionEvidenceRegistry::new();
    registry.register::<RegistryProbe>("proj.registered");

    assert!(
        registry.contains("proj.registered"),
        "PROPERTY: a registered projection id is reported present (kills contains -> false)"
    );
    assert!(
        !registry.contains("proj.absent"),
        "PROPERTY: an unregistered projection id is reported absent (kills contains -> true)"
    );
}

#[test]
fn registry_projection_ids_are_sorted() {
    // Pins the sorted-key iteration: registering out of order must still iterate
    // in ascending order (BTreeMap ordering), and the set must be exactly the
    // three registered ids.
    let mut registry: ProjectionEvidenceRegistry = ProjectionEvidenceRegistry::new();
    registry.register::<RegistryProbe>("charlie");
    registry.register::<RegistryProbe>("alpha");
    registry.register::<RegistryProbe>("bravo");

    let ids: Vec<&str> = registry.projection_ids().collect();
    assert_eq!(
        ids,
        vec!["alpha", "bravo", "charlie"],
        "PROPERTY: projection_ids yields the registered ids in sorted order"
    );
}

#[test]
fn registry_run_dispatches_only_registered_projections() {
    // Kills `run -> None` (a registered projection must produce Some(Ok(report)))
    // and confirms the real None path for an unregistered id. A no-op `register`
    // mutant also dies: the registered lookup would then miss and yield None.
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(StoreConfig::new(dir.path())).expect("open store");
    let entity = "entity:registry-run";
    let mut registry: ProjectionEvidenceRegistry = ProjectionEvidenceRegistry::new();
    registry.register::<RegistryProbe>("probe.projection");

    let absent = registry.run("probe.absent", &store, entity, &Freshness::Consistent);
    assert!(
        absent.is_none(),
        "PROPERTY: an unregistered projection id dispatches to None"
    );

    let report = registry
        .run("probe.projection", &store, entity, &Freshness::Consistent)
        .expect("registered projection id must dispatch (kills run -> None)")
        .expect("empty-entity evidence run succeeds");
    assert_eq!(
        report.body.schema_version, PROJECTION_RUN_REPORT_SCHEMA_VERSION,
        "PROPERTY: the dispatched runner returns a real projection-run report body"
    );

    store.close().expect("close store");
}
