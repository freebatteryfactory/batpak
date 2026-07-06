//! Fault-injecting filesystem LAYER for the real-`Store` crash-recovery
//! simulation.
//!
//! [`SimFs`] wraps ANY inner [`StoreFs`] backend (production default:
//! [`RealFs`]) and interposes the durability seam on the ABSTRACT handles the
//! seam mints — so the same seeded fault model drives a store over real
//! files, over an in-memory [`MemFs`], or over any embedder backend:
//!
//!   * On each handle sync ([`StoreFile::sync_data`] / [`StoreFile::sync_all`]),
//!     SimFs consults a seeded PRNG ([`fastrand`]) once. Most syncs are
//!     **honored**: the file's current inner length becomes its durable
//!     length. Under the seed's schedule a sync is **dropped**: the call still
//!     returns `Ok` to the store (a silently-lying disk), but the durable
//!     length is NOT advanced, so the most recent bytes are lost on the next
//!     crash.
//!   * [`SimFs::crash`] truncates every tracked file to its last durable
//!     length, discarding the write-but-unsynced (and sync-dropped) tail.
//!     This models power loss losing the OS page-cache tail. (Truncation
//!     reaches through the platform seam and therefore requires a
//!     real-file-backed inner today; the fault schedules themselves are
//!     backend-agnostic.)
//!
//! Reopening a real [`crate::store::Store`] over the same data directory after
//! a [`SimFs::crash`] then exercises the genuine cold-start recovery path over
//! the truncated files. The model-only determinism witness (no real `Store`)
//! lives in [`super::fault_model::InMemFaultFs`].
//!
//! Determinism: every sync-drop decision is a single draw from one seeded
//! [`fastrand::Rng`], advanced in the order the store reaches its sync seam.
//! Same seed ⇒ same drop schedule ⇒ same durable prefix ⇒ same recovered state.
//!
//! [`RealFs`]: crate::store::platform::fs::RealFs
//! [`MemFs`]: crate::store::MemFs

use crate::store::platform::fs::{
    DirEntryInfo, FileKind, FileStat, PositionedReadError, RealFs, StagedFile, StoreDirLockGuard,
    StoreFile, StoreFs,
};
use crate::store::StoreError;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Per-file durability bookkeeping: the byte length that has survived a sync.
#[derive(Default, Clone, Copy)]
struct DurableState {
    /// Length (bytes) of the file prefix that an honored sync has made durable.
    durable_len: u64,
}

/// The seeded fault state, shared between the [`SimFs`] layer and every
/// handle it mints (handles consult the same schedules the fs-level ops do).
struct SimState {
    /// Seeded PRNG; advanced once per sync to decide honor-vs-drop.
    rng: Mutex<fastrand::Rng>,
    /// Durable-length table keyed by the file path. Only files created
    /// through [`StoreFs::create_new_file`] (and fork/snapshot materializations)
    /// are tracked — the files whose torn tail a crash must discard.
    durable: Mutex<BTreeMap<PathBuf, DurableState>>,
    /// 1-in-N sync-drop rate. `0` disables drops entirely (every sync is
    /// honored), so the crash boundary is purely the unsynced tail.
    fsync_drop_one_in: u32,
    /// ENOSPC injection schedule for file-materialization ops
    /// ([`StoreFs::cow_copy_file`] / [`StoreFs::copy`]). Unarmed disables it.
    enospc_on_copy: Mutex<EnospcSchedule>,
    /// Deterministic atomic-op fault schedule for the crash-sensitive ops
    /// ([`StoreFs::rename`] / [`StoreFs::remove_file`] /
    /// [`StagedFile::persist`]). Unarmed disables it.
    op_fault: Mutex<OpFaultSchedule>,
    /// Deterministic positioned-read fault schedule for
    /// [`StoreFile::read_exact_at`] (the active-segment frame read). DISTINCT
    /// from the [`CrashOp`] schedule: a read is not an atomic publish, so it
    /// carries its own targeted-Nth counter and its own taxonomy
    /// ([`ReadFaultKind`], D1 = model BOTH a hard I/O error and a short
    /// read). Test-only: the production build never faults a read.
    #[cfg(test)]
    read_fault: Mutex<ReadFaultSchedule>,
}

