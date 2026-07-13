//! Namespace-truth-vs-byte-truth crash simulation filesystem ([`ShadowFs`]).
//!
//! [`ShadowFs`] is a self-contained, pure-in-memory [`StoreFs`] backend that
//! models the TWO orthogonal durability truths POSIX exposes to a crashing
//! store, so a real [`crate::store::Store`] can be driven, crashed at an exact
//! protocol boundary, reopened over the surviving image, and audited. Unlike
//! [`super::fs::SimFs`] — the byte-durability fault LAYER over an arbitrary
//! inner backend, whose file NAMES are always durable by design — [`ShadowFs`]
//! owns the namespace truth: names become durable only at an honored
//! parent-directory sync, and [`ShadowFs::crash`] resurrects removed names,
//! un-renames renames, and drops uncommitted names by swapping the visible tree
//! for the durable one. The two models are complementary; see the `SimFs` docs.
//!
//! # The two-truths state machine
//!
//! * **Byte truth** (per inode): `{ bytes, durable_len }` with
//!   `durable_len <= bytes.len()`. A honored handle/staged sync sets
//!   `durable_len := bytes.len()`; writes never advance it.
//! * **Namespace truth** (per directory entry): two name→inode maps, `visible`
//!   (what a running process observes now) and `durable` (what survives a
//!   crash). `durable` changes ONLY at an honored parent-dir sync (explicit
//!   [`StoreFs::sync_parent_dir`] or the persist-implicit sync). Directories
//!   are durable-on-create — a documented simplification (file-name durability
//!   is the modeled axis).
//!
//! | op | visible effect | durable effect |
//! |---|---|---|
//! | `create_new_file(p)` | `V(p) := fresh inode` | none |
//! | handle `write_all` | `bytes += buf` | none |
//! | handle sync (honored) | — | `durable_len := bytes.len()` |
//! | `rename(f, t)` | `V(f):=⊥; V(t):=inode` | none |
//! | `remove_file(p)` | `V(p):=⊥` | none (inode kept while D references it) |
//! | staged `persist(t)` | `V(t) := fresh inode` | implicit parent sync of `t.parent()` |
//! | `sync_parent_dir(p)` (honored) | — | `D under p.parent() := V under p.parent()` |
//! | `crash()` | `V := D`; inodes truncate to `durable_len`; unreferenced inodes dropped; locks cleared | D unchanged |
//!
//! # Constructing the mandate's seven crash states
//!
//! 1. *staged name durable, final absent*: persist a staged name + honored
//!    parent sync, then `crash()` before the final rename.
//! 2. *final rename visible, not durable*: rename, `arm_parent_sync_drop` the
//!    next parent sync, `crash()` → durable still maps the old name.
//! 3. *source rename visible, not durable*: same mechanism on the source rename.
//! 4. *source removal visible, not durable*: `remove_file`, drop the parent
//!    sync, `crash()` → the name resurrects at its durable byte prefix.
//! 5. *stale staged+final names*: make both durable in different steps; crash
//!    mid-protocol → both present.
//! 6. *interrupted publication*: `arm_op_error(CrashOp::PersistTemp, n)` (the
//!    publish never lands) or a dropped staged sync (name lands, bytes truncate
//!    to the durable prefix).
//! 7. *dropped parent-dir sync at a transition*: `arm_parent_sync_drop(nth)`
//!    keyed by the 1-based global parent-sync counter (explicit + implicit share
//!    one counter; [`ShadowFs::parent_syncs_observed`] arms relative to now).
//!
//! # Crash-at-boundary recipe (Plan 05)
//!
//! Compaction and every protocol under repair are synchronous, so
//! [`ShadowFs::arm_op_error`] / [`ShadowFs::arm_op_error_at`] abort the protocol
//! AT that op boundary (post-boundary ops never run); [`ShadowFs::crash`] +
//! reopen WITHOUT `close()` then reproduces power loss at that boundary.
//! [`ShadowFs::set_freeze_on_fault`] additionally poisons the fs so an
//! in-process rollback cannot heal the namespace once the simulated power event
//! begins, and [`ShadowFs::arm_crash_at_op`] pins a boundary by absolute index
//! into [`ShadowFs::mutation_ops`]. Determinism: all decisions come from
//! monotonic per-kind occurrence counters (armed as `BTreeSet<u32>` schedules,
//! NEVER reset on crash — differs from `SimFs`'s reset-on-arm) plus an optional
//! seeded background sync-drop stream. Same construction + same op order ⇒
//! identical durable tree ⇒ identical recovered state.
//!
//! # Handles boundary
//!
//! [`ShadowFs::crash`], the public arming/observation surface, and the model
//! state live here; `impl StoreFs for ShadowFs` and the handle types live in
//! the descendant module `handles` ([`shadow_fs_handles.rs`]). That module locks
//! `ShadowFs::state` once per op and drives the model through the private
//! [`ShadowState`] methods `op_error_strikes` (CreateNew/Write/Rename/
//! RemoveFile/PersistTemp), `record_parent_sync`, and `file_sync_outcome`,
//! which record every observed op into [`ShadowFs::mutation_ops`] and return the
//! fault decision — nothing else re-implements the schedule logic.
//!
//! [`StoreFs`]: crate::store::platform::fs::StoreFs
//! [`shadow_fs_handles.rs`]: super::shadow_fs_handles

