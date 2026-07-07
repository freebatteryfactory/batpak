#![no_main]
//! GAUNT-FUZZ-1 — `sidx_boundary` target.
//!
//! Drives `batpak::__fuzz::__fuzz_sidx_boundary(&[u8], segment_id)`, which
//! forwards into the segment footer-boundary detector. This is the scanner's
//! first decision point before it chooses whether frames end at an authenticated
//! SDX3 offset, an untrusted legacy/corrupt footer, or no footer at all.
//!
//! The extra scalar `segment_id: u64` is derived from the FIRST 8 bytes of
//! `data`; the rest is the file body.
//!
//! S1 contract: (a) never panic; (b) malformed or unrelated tails return a typed
//! outcome; (c) future SIDX-family versions such as SDX4 surface a typed
//! `StoreError::SidxFutureVersion`, not an over-read into footer bytes.

use libfuzzer_sys::fuzz_target;

use batpak::__fuzz::__fuzz_sidx_boundary;

fuzz_target!(|data: &[u8]| {
    let (segment_id, body) = split_u64_prefix(data);
    match __fuzz_sidx_boundary(body, segment_id) {
        Ok(outcome) => {
            let _ = outcome;
        }
        Err(err) => {
            let _ = format!("{err:?}");
        }
    }
});

fn split_u64_prefix(data: &[u8]) -> (u64, &[u8]) {
    if data.len() < 8 {
        return (0, &[]);
    }
    let (head, rest) = data.split_at(8);
    let scalar = u64::from_le_bytes(head.try_into().expect("8-byte prefix"));
    (scalar, rest)
}
