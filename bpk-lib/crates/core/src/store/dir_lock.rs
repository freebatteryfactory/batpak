use crate::store::platform::fs::{StoreDirLockGuard, StoreFs};
use crate::store::{StoreError, StoreLockMode};
use std::path::Path;

pub(crate) const STORE_LOCK_FILENAME: &str = ".batpak.lock";

/// Lifetime-held directory lock for a store root.
///
/// The underlying `File` owns the OS lock. Keeping this guard inside `Store`
/// guarantees the open mode remains reserved for the handle's full lifetime.
///
/// # Boundary (advisory lock)
/// This is an **advisory** OS lock (`flock`-class via `File::try_lock`): it only
/// excludes other processes that also take this lock through `acquire`. A
/// process that opens the store path WITHOUT cooperating can still read — and,
/// critically, `mmap` — a sealed segment that this owner assumes is immutable,
/// which violates the mmap immutability SAFETY contract (undefined behavior).
/// Advisory locks are also unreliable on networked filesystems (NFS/CIFS).
/// Single-process exclusion is covered in `store_locking.rs`; cross-process
/// refusal is witnessed by `dir_lock_two_process.rs` (0.8.3 audit C3).
pub(crate) struct StoreDirLock {
    _guard: Box<dyn StoreDirLockGuard>,
}

impl StoreDirLock {
    pub(crate) fn acquire(
        data_dir: &Path,
        mode: StoreLockMode,
        fs: &dyn StoreFs,
    ) -> Result<Self, StoreError> {
        let canonical_dir = fs.canonicalize(data_dir).map_err(StoreError::Io)?;
        let path = canonical_dir.join(STORE_LOCK_FILENAME);

        // Store opens are intentionally exclusive. Read-only handles are
        // rejected while any live owner exists until shared semantics are
        // explicitly designed and tested. The exclusion itself is the
        // backend's ([`StoreFs::try_lock_store_dir`]): the OS advisory lock
        // for RealFs, an in-process registry for virtual backends.
        match fs.try_lock_store_dir(&path)? {
            Some(guard) => Ok(Self { _guard: guard }),
            None => Err(StoreError::StoreLocked {
                path: canonical_dir,
                mode,
            }),
        }
    }
}
