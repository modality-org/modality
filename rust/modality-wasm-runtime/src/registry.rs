use std::collections::HashMap;
use anyhow::Result;

/// Registry for storing and managing WASM modules
pub struct ModuleRegistry {
    modules: HashMap<String, Vec<u8>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register a WASM module with a given name
    pub fn register(&mut self, name: String, wasm_bytes: Vec<u8>) -> Result<()> {
        self.modules.insert(name, wasm_bytes);
        Ok(())
    }

    /// Get a WASM module by name
    pub fn get(&self, name: &str) -> Option<&Vec<u8>> {
        self.modules.get(name)
    }

    /// Remove a WASM module by name
    pub fn unregister(&mut self, name: &str) -> Option<Vec<u8>> {
        self.modules.remove(name)
    }

    /// Check if a module is registered
    pub fn contains(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Get the number of registered modules
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_operations() {
        let mut registry = ModuleRegistry::new();
        assert!(registry.is_empty());

        let wasm_bytes = vec![0, 1, 2, 3];
        registry.register("test_module".to_string(), wasm_bytes.clone()).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_module"));
        assert_eq!(registry.get("test_module"), Some(&wasm_bytes));

        let removed = registry.unregister("test_module");
        assert_eq!(removed, Some(wasm_bytes));
        assert!(registry.is_empty());
    }
}

