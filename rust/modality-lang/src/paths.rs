//! Path Semantics for Modality Contracts
//!
//! Contracts maintain a typed key-value store addressable via paths:
//! - `/status/color.text` - a text value at /status/color
//! - `/members/alice.pubkey` - a public key at /members/alice
//! - `/balances/alice.balance` - a balance at /balances/alice
//!
//! Actions can reference paths in predicates:
//! - `+signed_by(/members/alice.pubkey)` - verify signature using key at path
//! - `+post_to(/status/state.text)` - modify the value at path
//!
//! # Types
//!
//! | Extension | Description |
//! |-----------|-------------|
//! | `.text`   | UTF-8 string |
//! | `.int`    | Signed integer |
//! | `.bool`   | Boolean |
//! | `.balance`| Token balance (u64) |
//! | `.pubkey` | Public key (ed25519) |
//! | `.set`    | Set of strings |
//! | `.list`   | Ordered list |
//! | `.json`   | Arbitrary JSON |

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Supported value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathValue {
    Text(String),
    Int(i64),
    Bool(bool),
    Balance(u64),
    PubKey(String),  // Hex-encoded public key
    Set(Vec<String>),
    List(Vec<String>),
    Json(serde_json::Value),
}

impl PathValue {
    /// Get the type extension for this value
    pub fn type_extension(&self) -> &'static str {
        match self {
            PathValue::Text(_) => "text",
            PathValue::Int(_) => "int",
            PathValue::Bool(_) => "bool",
            PathValue::Balance(_) => "balance",
            PathValue::PubKey(_) => "pubkey",
            PathValue::Set(_) => "set",
            PathValue::List(_) => "list",
            PathValue::Json(_) => "json",
        }
    }

    /// Parse a value from string given type
    pub fn parse(type_ext: &str, value: &str) -> Result<Self, String> {
        match type_ext {
            "text" | "string" => Ok(PathValue::Text(value.to_string())),
            "int" | "integer" => value.parse::<i64>()
                .map(PathValue::Int)
                .map_err(|e| format!("Invalid integer: {}", e)),
            "bool" | "boolean" => value.parse::<bool>()
                .map(PathValue::Bool)
                .map_err(|e| format!("Invalid boolean: {}", e)),
            "balance" | "bal" => value.parse::<u64>()
                .map(PathValue::Balance)
                .map_err(|e| format!("Invalid balance: {}", e)),
            "pubkey" | "key" => Ok(PathValue::PubKey(value.to_string())),
            "set" => serde_json::from_str(value)
                .map(PathValue::Set)
                .map_err(|e| format!("Invalid set: {}", e)),
            "list" => serde_json::from_str(value)
                .map(PathValue::List)
                .map_err(|e| format!("Invalid list: {}", e)),
            "json" => serde_json::from_str(value)
                .map(PathValue::Json)
                .map_err(|e| format!("Invalid JSON: {}", e)),
            _ => Err(format!("Unknown type: {}", type_ext)),
        }
    }

    /// Convert to string
    pub fn to_string_value(&self) -> String {
        match self {
            PathValue::Text(s) => s.clone(),
            PathValue::Int(n) => n.to_string(),
            PathValue::Bool(b) => b.to_string(),
            PathValue::Balance(n) => n.to_string(),
            PathValue::PubKey(s) => s.clone(),
            PathValue::Set(v) => serde_json::to_string(v).unwrap_or_default(),
            PathValue::List(v) => serde_json::to_string(v).unwrap_or_default(),
            PathValue::Json(v) => serde_json::to_string(v).unwrap_or_default(),
        }
    }
}

/// A parsed path
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Directory components (e.g., ["members", "alice"] for /members/alice.pubkey)
    pub dirs: Vec<String>,
    /// File name without extension (e.g., "alice")
    pub name: Option<String>,
    /// Type extension (e.g., "pubkey")
    pub type_ext: Option<String>,
}

