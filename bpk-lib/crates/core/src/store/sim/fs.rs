//! Fault-injecting in-memory filesystem model.
//!
//! [`SimFs`] implements the production [`StoreFs`] seam and, on top of the two
//! routed ops (`read_dir` / `create_dir_all`), models an in-memory directory
//! tree whose *write* path consults a seeded PRNG ([`fastrand`], a
//! ChaCha8-class stream cipher PRNG when built with that backend) to apply
//! faults keyed off [`InjectionPoint`]:
//!
//!   * **torn-write** — only a deterministic prefix of the bytes lands, the
//!     tail is dropped (models a crash mid-`write`).
//!   * **short-read** — a read returns fewer bytes than requested.
//!   * **fsync-drop** — `fsync` is silently skipped so the most recent
//!     unsynced bytes are lost on the next simulated crash.
//!
//! Determinism: every fault decision is drawn from a single seeded
//! [`fastrand::Rng`] advanced once per [`InjectionPoint`] consultation, in the
//! order the simulation reaches those points. Same seed ⇒ same fault sequence.
//!
//! Routed-op backing: `std::fs::ReadDir` has no public constructor, so the two
//! StoreFs trait methods delegate to a private sandbox tempdir. The novel,
//! fault-injecting surface (`write_bytes` / `read_bytes` / `fsync` over
//! [`SimFile`]) operates purely on the in-memory model and is what the
//! workload drives. Wiring the durability ops through this surface lands when
//! those ops are routed onto `StoreFs` (see GAUNTLET_ISSUES.md).

use crate::store::fault::InjectionPoint;
use crate::store::platform::fs::StoreFs;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A fault drawn for one [`InjectionPoint`] consultation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Fault {
    /// Proceed normally — the full op succeeds.
    None,
    /// Only `prefix_len` bytes of a write land; the tail is dropped.
    TornWrite {
        /// Number of bytes that actually persisted before the simulated tear.
        prefix_len: usize,
    },
    /// A read returns fewer bytes than requested.
    ShortRead {
        /// Number of bytes actually returned.
        returned: usize,
    },
    /// `fsync` is dropped: unsynced bytes remain lost-on-crash.
    FsyncDrop,
}

/// In-memory file: a byte vector split into a durable (fsynced) region and an
/// unsynced tail. A simulated crash truncates the file to its durable length.
#[derive(Default, Clone)]
struct SimFile {
    /// All written bytes (durable prefix + unsynced tail).
    bytes: Vec<u8>,
    /// Length of the prefix that has survived an `fsync`.
    durable_len: usize,
}

/// Deterministic, fault-injecting filesystem.
///
/// State lives behind [`Mutex`]es so the type is legitimately `Send + Sync`
/// (required by the [`StoreFs`] supertrait) without any `unsafe`; the simulation
/// drives it single-threaded so the locks are always uncontended.
pub(crate) struct SimFs {
    /// Seeded PRNG; advanced once per injection-point consultation.
    rng: Mutex<fastrand::Rng>,
    /// In-memory file table keyed by logical path.
    files: Mutex<BTreeMap<PathBuf, SimFile>>,
    /// Sandbox backing the two routed StoreFs ops (read_dir/create_dir_all).
    /// `None` only if the host could not allocate a tempdir, in which case the
    /// routed ops surface a typed io error rather than panicking.
    sandbox: Option<tempfile::TempDir>,
}

