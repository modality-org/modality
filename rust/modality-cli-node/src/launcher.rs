//! `modality node` with no subcommand: pick a node action in the TUI.

use anyhow::{bail, Result};
use clap::Args;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use modality_node::config_resolution::load_config_with_node_dir;
use modality_node::logging::LogRing;
use modality_node::pid::read_pid_file;

use super::picker::{pick_action, ActionItem, ActionMenu, PickedAction};
use super::runner::{self, CommonNodeOpts, NodeRole, RunningNode};
use super::tui;

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

/// Open the action picker. Run actions keep the node alive until Stop or quit.
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

    let dir = opts.dir.clone().unwrap_or(std::env::current_dir()?);
    let mut session: Option<RunningNode> = None;
    let log_ring = LogRing::new();

    loop {
        reclaim_finished(&mut session).await?;
        let menu = build_menu(&dir, opts.config.as_ref(), session.as_ref().map(|s| s.role))?;
        let Some(action) = pick_action(menu).await? else {
            if let Some(running) = session.take() {
                running.stop().await?;
            }
            return Ok(());
        };
        if let Err(err) = handle_action(opts, &dir, action, &mut session, &log_ring).await {
            eprintln!("{err:#}");
            wait_for_enter();
        }
    }
}

fn build_menu(
    dir: &PathBuf,
    config: Option<&PathBuf>,
    session_role: Option<NodeRole>,
) -> Result<ActionMenu> {
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
    let pid_running = read_pid_file(dir).ok().flatten().is_some();
    let session_running = session_role.is_some();

    let status_line = if let Some(role) = session_role {
        format!(
            "this CLI is running a {}  ·  q/Esc on the dashboard returns here",
            role.description()
        )
    } else if !config_exists {
        format!(
            "No config.json in {}. Create a node, or point --dir at an existing one.",
            dir.display()
        )
    } else if pid_running {
        format!("config present  ·  role {suggested}  ·  background PID file found")
    } else {
        format!("config present  ·  configured role {suggested}")
    };

    let mut items = Vec::new();
    if session_running {
        items.push(item(
            PickedAction::ViewDashboard,
            "View dashboard",
            "node keeps running if you leave it",
            true,
        ));
    }
    items.extend([
        item(
            PickedAction::RunFromConfig,
            "Run from config",
            if session_running {
                "open the dashboard for the node already running"
            } else {
                "use run_as / miner flags in config.json"
            },
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
            config_exists && !session_running,
        ),
        item(
            PickedAction::Stop,
            if session_running {
                "Stop this node"
            } else {
                "Stop background node"
            },
            if session_running {
                "shut down the node started from this menu"
            } else {
                "SIGTERM the PID in node.pid"
            },
            config_exists || session_running,
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
    ]);

    let selected = items
        .iter()
        .position(|item| {
            if session_running {
                item.action == PickedAction::ViewDashboard
            } else if config_exists {
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
        session_running,
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

fn common_opts(opts: &Opts, dir: &PathBuf) -> CommonNodeOpts {
    CommonNodeOpts {
        config: opts.config.clone(),
        dir: Some(dir.clone()),
        no_tui: true,
        tui: false,
    }
}

async fn reclaim_finished(session: &mut Option<RunningNode>) -> Result<()> {
    if session.as_ref().is_some_and(|s| !s.is_running()) {
        if let Some(finished) = session.take() {
            if let Err(err) = finished.join().await {
                eprintln!("node exited: {err:#}");
            }
        }
    }
    Ok(())
}

async fn open_dashboard(session: &RunningNode) -> Result<()> {
    tui::run(session.source.clone(), session.logs.clone(), false).await
}

async fn ensure_session(
    opts: &Opts,
    dir: &PathBuf,
    role: NodeRole,
    session: &mut Option<RunningNode>,
    log_ring: &LogRing,
) -> Result<()> {
    if session.as_ref().is_some_and(|s| s.is_running()) {
        return Ok(());
    }
    *session = None;
    if read_pid_file(dir).ok().flatten().is_some() {
        bail!(
            "A background node PID file exists in {}. Stop that node before running from this menu.",
            dir.display()
        );
    }
    *session =
        Some(runner::spawn_node(&common_opts(opts, dir), role, false, log_ring.clone()).await?);
    Ok(())
}

async fn handle_action(
    opts: &Opts,
    dir: &PathBuf,
    action: PickedAction,
    session: &mut Option<RunningNode>,
    log_ring: &LogRing,
) -> Result<()> {
    match action {
        PickedAction::ViewDashboard => {
            if let Some(running) = session.as_ref() {
                open_dashboard(running).await?;
            }
        }
        PickedAction::RunFromConfig => {
            let config = load_config_with_node_dir(opts.config.clone(), Some(dir.clone()))?;
            ensure_session(
                opts,
                dir,
                runner::role_from_config(&config)?,
                session,
                log_ring,
            )
            .await?;
            if let Some(running) = session.as_ref() {
                open_dashboard(running).await?;
            }
        }
        PickedAction::Run(role) => {
            ensure_session(opts, dir, role, session, log_ring).await?;
            if let Some(running) = session.as_ref() {
                open_dashboard(running).await?;
            }
        }
        PickedAction::Create => {
            super::create::run(&super::create::Opts::for_dir(dir.clone())).await?;
            wait_for_enter();
        }
        PickedAction::Start => {
            super::start::run(&super::start::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                node_type: None,
            })
            .await?;
            wait_for_enter();
        }
        PickedAction::Stop => {
            if let Some(running) = session.take() {
                running.stop().await?;
                println!("Stopped the node started from this menu.");
            }
            if dir.join("node.pid").exists() {
                super::stop::run(&super::stop::Opts {
                    config: opts.config.clone(),
                    dir: Some(dir.clone()),
                    force: false,
                })
                .await?;
            }
            wait_for_enter();
        }
        PickedAction::Info => {
            super::info::run(&super::info::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                verbose: false,
            })
            .await?;
            wait_for_enter();
        }
        PickedAction::Logs => {
            super::logs::run(&super::logs::Opts {
                config: opts.config.clone(),
                dir: Some(dir.clone()),
                lines: 50,
                follow: false,
                offline: true,
            })
            .await?;
            wait_for_enter();
        }
    }
    Ok(())
}

fn wait_for_enter() {
    print!("\nPress Enter to return to the menu.");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_without_config_selects_create() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let menu = build_menu(&dir, None, None).unwrap();
        assert!(!menu.items[0].enabled);
        assert!(menu.items[5].enabled);
        assert_eq!(menu.items[5].action, PickedAction::Create);
        assert_eq!(menu.selected, 5);
        assert!(!menu.session_running);
    }

    #[test]
    fn menu_with_session_puts_dashboard_first() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let menu = build_menu(&dir, None, Some(NodeRole::Miner)).unwrap();
        assert_eq!(menu.items[0].action, PickedAction::ViewDashboard);
        assert!(menu.items[0].enabled);
        assert_eq!(menu.selected, 0);
        assert!(menu.session_running);
        assert!(menu.status_line.contains("running"));
    }
}