impl Path {
    /// Parse a path string
    pub fn parse(path: &str) -> Result<Self, String> {
        if !path.starts_with('/') {
            return Err("Path must start with /".to_string());
        }

        let path = &path[1..]; // Remove leading /
        
        if path.is_empty() {
            return Ok(Self {
                dirs: vec![],
                name: None,
                type_ext: None,
            });
        }

        // Check if it's a file (has extension)
        if let Some(dot_pos) = path.rfind('.') {
            let before_dot = &path[..dot_pos];
            let ext = &path[dot_pos + 1..];
            
            let parts: Vec<&str> = before_dot.split('/').collect();
            let (dirs, name) = if parts.is_empty() {
                (vec![], None)
            } else {
                let name = parts.last().map(|s| s.to_string());
                let dirs: Vec<String> = parts[..parts.len().saturating_sub(1)]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                (dirs, name)
            };

            Ok(Self {
                dirs,
                name,
                type_ext: Some(ext.to_string()),
            })
        } else {
            // It's a directory
            let parts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
            Ok(Self {
                dirs: parts,
                name: None,
                type_ext: None,
            })
        }
    }

    /// Check if this is a directory path
    pub fn is_dir(&self) -> bool {
        self.name.is_none() && self.type_ext.is_none()
    }

    /// Check if this is a file path
    pub fn is_file(&self) -> bool {
        self.name.is_some() && self.type_ext.is_some()
    }

    /// Get the full path string
    pub fn to_string(&self) -> String {
        let mut result = String::from("/");
        
        if !self.dirs.is_empty() {
            result.push_str(&self.dirs.join("/"));
        }
        
        if let Some(ref name) = self.name {
            if !self.dirs.is_empty() {
                result.push('/');
            }
            result.push_str(name);
            
            if let Some(ref ext) = self.type_ext {
                result.push('.');
                result.push_str(ext);
            }
        }
        
        result
    }

    /// Get parent directories (including self if dir)
    pub fn parent_dirs(&self) -> Vec<String> {
        let mut result = vec!["/".to_string()];
        let mut current = String::from("/");
        
        for dir in &self.dirs {
            current.push_str(dir);
            result.push(current.clone());
            current.push('/');
        }
        
        result
    }
}

/// Contract state store
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractStore {
    /// Path → Value mapping
    values: HashMap<String, PathValue>,
    /// Directory listing cache
    directories: HashMap<String, Vec<String>>,
}

