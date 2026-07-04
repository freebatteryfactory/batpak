//! wasmi + WASI p1 runner for [`WasmBackend`](super::WasmBackend).
//!
//! The runner installs only the capabilities produced by `plan_build`: explicit
//! preopens, explicit env, stdio capture pipes, memory limits, and fuel. There is no
//! ambient filesystem inheritance and no socket capability installation.
//!
//! The confinement directory keeps all policy logic in synchronous `*_sync`
//! functions. `WasiDir` is future-shaped, so the trait methods are thin adapters
//! that return already-resolved futures; delegated wasi-common sync futures are
//! resolved with the local poll-once helper. No runtime or wait points are used
//! in this layer.

use super::plan_build::WasmRunConfig;
use super::poll_once::resolve_ready;
use super::warm::WarmCache;
use crate::contract::capability::FsAccess;
use crate::contract::report::{ExitStatus, Outcome};
use std::any::Any;
use std::future::{ready, Future};
use std::io::{Result as IoResult, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;
use wasmi::{Error as WasmiError, Linker, Store, StoreLimits, StoreLimitsBuilder, TrapCode};
use wasmi_wasi::sync::dir::{Dir as SyncDir, OpenResult as SyncOpenResult};
use wasmi_wasi::sync::{ambient_authority, clocks_ctx, random_ctx, sched_ctx, Dir as CapDir};
use wasmi_wasi::wasi_common::pipe::WritePipe;
use wasmi_wasi::wasi_common::table::Table;
use wasmi_wasi::wasi_common::{
    dir::{OpenResult, ReaddirCursor, ReaddirEntity, WasiDir},
    file::{FdFlags, Filestat, OFlags},
    Error, ErrorExt, SystemTimeSpec,
};
use wasmi_wasi::WasiCtx;

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

/// A boxed, already-resolved future returned by the sync `WasiDir` adapters.
type ReadyFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type ReaddirEntries = Box<dyn Iterator<Item = Result<ReaddirEntity, Error>> + Send>;

/// The terminal state of a WASI guest run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WasmTerminal {
    /// `_start` exited normally with this WASI code.
    Exit(i32),
    /// The guest trapped or failed through WASI.
    Failed(String),
    /// Wasmtime fuel was exhausted.
    Timeout(String),
    /// The backend could not honor the workload shape.
    Unsupported(String),
    /// The supervisor/setup path faulted.
    SupervisorFault(String),
}

impl WasmTerminal {
    /// Map the terminal to the report outcome.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        match self {
            Self::Exit(0) => Outcome::Completed,
            Self::Exit(_) | Self::Failed(_) => Outcome::Failed,
            Self::Timeout(_) => Outcome::Timeout,
            Self::Unsupported(_) => Outcome::Unsupported,
            Self::SupervisorFault(_) => Outcome::SupervisorFault,
        }
    }

    /// The portable exit status when the guest reached a WASI process exit.
    #[must_use]
    pub fn exit(&self) -> Option<ExitStatus> {
        match self {
            Self::Exit(code) => Some(ExitStatus::Code(*code)),
            Self::Failed(_)
            | Self::Timeout(_)
            | Self::Unsupported(_)
            | Self::SupervisorFault(_) => None,
        }
    }

    /// Stable terminal detail for report facts.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Exit(code) => format!("exit_code={code}"),
            Self::Failed(detail)
            | Self::Timeout(detail)
            | Self::Unsupported(detail)
            | Self::SupervisorFault(detail) => detail.clone(),
        }
    }
}

/// What the wasmi runner observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WasmRunObservation {
    pub module_ref: String,
    pub terminal: WasmTerminal,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub fuel_consumed: Option<u64>,
    pub wall_micros: Option<u64>,
    pub filesystem_confined: bool,
    pub notes: Vec<String>,
}

struct WasiStoreState {
    wasi: WasiCtx,
    limits: StoreLimits,
}

#[derive(Debug)]
struct CaptureBuffer {
    bytes: Vec<u8>,
    truncated: bool,
}

