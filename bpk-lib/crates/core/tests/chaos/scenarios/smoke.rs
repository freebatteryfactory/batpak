//! PROVES: the dm-flakey wrapper round-trips — a created device formats and
//! mounts ext4, `flip_to_error()` takes effect, and a post-flip write FAILS.
//! CATCHES: a dm-flakey helper that silently no-ops the error flip (writes still
//! succeeding after the flip), which would make every chaos scenario vacuous.
//! SEEDED: n/a — single deterministic device round-trip; requires
//! `BATPAK_RUN_CHAOS=1` and root, else it skips loudly on stderr.

use crate::chaos::dm_flakey::FlakeyDevice;
use std::io::Write as _;

fn chaos_enabled() -> bool {
    std::env::var_os("BATPAK_RUN_CHAOS").is_some()
}

#[test]
fn dm_flakey_wrapper_create_flip_teardown_round_trip() {
    if !chaos_enabled() {
        let _ = writeln!(
            std::io::stderr(),
            "skipping privileged dm-flakey smoke; set BATPAK_RUN_CHAOS=1 to run it"
        );
        return;
    }

    let device = FlakeyDevice::create(64 * 1024 * 1024).expect("create flakey device");
    device
        .format_and_mount_ext4_with_sync()
        .expect("format and mount");

    let test_file = device.mount_point.join("test.bin");
    std::fs::write(&test_file, b"before flip").expect("write before flip");

    device.flip_to_error().expect("flip");

    let after = std::fs::write(&test_file, b"after flip");
    assert!(after.is_err(), "PROPERTY: writes after flip must fail");
}
