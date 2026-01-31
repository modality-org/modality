use anyhow::Result;
use std::path::Path;

pub struct Refs;

impl Refs {
    pub fn read_head(contract_dir: &Path) -> Result<Option<String>> {
        let head_path = contract_dir.join(".contract").join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(head_path)?;
        Ok(Some(content.trim().to_string()))
    }

    pub fn write_head(contract_dir: &Path, commit_id: &str) -> Result<()> {
        let head_path = contract_dir.join(".contract").join("HEAD");
        std::fs::write(head_path, commit_id)?;
        Ok(())
    }

    pub fn read_remote_head(contract_dir: &Path, remote_name: &str) -> Result<Option<String>> {
        let remote_head_path = contract_dir
            .join(".contract")
            .join("refs")
            .join("remotes")
            .join(remote_name)
            .join("HEAD");
        
        if !remote_head_path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(remote_head_path)?;
        Ok(Some(content.trim().to_string()))
    }

    pub fn write_remote_head(contract_dir: &Path, remote_name: &str, commit_id: &str) -> Result<()> {
        let remote_dir = contract_dir
            .join(".contract")
            .join("refs")
            .join("remotes")
            .join(remote_name);
        
        std::fs::create_dir_all(&remote_dir)?;
        
        let remote_head_path = remote_dir.join("HEAD");
        std::fs::write(remote_head_path, commit_id)?;
        Ok(())
    }
}

