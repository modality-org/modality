use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::Module;

/// Cache entry for a compiled WASM module
#[derive(Clone)]
struct CacheEntry {
    /// The compiled Wasmtime module
    module: Arc<Module>,
    /// SHA256 hash for verification
    #[allow(dead_code)]
    hash: String,
    /// Size in bytes for cache management
    size: usize,
    /// Last access timestamp (for LRU)
    last_access: u64,
}

/// LRU cache for compiled WASM modules
/// 
/// Caches compiled Wasmtime modules to avoid recompilation overhead.
/// Uses LRU eviction when cache size exceeds limits.
pub struct WasmModuleCache {
    /// Cache entries keyed by (contract_id, module_path, hash)
    entries: HashMap<String, CacheEntry>,
    /// Maximum cache size in bytes
    max_size_bytes: usize,
    /// Current cache size in bytes
    current_size_bytes: usize,
    /// Maximum number of cached modules
    max_modules: usize,
    /// Cache hit counter
    hits: u64,
    /// Cache miss counter
    misses: u64,
}

impl WasmModuleCache {
    /// Create a new cache with size limits
    /// 
    /// # Arguments
    /// * `max_modules` - Maximum number of modules to cache (default: 100)
    /// * `max_size_mb` - Maximum cache size in megabytes (default: 50)
    pub fn new(max_modules: usize, max_size_mb: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size_bytes: 0,
            max_modules,
            hits: 0,
            misses: 0,
        }
    }

    /// Create a cache with default limits (100 modules, 50MB)
    pub fn default() -> Self {
        Self::new(100, 50)
    }

    /// Generate cache key from contract ID, path, and hash
    fn cache_key(contract_id: &str, module_path: &str, hash: &str) -> String {
        format!("{}:{}:{}", contract_id, module_path, hash)
    }

    /// Get a module from the cache
    /// 
    /// Returns `Some(module)` if found, `None` if not in cache
    pub fn get(&mut self, contract_id: &str, module_path: &str, hash: &str) -> Option<Arc<Module>> {
        let key = Self::cache_key(contract_id, module_path, hash);
        
        if let Some(entry) = self.entries.get_mut(&key) {
            // Update last access time for LRU
            entry.last_access = Self::current_timestamp();
            self.hits += 1;
            Some(entry.module.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a module into the cache
    /// 
    /// May trigger eviction of least recently used modules if cache is full
    pub fn insert(&mut self, contract_id: &str, module_path: &str, hash: &str, module: Module, wasm_size: usize) {
        let key = Self::cache_key(contract_id, module_path, hash);
        
        // Check if we need to evict
        while (self.entries.len() >= self.max_modules || self.current_size_bytes + wasm_size > self.max_size_bytes)
            && !self.entries.is_empty()
        {
            self.evict_lru();
        }

        // Insert new entry
        let entry = CacheEntry {
            module: Arc::new(module),
            hash: hash.to_string(),
            size: wasm_size,
            last_access: Self::current_timestamp(),
        };

        // Remove old entry if it exists (for cache key collision)
        if let Some(old_entry) = self.entries.remove(&key) {
            self.current_size_bytes -= old_entry.size;
        }

        self.current_size_bytes += wasm_size;
        self.entries.insert(key, entry);
    }

    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        if let Some((lru_key, lru_entry)) = self.entries.iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(k, e)| (k.clone(), e.clone()))
        {
            self.entries.remove(&lru_key);
            self.current_size_bytes -= lru_entry.size;
        }
    }

    /// Clear all entries from the cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size_bytes = 0;
    }

    /// Get current cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            size_bytes: self.current_size_bytes,
            max_size_bytes: self.max_size_bytes,
            max_modules: self.max_modules,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                (self.hits as f64) / ((self.hits + self.misses) as f64)
            } else {
                0.0
            },
        }
    }

    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Check if a module is in the cache
    pub fn contains(&self, contract_id: &str, module_path: &str, hash: &str) -> bool {
        let key = Self::cache_key(contract_id, module_path, hash);
        self.entries.contains_key(&key)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached modules
    pub entries: usize,
    /// Current cache size in bytes
    pub size_bytes: usize,
    /// Maximum cache size in bytes
    pub max_size_bytes: usize,
    /// Maximum number of modules
    pub max_modules: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Hit rate (hits / (hits + misses))
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Engine, Config};

    fn create_test_module() -> Module {
        // Create a minimal valid WASM module
        let wasm = wat::parse_str(r#"
            (module
                (func (export "test") (result i32)
                    i32.const 42
                )
            )
        "#).unwrap();
        
        let engine = Engine::new(&Config::new()).unwrap();
        Module::new(&engine, &wasm).unwrap()
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = WasmModuleCache::new(10, 10);
        let module = create_test_module();
        
        cache.insert("contract1", "/_code/test.wasm", "hash123", module, 100);
        
        let retrieved = cache.get("contract1", "/_code/test.wasm", "hash123");
        assert!(retrieved.is_some());
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = WasmModuleCache::new(10, 10);
        
        let retrieved = cache.get("contract1", "/_code/test.wasm", "hash123");
        assert!(retrieved.is_none());
        
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_eviction_by_count() {
        let mut cache = WasmModuleCache::new(2, 1000); // Max 2 modules
        
        let module1 = create_test_module();
        let module2 = create_test_module();
        let module3 = create_test_module();
        
        cache.insert("contract1", "/_code/m1.wasm", "hash1", module1, 10);
        // Give a small delay so timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.insert("contract2", "/_code/m2.wasm", "hash2", module2, 10);
        
        // Don't access either - module1 is oldest now
        
        // Insert module3, should evict module1 (LRU - oldest timestamp)
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.insert("contract3", "/_code/m3.wasm", "hash3", module3, 10);
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        
        // module1 should be evicted (oldest)
        assert!(cache.get("contract1", "/_code/m1.wasm", "hash1").is_none());
        // module2 should still be there
        assert!(cache.get("contract2", "/_code/m2.wasm", "hash2").is_some());
        // module3 should be there
        assert!(cache.get("contract3", "/_code/m3.wasm", "hash3").is_some());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = WasmModuleCache::new(10, 10);
        let module = create_test_module();
        
        cache.insert("contract1", "/_code/test.wasm", "hash123", module, 100);
        assert_eq!(cache.stats().entries, 1);
        
        cache.clear();
        assert_eq!(cache.stats().entries, 0);
        assert_eq!(cache.stats().size_bytes, 0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let mut cache = WasmModuleCache::new(10, 10);
        let module = create_test_module();
        
        cache.insert("contract1", "/_code/test.wasm", "hash123", module, 100);
        
        // 3 hits
        cache.get("contract1", "/_code/test.wasm", "hash123");
        cache.get("contract1", "/_code/test.wasm", "hash123");
        cache.get("contract1", "/_code/test.wasm", "hash123");
        
        // 1 miss
        cache.get("contract2", "/_code/other.wasm", "hash456");
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.75);
    }
}