/// Deterministic, fault-injecting filesystem layer.
///
/// State lives behind [`Mutex`]es so the type is legitimately `Send + Sync`
/// (required by the [`StoreFs`] supertrait) without any `unsafe`; the
/// simulation drives the store request/response per op, so the locks are
/// effectively uncontended.
pub(crate) struct SimFs {
    /// The backend the layer interposes. Production default: [`RealFs`].
    inner: Arc<dyn StoreFs>,
    state: Arc<SimState>,
}

/// ENOSPC-mid-copy injection bookkeeping. `fail_at` is the 1-based
/// materialize-op index that fails; `seen` counts materialize ops reached.
#[derive(Default)]
struct EnospcSchedule {
    fail_at: Option<u32>,
    seen: u32,
}

/// A crash-sensitive seam op a SimFs schedule can fault. These are the
/// atomic-rename / publish primitives the compaction swap/rollback, the
/// visibility-range persist, and the cursor-checkpoint persist reach through
/// the seam — so a seeded schedule can tear them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CrashOp {
    /// [`StoreFs::rename`] — the compaction relocate/rollback swap point.
    Rename,
    /// [`StoreFs::remove_file`] (and the provided `remove_file_if_present`) —
    /// the post-swap segment reclaim.
    RemoveFile,
    /// [`StagedFile::persist`] — the visibility-range, cursor-checkpoint,
    /// cold-start-artifact, and keyset atomic publish point.
    PersistTemp,
}

/// Deterministic atomic-op fault bookkeeping. `target` names the op kind and the
/// 1-based occurrence of THAT kind to fail; `seen` counts occurrences of the
/// targeted kind reached so far. The same target fails the same op every run.
#[derive(Default)]
struct OpFaultSchedule {
    target: Option<(CrashOp, u32)>,
    seen: u32,
}

/// How a scheduled [`StoreFile::read_exact_at`] fault manifests (DECISION D1 =
/// support BOTH). These map onto the two failure shapes the reader's
/// active-frame read already distinguishes:
///
///   * [`ReadFaultKind::Io`] — a hard positioned-read error
///     ([`PositionedReadError::Io`]); surfaces as [`StoreError::Io`].
///   * [`ReadFaultKind::ShortRead`] — the read ended before the requested slice
///     was filled ([`PositionedReadError::ShortRead`]). `bytes_read == 0` is an
///     EOF at the frame boundary (reader maps it to `corrupt_eof`); a non-zero
///     partial is a torn frame (reader maps it to a corrupt-segment error).
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadFaultKind {
    /// Inject a hard I/O error on the positioned read.
    Io,
    /// Inject a short read that stops after `bytes_read` bytes.
    ShortRead {
        /// Bytes "read" before the short read stops (`0` ⇒ EOF at the boundary).
        bytes_read: usize,
    },
}

/// Deterministic positioned-read fault bookkeeping. `target` is the 1-based
/// occurrence of [`StoreFile::read_exact_at`] to fault and the
/// [`ReadFaultKind`] to inject; `seen` counts positioned reads reached so far.
/// Same target ⇒ the same read faults every run. Kept distinct from
/// [`OpFaultSchedule`] so a read fault and an atomic-op fault can be armed
/// independently in one run.
#[cfg(test)]
#[derive(Default)]
struct ReadFaultSchedule {
    target: Option<(u32, ReadFaultKind)>,
    seen: u32,
}

impl SimState {
    /// Decide whether THIS sync is dropped, advancing the PRNG exactly once.
    /// A single draw per sync keeps the drop schedule a pure function of the
    /// order in which the store reaches its sync seam.
    fn fsync_dropped(&self) -> bool {
        let mut rng = self
            .rng
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let roll = rng.u32(..);
        self.fsync_drop_one_in != 0 && roll.is_multiple_of(self.fsync_drop_one_in)
    }

    /// Record an honored sync: advance `path`'s durable length to `len`. A
    /// dropped sync skips this, so the tail stays lost-on-crash.
    fn record_durable(&self, path: &Path, len: u64) {
        let mut durable = self
            .durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        durable.entry(path.to_path_buf()).or_default().durable_len = len;
    }