use super::fs::CrashOp;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// One observed mutating namespace/byte op, in the order the store reached it.
/// Sync events are included so a probe run can pin an absolute crash boundary
/// (see [`ShadowFs::arm_crash_at_op`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimOp {
    /// Which primitive was observed.
    pub kind: SimOpKind,
    /// The op's primary path (the source path for a rename).
    pub path: PathBuf,
    /// The destination path for a [`SimOpKind::Rename`]; `None` otherwise.
    pub to: Option<PathBuf>,
}

/// The mutating-op vocabulary [`ShadowFs::mutation_ops`] records. The five
/// namespace/byte primitives mirror [`CrashOp`]; the two sync events carry no
/// [`CrashOp`] because they fault through the parent/file-sync schedules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimOpKind {
    /// [`StoreFs::create_new_file`](crate::store::platform::fs::StoreFs::create_new_file).
    CreateNew,
    /// A handle `write_all` append.
    Write,
    /// [`StoreFs::rename`](crate::store::platform::fs::StoreFs::rename).
    Rename,
    /// [`StoreFs::remove_file`](crate::store::platform::fs::StoreFs::remove_file).
    RemoveFile,
    /// A staged-file `persist` (atomic publish).
    PersistTemp,
    /// A handle/staged `sync_all`/`sync_data`.
    SyncFile,
    /// A parent-directory sync (explicit or persist-implicit).
    SyncParentDir,
}

/// The outcome of a parent-dir sync or a file-handle sync: honored (durability
/// applied), silently dropped (returns `Ok` but nothing became durable — a
/// lying disk), or errored (the injected I/O error the store must surface).
pub(crate) enum ParentSyncOutcome {
    /// The sync was honored; the caller applies the durability effect.
    Applied,
    /// The sync was silently dropped; the caller returns `Ok` with no effect.
    Dropped,
    /// The sync failed with an injected error the caller must surface.
    Errored(io::Error),
}

/// One inode: the append-only byte buffer and the prefix a sync made durable.
#[derive(Default)]
struct ShadowInode {
    bytes: Vec<u8>,
    durable_len: usize,
}

/// The namespace + byte tree. `visible`/`durable` map file names to inode ids;
/// `dirs` holds durable-on-create directories; `locks` is the in-process lock
/// registry. `next_inode` is monotonic and never reused (crash keeps it).
#[derive(Default)]
struct ShadowTree {
    inodes: BTreeMap<u64, ShadowInode>,
    visible: BTreeMap<PathBuf, u64>,
    durable: BTreeMap<PathBuf, u64>,
    dirs: BTreeSet<PathBuf>,
    locks: BTreeSet<PathBuf>,
    next_inode: u64,
}

impl ShadowTree {
    /// Mint a fresh inode with `bytes`/`durable_len` and return its id.
    fn mint_inode(&mut self, bytes: Vec<u8>, durable_len: usize) -> u64 {
        let id = self.next_inode;
        self.next_inode = self.next_inode.wrapping_add(1);
        self.inodes.insert(id, ShadowInode { bytes, durable_len });
        id
    }
}

/// A deterministic 1-in-N background sync-drop stream (splitmix64), independent
/// of `fastrand` so [`ShadowFs`] compiles on the clean `conformance-harness`
/// surface where `fastrand` is not linked.
struct SeededDrops {
    state: u64,
    one_in: u32,
}

