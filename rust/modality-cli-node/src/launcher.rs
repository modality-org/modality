//! `modality node` with no subcommand: pick a node action in the TUI.

use anyhow::{bail, Result};
use clap::Args;
use std::io::IsTerminal;
use std::path::PathBuf;

use modality_node::config_resolution::load_config_with_node_dir;
use modality_node::pid::read_pid_file;

use super::picker::{pick_action, ActionItem, ActionMenu, PickedAction};
use super::runner::{self, CommonNodeOpts, NodeRole};

/// Options for the no-subcommand node launcher.
#[derive(Debug, Clone, Args)]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Do not open the action picker (print an error; pass a subcommand instead)
    #[clap(long)]
    pub no_tui: bool,

    /// Force the action picker even when stdout is not a TTY
    #[clap(long, conflicts_with = "no_tui")]
    pub tui: bool,
}

/// Open the action picker, then run the chosen command.
pub async fn run(opts: &Opts) -> Result<()> {
    if opts.no_tui || std::env::var_os("MODALITY_NO_TUI").is_some() {
        bail!(
            "`modality node` without a subcommand opens an action picker TUI.\n\
             Pass a command such as `run-hybrid`, or run this in a terminal without `--no-tui`."
        );
    }
    if !opts.tui && !std::io::stdout().is_terminal() {
        bail!(
            "`modality node` without a subcommand opens an action picker TUI.\n\
             Run it in a terminal, or pass a command such as `run-hybrid --dir …`."
        );
    }

    let dir = opts
        .dir
        .clone()
        .unwrap_or(std::env::current_dir()?);
    let menu = build_menu(&dir, opts.config.as_ref())?;
    let Some(action) = pick_action(menu).await? else {
        return Ok(());
    };
    dispatch(opts, &dir, action).await
}

fn build_menu(dir: &PathBuf, config: Option<&PathBuf>) -> Result<ActionMenu> {
    let config_path = config.cloned().unwrap_or_else(|| dir.join("config.json"));
    let config_exists = config_path.exists();
    let loaded = if config_exists {
        load_config_with_node_dir(config.cloned(), Some(dir.clone())).ok()
    } else {
        None
    };
    let suggested = loaded
        .as_ref()
        .map(|c| c.get_node_role())
        .unwrap_or_else(|| "none".to_string());
    let running = read_pid_file(dir).ok().flatten().is_some();

    let status_line = if !config_exists {
        format!(
            "No config.json in {}. Create a node, or point --dir at an existing one.",
            dir.display()
        )
    } else if running {
        format!("config present  ·  role {suggested}  ·  background PID file found")
    } else {
        format!("config present  ·  configured role {suggested}")
    };

    let items = vec![
        item(
            PickedAction::RunFromConfig,
            "Run from config",
            "use run_as / miner flags in config.json",
            config_exists,
        ),
        item(
            PickedAction::Run(NodeRole::Hybrid),
            "Run hybrid",
            "mine and sequence under N−2 lookback",
            config_exists,
        ),
        item(
            PickedAction::Run(NodeRole::Miner),
            "Run miner",
            "PoW block production",
            config_exists,
        ),
        item(
            PickedAction::Run(NodeRole::Validator),
            "Run validator",
            "sequence only; do not mine",
            config_exists,
        ),
        item(
            PickedAction::Run(NodeRole::Observer),
            "Run observer",
            "follow the chain; do not mine",
            config_exists,
        ),
        item(
            PickedAction::Create,
            "Create node",
            "write config.json and node.modal_passfile",
            !config_exists,
        ),
        item(
            PickedAction::Start,
            "Start in background",
            "detach with node.pid; no TUI",
            config_exists,
        ),
        item(
            PickedAction::Stop,
            "Stop background node",
            "SIGTERM the PID in node.pid",
            config_exists,
        ),
        item(
            PickedAction::Info,
            "Show info",
            "peer id, listeners, chain tip",
            config_exists,
        ),
        item(
            PickedAction::Logs,
            "Tail logs",
            "last 50 lines from the node log file",
            config_exists,
        ),
    ];

    let selected = items
        .iter()
        .position(|item| {
            if config_exists {
                item.action == PickedAction::RunFromConfig
            } else {
                item.action == PickedAction::Create
            }
        })
        .unwrap_or(0);

    Ok(ActionMenu {
        dir_display: dir.display().to_string(),
        status_line,
        items,
        selected,
    })
}

fn item(action: PickedAction, title: &str, hint: &str, enabled: bool) -> ActionItem {
    ActionItem {
        action,
        title: title.to_string(),
        hint: hint.to_string(),
        enabled,
    }
}

async fn dispatch(opts: &Opts, dir: &PathBuf, action: PickedAction) -> Result<()> {
    let common = CommonNodeOpts {
        config: opts.config.clone(),
        dir: Some(dir.clone()),
        no_tui: false,
        tui: opts.tui,
    };
    match action {
        PickedAction::RunFromConfig => runner::run_server(&common).await,
        PickedAction::Run(role) => runner::run_node(&common, role, true).await,
        PickedAction::Create => super::create::run(&super::create::Opts::for_dir(dir.clone())).await,
        PickedAction::Start => {
            super::start::run(&super::start::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                node_type: None,
            })
            .await
        }
        PickedAction::Stop => {
            super::stop::run(&super::stop::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                force: false,
            })
            .await
        }
        PickedAction::Info => {
            super::info::run(&super::info::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                verbose: false,
            })
            .await
        }
        PickedAction::Logs => {
            super::logs::run(&super::logs::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                lines: 50,
                follow: false,
                offline: true,
            })
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_without_config_selects_create() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let menu = build_menu(&dir, None).unwrap();
        assert!(!menu.items[0].enabled);
        assert!(menu.items[5].enabled);
        assert_eq!(menu.items[5].action, PickedAction::Create);
        assert_eq!(menu.selected, 5);
    }
}