    /// Mirror a POSIX rename: carry `from`'s durable bookkeeping to `to`,
    /// replacing any prior entry there. When `from` was untracked its bytes
    /// are untracked at `to` as well, so any stale `to` entry is dropped —
    /// otherwise a later `crash()` would truncate `to` to a length that
    /// belonged to whatever used to live at that path.
    fn move_durable(&self, from: &Path, to: &Path) {
        let mut durable = self
            .durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match durable.remove(from) {
            Some(state) => {
                durable.insert(to.to_path_buf(), state);
            }
            None => {
                durable.remove(to);
            }
        }
    }

    /// Forget `path`'s durable bookkeeping when the file is unlinked, so a
    /// recreated path starts fresh and `crash()` cannot resurrect a stale
    /// length for it.
    fn forget_durable(&self, path: &Path) {
        self.durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(path);
    }

    /// Advance the counter for `op` and return `true` when THIS occurrence must
    /// fault. A no-op (returns `false`) when no schedule targets `op`.
    fn op_fault_strikes(&self, op: CrashOp) -> bool {
        let mut sched = self
            .op_fault
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some((target, fail_at)) = sched.target else {
            return false;
        };
        if target != op {
            return false;
        }
        sched.seen = sched.seen.saturating_add(1);
        sched.seen == fail_at
    }

    /// The injected-fault I/O error a faulted atomic op returns.
    fn injected_op_fault(op: CrashOp) -> io::Error {
        io::Error::other(format!("SimFs: injected fault on {op:?}"))
    }

    /// Advance the positioned-read counter and return the [`ReadFaultKind`] when
    /// THIS read must fault. A no-op (returns `None`) when no schedule is armed.
    #[cfg(test)]
    fn read_fault_strikes(&self) -> Option<ReadFaultKind> {
        let mut sched = self
            .read_fault
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (fail_at, kind) = sched.target?;
        sched.seen = sched.seen.saturating_add(1);
        (sched.seen == fail_at).then_some(kind)
    }

    /// Advance the materialize counter and return `true` when THIS op must fail
    /// with ENOSPC. A no-op (returns `false`) when no schedule is armed.
    fn enospc_strikes_now(&self) -> bool {
        let mut sched = self
            .enospc_on_copy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(fail_at) = sched.fail_at else {
            return false;
        };
        sched.seen = sched.seen.saturating_add(1);
        sched.seen == fail_at
    }
}

impl SimFs {
    /// Construct a fault layer over the production backend ([`RealFs`]),
    /// seeded from `seed`, dropping roughly one in `fsync_drop_one_in` syncs
    /// (`0` ⇒ never drop; the crash boundary is then exactly the bytes not
    /// yet synced).
    pub(crate) fn new(seed: u64, fsync_drop_one_in: u32) -> Self {
        Self::layered(seed, fsync_drop_one_in, Arc::new(RealFs))
    }

    /// Construct the fault layer over an arbitrary `inner` backend — the same
    /// seeded schedules interpose an in-memory or embedder backend exactly as
    /// they do real files. ([`SimFs::crash`]'s truncation still requires a
    /// real-file-backed inner; the fault schedules do not.)
    pub(crate) fn layered(seed: u64, fsync_drop_one_in: u32, inner: Arc<dyn StoreFs>) -> Self {
        Self {
            inner,
            state: Arc::new(SimState {
                rng: Mutex::new(fastrand::Rng::with_seed(seed)),
                durable: Mutex::new(BTreeMap::new()),
                fsync_drop_one_in,
                enospc_on_copy: Mutex::new(EnospcSchedule::default()),
                op_fault: Mutex::new(OpFaultSchedule::default()),
                #[cfg(test)]
                read_fault: Mutex::new(ReadFaultSchedule::default()),
            }),
        }
    }

    /// Arm a deterministic fault on the `fail_at`-th occurrence (1-based) of
    /// crash-sensitive op `op`, consuming `self` (builder form for a `SimFs` not
    /// yet shared). See [`SimFs::arm_fault_on`].
    #[cfg(test)]
    pub(crate) fn with_fault_on(self, op: CrashOp, fail_at: u32) -> Self {
        self.arm_fault_on(op, fail_at);
        self
    }

