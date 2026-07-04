pub(crate) const PUBLISH_CRATES: &[&str] = &["batpak", "syncbat", "netbat", "hostbat", "bvisor"];

pub(crate) const FAMILY_CRATES: &[&str] = &["syncbat", "netbat"];

pub(crate) const RELEASE_CHAIN: &[&str] = &[
    "batpak-macros-support",
    "batpak-macros",
    "batpak-bench-support",
    "batpak",
    "syncbat",
    "netbat",
    "hostbat",
    "bvisor",
];

pub(crate) fn local_patch_overrides(package: &str) -> &'static [(&'static str, &'static str)] {
    match package {
        "batpak-macros" => &[("batpak-macros-support", "crates/macros-support")],
        "batpak" => &[
            ("batpak-macros-support", "crates/macros-support"),
            ("batpak-macros", "crates/macros"),
            ("batpak-bench-support", "crates/bench-support"),
        ],
        "syncbat" => &[
            ("batpak-macros-support", "crates/macros-support"),
            ("batpak-macros", "crates/macros"),
            ("batpak", "crates/core"),
        ],
        "netbat" => &[
            ("batpak-macros-support", "crates/macros-support"),
            ("batpak-macros", "crates/macros"),
            ("batpak", "crates/core"),
            ("syncbat", "crates/syncbat"),
        ],
        "hostbat" => &[
            ("batpak-macros-support", "crates/macros-support"),
            ("batpak-macros", "crates/macros"),
            ("batpak", "crates/core"),
            ("syncbat", "crates/syncbat"),
        ],
        "bvisor" => &[
            ("batpak-macros-support", "crates/macros-support"),
            ("batpak-macros", "crates/macros"),
            ("batpak-bench-support", "crates/bench-support"),
            ("batpak", "crates/core"),
            ("syncbat", "crates/syncbat"),
            ("hostbat", "crates/hostbat"),
        ],
        _ => &[],
    }
}
