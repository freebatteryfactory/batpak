//! Warm-execution cache (Phase 1: setup amortization).
//!
//! A shared wasmi [`Engine`] plus a content-addressed compiled-[`Module`] cache.
//! Repeated runs of the same guest skip the dominant per-run cost — `Module::new`
//! (parse + validate + translate to wasmi bytecode) — while reads, linking, and
//! instantiation stay per-run. The key is the module's blake3 byte identity (the
//! same `compute_hash` BatPak uses for report/content identity), so two distinct
//! modules can never alias onto one compiled artifact. Bounded (FIFO eviction) and
//! fully synchronous: no async, no runtime, no new dependency. A miss simply
//! recompiles, so the cache is a speed optimization and never a correctness input.

use batpak::event::hash::compute_hash;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use wasmi::{Config, Engine, Error as WasmiError, Module};

/// Maximum distinct compiled modules retained before the oldest is evicted.
const MODULE_CACHE_CAP: usize = 256;

/// Blake3 byte identity of a module — the cache key.
type ModuleKey = [u8; 32];

/// Shared engine + compiled-module cache, warmed once and reused across runs.
pub(super) struct WarmCache {
    engine: Engine,
    modules: Mutex<ModuleCache>,
}

impl WarmCache {
    /// Build the shared fuel-metered engine once.
    pub(super) fn new() -> Self {
        let mut config = Config::default();
        config.consume_fuel(true);
        Self {
            engine: Engine::new(&config),
            modules: Mutex::new(ModuleCache::new(MODULE_CACHE_CAP)),
        }
    }

    /// The shared engine. Every cached module is compiled against it, so a cached
    /// module is instantiable in any [`wasmi::Store`] created from this engine.
    pub(super) fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Resolve the compiled module for `bytes`, compiling only on a cache miss.
    pub(super) fn resolve(&self, bytes: &[u8]) -> Result<Arc<Module>, WasmiError> {
        let key = compute_hash(bytes);
        if let Some(module) = self.lock().get(&key) {
            return Ok(module);
        }
        // Compile OUTSIDE the lock so a slow compile cannot block other lookups; a
        // concurrent duplicate compile is harmless (both artifacts are valid, and
        // the first inserted wins — the second is dropped).
        let module = Arc::new(Module::new(&self.engine, bytes)?);
        self.lock().insert(key, Arc::clone(&module));
        Ok(module)
    }

    fn lock(&self) -> MutexGuard<'_, ModuleCache> {
        self.modules.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Bounded FIFO map from content key to compiled module.
struct ModuleCache {
    map: HashMap<ModuleKey, Arc<Module>>,
    order: VecDeque<ModuleKey>,
    cap: usize,
}

impl ModuleCache {
    fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    fn get(&self, key: &ModuleKey) -> Option<Arc<Module>> {
        self.map.get(key).map(Arc::clone)
    }

    fn insert(&mut self, key: ModuleKey, module: Arc<Module>) {
        if self.map.contains_key(&key) {
            return;
        }
        while self.map.len() >= self.cap {
            match self.order.pop_front() {
                Some(evicted) => {
                    self.map.remove(&evicted);
                }
                None => break,
            }
        }
        self.order.push_back(key);
        self.map.insert(key, module);
    }
}

#[cfg(test)]
mod tests {
    use super::{ModuleCache, WarmCache};
    use std::sync::Arc;

    #[test]
    fn resolve_returns_the_same_cached_module_for_identical_content() {
        let warm = WarmCache::new();
        let bytes = wat::parse_str("(module)").expect("empty module wat");
        let first = warm.resolve(&bytes).expect("first compile");
        let second = warm.resolve(&bytes).expect("cached hit");
        assert!(
            Arc::ptr_eq(&first, &second),
            "identical module bytes must return the one cached compiled module"
        );
    }

    #[test]
    fn distinct_content_compiles_to_distinct_artifacts() {
        let warm = WarmCache::new();
        let a = warm
            .resolve(&wat::parse_str("(module)").expect("wat a"))
            .expect("a");
        let b = warm
            .resolve(&wat::parse_str("(module (func))").expect("wat b"))
            .expect("b");
        assert!(
            !Arc::ptr_eq(&a, &b),
            "distinct module bytes must not alias onto one compiled artifact"
        );
    }

    #[test]
    fn bounded_cache_evicts_oldest_over_capacity() {
        let engine = {
            let mut config = wasmi::Config::default();
            config.consume_fuel(true);
            wasmi::Engine::new(&config)
        };
        let mut cache = ModuleCache::new(2);
        let m = |src: &str| {
            let wasm = wat::parse_str(src).expect("wat compiles");
            Arc::new(wasmi::Module::new(&engine, &wasm).expect("module compiles"))
        };
        cache.insert([1u8; 32], m("(module)"));
        cache.insert([2u8; 32], m("(module (func))"));
        cache.insert([3u8; 32], m("(module (func) (func))"));
        assert!(
            cache.get(&[1u8; 32]).is_none(),
            "oldest key must be evicted"
        );
        assert!(cache.get(&[2u8; 32]).is_some());
        assert!(cache.get(&[3u8; 32]).is_some());
    }
}
