pub mod config;
pub mod commit_file;
pub mod refs;
pub mod one_step_rule;

#[cfg(test)]
mod tests;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub use config::ContractConfig;
pub use commit_file::{CommitFile, RuleForThisCommit};
pub use refs::Refs;
pub use one_step_rule::{
    CommitSignature, CommitRuleFormula,
    parse_formula, parse_signatures,
    evaluate_formula, validate_rule_for_this_commit,
};

/// Parse a repost path in format $contract_id:/remote/path
/// Returns (contract_id, remote_path)
pub fn parse_repost_path(path: &str) -> Result<(&str, &str)> {
    if !path.starts_with('$') {
        anyhow::bail!("Repost path must start with '$', got: {}", path);
    }
    
    let colon_pos = path.find(":/")
        .ok_or_else(|| anyhow::anyhow!("Repost path must contain ':/', got: {}", path))?;
    
    let contract_id = &path[1..colon_pos];
    let remote_path = &path[colon_pos + 1..];
    
    if contract_id.is_empty() {
        anyhow::bail!("Repost path has empty contract_id");
    }
    
    if remote_path.is_empty() || !remote_path.starts_with('/') {
        anyhow::bail!("Repost remote path must start with '/'");
    }
    
    Ok((contract_id, remote_path))
}

pub struct ContractStore {
    pub root_dir: PathBuf,
}

impl ContractStore {
    /// Open an existing contract store
    pub fn open(dir: &Path) -> Result<Self> {
        // Walk up parent directories looking for .contract
        let mut current = dir.to_path_buf();
        loop {
            if current.join(".contract").exists() {
                return Ok(Self {
                    root_dir: current,
                });
            }
            if !current.pop() {
                break;
            }
        }
        anyhow::bail!("Not a contract directory (no .contract found in {} or any parent)", dir.display());
    }

    /// Initialize a new contract store
    pub fn init(dir: &Path, contract_id: String) -> Result<Self> {
        let contract_dir = dir.join(".contract");
        if contract_dir.exists() {
            anyhow::bail!("Contract already exists at: {}", dir.display());
        }

        // Create directory structure
        std::fs::create_dir_all(&contract_dir)?;
        std::fs::create_dir_all(contract_dir.join("commits"))?;
        std::fs::create_dir_all(contract_dir.join("refs").join("remotes"))?;

        // Create config
        let config = ContractConfig::new(contract_id);
        config.save(&contract_dir.join("config.json"))?;

        Ok(Self {
            root_dir: dir.to_path_buf(),
        })
    }

    /// Get the contract directory path
    pub fn contract_dir(&self) -> PathBuf {
        self.root_dir.join(".contract")
    }

    /// Load the contract config
    pub fn load_config(&self) -> Result<ContractConfig> {
        let config_path = self.contract_dir().join("config.json");
        ContractConfig::load(&config_path)
    }

    /// Save the contract config
    pub fn save_config(&self, config: &ContractConfig) -> Result<()> {
        let config_path = self.contract_dir().join("config.json");
        config.save(&config_path)
    }

    /// Save the genesis commit
    pub fn save_genesis(&self, genesis: &serde_json::Value) -> Result<()> {
        let genesis_path = self.contract_dir().join("genesis.json");
        let content = serde_json::to_string_pretty(genesis)?;
        std::fs::write(genesis_path, content)?;
        Ok(())
    }

    /// Load the genesis commit
    #[allow(unused)]
    pub fn load_genesis(&self) -> Result<serde_json::Value> {
        let genesis_path = self.contract_dir().join("genesis.json");
        let content = std::fs::read_to_string(genesis_path)?;
        let genesis: serde_json::Value = serde_json::from_str(&content)?;
        Ok(genesis)
    }

    /// Save a commit
    pub fn save_commit(&self, commit_id: &str, commit: &CommitFile) -> Result<()> {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        commit.save(&commit_path)
    }

    /// Load a commit
    pub fn load_commit(&self, commit_id: &str) -> Result<CommitFile> {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        CommitFile::load(&commit_path)
    }

    /// Check if a commit exists
    pub fn has_commit(&self, commit_id: &str) -> bool {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        commit_path.exists()
    }

