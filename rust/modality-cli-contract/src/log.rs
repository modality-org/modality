use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::{CommitFile, ContractStore};

#[derive(Debug, Parser)]
#[command(about = "Show commit history for a contract")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Number of commits to show (default: all)
    #[clap(short = 'n', long)]
    limit: Option<usize>,

    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;

    // Get HEAD and walk backwards through commits
    let head = store.get_head()?;

    if head.is_none() {
        if opts.output == "json" {
            println!("{{\"commits\": []}}");
        } else {
            println!("No commits yet.");
        }
        return Ok(());
    }

    let mut commits = Vec::new();
    let mut current = head;
    let mut count = 0;

    while let Some(commit_id) = current {
        if let Some(limit) = opts.limit {
            if count >= limit {
                break;
            }
        }

        let commit = store.load_commit(&commit_id)?;
        commits.push((commit_id.clone(), commit.clone()));
        current = commit.head.parent.clone();
        count += 1;
    }

    if opts.output == "json" {
        let json_commits: Vec<serde_json::Value> = commits
            .iter()
            .map(|(id, commit)| commit_summary_json(id, commit))
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "commits": json_commits
            }))?
        );
    } else {
        let config = store.load_config()?;
        println!("Contract: {}", config.contract_id);
        println!("Commits: {}\n", commits.len());

        for (id, commit) in &commits {
            let short_id = if id.len() > 12 { &id[..12] } else { id };

            println!("commit {} ({}...)", short_id, &id[..8.min(id.len())]);
            if let Some(parent) = &commit.head.parent {
                let short_parent = if parent.len() > 12 {
                    &parent[..12]
                } else {
                    parent
                };
                println!("Parent: {}...", short_parent);
            }
            if let Some(message) = &commit.head.message {
                println!("Message: {}", message);
            }

            let signers = commit_signers(commit);
            println!("Signatures: {}", signers.len());
            if !signers.is_empty() {
                println!("Signers:");
                for signer in &signers {
                    println!("  {}", signer);
                }
            }

            // Show actions summary
            if !commit.body.is_empty() {
                println!("Actions:");
                for action in &commit.body {
                    let path = action.path.as_deref().unwrap_or("/");
                    println!("  {} {}", action.method, path);
                }
            }
            println!();
        }
    }

    Ok(())
}

fn commit_summary_json(id: &str, commit: &CommitFile) -> serde_json::Value {
    let signers = commit_signers(commit);

    serde_json::json!({
        "id": id,
        "parent": commit.head.parent,
        "message": commit.head.message,
        "actions": commit.body.len(),
        "signature_count": signers.len(),
        "signers": signers,
    })
}

fn commit_signers(commit: &CommitFile) -> Vec<String> {
    let mut signers = commit
        .head
        .signatures
        .as_ref()
        .and_then(|signatures| signatures.as_object())
        .map(|signatures| signatures.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    signers.sort();
    signers
}

#[cfg(test)]
mod tests {
    use super::{commit_signers, commit_summary_json};
    use modal_common::contract_store::CommitFile;

    #[test]
    fn json_summary_reports_signature_evidence() {
        let mut commit = CommitFile::with_parent("parent-id".to_string());
        commit.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            serde_json::json!("alice-id"),
        );
        commit.head.signatures = Some(serde_json::json!({
            "alice-id": "sig"
        }));
        commit.head.message = Some("Initial contract setup".to_string());

        let summary = commit_summary_json("commit-id", &commit);

        assert_eq!(summary["id"], "commit-id");
        assert_eq!(summary["parent"], "parent-id");
        assert_eq!(summary["message"], "Initial contract setup");
        assert_eq!(summary["actions"], 1);
        assert_eq!(summary["signature_count"], 1);
        assert!(summary["signers"]
            .as_array()
            .expect("signers should be an array")
            .iter()
            .any(|signer| signer == "alice-id"));
    }

    #[test]
    fn commit_signers_are_stable_for_text_and_json_logs() {
        let mut commit = CommitFile::with_parent("parent-id".to_string());
        commit.head.signatures = Some(serde_json::json!({
            "bob-id": "sig-b",
            "alice-id": "sig-a"
        }));

        assert_eq!(
            commit_signers(&commit),
            vec!["alice-id".to_string(), "bob-id".to_string()]
        );
    }
}
