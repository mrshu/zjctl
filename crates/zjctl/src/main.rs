//! zjctl - Missing CLI surface for Zellij
//!
//! Provides pane-addressed operations via RPC to the zrpc plugin.

use clap::{Parser, Subcommand};

mod client;
mod commands;

const HELP_AFTER: &str = r#"Examples:
  zjctl doctor
  zjctl status --json
  zjctl panes ls --json
  zjctl pane send --pane id:terminal:3 -- "ls -la\n"
  zjctl pane send --pane title:server -- "cargo run\n"
  zjctl pane send --pane cmd:/python/ -- "print('hi')\n"
  zjctl pane launch
  zjctl pane launch --direction right -- "zsh"
  zjctl pane capture --pane focused
  zjctl pane wait-idle --pane focused --idle-time 2 --timeout 30
  zjctl pane interrupt --pane title:vim
  zjctl pane escape --pane title:vim

Selectors:
  id:terminal:N   id:plugin:N   focused
  title:substring title:/regex/
  cmd:substring   cmd:/regex/
  tab:N:index:M

Plugin path:
  --plugin / ZJCTL_PLUGIN_PATH override the default plugin path.
"#;

const PANE_HELP: &str = r#"Pane examples:
  zjctl pane send --pane id:terminal:3 -- "ls -la\n"
  zjctl pane focus --pane title:server
  zjctl pane rename --pane focused "API Server"
  zjctl pane resize --pane focused --increase --direction right --step 5
  zjctl pane launch
  zjctl pane launch --direction right -- "zsh"
  zjctl pane capture --pane focused --full
  zjctl pane wait-idle --pane focused --idle-time 2 --timeout 30
"#;

const PANES_HELP: &str = r#"Panes examples:
  zjctl panes ls
  zjctl panes ls --json
"#;

/// zjctl - Missing CLI surface for Zellij
#[derive(Parser, Debug)]
#[command(
    name = "zjctl",
    version,
    about,
    long_about = None,
    after_help = HELP_AFTER
)]
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
        /// Add the plugin to config.kdl load_plugins (default: true)
        #[arg(long, default_value_t = true)]
        auto_load: bool,
        /// Do not add the plugin to config.kdl load_plugins
        #[arg(long, conflicts_with = "auto_load")]
        no_auto_load: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(after_help = PANES_HELP)]
enum PanesCommands {
    /// List all panes
    Ls {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(after_help = PANE_HELP)]
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
    /// Launch a new pane and print its selector
    Launch {
        /// Direction to open the pane (right, down)
        #[arg(long)]
        direction: Option<String>,
        /// Open the pane in floating mode
        #[arg(long)]
        floating: bool,
        /// Name of the new pane
        #[arg(long)]
        name: Option<String>,
        /// Working directory for the new pane
        #[arg(long)]
        cwd: Option<String>,
        /// Close the pane when the command exits
        #[arg(long)]
        close_on_exit: bool,
        /// Open the pane in-place, suspending the current pane
        #[arg(long)]
        in_place: bool,
        /// Start the command suspended until Enter is pressed
        #[arg(long)]
        start_suspended: bool,
        /// Command to run in the new pane (after --)
        #[arg(last = true)]
        command: Vec<String>,
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
        Commands::Install {
            print,
            force,
            load,
            auto_load,
            no_auto_load,
        } => {
            let auto_load = if no_auto_load { false } else { auto_load };
            commands::install::run(plugin, print, force, load, auto_load)?;
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
            PaneCommands::Launch {
                direction,
                floating,
                name,
                cwd,
                close_on_exit,
                in_place,
                start_suspended,
                command,
            } => {
                commands::pane::launch(
                    plugin,
                    direction.as_deref(),
                    floating,
                    name.as_deref(),
                    cwd.as_deref(),
                    close_on_exit,
                    in_place,
                    start_suspended,
                    &command,
                )?;
            }
        },
    }
    Ok(())
}
