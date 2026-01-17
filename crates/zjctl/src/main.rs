//! zjctl - Missing CLI surface for Zellij
//!
//! Provides pane-addressed operations via RPC to the zrpc plugin.

use clap::{Parser, Subcommand};

mod client;
mod commands;

/// zjctl - Missing CLI surface for Zellij
#[derive(Parser, Debug)]
#[command(name = "zjctl", version, about, long_about = None)]
pub struct Cli {
    /// Path to the zrpc plugin wasm file
    #[arg(long, env = "ZJCTL_PLUGIN_PATH")]
    plugin: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List panes in the current session
    Panes {
        #[command(subcommand)]
        cmd: PanesCommands,
    },
    /// Pane operations
    Pane {
        #[command(subcommand)]
        cmd: PaneCommands,
    },
    /// Show focused pane and tab status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Pass-through to zellij action
    Action {
        /// Arguments to pass to zellij action
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run setup checks for zjctl + zrpc
    Doctor {
        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,
    },
    /// Install the zrpc plugin
    Install {
        /// Print the commands that would be run
        #[arg(long)]
        print: bool,
        /// Re-download plugin even if it exists
        #[arg(long)]
        force: bool,
        /// Attempt to load the plugin in the current Zellij session
        #[arg(long)]
        load: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PanesCommands {
    /// List all panes
    Ls {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PaneCommands {
    /// Send input to a pane
    Send {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
        /// Bytes to send (after --)
        #[arg(last = true)]
        bytes: Vec<String>,
    },
    /// Focus a pane
    Focus {
        /// Pane selector
        #[arg(long)]
        pane: String,
    },
    /// Send Ctrl+C to a pane
    Interrupt {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
    },
    /// Send Escape to a pane
    Escape {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
    },
    /// Capture pane output to stdout
    Capture {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Include scrollback
        #[arg(long)]
        full: bool,
        /// Keep focus on captured pane
        #[arg(long)]
        no_restore: bool,
    },
    /// Wait for pane output to stop changing
    WaitIdle {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Idle time in seconds
        #[arg(long, default_value = "2.0")]
        idle_time: f64,
        /// Timeout in seconds
        #[arg(long, default_value = "30.0")]
        timeout: f64,
        /// Include scrollback in checks
        #[arg(long)]
        full: bool,
        /// Keep focus on pane after waiting
        #[arg(long)]
        no_restore: bool,
    },
    /// Rename a pane
    Rename {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// New name for the pane
        name: String,
    },
    /// Resize a pane
    Resize {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Increase pane size
        #[arg(long, conflicts_with = "decrease")]
        increase: bool,
        /// Decrease pane size
        #[arg(long, conflicts_with = "increase")]
        decrease: bool,
        /// Direction (left, right, up, down)
        #[arg(long)]
        direction: Option<String>,
        /// Step size
        #[arg(long, default_value = "1")]
        step: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let plugin = cli.plugin.as_deref();

    match cli.command {
        Commands::Action { args } => {
            commands::action::run(&args)?;
        }
        Commands::Doctor { json } => {
            commands::doctor::run(plugin, json)?;
        }
        Commands::Install { print, force, load } => {
            commands::install::run(plugin, print, force, load)?;
        }
        Commands::Status { json } => {
            commands::status::run(plugin, json)?;
        }
        Commands::Panes { cmd } => match cmd {
            PanesCommands::Ls { json } => {
                commands::panes::ls(plugin, json)?;
            }
        },
        Commands::Pane { cmd } => match cmd {
            PaneCommands::Send { pane, all, bytes } => {
                commands::pane::send(plugin, &pane, all, &bytes)?;
            }
            PaneCommands::Focus { pane } => {
                commands::pane::focus(plugin, &pane)?;
            }
            PaneCommands::Interrupt { pane, all } => {
                commands::pane::interrupt(plugin, &pane, all)?;
            }
            PaneCommands::Escape { pane, all } => {
                commands::pane::escape(plugin, &pane, all)?;
            }
            PaneCommands::Capture {
                pane,
                full,
                no_restore,
            } => {
                commands::pane::capture(plugin, &pane, full, no_restore)?;
            }
            PaneCommands::WaitIdle {
                pane,
                idle_time,
                timeout,
                full,
                no_restore,
            } => {
                commands::pane::wait_idle(plugin, &pane, idle_time, timeout, full, no_restore)?;
            }
            PaneCommands::Rename { pane, name } => {
                commands::pane::rename(plugin, &pane, &name)?;
            }
            PaneCommands::Resize {
                pane,
                increase,
                decrease,
                direction,
                step,
            } => {
                commands::pane::resize(
                    plugin,
                    &pane,
                    increase,
                    decrease,
                    direction.as_deref(),
                    step,
                )?;
            }
        },
    }
    Ok(())
}