impl SimFs {
    /// Construct a filesystem model seeded from `seed`. All fault decisions are
    /// a pure function of `seed` and the order of injection-point consultations.
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            rng: Mutex::new(fastrand::Rng::with_seed(seed)),
            files: Mutex::new(BTreeMap::new()),
            // Tempdir allocation effectively never fails; if it does, routed
            // ops fail closed with an io error instead of panicking.
            sandbox: tempfile::tempdir().ok(),
        }
    }

    /// Decide which fault (if any) fires at `point`, advancing the PRNG exactly
    /// once. The decision distribution is keyed off the injection-point family
    /// so that, e.g., fsync points can only ever drop an fsync. The single draw
    /// per call is what keeps the fault stream a pure function of consultation
    /// order.
    pub(crate) fn decide_fault(&self, point: &InjectionPoint, op_len: usize) -> Fault {
        let mut rng = self
            .rng
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let roll = rng.u32(..);
        // ~1-in-8 fault rate; the family of the point selects the fault kind.
        if !roll.is_multiple_of(8) {
            return Fault::None;
        }
        // Bounded modulo against the op length, never zero-divides.
        let bounded = |op_len: usize| -> usize {
            if op_len == 0 {
                0
            } else {
                (roll as usize) % op_len
            }
        };
        match point {
            InjectionPoint::BatchFsync { .. }
            | InjectionPoint::SingleAppendWritten { .. }
            | InjectionPoint::BatchCommitWritten { .. } => Fault::FsyncDrop,
            InjectionPoint::ReadAt { .. } | InjectionPoint::ColdStartScanFrame { .. } => {
                Fault::ShortRead {
                    returned: bounded(op_len),
                }
            }
            // Every other write-path injection point models a torn write. An
            // explicit fallthrough (not a bare `_`) keeps the match exhaustive
            // in intent: new write points get torn-write semantics by default.
            InjectionPoint::BatchStart { .. }
            | InjectionPoint::BatchBeginWritten { .. }
            | InjectionPoint::BatchItemWritten { .. }
            | InjectionPoint::BatchItemsComplete { .. }
            | InjectionPoint::BatchPrePublish { .. }
            | InjectionPoint::SingleAppendStart { .. }
            | InjectionPoint::SingleAppendPublished { .. }
            | InjectionPoint::SegmentRotationCreate { .. }
            | InjectionPoint::SegmentRotation { .. }
            | InjectionPoint::MmapIndexLoad
            | InjectionPoint::IndexFooterDecode { .. }
            | InjectionPoint::CheckpointDecode
            | InjectionPoint::HiddenRangesLoad => Fault::TornWrite {
                prefix_len: bounded(op_len),
            },
        }
    }

    /// Append `data` to the file at `path`, applying any fault decided for
    /// `point`. Returns the number of bytes that landed (may be short under a
    /// torn write). New bytes are unsynced until [`SimFs::fsync`].
    pub(crate) fn write_bytes(&self, path: &Path, point: &InjectionPoint, data: &[u8]) -> usize {
        let landed = match self.decide_fault(point, data.len()) {
            Fault::TornWrite { prefix_len } => prefix_len,
            Fault::None | Fault::ShortRead { .. } | Fault::FsyncDrop => data.len(),
        };
        let mut files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let file = files.entry(path.to_path_buf()).or_default();
        file.bytes.extend_from_slice(&data[..landed]);
        landed
    }

    /// Read up to `len` bytes from `offset` in `path`, applying short-read
    /// faults decided for `point`. Returns the bytes actually delivered.
    pub(crate) fn read_bytes(
        &self,
        path: &Path,
        point: &InjectionPoint,
        offset: usize,
        len: usize,
    ) -> Vec<u8> {
        let files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(file) = files.get(path) else {
            return Vec::new();
        };
        let available = file.bytes.len().saturating_sub(offset);
        let want = len.min(available);
        let deliver = match self.decide_fault(point, want) {
            Fault::ShortRead { returned } => returned.min(want),
            Fault::None | Fault::TornWrite { .. } | Fault::FsyncDrop => want,
        };
        file.bytes[offset..offset + deliver].to_vec()
    }

    /// Fsync `path`, marking its current length durable — unless the fault
    /// decided for `point` drops the sync, in which case the durable length is
    /// left behind and those bytes are lost on the next simulated crash.
    /// Returns whether the sync was honored.
    pub(crate) fn fsync(&self, path: &Path, point: &InjectionPoint) -> bool {
        let dropped = matches!(self.decide_fault(point, 0), Fault::FsyncDrop);
        if dropped {
            return false;
        }
        let mut files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(file) = files.get_mut(path) {
            file.durable_len = file.bytes.len();
        }
        true
    }

    /// Simulate a crash: every file is truncated to its last durable length,
    /// discarding unsynced (and fsync-dropped) tails. Models power loss.
    pub(crate) fn crash(&self) {
        let mut files = self
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for file in files.values_mut() {
            file.bytes.truncate(file.durable_len);
        }
    }

    /// Durable byte length of `path` (what survives a crash). Used by the
    /// no-loss invariant after a simulated crash/recover.
    pub(crate) fn durable_len(&self, path: &Path) -> usize {
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .map_or(0, |f| f.durable_len)
    }

    /// Current (possibly unsynced) byte length of `path`.
    pub(crate) fn len(&self, path: &Path) -> usize {
        self.files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .map_or(0, |f| f.bytes.len())
    }
}