impl SeededDrops {
    /// Advance the stream once and return the next value (splitmix64).
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Draw once (always advancing the stream, so the schedule is a pure
    /// function of sync-seam order) and decide whether this sync is dropped.
    fn drops(&mut self) -> bool {
        let roll = self.next_u64();
        self.one_in != 0 && roll.is_multiple_of(u64::from(self.one_in))
    }
}

/// A path-suffix-targeted op-error schedule: the `nth` op of kind `op` whose
/// path (or rename source) ends with `suffix` faults. `seen` counts matching
/// ops reached so far and is monotonic across the run.
struct OpErrorAt {
    op: u8,
    suffix: String,
    nth: u32,
    seen: u32,
}

/// All armed fault schedules and monotonic observation counters. Armed one-shot
/// sets and the poison flags are cleared by [`ShadowFs::crash`]; the counters,
/// the fired witness, the seeded stream, and the op log persist.
#[derive(Default)]
struct ShadowSchedules {
    parent_sync_drops: BTreeSet<u32>,
    parent_sync_drop_all: bool,
    parent_sync_errors: BTreeSet<u32>,
    parent_syncs_seen: u32,
    op_errors: BTreeMap<u8, BTreeSet<u32>>,
    op_errors_at: Vec<OpErrorAt>,
    ops_seen: BTreeMap<u8, u32>,
    file_sync_drops: BTreeSet<u32>,
    file_sync_errors: BTreeSet<u32>,
    file_syncs_seen: u32,
    crash_at_op: Option<usize>,
    freeze_on_fault: bool,
    frozen: bool,
    fired: u32,
    seeded: Option<SeededDrops>,
    ops_log: Vec<SimOp>,
}

impl ShadowSchedules {
    /// Reset the poison flags and every one-shot armed schedule so a reopen over
    /// a crashed fs runs clean; the monotonic counters, `fired`, the seeded
    /// stream, and the op log survive the power event.
    fn reset_after_crash(&mut self) {
        self.parent_sync_drops.clear();
        self.parent_sync_drop_all = false;
        self.parent_sync_errors.clear();
        self.op_errors.clear();
        self.op_errors_at.clear();
        self.file_sync_drops.clear();
        self.file_sync_errors.clear();
        self.crash_at_op = None;
        self.freeze_on_fault = false;
        self.frozen = false;
    }

