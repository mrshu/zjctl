//! Diagnostics and setup checks

use std::process::Command;

use crate::client::{self, ClientError};
use zjctl_proto::methods;

pub fn run(plugin: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let default_url = client::default_plugin_url();
    let plugin_url = plugin.unwrap_or(default_url.as_str());
    let mut ok = true;

    println!("zjctl doctor");
    println!("============");

    let mut zellij_ok = false;
    match Command::new("zellij").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.is_empty() {
                println!("zellij: ok");
            } else {
                println!("zellij: ok ({version})");
            }
            zellij_ok = true;
        }
        Ok(output) => {
            ok = false;
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let note = if stderr.is_empty() {
                "exit code non-zero".to_string()
            } else {
                stderr
            };
            println!("zellij: fail ({note})");
        }
        Err(err) => {
            ok = false;
            println!("zellij: missing ({err})");
        }
    }

    println!("plugin url: {plugin_url}");

    let plugin_path = client::plugin_file_path(plugin_url);
    let plugin_file_ok = match &plugin_path {
        Some(path) if path.is_file() => {
            println!("plugin file: ok ({})", path.display());
            true
        }
        Some(path) => {
            ok = false;
            println!("plugin file: missing ({})", path.display());
            let (install_cmd, download_cmd, launch_cmd) =
                client::plugin_install_commands(plugin_url, path);
            println!("install: {install_cmd}");
            println!("install: {download_cmd}");
            println!("load: {launch_cmd}");
            false
        }
        None => {
            println!("plugin file: skip (non-file plugin url)");
            true
        }
    };

    let mut sessions_ok = false;
    if zellij_ok {
        match Command::new("zellij").arg("list-sessions").output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sessions: Vec<&str> =
                    stdout.lines().filter(|line| !line.trim().is_empty()).collect();
                if sessions.is_empty() {
                    ok = false;
                    println!("sessions: none (start Zellij to load plugins)");
                } else {
                    sessions_ok = true;
                    let count = sessions.len();
                    let suffix = if count == 1 { "" } else { "s" };
                    println!("sessions: ok ({count} session{suffix})");
                }
            }
            Ok(output) => {
                ok = false;
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let note = if stderr.is_empty() {
                    "exit code non-zero".to_string()
                } else {
                    stderr
                };
                println!("sessions: fail ({note})");
            }
            Err(err) => {
                ok = false;
                println!("sessions: fail ({err})");
            }
        }
    } else {
        println!("sessions: skip (zellij missing)");
    }

    if zellij_ok && plugin_file_ok && sessions_ok {
        match client::rpc_call(plugin, methods::PANES_LIST, serde_json::json!({})) {
            Ok(_) => println!("rpc: ok (plugin responding)"),
            Err(err) => {
                ok = false;
                match err {
                    ClientError::PluginNotLoaded { launch_cmd } => {
                        println!("rpc: fail (no response from plugin)");
                        println!("load: {launch_cmd}");
                    }
                    ClientError::PipeError { stderr, .. } => {
                        println!("rpc: fail ({stderr})");
                    }
                    ClientError::ZellijMissing => {
                        println!("rpc: fail (zellij not found)");
                    }
                    other => {
                        println!("rpc: fail ({other})");
                    }
                }
            }
        }
    } else if zellij_ok && plugin_file_ok {
        ok = false;
        println!("rpc: skip (no active sessions)");
    } else {
        println!("rpc: skip (missing prerequisites)");
    }

    if ok {
        Ok(())
    } else {
        Err("doctor found issues".into())
    }
}
