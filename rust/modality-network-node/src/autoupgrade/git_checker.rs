use anyhow::{Context, Result};
use std::process::Command;

/// Get the current commit hash for a git branch using `git ls-remote`
pub async fn get_current_commit(repo: &str, branch: &str) -> Result<String> {
    // Use tokio::task::spawn_blocking to run the blocking command
    let repo = repo.to_string();
    let branch = branch.to_string();
    
    let output = tokio::task::spawn_blocking(move || {
        Command::new("git")
            .arg("ls-remote")
            .arg(&repo)
            .arg(format!("refs/heads/{}", branch))
            .output()
    })
    .await
    .context("Failed to spawn git ls-remote command")?
    .context("Failed to execute git ls-remote")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git ls-remote failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)
        .context("git ls-remote output is not valid UTF-8")?;

    // Parse the output: "COMMIT_HASH\trefs/heads/BRANCH"
    let commit = stdout
        .split_whitespace()
        .next()
        .context("Failed to parse commit hash from git ls-remote output")?
        .to_string();

    if commit.is_empty() {
        anyhow::bail!("Empty commit hash received from git ls-remote");
    }

    Ok(commit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run manually as it requires network access
    async fn test_get_current_commit() {
        let repo = "https://github.com/torvalds/linux.git";
        let branch = "master";
        
        let commit = get_current_commit(repo, branch).await.unwrap();
        
        assert_eq!(commit.len(), 40); // Git commit hashes are 40 characters
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