    /// Advance the path-suffix schedules for a `key`-kind op at `path` and
    /// report whether one of them strikes this occurrence.
    fn op_error_at_strikes(&mut self, key: u8, path: &Path) -> bool {
        for entry in self.op_errors_at.iter_mut() {
            if entry.op == key && path_ends_with(path, &entry.suffix) {
                entry.seen = entry.seen.saturating_add(1);
                if entry.seen == entry.nth {
                    return true;
                }
            }
        }
        false
    }
}

/// The mutable model state behind the shared mutex.
#[derive(Default)]
struct ShadowState {
    tree: ShadowTree,
    sched: ShadowSchedules,
}

impl ShadowState {
    /// Record a mutating namespace/byte op and decide whether it must fault
    /// (nothing applied): a frozen fs after a prior fault, an absolute-index
    /// crash point, a path-suffix/`nth` strike, or a per-kind `nth` strike.
    /// Called by the handles module for CreateNew/Write/Rename/RemoveFile/
    /// PersistTemp; `to` carries the rename destination.
    fn op_error_strikes(&mut self, op: CrashOp, path: &Path, to: Option<&Path>) -> Option<io::Error> {
        let key = crashop_key(op);
        let index = self.sched.ops_log.len();
        self.sched.ops_log.push(SimOp {
            kind: simopkind_of(op),
            path: path.to_path_buf(),
            to: to.map(Path::to_path_buf),
        });
        let counter = self.sched.ops_seen.entry(key).or_insert(0);
        *counter = counter.saturating_add(1);
        let n = *counter;

        // A poisoned fs fails every subsequent mutating op — the original fault
        // already counted, so a cascade block does not advance `fired`.
        if self.sched.frozen {
            return Some(injected("mutating op after fault-poison"));
        }
        if self.sched.crash_at_op == Some(index) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            self.sched.frozen = true;
            return Some(injected("crash-at-op boundary"));
        }
        if self.sched.op_error_at_strikes(key, path) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            if self.sched.freeze_on_fault {
                self.sched.frozen = true;
            }
            return Some(injected("path-suffix op error"));
        }
        let strikes = self
            .sched
            .op_errors
            .get(&key)
            .is_some_and(|set| set.contains(&n));
        if strikes {
            self.sched.fired = self.sched.fired.saturating_add(1);
            if self.sched.freeze_on_fault {
                self.sched.frozen = true;
            }
            return Some(injected("per-kind op error"));
        }
        None
    }

    /// Record a parent-dir sync (explicit or persist-implicit) and apply its
    /// outcome. On [`ParentSyncOutcome::Applied`] the durable names under
    /// `parent` are set equal to the visible names under `parent` (names only —
    /// byte durability stays per-inode). Called by the handles module.
    fn record_parent_sync(&mut self, parent: Option<&Path>) -> ParentSyncOutcome {
        let index = self.sched.ops_log.len();
        self.sched.ops_log.push(SimOp {
            kind: SimOpKind::SyncParentDir,
            path: parent.map(Path::to_path_buf).unwrap_or_default(),
            to: None,
        });
        self.sched.parent_syncs_seen = self.sched.parent_syncs_seen.saturating_add(1);
        let n = self.sched.parent_syncs_seen;

        if self.sched.crash_at_op == Some(index) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            self.sched.frozen = true;
            return ParentSyncOutcome::Errored(injected("crash-at-op on parent sync"));
        }
        if self.sched.parent_sync_errors.contains(&n) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            return ParentSyncOutcome::Errored(injected("parent-dir sync"));
        }
        if self.sched.parent_sync_drop_all || self.sched.parent_sync_drops.contains(&n) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            return ParentSyncOutcome::Dropped;
        }
        if let Some(parent) = parent {
            self.apply_parent_sync(parent);
        }
        ParentSyncOutcome::Applied
    }

    /// Copy the visible file names under `parent` into the durable map,
    /// dropping durable names under `parent` that are no longer visible.
    fn apply_parent_sync(&mut self, parent: &Path) {
        let stale: Vec<PathBuf> = self
            .tree
            .durable
            .keys()
            .filter(|name| name.parent() == Some(parent))
            .cloned()
            .collect();
        for name in stale {
            self.tree.durable.remove(&name);
        }
        let fresh: Vec<(PathBuf, u64)> = self
            .tree
            .visible
            .iter()
            .filter(|(name, _)| name.parent() == Some(parent))
            .map(|(name, id)| (name.clone(), *id))
            .collect();
        for (name, id) in fresh {
            self.tree.durable.insert(name, id);
        }
    }

    /// Record a handle/staged sync and decide its outcome. The caller applies
    /// the byte-durability advance on [`ParentSyncOutcome::Applied`]. `path` is
    /// the handle path when known (staged syncs pass `None`).
    fn file_sync_outcome(&mut self, path: Option<&Path>) -> ParentSyncOutcome {
        let index = self.sched.ops_log.len();
        self.sched.ops_log.push(SimOp {
            kind: SimOpKind::SyncFile,
            path: path.map(Path::to_path_buf).unwrap_or_default(),
            to: None,
        });
        self.sched.file_syncs_seen = self.sched.file_syncs_seen.saturating_add(1);
        let n = self.sched.file_syncs_seen;
        let background_drop = self
            .sched
            .seeded
            .as_mut()
            .is_some_and(SeededDrops::drops);

        if self.sched.crash_at_op == Some(index) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            self.sched.frozen = true;
            return ParentSyncOutcome::Errored(injected("crash-at-op on file sync"));
        }
        if self.sched.file_sync_errors.contains(&n) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            return ParentSyncOutcome::Errored(injected("file sync"));
        }
        if self.sched.file_sync_drops.contains(&n) {
            self.sched.fired = self.sched.fired.saturating_add(1);
            return ParentSyncOutcome::Dropped;
        }
        if background_drop {
            return ParentSyncOutcome::Dropped;
        }
        ParentSyncOutcome::Applied
    }
}

/// Namespace-truth crash simulation backend. `Clone` shares the tree (`Arc`
/// state, `MemFs` precedent), so a config holder and a test observe one store's
/// image. See the module docs for the two-truths model and construction
/// recipes.
#[derive(Clone, Default)]
pub struct ShadowFs {
    state: Arc<Mutex<ShadowState>>,
}

impl ShadowFs {
    /// An empty namespace-truth simulation filesystem.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A simulation with an optional deterministic 1-in-`fsync_drop_one_in`
    /// background sync-drop stream layered ON TOP of any explicit schedules
    /// (`0` disables it). The stream is a pure function of sync-seam order, so
    /// the same construction + op order reproduces the same drops.
    #[must_use]
    pub fn with_seeded_sync_drops(seed: u64, fsync_drop_one_in: u32) -> Self {
        let fs = Self::new();
        fs.state().sched.seeded = Some(SeededDrops {
            state: seed,
            one_in: fsync_drop_one_in,
        });
        fs
    }

