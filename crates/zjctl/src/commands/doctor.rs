//! Diagnostics and setup checks

use std::process::Command;

use crate::client::{self, ClientError};
use serde::Serialize;
use zjctl_proto::methods;

#[derive(Serialize)]
struct DoctorReport {
    ok: bool,
    plugin_url: String,
    plugin_path: Option<String>,
    checks: Vec<CheckReport>,
}

#[derive(Serialize)]
struct CheckReport {
    name: String,
    status: String,
    detail: Option<String>,
    commands: Vec<String>,
}

struct Check {
    name: &'static str,
    status: &'static str,
    detail: Option<String>,
    commands: Vec<String>,
}

fn push_check(
    checks: &mut Vec<Check>,
    ok: &mut bool,
    name: &'static str,
    status: &'static str,
    detail: Option<String>,
    commands: Vec<String>,
) {
    if status == "fail" {
        *ok = false;
    }
    checks.push(Check {
        name,
        status,
        detail,
        commands,
    });
}

pub fn run(plugin: Option<&str>, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let default_url = client::default_plugin_url();
    let plugin_url = plugin.unwrap_or(default_url.as_str()).to_string();
    let plugin_path = client::plugin_file_path(&plugin_url);
    let plugin_path_display = plugin_path.as_ref().map(|path| path.display().to_string());
    let mut ok = true;
    let mut checks = Vec::new();

    let mut zellij_ok = false;
    match Command::new("zellij").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if version.is_empty() {
                None
            } else {
                Some(version)
            };
            push_check(&mut checks, &mut ok, "zellij", "ok", detail, Vec::new());
            zellij_ok = true;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let note = if stderr.is_empty() {
                "exit code non-zero".to_string()
            } else {
                stderr
            };
            push_check(
                &mut checks,
                &mut ok,
                "zellij",
                "fail",
                Some(note),
                Vec::new(),
            );
        }
        Err(err) => {
            push_check(
                &mut checks,
                &mut ok,
                "zellij",
                "fail",
                Some(err.to_string()),
                Vec::new(),
            );
        }
    }

    let plugin_file_ok = match &plugin_path {
        Some(path) if path.is_file() => {
            push_check(
                &mut checks,
                &mut ok,
                "plugin-file",
                "ok",
                Some(path.display().to_string()),
                Vec::new(),
            );
            true
        }
        Some(path) => {
            let (install_cmd, download_cmd, launch_cmd) =
                client::plugin_install_commands(&plugin_url, path);
            push_check(
                &mut checks,
                &mut ok,
                "plugin-file",
                "fail",
                Some(path.display().to_string()),
                vec![install_cmd, download_cmd, launch_cmd],
            );
            false
        }
        None => {
            push_check(
                &mut checks,
                &mut ok,
                "plugin-file",
                "skip",
                Some("non-file plugin url".to_string()),
                Vec::new(),
            );
            true
        }
    };

    let mut sessions_ok = false;
    if zellij_ok {
        match Command::new("zellij").arg("list-sessions").output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let sessions: Vec<&str> = stdout
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect();
                if sessions.is_empty() {
                    push_check(
                        &mut checks,
                        &mut ok,
                        "sessions",
                        "fail",
                        Some("no active sessions".to_string()),
                        Vec::new(),
                    );
                } else {
                    sessions_ok = true;
                    let count = sessions.len();
                    let suffix = if count == 1 { "" } else { "s" };
                    push_check(
                        &mut checks,
                        &mut ok,
                        "sessions",
                        "ok",
                        Some(format!("{count} session{suffix}")),
                        Vec::new(),
                    );
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let note = if stderr.is_empty() {
                    "exit code non-zero".to_string()
                } else {
                    stderr
                };
                push_check(
                    &mut checks,
                    &mut ok,
                    "sessions",
                    "fail",
                    Some(note),
                    Vec::new(),
                );
            }
            Err(err) => {
                push_check(
                    &mut checks,
                    &mut ok,
                    "sessions",
                    "fail",
                    Some(err.to_string()),
                    Vec::new(),
                );
            }
        }
    } else {
        push_check(
            &mut checks,
            &mut ok,
            "sessions",
            "skip",
            Some("zellij missing".to_string()),
            Vec::new(),
        );
    }

    if zellij_ok && plugin_file_ok && sessions_ok {
        match client::rpc_call(plugin, methods::PANES_LIST, serde_json::json!({})) {
            Ok(_) => push_check(
                &mut checks,
                &mut ok,
                "rpc",
                "ok",
                Some("plugin responding".to_string()),
                Vec::new(),
            ),
            Err(err) => match err {
                ClientError::PluginNotLoaded { launch_cmd } => push_check(
                    &mut checks,
                    &mut ok,
                    "rpc",
                    "fail",
                    Some("no response from plugin".to_string()),
                    vec![launch_cmd],
                ),
                ClientError::PipeError { stderr, .. } => push_check(
                    &mut checks,
                    &mut ok,
                    "rpc",
                    "fail",
                    Some(stderr),
                    Vec::new(),
                ),
                ClientError::ZellijMissing => push_check(
                    &mut checks,
                    &mut ok,
                    "rpc",
                    "fail",
                    Some("zellij not found".to_string()),
                    Vec::new(),
                ),
                other => push_check(
                    &mut checks,
                    &mut ok,
                    "rpc",
                    "fail",
                    Some(other.to_string()),
                    Vec::new(),
                ),
            },
        }
    } else if zellij_ok && plugin_file_ok {
        push_check(
            &mut checks,
            &mut ok,
            "rpc",
            "skip",
            Some("no active sessions".to_string()),
            Vec::new(),
        );
    } else {
        push_check(
            &mut checks,
            &mut ok,
            "rpc",
            "skip",
            Some("missing prerequisites".to_string()),
            Vec::new(),
        );
    }

    if json {
        let report = DoctorReport {
            ok,
            plugin_url: plugin_url.clone(),
            plugin_path: plugin_path_display,
            checks: checks
                .iter()
                .map(|check| CheckReport {
                    name: check.name.to_string(),
                    status: check.status.to_string(),
                    detail: check.detail.clone(),
                    commands: check.commands.clone(),
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("zjctl doctor");
        println!("============");
        println!("plugin url: {plugin_url}");
        if let Some(path) = &plugin_path_display {
            println!("plugin path: {path}");
        }

        for check in &checks {
            match check.status {
                "ok" => {
                    if let Some(detail) = &check.detail {
                        println!("{}: ok ({detail})", check.name);
                    } else {
                        println!("{}: ok", check.name);
                    }
                }
                "fail" => {
                    if let Some(detail) = &check.detail {
                        println!("{}: fail ({detail})", check.name);
                    } else {
                        println!("{}: fail", check.name);
                    }
                }
                "skip" => {
                    if let Some(detail) = &check.detail {
                        println!("{}: skip ({detail})", check.name);
                    } else {
                        println!("{}: skip", check.name);
                    }
                }
                other => println!("{}: {other}", check.name),
            }

            for cmd in &check.commands {
                println!("  fix: {cmd}");
            }
        }
    }

    if ok {
        Ok(())
    } else {
        Err("doctor found issues".into())
    }
}