    /// Arm (or re-arm) a deterministic fault on the `fail_at`-th occurrence
    /// (1-based) of crash-sensitive op `op`. The faulted op returns an injected
    /// I/O error instead of performing, modelling a torn atomic-rename / persist
    /// on the compaction swap, visibility-range persist, or cursor-checkpoint
    /// persist path. Same `(op, fail_at)` ⇒ the same op fails every run.
    ///
    /// Interior-mutable (takes `&self`) so a test can build a `Store` over a
    /// shared `Arc<SimFs>` FIRST and arm the fault only once the store is open —
    /// the occurrence counter resets here, so the crash-sensitive ops the build
    /// itself performed are not counted toward `fail_at`.
    #[cfg(test)]
    pub(crate) fn arm_fault_on(&self, op: CrashOp, fail_at: u32) {
        let mut sched = self
            .state
            .op_fault
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sched.target = Some((op, fail_at));
        sched.seen = 0;
    }

    /// Arm a deterministic positioned-read fault on the `fail_at`-th (1-based)
    /// [`StoreFile::read_exact_at`], consuming `self` (builder form for a
    /// `SimFs` not yet shared). See [`SimFs::arm_read_fault_on`].
    #[cfg(test)]
    pub(crate) fn with_read_fault_on(self, fail_at: u32, kind: ReadFaultKind) -> Self {
        self.arm_read_fault_on(fail_at, kind);
        self
    }

    /// Arm (or re-arm) a deterministic positioned-read fault: the `fail_at`-th
    /// (1-based) [`StoreFile::read_exact_at`] injects `kind` instead of
    /// reading, modelling a torn/short active-segment frame read. Same
    /// `(fail_at, kind)` ⇒ the same read faults every run.
    ///
    /// Interior-mutable (takes `&self`) so a test can build a `Store` over a
    /// shared `Arc<SimFs>` FIRST and arm the fault only once the store is open —
    /// the occurrence counter resets here, so any reads the build itself
    /// performed are not counted toward `fail_at`.
    #[cfg(test)]
    pub(crate) fn arm_read_fault_on(&self, fail_at: u32, kind: ReadFaultKind) {
        let mut sched = self
            .state
            .read_fault
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sched.target = Some((fail_at, kind));
        sched.seen = 0;
    }

    /// Arm deterministic ENOSPC injection: the `fail_at`-th file-materialization
    /// op (1-based; `cow_copy_file` or `copy`) fails with
    /// [`io::ErrorKind::StorageFull`]. Used by the offensive `fork_hostile_fs`
    /// fixture to force a disk-full mid-fork and prove the fork does not
    /// publish a partial copy.
    pub(crate) fn with_enospc_on_copy(self, fail_at: u32) -> Self {
        {
            let mut sched = self
                .state
                .enospc_on_copy
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sched.fail_at = Some(fail_at);
            sched.seen = 0;
        }
        self
    }

    /// Durable byte length recorded for `path` (what survives a crash). `0` for
    /// an untracked path. Test-facing witness for the no-loss invariant.
    #[cfg(test)]
    pub(crate) fn durable_len(&self, path: &Path) -> u64 {
        self.state
            .durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .map_or(0, |state| state.durable_len)
    }

    /// Simulate a crash: truncate every tracked file to its last durable
    /// length, discarding the unsynced (and sync-dropped) tail. Power-loss
    /// model. After this returns, reopening a real [`crate::store::Store`] over
    /// the same data directory cold-starts over the durable prefix only.
    ///
    /// Inherent (not a [`StoreFs`] trait method) because only the fault layer
    /// models a crash — the production
    /// [`RealFs`](crate::store::platform::fs::RealFs) has no such concept, and
    /// adding a no-op trait method would leave a dead production vtable entry.
    /// The truncation reaches the real files through the platform seam, so it
    /// requires a real-file-backed inner.
    pub(crate) fn crash(&self) {
        let durable = self
            .state
            .durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (path, state) in durable.iter() {
            // A real-file-backed inner truncates its OS file in place (RealFs
            // behavior, unchanged). A virtual inner (e.g. MemFs) has no OS file,
            // so it is rewritten to its durable prefix through the seam — the
            // crash model is backend-agnostic. Probe the INNER handle to decide:
            // a blind host `truncate_file_to` could otherwise succeed against an
            // unrelated same-named host file when the inner is virtual, skip the
            // seam rewrite, and leave the virtual file at its full pre-crash
            // length (and mutate that host file as a side effect).
            let inner_is_real_file = matches!(
                self.inner.open_file(path),
                Ok(handle) if handle.as_std_file().is_some()
            );
            if inner_is_real_file
                && crate::store::platform::fs::truncate_file_to(path, state.durable_len).is_ok()
            {
                continue;
            }
            self.crash_truncate_via_seam(path, state.durable_len);
        }
    }

