//! zjctl - Missing CLI surface for Zellij
//!
//! Provides pane-addressed operations via RPC to the zrpc plugin.

use clap::{ArgAction, Parser, Subcommand};

mod client;
mod commands;
mod zellij;

const HELP_AFTER: &str = r#"Quickstart:
  # Install + verify the plugin
  zjctl install --load

  zjctl doctor

  # Launch a shell pane, run a command, capture output
  pane=$(zjctl pane launch -- "zsh")

  zjctl pane send --pane "$pane" -- "ls -la\n"

  # Wait until output stops changing for 2s (or timeout after 30s)
  zjctl pane wait-idle --pane "$pane" --idle-time 2 --timeout 30

  zjctl pane capture --pane "$pane"

  zjctl pane close --pane "$pane"

Selectors:
  id:terminal:N   id:plugin:N   focused
  title:substring title:/regex/
  cmd:substring   cmd:/regex/
  tab:N:index:M

Plugin path:
  --plugin / ZJCTL_PLUGIN_PATH override the default plugin path.

More help:
  zjctl help
"#;

const PANE_HELP: &str = r#"Pane examples:
  # Send input to a pane (default: delay then Enter)
  zjctl pane send --pane id:terminal:3 -- "ls -la\n"

  # Focus by selector
  zjctl pane focus --pane title:server

  # Rename the focused pane
  zjctl pane rename --pane focused "API Server"

  # Resize the focused pane
  zjctl pane resize --pane focused --increase --direction right --step 5

  # Capture output and wait for idle
  zjctl pane capture --pane focused --full

  zjctl pane wait-idle --pane focused --idle-time 2 --timeout 30

  # Close a pane safely (use --force to close focused)
  zjctl pane close --pane id:terminal:3

  # Launch a new pane and print its selector
  zjctl pane launch --direction right -- "zsh"

"#;

const PANES_HELP: &str = r#"Panes examples:
  zjctl panes ls
  zjctl panes ls --json
"#;

const HELP_QUICKSTART: &str = r#"Quickstart:
  # Setup + verify
  zjctl install --load

  zjctl doctor

  # Launch a shell pane and run a command
  pane=$(zjctl pane launch -- "zsh")

  zjctl pane send --pane "$pane" -- "ls -la\n"

  # Wait for output, capture it, then close the pane
  # `wait-idle` repeatedly captures the pane’s rendered screen and returns once it
  # stops changing for `--idle-time` seconds (or errors after `--timeout`).
  # It focuses the pane while checking; by default it restores your previous focus
  # (use --no-restore to keep focus on the pane).
  zjctl pane wait-idle --pane "$pane" --idle-time 2 --timeout 30

  zjctl pane capture --pane "$pane"

  zjctl pane close --pane "$pane"

Tips:
  - Use `zjctl pane <cmd> --help` for command-specific examples
  - Use `zjctl panes ls --json` to drive automation
"#;

const PANE_SEND_HELP: &str = r#"Examples:
  # Send text + Enter (default 1s delay)
  zjctl pane send --pane id:terminal:3 -- "ls -la\n"

  # Send without Enter
  zjctl pane send --pane id:terminal:3 --enter=false -- "ls -la"
"#;

const PANE_FOCUS_HELP: &str = r#"Examples:
  # Focus by title or id
  zjctl pane focus --pane title:server

  zjctl pane focus --pane id:terminal:3
"#;

const PANE_INTERRUPT_HELP: &str = r#"Examples:
  # Send Ctrl+C
  zjctl pane interrupt --pane id:terminal:3
"#;

const PANE_ESCAPE_HELP: &str = r#"Examples:
  # Send Escape
  zjctl pane escape --pane id:terminal:3
"#;

const PANE_CAPTURE_HELP: &str = r#"Examples:
  # Capture output
  zjctl pane capture --pane focused

  zjctl pane capture --pane focused --full
"#;

const PANE_WAIT_HELP: &str = r#"What it does:
  `wait-idle` watches what’s *rendered* in a pane (not the process state).
  It repeatedly captures the pane’s screen and returns once it stops changing for
  at least `--idle-time` seconds (or errors after `--timeout`).

  It focuses the pane while checking; by default it restores your previous focus
  (use `--no-restore` to keep focus on the pane).

Examples:
  # After sending a command, wait until output settles
  zjctl pane wait-idle --pane focused --idle-time 2 --timeout 30
"#;

const PANE_RENAME_HELP: &str = r#"Examples:
  # Rename the focused pane
  zjctl pane rename --pane focused "API Server"
"#;

const PANE_RESIZE_HELP: &str = r#"Examples:
  # Resize the focused pane
  zjctl pane resize --pane focused --increase --direction right --step 5

  # Resize to an exact terminal size
  zjctl pane resize --pane focused --cols 120
  zjctl pane resize --pane focused --rows 40
"#;

const PANE_CLOSE_HELP: &str = r#"Examples:
  # Close a pane (safe by default)
  zjctl pane close --pane id:terminal:3

  zjctl pane close --pane focused --force