    /// Lock the model state, healing a poisoned mutex (`SimFs`/`MemFs`
    /// precedent — never `.unwrap()`). Private (returns a private type); the
    /// descendant handles module reaches it to drive the model through the
    /// [`ShadowState`] methods.
    fn state(&self) -> std::sync::MutexGuard<'_, ShadowState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Silently drop (return `Ok`, apply nothing) the `nth` 1-based
    /// parent-dir-sync event (explicit `sync_parent_dir` + persist-implicit
    /// share ONE monotonic counter).
    pub fn arm_parent_sync_drop(&self, nth: u32) {
        self.state().sched.parent_sync_drops.insert(nth);
    }

    /// Silently drop every parent-dir-sync event until [`ShadowFs::crash`].
    pub fn arm_parent_sync_drop_all(&self) {
        self.state().sched.parent_sync_drop_all = true;
    }

    /// The `nth` 1-based parent-dir-sync event returns an injected I/O error.
    pub fn arm_parent_sync_error(&self, nth: u32) {
        self.state().sched.parent_sync_errors.insert(nth);
    }

    /// The `nth` 1-based occurrence of `op` returns an injected I/O error with
    /// nothing applied (torn atomic op). Counters are monotonic and never reset,
    /// so arm relative to [`ShadowFs::ops_observed`].
    pub fn arm_op_error(&self, op: CrashOp, nth: u32) {
        self.state()
            .sched
            .op_errors
            .entry(crashop_key(op))
            .or_default()
            .insert(nth);
    }

    /// The `nth` 1-based matching op of kind `op` whose path (or rename source)
    /// ends with `path_suffix` fails with an injected I/O error, nothing
    /// applied.
    pub fn arm_op_error_at(&self, op: CrashOp, path_suffix: &str, nth: u32) {
        self.state().sched.op_errors_at.push(OpErrorAt {
            op: crashop_key(op),
            suffix: path_suffix.to_string(),
            nth,
            seen: 0,
        });
    }

    /// When `true`, any armed fault that fires poisons the fs: every subsequent
    /// MUTATING op fails with an injected error (reads unaffected) until
    /// [`ShadowFs::crash`]. Default `false`.
    pub fn set_freeze_on_fault(&self, freeze: bool) {
        self.state().sched.freeze_on_fault = freeze;
    }

    /// Silently drop the `nth` 1-based handle/staged sync (lying disk).
    pub fn arm_file_sync_drop(&self, nth: u32) {
        self.state().sched.file_sync_drops.insert(nth);
    }

    /// The `nth` 1-based handle/staged sync returns an injected I/O error.
    pub fn arm_file_sync_error(&self, nth: u32) {
        self.state().sched.file_sync_errors.insert(nth);
    }

    /// The op at `absolute_index` (0-based into [`ShadowFs::mutation_ops`]
    /// order) fails AND poisons the fs regardless of the freeze flag — the
    /// workload-boundary crash walker.
    pub fn arm_crash_at_op(&self, absolute_index: usize) {
        self.state().sched.crash_at_op = Some(absolute_index);
    }

    /// Simulate power loss: `visible := durable`; every durable-referenced inode
    /// truncates to its durable byte prefix; unreferenced inodes are dropped;
    /// unpersisted stages and the lock registry are cleared, and the poison
    /// flags plus one-shot armed schedules reset so a reopen runs clean.
    /// Precondition: the store over this fs was dropped/abandoned first.
    pub fn crash(&self) {
        let mut guard = self.state();
        let state = &mut *guard;
        Self::crash_namespace(&mut state.tree);
        Self::crash_bytes(&mut state.tree);
        state.tree.locks.clear();
        state.sched.reset_after_crash();
    }

    /// Namespace half of a crash: the visible file names become exactly the
    /// durable ones (directories are durable-on-create and untouched).
    fn crash_namespace(tree: &mut ShadowTree) {
        tree.visible = tree.durable.clone();
    }

    /// Byte half of a crash: truncate every durable-referenced inode to its
    /// durable prefix and drop inodes no durable name references.
    fn crash_bytes(tree: &mut ShadowTree) {
        let live: BTreeSet<u64> = tree.durable.values().copied().collect();
        tree.inodes.retain(|id, inode| {
            if live.contains(id) {
                inode.bytes.truncate(inode.durable_len);
                true
            } else {
                false
            }
        });
    }