    /// Backend-agnostic crash truncation for a virtual inner: rewrite `path` to
    /// its first `durable_len` bytes through the seam's atomic publish, so a
    /// backend without real OS files (e.g. [`MemFs`](crate::store::MemFs))
    /// still loses the write-but-unsynced tail on crash.
    fn crash_truncate_via_seam(&self, path: &Path, durable_len: u64) {
        let Ok(full) = self.inner.read(path) else {
            return;
        };
        let keep = usize::try_from(durable_len)
            .unwrap_or(usize::MAX)
            .min(full.len());
        if keep >= full.len() {
            return;
        }
        let Some(dir) = path.parent() else {
            return;
        };
        let Ok(mut tmp) = self.inner.named_temp_in(dir) else {
            return;
        };
        if tmp.write_all(&full[..keep]).is_err() || tmp.sync_all().is_err() {
            return;
        }
        let Ok(admission) = crate::store::platform::sync::admit_current_parent_dir_sync() else {
            return;
        };
        let _ = tmp.persist(path, admission);
    }

    /// Register a file written outside the segment sync seam (fork copy /
    /// snapshot copy): its full inner length is durable on materialization.
    fn track_materialized_file(&self, path: &Path) {
        let Ok(meta) = self.inner.metadata(path) else {
            return;
        };
        self.state.record_durable(path, meta.len);
    }
}

/// [`StoreFile`] wrapper: the durability/read-fault interposition on a handle
/// minted by the inner backend.
struct SimStoreFile {
    inner: Box<dyn StoreFile>,
    path: PathBuf,
    state: Arc<SimState>,
}

impl StoreFile for SimStoreFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        // Writes are honest; bytes become durable only when an honored sync
        // advances the recorded durable length.
        self.inner.write_all(buf)
    }

    fn sync_data(&mut self) -> io::Result<()> {
        // A dropped sync returns Ok (the store believes it durable) but does
        // NOT advance the durable length — modelling a silently-lying disk.
        // The bytes are then lost on the next crash, which is precisely the
        // violation the recovery oracle must never observe for an
        // acknowledged-durable commit.
        if self.state.fsync_dropped() {
            return Ok(());
        }
        self.state.record_durable(&self.path, self.inner.len()?);
        Ok(())
    }

    fn sync_all(&mut self) -> io::Result<()> {
        if self.state.fsync_dropped() {
            return Ok(());
        }
        self.state.record_durable(&self.path, self.inner.len()?);
        Ok(())
    }

    fn len(&self) -> io::Result<u64> {
        self.inner.len()
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read_at(offset, buf)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), PositionedReadError> {
        // A faulted read never touches the inner handle: it returns the
        // injected positioned-read error directly, so the reader's
        // active-frame read surfaces the same StoreError it would on a
        // genuinely torn read. The fault path is test-only; production
        // `Store`-over-`SimFs` fixtures compile straight to the honest
        // delegate.
        #[cfg(test)]
        if let Some(kind) = self.state.read_fault_strikes() {
            return Err(match kind {
                ReadFaultKind::Io => PositionedReadError::Io(io::Error::other(
                    "SimFs: injected positioned-read fault",
                )),
                ReadFaultKind::ShortRead { bytes_read } => {
                    PositionedReadError::ShortRead { bytes_read }
                }
            });
        }
        self.inner.read_exact_at(offset, buf)
    }

    fn as_std_file(&self) -> Option<&std::fs::File> {
        // Delegate the native escape: over RealFs the sealed-segment mmap
        // admits exactly as before (read faults target the active-segment
        // FD path, not sealed mmaps); over a virtual inner this is None and
        // the byte-identical fallback serves every read.
        self.inner.as_std_file()
    }
}