impl CaptureBuffer {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(MAX_CAPTURE_BYTES.min(8192)),
            truncated: false,
        }
    }

    fn into_parts(self) -> (Vec<u8>, bool) {
        (self.bytes, self.truncated)
    }
}

impl Write for CaptureBuffer {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(self.bytes.len());
        let accepted = remaining.min(buf.len());
        self.bytes.extend_from_slice(&buf[..accepted]);
        if accepted < buf.len() {
            self.truncated = true;
        }
        Ok(buf.len())
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> IoResult<usize> {
        let mut total = 0usize;
        for buf in bufs {
            self.write(buf)?;
            total = total.saturating_add(buf.len());
        }
        Ok(total)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

/// Instantiate and run the configured guest.
///
/// Returns the run observation; its `terminal` carries the outcome — a guest
/// runtime terminal (`Exit`/`Failed`/`Timeout`) or a setup-failed-closed terminal
/// (`Unsupported`/`SupervisorFault`).
pub(super) fn run(warm: &WarmCache, config: &WasmRunConfig) -> WasmRunObservation {
    let module_ref = config.module_ref.clone();
    let mut notes = config.notes.clone();
    let bytes = match read_module(config, &mut notes) {
        Ok(bytes) => bytes,
        Err(observation) => return *observation,
    };

    // Phase 1: resolve the compiled module from the warm cache (compile only on a
    // miss) against the shared engine; both the engine and the linker below are
    // built from `warm`, never per-run from scratch.
    let module = match warm.resolve(&bytes) {
        Ok(module) => module,
        Err(error) => {
            notes.push(format!("module_compile_failed={error}"));
            return observation(
                config,
                WasmTerminal::Unsupported(error.to_string()),
                Vec::new(),
                Vec::new(),
                None,
                None,
                notes,
            );
        }
    };

    let stdout_pipe = WritePipe::new(CaptureBuffer::new());
    let stderr_pipe = WritePipe::new(CaptureBuffer::new());
    let state = match wasi_state(config, stdout_pipe.clone(), stderr_pipe.clone()) {
        Ok(state) => state,
        Err(error) => {
            notes.push(format!("wasi_ctx_failed={error}"));
            return observation(
                config,
                WasmTerminal::SupervisorFault(error),
                Vec::new(),
                Vec::new(),
                None,
                None,
                notes,
            );
        }
    };

    let mut linker: Linker<WasiStoreState> = Linker::new(warm.engine());
    if let Err(error) = wasmi_wasi::add_to_linker(&mut linker, |state| &mut state.wasi) {
        notes.push(format!("wasi_linker_failed={error}"));
        return observation(
            config,
            WasmTerminal::SupervisorFault(error.to_string()),
            Vec::new(),
            Vec::new(),
            None,
            None,
            notes,
        );
    }

    let mut store = Store::new(warm.engine(), state);
    store.limiter(|state| &mut state.limits);
    if let Err(error) = store.set_fuel(config.fuel) {
        notes.push(format!("fuel_install_failed={error}"));
        return observation(
            config,
            WasmTerminal::SupervisorFault(error.to_string()),
            Vec::new(),
            Vec::new(),
            None,
            None,
            notes,
        );
    }

    let started = Instant::now();
    let terminal = match linker.instantiate_and_start(&mut store, &module) {
        Ok(instance) => match instance.get_typed_func::<(), ()>(&mut store, "_start") {
            Ok(start) => match start.call(&mut store, ()) {
                Ok(()) => WasmTerminal::Exit(0),
                Err(error) => terminal_from_error(&error),
            },
            Err(error) => WasmTerminal::SupervisorFault(error.to_string()),
        },
        Err(error) => terminal_from_error(&error),
    };
    let wall_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let fuel_consumed = store
        .get_fuel()
        .ok()
        .map(|remaining| config.fuel.saturating_sub(remaining));
    drop(store);
    let stdout = pipe_contents(stdout_pipe, &mut notes, "stdout");
    let stderr = pipe_contents(stderr_pipe, &mut notes, "stderr");
    cleanup_temp_roots(config, &mut notes);

    WasmRunObservation {
        module_ref,
        terminal,
        stdout,
        stderr,
        fuel_consumed,
        wall_micros: Some(wall_micros),
        filesystem_confined: config.filesystem_confined,
        notes,
    }
}

fn read_module(
    config: &WasmRunConfig,
    notes: &mut Vec<String>,
) -> Result<Vec<u8>, Box<WasmRunObservation>> {
    match std::fs::read(&config.module_ref) {
        Ok(bytes) => Ok(bytes),
        Err(error) => {
            notes.push(format!("module_read_failed={error}"));
            Err(Box::new(observation(
                config,
                WasmTerminal::SupervisorFault(error.to_string()),
                Vec::new(),
                Vec::new(),
                None,
                None,
                std::mem::take(notes),
            )))
        }
    }
}

fn wasi_state(
    config: &WasmRunConfig,
    stdout: WritePipe<CaptureBuffer>,
    stderr: WritePipe<CaptureBuffer>,
) -> Result<WasiStoreState, String> {
    let mut wasi = WasiCtx::new(random_ctx(), clocks_ctx(), sched_ctx(), Table::new());
    wasi.set_stdout(Box::new(stdout));
    wasi.set_stderr(Box::new(stderr));
    for arg in &config.args {
        wasi.push_arg(arg)
            .map_err(|error| format!("wasi arg failed: {error}"))?;
    }
    for (name, value) in &config.env {
        wasi.push_env(name, value)
            .map_err(|error| format!("wasi env failed: {error}"))?;
    }
    for preopen in &config.preopens {
        let dir =
            CapDir::open_ambient_dir(&preopen.host_path, ambient_authority()).map_err(|error| {
                format!(
                    "open preopen {} failed: {error}",
                    preopen.host_path.display()
                )
            })?;
        let limited = AccessLimitedDir::new(SyncDir::from_cap_std(dir), preopen.access);
        wasi.push_preopened_dir(Box::new(limited), &preopen.guest_path)
            .map_err(|error| {
                format!(
                    "preopen {} as {} failed: {error}",
                    preopen.host_path.display(),
                    preopen.guest_path
                )
            })?;
    }
    let limits = StoreLimitsBuilder::new()
        .memory_size(config.memory_limit)
        .instances(1)
        .memories(4)
        .tables(16)
        .trap_on_grow_failure(true)
        .build();
    Ok(WasiStoreState { wasi, limits })
}

struct AccessLimitedDir {
    inner: SyncDir,
    access: FsAccess,
}

impl AccessLimitedDir {
    fn new(inner: SyncDir, access: FsAccess) -> Self {
        Self { inner, access }
    }

    fn can_read(&self) -> bool {
        matches!(self.access, FsAccess::Read | FsAccess::ReadWrite)
    }

    fn can_write(&self) -> bool {
        matches!(self.access, FsAccess::Write | FsAccess::ReadWrite)
    }

    fn check_read(&self, operation: &str) -> Result<(), Error> {
        if self.can_read() {
            Ok(())
        } else {
            Err(Error::perm().context(operation))
        }
    }

    fn check_write(&self, operation: &str) -> Result<(), Error> {
        if self.can_write() {
            Ok(())
        } else {
            Err(Error::perm().context(operation))
        }
    }

    fn wrap_open_result(&self, result: SyncOpenResult) -> OpenResult {
        match result {
            SyncOpenResult::File(file) => OpenResult::File(Box::new(file)),
            SyncOpenResult::Dir(dir) => OpenResult::Dir(Box::new(Self::new(dir, self.access))),
        }
    }

    fn open_file_sync(
        &self,
        symlink_follow: bool,
        path: &str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> Result<OpenResult, Error> {
        if read {
            self.check_read("open read")?;
        }
        if write || oflags.intersects(OFlags::CREATE | OFlags::EXCLUSIVE | OFlags::TRUNCATE) {
            self.check_write("open write")?;
        }
        let result = self
            .inner
            .open_file_(symlink_follow, path, oflags, read, write, fdflags)?;
        Ok(self.wrap_open_result(result))
    }

    fn create_dir_sync(&self, path: &str) -> Result<(), Error> {
        self.check_write("create dir")?;
        now(resolve_ready(self.inner.create_dir(path)))
    }

    fn readdir_sync(&self, cursor: ReaddirCursor) -> Result<ReaddirEntries, Error> {
        self.check_read("readdir")?;
        now(resolve_ready(self.inner.readdir(cursor)))
    }

    fn symlink_sync(&self, old_path: &str, new_path: &str) -> Result<(), Error> {
        self.check_write("symlink")?;
        now(resolve_ready(self.inner.symlink(old_path, new_path)))
    }

    fn remove_dir_sync(&self, path: &str) -> Result<(), Error> {
        self.check_write("remove dir")?;
        now(resolve_ready(self.inner.remove_dir(path)))
    }

    fn unlink_file_sync(&self, path: &str) -> Result<(), Error> {
        self.check_write("unlink file")?;
        now(resolve_ready(self.inner.unlink_file(path)))
    }

    fn read_link_sync(&self, path: &str) -> Result<PathBuf, Error> {
        self.check_read("read link")?;
        now(resolve_ready(self.inner.read_link(path)))
    }

    fn get_filestat_sync(&self) -> Result<Filestat, Error> {
        self.check_read("dir filestat")?;
        now(resolve_ready(self.inner.get_filestat()))
    }

    fn get_path_filestat_sync(&self, path: &str, follow_symlinks: bool) -> Result<Filestat, Error> {
        self.check_read("path filestat")?;
        now(resolve_ready(
            self.inner.get_path_filestat(path, follow_symlinks),
        ))
    }

    fn rename_sync(
        &self,
        path: &str,
        dest_dir: &dyn WasiDir,
        dest_path: &str,
    ) -> Result<(), Error> {
        self.check_write("rename source")?;
        let dest_dir = dest_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or_else(|| Error::badf().context("failed downcast to wasm access-limited dir"))?;
        dest_dir.check_write("rename destination")?;
        self.inner.rename_(path, &dest_dir.inner, dest_path)
    }

    fn hard_link_sync(
        &self,
        path: &str,
        target_dir: &dyn WasiDir,
        target_path: &str,
    ) -> Result<(), Error> {
        self.check_write("hard link source")?;
        let target_dir = target_dir
            .as_any()
            .downcast_ref::<Self>()
            .ok_or_else(|| Error::badf().context("failed downcast to wasm access-limited dir"))?;
        target_dir.check_write("hard link target")?;
        self.inner.hard_link_(path, &target_dir.inner, target_path)
    }

    fn set_times_sync(
        &self,
        path: &str,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> Result<(), Error> {
        self.check_write("set times")?;
        now(resolve_ready(self.inner.set_times(
            path,
            atime,
            mtime,
            follow_symlinks,
        )))
    }
}

fn now<T>(result: Option<Result<T, Error>>) -> Result<T, Error> {
    result.unwrap_or_else(|| Err(Error::io().context("wasi-common sync future returned Pending")))
}

impl WasiDir for AccessLimitedDir {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn open_file<'life0, 'life1, 'ready>(
        &'life0 self,
        symlink_follow: bool,
        path: &'life1 str,
        oflags: OFlags,
        read: bool,
        write: bool,
        fdflags: FdFlags,
    ) -> ReadyFut<'ready, Result<OpenResult, Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.open_file_sync(
            symlink_follow,
            path,
            oflags,
            read,
            write,
            fdflags,
        )))
    }

    fn create_dir<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.create_dir_sync(path)))
    }

    fn readdir<'life0, 'ready>(
        &'life0 self,
        cursor: ReaddirCursor,
    ) -> ReadyFut<'ready, Result<ReaddirEntries, Error>>
    where
        'life0: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.readdir_sync(cursor)))
    }

    fn symlink<'life0, 'life1, 'life2, 'ready>(
        &'life0 self,
        old_path: &'life1 str,
        new_path: &'life2 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        'life2: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.symlink_sync(old_path, new_path)))
    }

    fn remove_dir<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.remove_dir_sync(path)))
    }

    fn unlink_file<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.unlink_file_sync(path)))
    }

    fn read_link<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
    ) -> ReadyFut<'ready, Result<PathBuf, Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.read_link_sync(path)))
    }

    fn get_filestat<'life0, 'ready>(&'life0 self) -> ReadyFut<'ready, Result<Filestat, Error>>
    where
        'life0: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.get_filestat_sync()))
    }

    fn get_path_filestat<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
        follow_symlinks: bool,
    ) -> ReadyFut<'ready, Result<Filestat, Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.get_path_filestat_sync(path, follow_symlinks)))
    }

    fn rename<'life0, 'life1, 'life2, 'life3, 'ready>(
        &'life0 self,
        path: &'life1 str,
        dest_dir: &'life2 dyn WasiDir,
        dest_path: &'life3 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        'life2: 'ready,
        'life3: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.rename_sync(path, dest_dir, dest_path)))
    }

    fn hard_link<'life0, 'life1, 'life2, 'life3, 'ready>(
        &'life0 self,
        path: &'life1 str,
        target_dir: &'life2 dyn WasiDir,
        target_path: &'life3 str,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        'life2: 'ready,
        'life3: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.hard_link_sync(path, target_dir, target_path)))
    }

    fn set_times<'life0, 'life1, 'ready>(
        &'life0 self,
        path: &'life1 str,
        atime: Option<SystemTimeSpec>,
        mtime: Option<SystemTimeSpec>,
        follow_symlinks: bool,
    ) -> ReadyFut<'ready, Result<(), Error>>
    where
        'life0: 'ready,
        'life1: 'ready,
        Self: Sync + 'ready,
    {
        Box::pin(ready(self.set_times_sync(
            path,
            atime,
            mtime,
            follow_symlinks,
        )))
    }
}