impl StoreFs for SimFs {
    fn read_dir(&self, path: &Path) -> io::Result<std::fs::ReadDir> {
        // ReadDir has no public constructor, so the routed op reads the
        // sandbox-relative shadow of `path` through the platform seam (not
        // std::fs directly, per the platform-boundary gate). The in-memory
        // model owns the novel fault surface; this op delegates honestly.
        crate::store::platform::fs::read_dir(&self.shadow(path)?)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        crate::store::platform::fs::create_dir_all(&self.shadow(path)?)
    }
}

impl SimFs {
    /// Map a logical path into the private sandbox so routed ops never escape
    /// the tempdir. Absolute paths are reparented under the sandbox by their
    /// normal-component chain; relative paths are joined directly. Fails closed
    /// with an io error if the sandbox tempdir could not be allocated.
    fn shadow(&self, path: &Path) -> io::Result<PathBuf> {
        let sandbox = self
            .sandbox
            .as_ref()
            .ok_or_else(|| io::Error::other("sim sandbox tempdir was not available"))?;
        let mut out = sandbox.path().to_path_buf();
        for comp in path.components() {
            if let std::path::Component::Normal(name) = comp {
                out.push(name);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> InjectionPoint {
        InjectionPoint::SingleAppendStart {
            entity: "e".to_string(),
        }
    }

    #[test]
    fn same_seed_same_fault_sequence() {
        let a = SimFs::new(99);
        let b = SimFs::new(99);
        let pa: Vec<_> = (0..32).map(|_| a.decide_fault(&point(), 100)).collect();
        let pb: Vec<_> = (0..32).map(|_| b.decide_fault(&point(), 100)).collect();
        assert_eq!(
            pa, pb,
            "PROPERTY: identical seeds produce identical fault sequences"
        );
    }

    #[test]
    fn crash_truncates_to_durable_length() {
        let fs = SimFs::new(1);
        let p = Path::new("seg.fbat");
        // Write some bytes (a torn write may shorten them) and then assert the
        // crash/durable semantics regardless of how many bytes landed.
        let landed = fs.write_bytes(p, &point(), b"hello world");
        // fsync may or may not be dropped depending on the seed; loop a fixed,
        // deterministic number of times until durability advances, then crash.
        for _ in 0..16 {
            if fs.fsync(p, &point()) {
                break;
            }
        }
        let durable = fs.durable_len(p);
        fs.crash();
        assert_eq!(
            fs.len(p),
            durable,
            "PROPERTY: a crash truncates each file to its last durable length"
        );
        assert!(
            landed <= 11,
            "torn writes never exceed the requested length"
        );
    }

    #[test]
    fn routed_ops_stay_in_sandbox() -> io::Result<()> {
        let fs = SimFs::new(2);
        let dir = Path::new("/var/data/batpak/nested");
        fs.create_dir_all(dir)?;
        let mut count = 0;
        for entry in fs.read_dir(Path::new("/var/data/batpak"))? {
            entry?;
            count += 1;
        }
        assert!(
            count >= 1,
            "PROPERTY: create_dir_all then read_dir observes the created subtree in the sandbox"
        );
        Ok(())
    }
}