/// [`StagedFile`] wrapper. Two fault mechanisms ride the stage: the
/// atomic-publish fault ([`CrashOp::PersistTemp`]) drops the whole publish, and
/// the sync-drop schedule decides whether the staged `sync_all` makes the
/// published bytes durable — a dropped staged sync leaves the metadata publish
/// (cursor checkpoint, keyset, visibility ranges, cold-start artifact) at a zero
/// durable prefix, so `crash()` loses it.
struct SimStagedFile {
    inner: Box<dyn StagedFile>,
    state: Arc<SimState>,
    /// Total bytes written to the stage so far.
    written: u64,
    /// Bytes made durable by an honored staged sync; applied to the final path
    /// in `persist` so `crash()` truncates the publish to this prefix.
    durable_staged: u64,
}

impl StagedFile for SimStagedFile {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.written = self
            .written
            .saturating_add(u64::try_from(buf.len()).unwrap_or(u64::MAX));
        Ok(())
    }

    fn sync_all(&mut self) -> io::Result<()> {
        // Mirror `SimStoreFile::sync_all`: a dropped staged sync returns Ok but
        // does NOT advance the durable prefix, so the published bytes are lost
        // on crash. Only an honored sync makes the staged bytes durable.
        if self.state.fsync_dropped() {
            return Ok(());
        }
        self.inner.sync_all()?;
        self.durable_staged = self.written;
        Ok(())
    }

    fn persist(
        self: Box<Self>,
        final_path: &Path,
        admission: crate::store::platform::sync::ParentDirSyncAdmission,
    ) -> io::Result<()> {
        // A faulted persist drops the publish entirely: the staged bytes are
        // left un-published, so the store's belief that the metadata is
        // durable is falsified — exactly the torn atomic publish the crash
        // harness needs.
        if self.state.op_fault_strikes(CrashOp::PersistTemp) {
            return Err(SimState::injected_op_fault(CrashOp::PersistTemp));
        }
        self.inner.persist(final_path, admission)?;
        // Apply the staged durable prefix to the published path: an honored
        // staged sync makes the bytes durable (full length); a dropped one
        // leaves it at 0, so `crash()` truncates the whole publish away — the
        // metadata write whose fsync silently never reached disk is lost.
        self.state.record_durable(final_path, self.durable_staged);
        Ok(())
    }
}

impl StoreFs for SimFs {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
        self.inner.read_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn create_new_file(&self, path: &Path) -> Result<Box<dyn StoreFile>, StoreError> {
        let inner = self.inner.create_new_file(path)?;
        // Register the file with durable_len = 0; its bytes become durable
        // only as honored syncs advance the recorded length.
        self.state
            .durable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            // `insert` (not `entry().or_default()`): a recreated path — e.g.
            // compaction reusing a merged segment id — must RESET to durable_len
            // 0, not inherit the previous file's durable length, or a dropped
            // sync on the new file lets `crash()` keep never-synced bytes.
            .insert(path.to_path_buf(), DurableState::default());
        Ok(Box::new(SimStoreFile {
            inner,
            path: path.to_path_buf(),
            state: Arc::clone(&self.state),
        }))
    }

    fn open_file(&self, path: &Path) -> io::Result<Box<dyn StoreFile>> {
        // Read handles are wrapped too, so the positioned-read fault schedule
        // covers the FD-cache read path exactly as the old fs-level
        // interposition did.
        let inner = self.inner.open_file(path)?;
        Ok(Box::new(SimStoreFile {
            inner,
            path: path.to_path_buf(),
            state: Arc::clone(&self.state),
        }))
    }

    fn sync_parent_dir(&self, _path: &Path) -> Result<(), StoreError> {
        // The directory entry is modelled as always durable once the file is
        // created: the crash truncates file CONTENTS, it does not unlink files.
        Ok(())
    }