fn pipe_contents(pipe: WritePipe<CaptureBuffer>, notes: &mut Vec<String>, label: &str) -> Vec<u8> {
    match pipe.try_into_inner() {
        Ok(buffer) => {
            let (bytes, truncated) = buffer.into_parts();
            if truncated {
                notes.push(format!("{label}_capture_truncated={MAX_CAPTURE_BYTES}"));
            }
            bytes
        }
        Err(_) => {
            notes.push(format!("{label}_capture_unavailable=shared_pipe"));
            Vec::new()
        }
    }
}

fn terminal_from_error(error: &WasmiError) -> WasmTerminal {
    if let Some(exit) = error.i32_exit_status() {
        return WasmTerminal::Exit(exit);
    }
    if error.as_trap_code() == Some(TrapCode::OutOfFuel) {
        return WasmTerminal::Timeout(error.to_string());
    }
    let detail = error.to_string();
    let lowered = detail.to_ascii_lowercase();
    if lowered.contains("fuel") || lowered.contains("all fuel consumed") {
        WasmTerminal::Timeout(detail)
    } else {
        WasmTerminal::Failed(detail)
    }
}

fn observation(
    config: &WasmRunConfig,
    terminal: WasmTerminal,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    fuel_consumed: Option<u64>,
    wall_micros: Option<u64>,
    mut notes: Vec<String>,
) -> WasmRunObservation {
    cleanup_temp_roots(config, &mut notes);
    WasmRunObservation {
        module_ref: config.module_ref.clone(),
        terminal,
        stdout,
        stderr,
        fuel_consumed,
        wall_micros,
        filesystem_confined: config.filesystem_confined,
        notes,
    }
}

fn cleanup_temp_roots(config: &WasmRunConfig, notes: &mut Vec<String>) {
    for root in &config.temp_roots {
        match std::fs::remove_dir_all(root) {
            Ok(()) => notes.push(format!("temp_root_removed={}", root.display())),
            Err(error) => notes.push(format!("temp_root_cleanup_failed={error}")),
        }
    }
}
