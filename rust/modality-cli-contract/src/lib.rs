//! Contract management commands for Modal CLI.

pub mod add_rule;
pub mod ai;
pub mod assets;
pub mod checkout;
pub mod commit;
pub mod commit_id;
pub mod create;
pub mod diff;
pub mod download;
pub mod id;
pub mod log;
#[cfg(feature = "model-status")]
mod model_governance;
pub mod pack;
pub mod pull;
pub mod push;
pub mod remote;
pub mod repost;
pub mod set;
pub mod set_named_id;
pub mod status;
pub mod unpack;
pub mod wasm_upload;

#[cfg(test)]
mod tests {
    use clap::Parser;
    use modality_common::contract_store::ContractStore;
    use serde_json::Value;
    use tempfile::TempDir;

    #[tokio::test]
    async fn empty_directory_contract_log_flow_smoke() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let contract_dir = temp_dir.path().join("first-contract");
        let contract_dir_arg = contract_dir.to_string_lossy().to_string();

        let create_opts = crate::create::Opts::parse_from([
            "create",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        crate::create::run(&create_opts).await?;

        let store = ContractStore::open(&contract_dir)?;
        let contract_id = store.load_config()?.contract_id;
        let genesis_head = store
            .get_head()?
            .expect("contract creation should write a genesis HEAD");
        assert_eq!(store.list_commits()?.len(), 1);
        assert!(contract_dir.join(".contract/config.json").exists());
        assert!(contract_dir.join("model/default.modality").exists());

        let checkout_opts =
            crate::checkout::Opts::parse_from(["checkout", "--dir", contract_dir_arg.as_str()]);
        crate::checkout::run(&checkout_opts).await?;
        assert!(contract_dir.join("state").exists());
        assert!(contract_dir.join("rules").exists());

        let set_opts = crate::set::Opts::parse_from([
            "set",
            "/parties/alice.id",
            contract_id.as_str(),
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        crate::set::run(&set_opts).await?;
        assert_eq!(
            std::fs::read_to_string(contract_dir.join("state/parties/alice.id"))?,
            contract_id.as_str()
        );

        let commit_opts = crate::commit::Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
            "--message",
            "Add Alice",
        ]);
        crate::commit::run(&commit_opts).await?;

        let committed_head = store
            .get_head()?
            .expect("committing state should advance HEAD");
        assert_ne!(committed_head, genesis_head);
        let committed = store.load_commit(&committed_head)?;
        assert!(
            committed.body.iter().any(|action| {
                action.method == "model"
                    && action.path.as_deref() == Some("/model/default.modality")
            }),
            "first --all commit should persist the checked-out governing model"
        );
        assert_eq!(store.list_commits()?.len(), 2);
        assert_eq!(
            store.build_state_from_commits()?.get("/parties/alice.id"),
            Some(&Value::String(contract_id))
        );

        let id_opts = crate::id::Opts::parse_from(["id", "--dir", contract_dir_arg.as_str()]);
        crate::id::run(&id_opts).await?;

        let status_opts = crate::status::Opts::parse_from([
            "status",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        crate::status::run(&status_opts).await?;

        let log_opts = crate::log::Opts::parse_from([
            "log",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        crate::log::run(&log_opts).await?;

        Ok(())
    }

    #[tokio::test]
    async fn repost_from_local_dir_commits_as_repost() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");
        let source_dir_arg = source_dir.to_string_lossy().to_string();
        let dest_dir_arg = dest_dir.to_string_lossy().to_string();

        crate::create::run(&crate::create::Opts::parse_from([
            "create",
            "--dir",
            source_dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;
        crate::create::run(&crate::create::Opts::parse_from([
            "create",
            "--dir",
            dest_dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;

        let source_store = ContractStore::open(&source_dir)?;
        let source_id = source_store.load_config()?.contract_id;

        crate::set::run(&crate::set::Opts::parse_from([
            "set",
            "/notes/hello.text",
            "hello from source",
            "--dir",
            source_dir_arg.as_str(),
        ]))
        .await?;
        crate::commit::run(&crate::commit::Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            source_dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;

        crate::repost::run(&crate::repost::Opts::parse_from([
            "repost",
            source_id.as_str(),
            "/notes/hello.text",
            "--from-dir",
            source_dir_arg.as_str(),
            "--dir",
            dest_dir_arg.as_str(),
        ]))
        .await?;

        crate::commit::run(&crate::commit::Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            dest_dir_arg.as_str(),
            "--output",
            "json",
        ]))
        .await?;

        let dest_store = ContractStore::open(&dest_dir)?;
        let head = dest_store.get_head()?.expect("dest should have HEAD");
        let commit = dest_store.load_commit(&head)?;
        let repost = commit
            .body
            .iter()
            .find(|action| action.method == "repost")
            .expect("commit --all should emit method repost");
        let expected_dest = format!("/reposts/{source_id}/notes/hello.text");
        assert_eq!(repost.path.as_deref(), Some(expected_dest.as_str()));
        assert_eq!(repost.source_contract.as_deref(), Some(source_id.as_str()));
        assert_eq!(repost.source_path.as_deref(), Some("/notes/hello.text"));
        assert!(repost.source_commit.is_some());
        assert_eq!(
            dest_store.build_state_from_commits()?.get(&expected_dest),
            Some(&Value::String("hello from source".to_string()))
        );

        Ok(())
    }
}