    fn reject_symlink_leaf(&self, path: &Path, purpose: &str) -> Result<(), StoreError> {
        self.inner.reject_symlink_leaf(path, purpose)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        // Reads see the inner contents; the fault model interposes durability
        // (syncs, publishes, crash truncation), not read bytes.
        self.inner.read(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileStat> {
        self.inner.symlink_metadata(path)
    }

    fn cow_copy_file(
        &self,
        from: &Path,
        to: &Path,
        preference: crate::store::CopyPreference,
    ) -> io::Result<crate::store::platform::fs::CowStrategyUsed> {
        if self.state.enospc_strikes_now() {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "SimFs: injected ENOSPC mid-fork on cow_copy_file",
            ));
        }
        let used = self.inner.cow_copy_file(from, to, preference)?;
        self.track_materialized_file(to);
        Ok(used)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        if self.state.enospc_strikes_now() {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "SimFs: injected ENOSPC mid-fork on copy",
            ));
        }
        let bytes = self.inner.copy(from, to)?;
        self.track_materialized_file(to);
        Ok(bytes)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileStat> {
        self.inner.metadata(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if self.state.op_fault_strikes(CrashOp::Rename) {
            return Err(SimState::injected_op_fault(CrashOp::Rename));
        }
        self.inner.rename(from, to)?;
        // A rename moves the inode and replaces `to`: carry the durable-length
        // bookkeeping with it so a later `crash()` truncates the renamed path
        // to ITS durable prefix, never a stale length keyed to the old path.
        // A compaction rollback renames `.compact-src` back over a recreated
        // segment id — without this the restored segment inherits the aborted
        // segment's durable length and `crash()` truncates it wrongly.
        self.state.move_durable(from, to);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        // `remove_file_if_present` is the provided default in terms of this
        // method, so both the direct reclaim and the if-present probes funnel
        // through one faultable primitive.
        //
        // EXEMPTION: the mmap-capability probe (platform evidence collection) is
        // transient infrastructure, not store data — its scratch removes must
        // NOT consume the crash-fault schedule, or an evidence probe run after
        // arming a RemoveFile fault would swallow the fault the experiment
        // intended for a real segment/reclaim remove.
        let is_probe_scratch =
            path.file_name()
                .and_then(|leaf| leaf.to_str())
                .is_some_and(|leaf| {
                    leaf.starts_with(crate::store::platform::evidence::MMAP_PROBE_PREFIX)
                });
        if !is_probe_scratch && self.state.op_fault_strikes(CrashOp::RemoveFile) {
            return Err(SimState::injected_op_fault(CrashOp::RemoveFile));
        }
        self.inner.remove_file(path)?;
        // The file is unlinked: forget its durable prefix so a recreated path
        // starts fresh and `crash()` never resurrects a stale length. The
        // `remove_dir_all` override below funnels child removals through this
        // method, so recursive removals inherit the same cleanup.
        self.state.forget_durable(path);
        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        // Clear the contents through the per-file faultable path (as the trait
        // default does) so recursive removals still model crashes and forget
        // each removed child's durable prefix via `remove_file`...
        for entry in self.read_dir(path)? {
            let child = path.join(&entry.name);
            if entry.kind == FileKind::Dir {
                self.remove_dir_all(&child)?;
            } else {
                self.remove_file_if_present(&child)?;
            }
        }
        // ...then remove the now-empty directory ENTRY itself through the inner
        // backend. The trait default stops after clearing contents, which would
        // leave the directory alive under RealFs and MemFs alike — letting
        // `clear_fork_store_artifacts` / `clear_snapshot_store_artifacts` report
        // the cursor directory removed while stale destination state survives.
        self.inner.remove_dir_all(path)
    }

    fn named_temp_in(&self, dir: &Path) -> io::Result<Box<dyn StagedFile>> {
        // The stage is wrapped so the publish fault ([`CrashOp::PersistTemp`])
        // rides the handle; staging writes/syncs are faithful delegates.
        let inner = self.inner.named_temp_in(dir)?;
        Ok(Box::new(SimStagedFile {
            inner,
            state: Arc::clone(&self.state),
            written: 0,
            durable_staged: 0,
        }))
    }

    fn try_lock_store_dir(
        &self,
        lock_path: &Path,
    ) -> Result<Option<Box<dyn StoreDirLockGuard>>, StoreError> {
        // Locking is not part of the fault model; the inner backend's
        // exclusion semantics apply unchanged.
        self.inner.try_lock_store_dir(lock_path)
    }
}

#[cfg(test)]
#[path = "fs_tests.rs"]
mod tests;