    /// List all commit IDs
    pub fn list_commits(&self) -> Result<Vec<String>> {
        let commits_dir = self.contract_dir().join("commits");
        let mut commit_ids = Vec::new();

        for entry in std::fs::read_dir(commits_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    commit_ids.push(stem.to_string());
                }
            }
        }

        Ok(commit_ids)
    }

    /// Get HEAD commit ID
    pub fn get_head(&self) -> Result<Option<String>> {
        Refs::read_head(&self.root_dir)
    }

    /// Set HEAD commit ID
    pub fn set_head(&self, commit_id: &str) -> Result<()> {
        Refs::write_head(&self.root_dir, commit_id)
    }

    /// Get remote HEAD commit ID
    pub fn get_remote_head(&self, remote_name: &str) -> Result<Option<String>> {
        Refs::read_remote_head(&self.root_dir, remote_name)
    }

    /// Set remote HEAD commit ID
    pub fn set_remote_head(&self, remote_name: &str, commit_id: &str) -> Result<()> {
        Refs::write_remote_head(&self.root_dir, remote_name, commit_id)
    }

    /// Get the state directory path (working directory for editable files)
    pub fn state_dir(&self) -> PathBuf {
        self.root_dir.join("state")
    }

    /// Get the rules directory path (sister of state)
    pub fn rules_dir(&self) -> PathBuf {
        self.root_dir.join("rules")
    }

    /// Initialize the state directory
    pub fn init_state_dir(&self) -> Result<()> {
        let state_dir = self.state_dir();
        if !state_dir.exists() {
            std::fs::create_dir_all(&state_dir)?;
        }
        Ok(())
    }

    /// Initialize the rules directory
    pub fn init_rules_dir(&self) -> Result<()> {
        let rules_dir = self.rules_dir();
        if !rules_dir.exists() {
            std::fs::create_dir_all(&rules_dir)?;
        }
        Ok(())
    }

    /// Write a value to the state directory
    pub fn write_state(&self, path: &str, value: &serde_json::Value) -> Result<()> {
        let file_path = self.state_dir().join(path.trim_start_matches('/'));
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write the value (as JSON for complex types, raw for simple)
        let content = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => serde_json::to_string_pretty(value)?,
        };
        
        std::fs::write(&file_path, content)?;
        Ok(())
    }

    /// Read a value from the state directory
    #[allow(clippy::unnecessary_lazy_evaluations)]
    pub fn read_state(&self, path: &str) -> Result<Option<serde_json::Value>> {
        let file_path = self.state_dir().join(path.trim_start_matches('/'));
        
        if !file_path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(&file_path)?;
        
        // Try to parse as JSON, fallback to string
        let value = serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::Value::String(content));
        
        Ok(Some(value))
    }

    /// List all files in the state directory
    pub fn list_state_files(&self) -> Result<Vec<String>> {
        let state_dir = self.state_dir();
        if !state_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut files = Vec::new();
        self.collect_files(&state_dir, &state_dir, "", &mut files)?;
        Ok(files)
    }

    /// List all files in the rules directory
    pub fn list_rules_files(&self) -> Result<Vec<String>> {
        let rules_dir = self.rules_dir();
        if !rules_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut files = Vec::new();
        self.collect_files(&rules_dir, &rules_dir, "/rules", &mut files)?;
        Ok(files)
    }

    /// Read a rule file
    pub fn read_rule(&self, path: &str) -> Result<Option<serde_json::Value>> {
        // path is like /rules/auth.modality, strip the /rules prefix
        let relative = path.trim_start_matches("/rules/");
        let file_path = self.rules_dir().join(relative);
        
        if !file_path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(&file_path)?;
        Ok(Some(serde_json::Value::String(content)))
    }

    /// Write a rule file
    pub fn write_rule(&self, path: &str, value: &serde_json::Value) -> Result<()> {
        let relative = path.trim_start_matches("/rules/");
        let file_path = self.rules_dir().join(relative);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = match value {
            serde_json::Value::String(s) => s.clone(),
            _ => serde_json::to_string_pretty(value)?,
        };
        
        std::fs::write(&file_path, content)?;
        Ok(())
    }

    fn collect_files(&self, base: &Path, dir: &Path, prefix: &str, files: &mut Vec<String>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.collect_files(base, &path, prefix, files)?;
            } else if path.is_file() {
                let relative = path.strip_prefix(base)?;
                files.push(format!("{}/{}", prefix, relative.display()));
            }
        }
        Ok(())
    }

    /// Build current state by replaying all commits
    pub fn build_state_from_commits(&self) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        use std::collections::HashMap;
        
        let mut state: HashMap<String, serde_json::Value> = HashMap::new();
        
        // Get all commits in order (oldest first)
        let head = self.get_head()?;
        if head.is_none() {
            return Ok(state);
        }
        
        // Collect commits from HEAD to genesis
        let mut commits = Vec::new();
        let mut current = head;
        while let Some(commit_id) = current {
            let commit = self.load_commit(&commit_id)?;
            commits.push(commit.clone());
            current = commit.head.parent;
        }
        
        // Replay in order (oldest first)
        commits.reverse();
        for commit in commits {
            for action in &commit.body {
                if let Some(path) = &action.path {
                    match action.method.as_str() {
                        "post" | "genesis" | "rule" | "repost" => {
                            // repost stores data in $contract_id:/path namespace
                            state.insert(path.clone(), action.value.clone());
                        }
                        // Add other methods as needed
                        _ => {}
                    }
                }
            }
        }
        
        Ok(state)
    }

    /// Sync state and rules directories from commits (checkout)
    pub fn checkout_state(&self) -> Result<()> {
        self.init_state_dir()?;
        self.init_rules_dir()?;
        
        let state = self.build_state_from_commits()?;
        
        for (path, value) in state {
            if path.starts_with("/rules/") {
                self.write_rule(&path, &value)?;
            } else if path.starts_with('$') {
                // Reposted data from external contract: $contract_id:/path
                self.write_repost(&path, &value)?;
            } else {
                self.write_state(&path, &value)?;
            }
        }
        
        Ok(())
    }

    /// Get the reposts directory path (for data from other contracts)
    pub fn reposts_dir(&self) -> PathBuf {
        self.root_dir.join("reposts")
    }

    /// Initialize the reposts directory
    pub fn init_reposts_dir(&self) -> Result<()> {
        let reposts_dir = self.reposts_dir();
        if !reposts_dir.exists() {
            std::fs::create_dir_all(&reposts_dir)?;
        }
        Ok(())
    }

    /// Write reposted data from an external contract
    /// Path format: $contract_id:/remote/path.ext
    /// Stored at: reposts/{contract_id}/remote/path.ext
    pub fn write_repost(&self, path: &str, value: &serde_json::Value) -> Result<()> {
        self.init_reposts_dir()?;
        
        // Parse $contract_id:/remote/path
        let (contract_id, remote_path) = parse_repost_path(path)?;
        
        // Build local file path: reposts/{contract_id}{remote_path}
        let file_path = self.reposts_dir()
            .join(contract_id)
            .join(remote_path.trim_start_matches('/'));
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write the value
        let content = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => serde_json::to_string_pretty(value)?,
        };
        
        std::fs::write(&file_path, content)?;
        Ok(())
    }

    /// Read reposted data from an external contract
    #[allow(clippy::unnecessary_lazy_evaluations)]
    pub fn read_repost(&self, path: &str) -> Result<Option<serde_json::Value>> {
        let (contract_id, remote_path) = parse_repost_path(path)?;
        
        let file_path = self.reposts_dir()
            .join(contract_id)
            .join(remote_path.trim_start_matches('/'));
        
        if !file_path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(&file_path)?;
        
        // Try to parse as JSON, fallback to string
        let value = serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::Value::String(content));
        
        Ok(Some(value))
    }

    /// List all reposted files
    pub fn list_repost_files(&self) -> Result<Vec<String>> {
        let reposts_dir = self.reposts_dir();
        if !reposts_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut files = Vec::new();
        
        // Iterate over contract_id directories
        for entry in std::fs::read_dir(&reposts_dir)? {
            let entry = entry?;
            let contract_id = entry.file_name().to_string_lossy().to_string();
            let contract_dir = entry.path();
            
            if contract_dir.is_dir() {
                self.collect_repost_files(&contract_dir, &contract_id, &mut files)?;
            }
        }
        
        Ok(files)
    }

    fn collect_repost_files(&self, dir: &Path, contract_id: &str, files: &mut Vec<String>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.collect_repost_files(&path, contract_id, files)?;
            } else if path.is_file() {
                let relative = path.strip_prefix(self.reposts_dir().join(contract_id))?;
                files.push(format!("${}:/{}", contract_id, relative.display()));
            }
        }
        Ok(())
    }

    /// Validate a commit against all accumulated contract rules
    /// 
    /// Loads all rules from commit history, builds current state,
    /// and evaluates each rule's predicates against the pending commit.
    pub fn validate_commit_against_rules(&self, commit: &CommitFile) -> Result<()> {
        use crate::contract_store::one_step_rule::EvalContext;
        
        // Build current state and collect rules
        let (state, rules) = self.build_state_and_rules()?;
        
        if rules.is_empty() {
            return Ok(()); // No rules to validate against
        }
        
        // Extract signers from commit head
        let signers = self.extract_signers_from_commit(commit);
        
        // Build commit body as Value for EvalContext
        let body_value = serde_json::to_value(&commit.body)?;
        
        // Create evaluation context
        let ctx = EvalContext::new(&signers, &state, &body_value);
        
        // Validate each rule
        for rule_content in &rules {
            self.validate_single_rule(rule_content, &ctx)?;
        }
        
        Ok(())
    }
    
    /// Build current state and collect all rules from commits
    fn build_state_and_rules(&self) -> Result<(serde_json::Value, Vec<String>)> {
        use std::collections::HashMap;
        
        let mut state: HashMap<String, serde_json::Value> = HashMap::new();
        let mut rules: Vec<String> = Vec::new();
        
        // Get all commits in order (oldest first)
        let head = self.get_head()?;
        if head.is_none() {
            return Ok((serde_json::json!({}), rules));
        }
        
        // Collect commits from HEAD to genesis
        let mut commits = Vec::new();
        let mut current = head;
        while let Some(commit_id) = current {
            let commit = self.load_commit(&commit_id)?;
            commits.push(commit.clone());
            current = commit.head.parent;
        }
        
        // Replay in order (oldest first)
        commits.reverse();
        for commit in commits {
            for action in &commit.body {
                if let Some(path) = &action.path {
                    match action.method.as_str() {
                        "post" | "genesis" | "repost" => {
                            let normalized = path.trim_start_matches('/').to_string();
                            state.insert(normalized, action.value.clone());
                        }
                        "rule" => {
                            // Collect rule content
                            if let Some(rule_str) = action.value.as_str() {
                                rules.push(rule_str.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok((serde_json::json!(state), rules))
    }
    
    /// Extract signer identities from commit signatures
    fn extract_signers_from_commit(&self, commit: &CommitFile) -> Vec<String> {
        let mut signers = Vec::new();
        
        if let Some(sigs) = &commit.head.signatures {
            if let Some(obj) = sigs.as_object() {
                // Format: { "pubkey": "signature" }
                for key in obj.keys() {
                    signers.push(key.clone());
                }
            }
        }
        
        signers
    }
    
    /// Validate a single rule against the evaluation context
    fn validate_single_rule(
        &self, 
        rule_content: &str, 
        ctx: &one_step_rule::EvalContext,
    ) -> Result<()> {
        use crate::contract_store::one_step_rule::{parse_formula, evaluate_formula_full};
        
        // Extract the formula from rule syntax
        // Format: rule name { formula { <expression> } }
        let formula_str = match self.extract_formula_from_rule(rule_content) {
            Some(f) => f,
            None => return Ok(()), // Can't parse, skip (might be a different rule format)
        };
        
        // Handle temporal operators and implications
        // "always (+any_signed(/members))" -> evaluate any_signed(/members)
        // "always (+modifies(/members) implies +all_signed(/members))" -> conditional check
        
        let formula_str = formula_str.trim();
        
        // Strip "always" wrapper if present
        let inner = if formula_str.starts_with("always") {
            self.extract_inner_formula(formula_str, "always")
                .unwrap_or(formula_str.to_string())
        } else {
            formula_str.to_string()
        };
        
        // Handle implication: "A implies B" means "if A then B"
        if inner.contains(" implies ") {
            return self.validate_implication(&inner, ctx);
        }
        
        // Strip + prefix from predicates for parsing
        let formula_normalized = self.normalize_predicate_syntax(&inner);
        
        // Parse and evaluate
        match parse_formula(&formula_normalized) {
            Ok(formula) => {
                if !evaluate_formula_full(&formula, ctx) {
                    anyhow::bail!(
                        "Rule violation: {} (signers: {:?})",
                        rule_content.chars().take(100).collect::<String>(),
                        ctx.signers
                    );
                }
                Ok(())
            }
            Err(_) => {
                // Can't parse this formula format, skip validation
                // This might be a more complex formula that needs model checking
                Ok(())
            }
        }
    }
    
    /// Extract formula expression from rule declaration
    fn extract_formula_from_rule(&self, rule_content: &str) -> Option<String> {
        // Find "formula" keyword and extract content
        let formula_start = rule_content.find("formula")?;
        let after_formula = &rule_content[formula_start..];
        
        // Find first { after formula
        let brace_start = after_formula.find('{')?;
        let content_start = formula_start + brace_start + 1;
        
        // Find matching closing brace
        let mut depth = 1;
        let mut end = content_start;
        for (i, c) in rule_content[content_start..].chars().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = content_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        
        Some(rule_content[content_start..end].trim().to_string())
    }
    
    /// Extract inner formula from temporal operator
    fn extract_inner_formula(&self, formula: &str, operator: &str) -> Option<String> {
        let trimmed = formula.trim();
        if !trimmed.starts_with(operator) {
            return None;
        }
        
        let after_op = &trimmed[operator.len()..].trim_start();
        
        // Handle both "always (expr)" and "always expr"
        if after_op.starts_with('(') {
            // Find matching paren
            let mut depth = 0;
            let mut end = 0;
            for (i, c) in after_op.chars().enumerate() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            Some(after_op[1..end].trim().to_string())
        } else {
            Some(after_op.to_string())
        }
    }
    
    /// Normalize predicate syntax: "+any_signed(x)" -> "any_signed(x)"
    fn normalize_predicate_syntax(&self, formula: &str) -> String {
        formula
            .replace("+any_signed", "any_signed")
            .replace("+all_signed", "all_signed")
            .replace("+modifies", "modifies")
            .replace("+signed_by", "signed_by")
            .replace("-modifies", "!modifies") // Negative predicate
    }
    
    /// Validate an implication: "A implies B" means if A is true, B must be true
    fn validate_implication(
        &self,
        formula: &str,
        ctx: &one_step_rule::EvalContext,
    ) -> Result<()> {
        use crate::contract_store::one_step_rule::{parse_formula, evaluate_formula_full};
        
        // Split on "implies"
        let parts: Vec<&str> = formula.split(" implies ").collect();
        if parts.len() != 2 {
            return Ok(()); // Can't parse, skip
        }
        
        let antecedent = self.normalize_predicate_syntax(parts[0].trim());
        let consequent = self.normalize_predicate_syntax(parts[1].trim());
        
        // Parse antecedent
        let antecedent_formula = match parse_formula(&antecedent) {
            Ok(f) => f,
            Err(_) => return Ok(()), // Can't parse, skip
        };
        
        // If antecedent is false, implication is satisfied
        if !evaluate_formula_full(&antecedent_formula, ctx) {
            return Ok(());
        }
        
        // Antecedent is true, so consequent must also be true
        let consequent_formula = match parse_formula(&consequent) {
            Ok(f) => f,
            Err(_) => return Ok(()), // Can't parse, skip
        };
        
        if !evaluate_formula_full(&consequent_formula, ctx) {
            anyhow::bail!(
                "Rule violation: {} implies {} (antecedent true but consequent false, signers: {:?})",
                antecedent,
                consequent,
                ctx.signers
            );
        }
        
        Ok(())
    }

    /// Get commits that need to be pushed (between remote HEAD and local HEAD)
    pub fn get_unpushed_commits(&self, remote_name: &str) -> Result<Vec<String>> {
        let local_head = self.get_head()?;
        let remote_head = self.get_remote_head(remote_name)?;

        if local_head.is_none() {
            return Ok(Vec::new());
        }

        let mut unpushed = Vec::new();
        let mut current = local_head;

        // Walk backwards from HEAD until we reach remote HEAD or genesis
        while let Some(commit_id) = current {
            if Some(&commit_id) == remote_head.as_ref() {
                break;
            }

            unpushed.push(commit_id.clone());

            // Load commit and get parent
            let commit = self.load_commit(&commit_id)?;
            current = commit.head.parent;
        }

        // Reverse to get chronological order
        unpushed.reverse();
        Ok(unpushed)
    }
}

