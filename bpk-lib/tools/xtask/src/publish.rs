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

/// Map a workspace package NAME to its directory under `crates/`.
///
/// Single source of truth for the name→dir mapping the release tooling repeats
/// (public-api, msrv-check, release-manifest, sbom). `batpak` lives at
/// `crates/core`; the `batpak-`-prefixed support crates drop the prefix in their
/// directory; every other family crate's directory equals its name. Returns
/// `None` for a name outside the known family so callers that must fail closed
/// (e.g. the MSRV gate's vacuity guard) can distinguish "unknown" from identity.
pub(crate) fn package_dir(package: &str) -> Option<&'static str> {
    Some(match package {
        "batpak" => "core",
        "batpak-macros" => "macros",
        "batpak-macros-support" => "macros-support",
        "batpak-bench-support" => "bench-support",
        "syncbat" => "syncbat",
        "netbat" => "netbat",
        "hostbat" => "hostbat",
        "bvisor" => "bvisor",
        _ => return None,
    })
}

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

#[cfg(test)]
mod tests {
    use super::{package_dir, PUBLISH_CRATES, RELEASE_CHAIN};
    use crate::util::repo_root;
    use std::collections::BTreeSet;

    /// Whether a `[package] publish = …` value marks the crate publishable.
    /// Mirrors cargo's own rule (and `architecture_ir`'s oracle): absent or
    /// `true` is publishable, `false`/`[]` is not, a non-empty registry list is
    /// a restricted-but-still-publishable crate.
    fn publish_value_is_publishable(publish: Option<&toml::Value>) -> bool {
        match publish {
            None => true,
            Some(toml::Value::Boolean(flag)) => *flag,
            Some(toml::Value::Array(registries)) => !registries.is_empty(),
            Some(_) => true,
        }
    }

    /// GAUNT-PUBLISH-CRATES-ORACLE: the crate lists in `publish.rs` are the
    /// single source the release tooling (coverage, msrv, public-api, sbom,
    /// release-manifest, the release chain) derives its crate set from. This
    /// gate reconciles them against the actual publishable workspace members
    /// read from the manifests, so a newly-added publishable crate — or one that
    /// flips `publish = false` — cannot silently drift out of the family the
    /// release train ships.
    ///
    /// The publishable set is `RELEASE_CHAIN` (the full crates.io train), NOT
    /// `PUBLISH_CRATES`: the three `batpak-macros*`/`batpak-bench-support`
    /// support crates are published as dependencies of the headline crates but
    /// are not part of the `PUBLISH_CRATES` "headline family" that carries
    /// public-api baselines / coverage / msrv. `PUBLISH_CRATES` must therefore
    /// be a subset of `RELEASE_CHAIN`, and `RELEASE_CHAIN` must equal the
    /// publishable members exactly.
    #[test]
    fn publish_crates_match_publishable_workspace_members() {
        let root = repo_root().expect("locate workspace root");
        let workspace = root.join("Cargo.toml");
        let text = std::fs::read_to_string(&workspace).expect("read workspace Cargo.toml");
        let parsed: toml::Value = toml::from_str(&text).expect("parse workspace Cargo.toml");
        let members = parsed
            .get("workspace")
            .and_then(|workspace| workspace.get("members"))
            .and_then(toml::Value::as_array)
            .expect("workspace.members array");

        let mut publishable = BTreeSet::new();
        for member in members {
            let member = member.as_str().expect("member is a string");
            let manifest = root.join(member).join("Cargo.toml");
            let member_text = std::fs::read_to_string(&manifest).expect("read member Cargo.toml");
            let member_toml: toml::Value =
                toml::from_str(&member_text).expect("parse member Cargo.toml");
            let package = member_toml
                .get("package")
                .and_then(toml::Value::as_table)
                .expect("member manifest has a [package] table");
            let name = package
                .get("name")
                .and_then(toml::Value::as_str)
                .expect("member manifest has a package.name");
            if publish_value_is_publishable(package.get("publish")) {
                publishable.insert(name.to_owned());
            }
        }

        let release_chain = RELEASE_CHAIN
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            publishable, release_chain,
            "RELEASE_CHAIN ({release_chain:?}) drifted from the actual publishable workspace \
             members ({publishable:?}); the release train would publish the wrong set of crates"
        );

        // PUBLISH_CRATES (the headline family carrying public-api baselines,
        // coverage, msrv, sbom) must stay a subset of the full publishable chain.
        let headline = PUBLISH_CRATES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        assert!(
            headline.is_subset(&publishable),
            "PUBLISH_CRATES ({headline:?}) contains a crate that is not a publishable workspace \
             member ({publishable:?})"
        );
    }

    /// GAUNT-FAMILY-VERSION-LOCKSTEP: every publishable family crate must
    /// inherit its version via `version.workspace = true` (resolving to
    /// `[workspace.package] version`), never a literal pin. This is what makes
    /// one workspace-version edit bump the whole family; a crate that re-pins a
    /// divergent literal reds here. (`tools/*` keep independent versions and are
    /// deliberately out of scope.)
    #[test]
    fn family_crates_inherit_workspace_version() {
        let root = repo_root().expect("locate workspace root");
        for package in RELEASE_CHAIN {
            let dir = package_dir(package).expect("release-chain crate resolves to a dir");
            let manifest = root.join("crates").join(dir).join("Cargo.toml");
            let text =
                std::fs::read_to_string(&manifest).expect("read release-chain crate manifest");
            let parsed: toml::Value =
                toml::from_str(&text).expect("parse release-chain crate manifest");
            let inherits = parsed
                .get("package")
                .and_then(|package| package.get("version"))
                .and_then(toml::Value::as_table)
                .and_then(|version| version.get("workspace"))
                .and_then(toml::Value::as_bool)
                == Some(true);
            assert!(
                inherits,
                "{package} ({}) must use `version.workspace = true` so the family bumps in \
                 lockstep off `[workspace.package] version`; a literal pin can silently diverge",
                manifest.display()
            );
        }
    }

    /// The name→dir map must resolve every crate the release train touches —
    /// both the publish family and the full release chain (macro/bench support
    /// crates included). A gap here would make a release step build a path to a
    /// non-existent directory.
    #[test]
    fn package_dir_resolves_every_release_chain_crate() {
        for package in RELEASE_CHAIN {
            assert!(
                package_dir(package).is_some(),
                "package_dir({package}) must resolve — the release chain iterates it"
            );
        }
        for package in PUBLISH_CRATES {
            assert!(
                package_dir(package).is_some(),
                "package_dir({package}) must resolve — publish tooling iterates it"
            );
        }
        assert_eq!(package_dir("batpak"), Some("core"));
        assert_eq!(package_dir("batpak-macros-support"), Some("macros-support"));
        assert_eq!(package_dir("syncbat"), Some("syncbat"));
        assert_eq!(package_dir("not-a-crate"), None);
    }
}
