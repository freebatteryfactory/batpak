//! PROVES: the visibility-range boundary math — range merging, half-open
//! overlap, per-lane normalization/empty-drop, the on-disk header length gate,
//! the not-found vs corrupt read ladder, and the "nothing to persist" branch.
//! CATCHES: merge `<=`->`>` (fails to coalesce adjacent cancelled ranges), the
//! header-length `<`->`<=`/`==` boundary mutants (rejecting a valid file /
//! reading past a short one), the not-found guard widened to `true` (a corrupt
//! metadata file silently forgotten), the `!ranges.is_empty()` inversion, the
//! `normalize_lane_ranges` constant-body replacements, and the persist-guard
//! `&&`->`||` that would drop lane ranges whenever the global set is empty.
//! SEEDED: deterministic / no randomness.

use super::{
    load_cancelled_ranges, normalize_lane_ranges, normalize_ranges, write_cancelled_ranges,
    CancelledVisibilityRanges, VISIBILITY_RANGES_FILENAME,
};
use crate::store::platform::fs::RealFs;
use crate::store::{HiddenRangesCorruption, StoreError};
use std::collections::BTreeMap;

fn lane_map(
    entries: impl IntoIterator<Item = (u32, Vec<(u64, u64)>)>,
) -> BTreeMap<u32, Vec<(u64, u64)>> {
    entries.into_iter().collect()
}

#[test]
fn normalize_ranges_merges_adjacent_and_overlapping_but_keeps_disjoint_separate() {
    // Kills hidden_ranges.rs:50 `start <= *merged_end` -> `>`. The merge step
    // coalesces a range whose start touches or falls within the running merged
    // end. Flipping `<=` to `>` inverts the predicate: it fails to merge the
    // adjacent/overlapping cases and instead merges the disjoint case.
    assert_eq!(
        normalize_ranges(&[(0, 5), (5, 10)]).expect("valid"),
        vec![(0, 10)],
        "PROPERTY: adjacent ranges touching at the boundary (start == merged_end) merge"
    );
    assert_eq!(
        normalize_ranges(&[(0, 5), (3, 8)]).expect("valid"),
        vec![(0, 8)],
        "PROPERTY: overlapping ranges merge to their union"
    );
    assert_eq!(
        normalize_ranges(&[(0, 5), (10, 15)]).expect("valid"),
        vec![(0, 5), (10, 15)],
        "PROPERTY: disjoint ranges stay separate; the `>` mutant would fuse them"
    );
}

#[test]
fn normalize_lane_ranges_normalizes_present_lanes_and_drops_empty_ones() {
    // Kills hidden_ranges.rs:66 `if !ranges.is_empty()` (delete `!`) AND every
    // constant-body replacement of hidden_ranges.rs:63 `normalize_lane_ranges`
    // (all `Ok(BTreeMap::from_iter([...]))` / `Ok(BTreeMap::new())` variants).
    // A single exact expected map that matches NONE of those constants pins the
    // behavior: lane 5's overlapping ranges merge, and empty lane 7 is dropped.
    let input = lane_map([(5u32, vec![(10, 20), (15, 25)]), (7u32, Vec::new())]);
    let expected = lane_map([(5u32, vec![(10, 25)])]);

    assert_eq!(
        normalize_lane_ranges(&input).expect("valid lane ranges"),
        expected,
        "PROPERTY: normalize_lane_ranges merges each present lane's ranges and drops \
         lanes that normalize to empty; the `delete !` inversion would keep only the \
         empty lane, and every constant-body mutant returns a fixed map that differs \
         from {{5:[(10,25)]}}"
    );
}

#[test]
fn load_cancelled_ranges_rejects_a_short_file_and_reads_a_full_length_header() {
    // Kills hidden_ranges.rs:160 `raw.len() < HEADER_LEN` -> `<=` AND -> `==`.
    // A file shorter than the 12-byte header is TooShort; a file of exactly the
    // header length must pass the length gate and be judged on its content
    // (here: bad magic). `<=` would reject the exactly-12 file as TooShort;
    // `==` would let a 5-byte file slip past the gate and index out of bounds.
    let short_dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::write(short_dir.path().join(VISIBILITY_RANGES_FILENAME), [0u8; 5])
        .expect("write a 5-byte truncated file");
    let short_result = load_cancelled_ranges(short_dir.path());
    assert!(
        matches!(
            &short_result,
            Err(StoreError::HiddenRangesCorrupt {
                kind: HiddenRangesCorruption::TooShort {
                    actual: 5,
                    required: 12,
                },
                ..
            })
        ),
        "PROPERTY: a 5-byte file is TooShort{{actual:5,required:12}}; the `==` mutant \
         would read past its end. Got {short_result:?}"
    );

    let exact_dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::write(exact_dir.path().join(VISIBILITY_RANGES_FILENAME), [0u8; 12])
        .expect("write an exactly-header-length file");
    let exact_result = load_cancelled_ranges(exact_dir.path());
    assert!(
        matches!(
            &exact_result,
            Err(StoreError::HiddenRangesCorrupt {
                kind: HiddenRangesCorruption::BadMagic,
                ..
            })
        ),
        "PROPERTY: an exactly-12-byte file passes the length gate and is judged on its \
         (here invalid) magic; the `<=` mutant rejects it as TooShort. Got {exact_result:?}"
    );
}

#[test]
fn load_cancelled_ranges_fails_closed_on_a_non_not_found_read_error() {
    // Kills hidden_ranges.rs:150 match guard `error.kind() == NotFound` -> `true`.
    // Only a genuinely absent file may return Ok(None); any other read failure
    // must fail closed as HiddenRangesCorrupt so previously-hidden events are not
    // resurrected. A directory at the metadata path yields a non-NotFound read
    // error, which the `true` mutant would silently swallow into Ok(None).
    let dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir_all(dir.path().join(VISIBILITY_RANGES_FILENAME))
        .expect("materialize a directory at the metadata path");

    let result = load_cancelled_ranges(dir.path());
    assert!(
        matches!(
            &result,
            Err(StoreError::HiddenRangesCorrupt {
                kind: HiddenRangesCorruption::ReadFailed(_),
                ..
            })
        ),
        "PROPERTY: a non-NotFound read failure must fail closed as ReadFailed, not \
         collapse to Ok(None) as the `true` guard mutant does. Got {result:?}"
    );
}

#[test]
fn write_persists_lane_ranges_even_when_the_global_set_is_empty() {
    // Kills hidden_ranges.rs:83 `normalized_global.is_empty() &&
    // normalized_lanes.is_empty()` -> `||`. The file is removed (nothing to
    // persist) ONLY when BOTH the global and lane sets are empty. With global
    // empty but lane ranges present, the file MUST still be written. The `||`
    // mutant removes/skips the write whenever global is empty, silently dropping
    // live lane cancellations.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let ranges = CancelledVisibilityRanges {
        global: Vec::new(),
        lanes: lane_map([(5u32, vec![(10, 20)])]),
    };
    write_cancelled_ranges(dir.path(), &ranges, &RealFs).expect("persist lane-only ranges");

    let loaded = load_cancelled_ranges(dir.path()).expect("load persisted ranges");
    assert_eq!(
        loaded,
        Some(CancelledVisibilityRanges {
            global: Vec::new(),
            lanes: lane_map([(5u32, vec![(10, 20)])]),
        }),
        "PROPERTY: lane ranges persist even with an empty global set; the `||` mutant \
         drops the write and load returns None"
    );
}