"#;

const PANE_LAUNCH_HELP: &str = r#"Examples:
  # Launch a new pane and print its selector
  zjctl pane launch -- "zsh"

  zjctl pane launch --direction right -- "python"
"#;

/// zjctl - Missing CLI surface for Zellij
#[derive(Parser, Debug)]
#[command(
    name = "zjctl",
    version,
    about,
    long_about = None,
    after_help = HELP_AFTER,
    disable_help_subcommand = true
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
    /// Agent-friendly quickstart
    Help,
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
    #[command(after_help = PANE_SEND_HELP)]
    Send {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
        /// Send Enter after the text (true/false)
        #[arg(long, action = ArgAction::Set, default_value_t = true)]
        enter: bool,
        /// Delay before sending Enter (seconds)
        #[arg(long, default_value = "1.0")]
        delay_enter: f64,
        /// Bytes to send (after --)
        #[arg(last = true)]
        bytes: Vec<String>,
    },
    /// Focus a pane
    #[command(after_help = PANE_FOCUS_HELP)]
    Focus {
        /// Pane selector
        #[arg(long)]
        pane: String,
    },
    /// Send Ctrl+C to a pane
    #[command(after_help = PANE_INTERRUPT_HELP)]
    Interrupt {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
    },
    /// Send Escape to a pane
    #[command(after_help = PANE_ESCAPE_HELP)]
    Escape {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Send to all matching panes
        #[arg(long)]
        all: bool,
    },
    /// Capture pane output to stdout
    #[command(after_help = PANE_CAPTURE_HELP)]
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
    #[command(after_help = PANE_WAIT_HELP)]
    WaitIdle {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// How long output must remain unchanged (seconds)
        #[arg(long, default_value = "2.0")]
        idle_time: f64,
        /// Maximum time to wait before erroring (seconds)
        #[arg(long, default_value = "30.0")]
        timeout: f64,
        /// Include scrollback when checking for changes
        #[arg(long)]
        full: bool,
        /// Keep focus on the pane after waiting
        #[arg(long)]
        no_restore: bool,
    },
    /// Rename a pane
    #[command(after_help = PANE_RENAME_HELP)]
    Rename {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// New name for the pane
        name: String,
    },
    /// Resize a pane
    #[command(after_help = PANE_RESIZE_HELP)]
    Resize {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Increase pane size
        #[arg(long, conflicts_with_all = ["decrease", "cols", "rows"])]
        increase: bool,
        /// Decrease pane size
        #[arg(long, conflicts_with_all = ["increase", "cols", "rows"])]
        decrease: bool,
        /// Resize to a target number of columns (terminal size)
        #[arg(long, conflicts_with_all = ["increase", "decrease", "step"])]
        cols: Option<usize>,
        /// Resize to a target number of rows (terminal size)
        #[arg(long, conflicts_with_all = ["increase", "decrease", "step"])]
        rows: Option<usize>,
        /// Direction (left, right, up, down)
        #[arg(long)]
        direction: Option<String>,
        /// Step size
        #[arg(long, default_value = "1")]
        step: u32,
        /// Maximum resize steps when using --cols/--rows
        #[arg(long, default_value = "200")]
        max_steps: u32,
    },
    /// Close a pane (refuses to close focused unless --force)
    #[command(after_help = PANE_CLOSE_HELP)]
    Close {
        /// Pane selector
        #[arg(long)]
        pane: String,
        /// Force closing focused pane
        #[arg(long)]
        force: bool,
    },
    /// Launch a new pane and print its selector
    #[command(after_help = PANE_LAUNCH_HELP)]
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
        Commands::Help => {
            print_help_quickstart();
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
            PaneCommands::Send {
                pane,
                all,
                enter,
                delay_enter,
                bytes,
            } => {
                commands::pane::send(plugin, &pane, all, enter, delay_enter, &bytes)?;
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
                cols,
                rows,
                direction,
                step,
                max_steps,
            } => {
                commands::pane::resize(
                    plugin,
                    commands::pane::ResizeOptions {
                        selector: &pane,
                        increase,
                        decrease,
                        cols,
                        rows,
                        direction: direction.as_deref(),
                        step,
                        max_steps,
                    },
                )?;
            }
            PaneCommands::Close { pane, force } => {
                commands::pane::close(plugin, &pane, force)?;
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
                let options = commands::pane::LaunchOptions {
                    direction: direction.as_deref(),
                    floating,
                    name: name.as_deref(),
                    cwd: cwd.as_deref(),
                    close_on_exit,
                    in_place,
                    start_suspended,
                    command: &command,
                };
                commands::pane::launch(plugin, options)?;
            }
        },
    }
    Ok(())
}

fn print_help_quickstart() {
    println!("zjctl help");
    println!("==========");
    println!("{HELP_QUICKSTART}");
    println!("See `zjctl --help` for the full CLI reference.");
}