    /// The 1-based count of parent-dir-sync events observed so far (monotonic).
    #[must_use]
    pub fn parent_syncs_observed(&self) -> u32 {
        self.state().sched.parent_syncs_seen
    }

    /// The count of `op`-kind ops observed so far (monotonic; never reset).
    #[must_use]
    pub fn ops_observed(&self, op: CrashOp) -> u32 {
        self.state()
            .sched
            .ops_seen
            .get(&crashop_key(op))
            .copied()
            .unwrap_or(0)
    }

    /// Anti-vacuity witness: how many armed faults actually fired.
    #[must_use]
    pub fn armed_faults_fired(&self) -> u32 {
        self.state().sched.fired
    }

    /// The ordered log of every mutating namespace/byte op observed (sync events
    /// included), for absolute-index crash-boundary pinning.
    #[must_use]
    pub fn mutation_ops(&self) -> Vec<SimOp> {
        self.state().sched.ops_log.clone()
    }

    /// The immediate child names (files + directories) visible now under `dir`.
    #[must_use]
    pub fn visible_entries(&self, dir: &Path) -> Vec<OsString> {
        let guard = self.state();
        entries_under(&guard.tree.visible, &guard.tree.dirs, dir)
    }

    /// The immediate child names (files + directories) that would survive a
    /// crash under `dir`.
    #[must_use]
    pub fn durable_entries(&self, dir: &Path) -> Vec<OsString> {
        let guard = self.state();
        entries_under(&guard.tree.durable, &guard.tree.dirs, dir)
    }

    /// Whether `path`'s name is durable (survives a crash).
    #[must_use]
    pub fn is_name_durable(&self, path: &Path) -> bool {
        self.state().tree.durable.contains_key(path)
    }

    /// The durable byte length of the durable name at `path`, or `None` when the
    /// name is not durable.
    #[must_use]
    pub fn durable_byte_len(&self, path: &Path) -> Option<u64> {
        let guard = self.state();
        let id = guard.tree.durable.get(path)?;
        let inode = guard.tree.inodes.get(id)?;
        Some(u64::try_from(inode.durable_len).unwrap_or(u64::MAX))
    }
}

/// The stable u8 key for a [`CrashOp`] (the enum is not `Ord`, so it cannot key
/// a `BTreeMap` directly). Exhaustive over the promoted five-variant set.
fn crashop_key(op: CrashOp) -> u8 {
    match op {
        CrashOp::Rename => 0,
        CrashOp::RemoveFile => 1,
        CrashOp::PersistTemp => 2,
        CrashOp::CreateNew => 3,
        CrashOp::Write => 4,
    }
}

/// The [`SimOpKind`] a mutating [`CrashOp`] records in the op log.
fn simopkind_of(op: CrashOp) -> SimOpKind {
    match op {
        CrashOp::Rename => SimOpKind::Rename,
        CrashOp::RemoveFile => SimOpKind::RemoveFile,
        CrashOp::PersistTemp => SimOpKind::PersistTemp,
        CrashOp::CreateNew => SimOpKind::CreateNew,
        CrashOp::Write => SimOpKind::Write,
    }
}

/// Whether `path` ends with `suffix` (lossy — sim paths are literal ASCII).
fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().ends_with(suffix)
}

/// The injected I/O error a faulted sim op returns.
fn injected(context: &str) -> io::Error {
    io::Error::other(format!("ShadowFs: injected fault ({context})"))
}

/// The immediate child leaf names under `dir` from a name→inode map plus the
/// directory set, sorted for deterministic comparison.
fn entries_under(
    names: &BTreeMap<PathBuf, u64>,
    dirs: &BTreeSet<PathBuf>,
    dir: &Path,
) -> Vec<OsString> {
    let mut out = Vec::new();
    for name in names.keys() {
        if name.parent() == Some(dir) {
            if let Some(leaf) = name.file_name() {
                out.push(leaf.to_os_string());
            }
        }
    }
    for child in dirs {
        if child.parent() == Some(dir) {
            if let Some(leaf) = child.file_name() {
                out.push(leaf.to_os_string());
            }
        }
    }
    out.sort();
    out
}

#[path = "shadow_fs_handles.rs"]
mod handles;

#[cfg(test)]
#[path = "shadow_fs_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shadow_fs_arming_tests.rs"]
mod arming_tests;
