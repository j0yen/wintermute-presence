//! `wm-presence` — CLI entrypoint.
//!
//! Subcommands:
//! - `daemon`  — run the subscribe loop (systemd `wm-presence.service`)
//! - `status`  — print today's count + last interaction + window

#![allow(clippy::print_stdout)]

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use wintermute_presence::config;
use wintermute_presence::daemon;
use wintermute_presence::state::StateStore;
use wintermute_presence::status::format_status;

/// Privacy-first presence heartbeat daemon.
#[derive(Parser, Debug)]
#[command(name = "wm-presence", version, about)]
pub struct Cli {
    /// Path to config directory (default: /etc/wintermute/conf.d)
    #[arg(long, env = "WM_CONF_DIR", default_value = "/etc/wintermute/conf.d")]
    pub conf_dir: PathBuf,

    /// Path to agorabus socket.
    #[arg(long, env = "AGORABUS_SOCK")]
    pub bus_sock: Option<PathBuf>,

    /// Path to the state file.
    #[arg(long, env = "WM_PRESENCE_STATE")]
    pub state_path: Option<PathBuf>,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands for `wm-presence`.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the subscribe loop (long-lived service).
    Daemon,
    /// Print today's interaction count and last-interaction timestamp.
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let cfg = config::load(&cli.conf_dir).context("loading presence config")?;

    let state_path = match cli.state_path {
        Some(p) => p,
        None => StateStore::default_path()?,
    };
    let store = StateStore::new(state_path);

    match cli.command {
        Command::Daemon => {
            let sock = cli
                .bus_sock
                .unwrap_or_else(agorabus::default_socket_path);
            daemon::run(&sock, &cfg, &store).await?;
        }
        Command::Status => {
            let state = store.load().await.context("loading state")?;
            println!("{}", format_status(&state));
        }
    }

    Ok(())
}