impl ContractStore {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            directories: HashMap::new(),
        }
    }

    /// Set a value at a path
    pub fn set(&mut self, path: &str, value: PathValue) -> Result<(), String> {
        let parsed = Path::parse(path)?;
        
        if !parsed.is_file() {
            return Err("Can only set values at file paths".to_string());
        }

        // Validate type matches extension
        if let Some(ref ext) = parsed.type_ext {
            let expected = value.type_extension();
            // Allow compatible types
            let compatible = match (ext.as_str(), expected) {
                ("text", "text") | ("string", "text") => true,
                ("int", "int") | ("integer", "int") => true,
                ("bool", "bool") | ("boolean", "bool") => true,
                ("balance", "balance") | ("bal", "balance") => true,
                ("pubkey", "pubkey") | ("key", "pubkey") => true,
                ("set", "set") => true,
                ("list", "list") => true,
                ("json", "json") => true,
                _ => ext == expected,
            };
            
            if !compatible {
                return Err(format!(
                    "Type mismatch: path expects {}, got {}",
                    ext, expected
                ));
            }
        }

        // Update directory listings
        for parent in parsed.parent_dirs() {
            self.directories.entry(parent).or_insert_with(Vec::new);
        }

        self.values.insert(path.to_string(), value);
        Ok(())
    }

    /// Get a value at a path
    pub fn get(&self, path: &str) -> Option<&PathValue> {
        self.values.get(path)
    }

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> bool {
        self.values.contains_key(path) || self.directories.contains_key(path)
    }

    /// List entries in a directory
    pub fn list_dir(&self, dir_path: &str) -> Vec<String> {
        let dir_prefix = if dir_path.ends_with('/') {
            dir_path.to_string()
        } else {
            format!("{}/", dir_path)
        };

        self.values.keys()
            .filter(|k| k.starts_with(&dir_prefix))
            .cloned()
            .collect()
    }

    /// Delete a value
    pub fn delete(&mut self, path: &str) -> Option<PathValue> {
        self.values.remove(path)
    }

    /// Get a pubkey value (convenience method)
    pub fn get_pubkey(&self, path: &str) -> Option<&str> {
        match self.get(path) {
            Some(PathValue::PubKey(key)) => Some(key.as_str()),
            _ => None,
        }
    }

    /// Get a balance value (convenience method)
    pub fn get_balance(&self, path: &str) -> Option<u64> {
        match self.get(path) {
            Some(PathValue::Balance(b)) => Some(*b),
            _ => None,
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Actions that can modify the contract store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoreAction {
    /// Set a value: POST /path/name.type value
    Post { path: String, value: PathValue },
    /// Add a rule: RULE formula
    Rule { formula: String },
    /// Delete a value
    Delete { path: String },
}

/// Parse a path reference in a predicate
///
/// Example: `signed_by(/members/alice.pubkey)` → path = "/members/alice.pubkey"
pub fn parse_path_reference(predicate: &str) -> Option<(String, String)> {
    // Match: name(/path/to/value.type)
    let re = regex::Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\((/[^)]+)\)$").ok()?;
    let caps = re.captures(predicate)?;
    
    Some((
        caps.get(1)?.as_str().to_string(),
        caps.get(2)?.as_str().to_string(),
    ))
}

/// Resolve a path reference using the contract store
pub fn resolve_path_value(store: &ContractStore, path: &str) -> Option<PathValue> {
    store.get(path).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_parsing() {
        let path = Path::parse("/members/alice.pubkey").unwrap();
        assert_eq!(path.dirs, vec!["members"]);
        assert_eq!(path.name, Some("alice".to_string()));
        assert_eq!(path.type_ext, Some("pubkey".to_string()));
    }

    #[test]
    fn test_path_dir() {
        let path = Path::parse("/members/admins").unwrap();
        assert!(path.is_dir());
        assert!(!path.is_file());
    }

    #[test]
    fn test_store_set_get() {
        let mut store = ContractStore::new();
        
        store.set("/status/color.text", PathValue::Text("red".to_string())).unwrap();
        
        let value = store.get("/status/color.text");
        assert_eq!(value, Some(&PathValue::Text("red".to_string())));
    }

    #[test]
    fn test_store_pubkey() {
        let mut store = ContractStore::new();
        
        let pubkey = "abc123def456";
        store.set("/members/alice.pubkey", PathValue::PubKey(pubkey.to_string())).unwrap();
        
        assert_eq!(store.get_pubkey("/members/alice.pubkey"), Some(pubkey));
    }

    #[test]
    fn test_store_balance() {
        let mut store = ContractStore::new();
        
        store.set("/balances/alice.balance", PathValue::Balance(1000)).unwrap();
        
        assert_eq!(store.get_balance("/balances/alice.balance"), Some(1000));
    }

    #[test]
    fn test_type_mismatch() {
        let mut store = ContractStore::new();
        
        let result = store.set("/value.int", PathValue::Text("not an int".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_path_reference_parsing() {
        let (name, path) = parse_path_reference("signed_by(/members/alice.pubkey)").unwrap();
        assert_eq!(name, "signed_by");
        assert_eq!(path, "/members/alice.pubkey");
    }

    #[test]
    fn test_parent_dirs() {
        let path = Path::parse("/members/admins/alice.pubkey").unwrap();
        let parents = path.parent_dirs();
        assert_eq!(parents, vec!["/", "/members", "/members/admins"]);
    }

    #[test]
    fn test_store_serialization() {
        let mut store = ContractStore::new();
        store.set("/status/active.bool", PathValue::Bool(true)).unwrap();
        store.set("/members/alice.pubkey", PathValue::PubKey("abc123".to_string())).unwrap();
        
        let json = store.to_json().unwrap();
        let restored = ContractStore::from_json(&json).unwrap();
        
        assert_eq!(restored.get("/status/active.bool"), Some(&PathValue::Bool(true)));
    }
}
