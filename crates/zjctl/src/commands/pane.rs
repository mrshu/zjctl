//! Individual pane operation commands

use crate::client;
use crate::commands::panes;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};
use zjctl_proto::methods;

pub fn send(
    plugin: Option<&str>,
    selector: &str,
    all: bool,
    bytes: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let text = bytes.join(" ");

    let params = serde_json::json!({
        "selector": selector,
        "all": all,
        "text": text,
    });

    client::rpc_call(plugin, methods::PANE_SEND, params)?;
    Ok(())
}

pub fn interrupt(
    plugin: Option<&str>,
    selector: &str,
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    send_raw(plugin, selector, all, "\u{3}")
}

pub fn escape(
    plugin: Option<&str>,
    selector: &str,
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    send_raw(plugin, selector, all, "\u{1b}")
}

pub fn capture(
    plugin: Option<&str>,
    selector: &str,
    full: bool,
    no_restore: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let selection = resolve_selection(plugin, selector)?;
    let restore = if no_restore {
        None
    } else {
        selection.restore_selector.clone()
    };

    focus_target(plugin, &selection.target_selector)?;
    let output = dump_screen(full)?;

    if let Some(selector) = restore {
        let _ = focus_target(plugin, &selector);
    }

    let mut stdout = std::io::stdout();
    stdout.write_all(&output)?;
    Ok(())
}

pub fn wait_idle(
    plugin: Option<&str>,
    selector: &str,
    idle_time: f64,
    timeout: f64,
    full: bool,
    no_restore: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if idle_time <= 0.0 {
        return Err("idle_time must be greater than 0".into());
    }
    if timeout <= 0.0 {
        return Err("timeout must be greater than 0".into());
    }

    let selection = resolve_selection(plugin, selector)?;
    let restore = if no_restore {
        None
    } else {
        selection.restore_selector.clone()
    };

    focus_target(plugin, &selection.target_selector)?;

    let idle_duration = Duration::from_secs_f64(idle_time);
    let timeout_duration = Duration::from_secs_f64(timeout);
    let poll_interval = poll_interval(idle_time);

    let start = Instant::now();
    let mut last_change = Instant::now();
    let mut last_hash = hash_bytes(&dump_screen(full)?);

    loop {
        if last_change.elapsed() >= idle_duration {
            break;
        }
        if start.elapsed() >= timeout_duration {
            if let Some(selector) = restore {
                let _ = focus_target(plugin, &selector);
            }
            return Err(format!("timed out after {timeout:.1}s").into());
        }

        sleep(poll_interval);
        let current_hash = hash_bytes(&dump_screen(full)?);
        if current_hash != last_hash {
            last_hash = current_hash;
            last_change = Instant::now();
        }
    }

    if let Some(selector) = restore {
        let _ = focus_target(plugin, &selector);
    }

    Ok(())
}

pub fn focus(plugin: Option<&str>, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
    let params = serde_json::json!({
        "selector": selector,
    });

    client::rpc_call(plugin, methods::PANE_FOCUS, params)?;
    Ok(())
}

pub fn rename(
    plugin: Option<&str>,
    selector: &str,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = serde_json::json!({
        "selector": selector,
        "name": name,
    });

    client::rpc_call(plugin, methods::PANE_RENAME, params)?;
    Ok(())
}

pub fn resize(
    plugin: Option<&str>,
    selector: &str,
    increase: bool,
    decrease: bool,
    direction: Option<&str>,
    step: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let resize_type = if increase {
        "increase"
    } else if decrease {
        "decrease"
    } else {
        return Err("must specify --increase or --decrease".into());
    };

    let params = serde_json::json!({
        "selector": selector,
        "resize_type": resize_type,
        "direction": direction,
        "step": step,
    });

    client::rpc_call(plugin, methods::PANE_RESIZE, params)?;
    Ok(())
}

fn send_raw(
    plugin: Option<&str>,
    selector: &str,
    all: bool,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = serde_json::json!({
        "selector": selector,
        "all": all,
        "text": text,
    });

    client::rpc_call(plugin, methods::PANE_SEND, params)?;
    Ok(())
}

struct Selection {
    target_selector: String,
    restore_selector: Option<String>,
}

fn resolve_selection(
    plugin: Option<&str>,
    selector: &str,
) -> Result<Selection, Box<dyn std::error::Error>> {
    let panes = panes::list(plugin)?;
    let focused = panes.iter().find(|pane| pane.focused);
    let focused_selector = focused.and_then(|pane| pane_id_to_selector(&pane.id));

    let target_selector = if selector == "focused" {
        focused_selector
            .clone()
            .unwrap_or_else(|| "focused".to_string())
    } else {
        selector.to_string()
    };

    let restore_selector = focused_selector.filter(|focused| *focused != target_selector);

    Ok(Selection {
        target_selector,
        restore_selector,
    })
}

fn focus_target(plugin: Option<&str>, selector: &str) -> Result<(), Box<dyn std::error::Error>> {
    if selector == "focused" {
        return Ok(());
    }
    focus(plugin, selector)
}

fn pane_id_to_selector(id: &str) -> Option<String> {
    let mut parts = id.split(':');
    let pane_type = parts.next()?;
    let numeric = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    match pane_type {
        "terminal" | "plugin" => Some(format!("id:{pane_type}:{numeric}")),
        _ => None,
    }
}

fn dump_screen(full: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = dump_path();
    run_dump_screen(&path, full)?;
    let output = fs::read(&path)?;
    let _ = fs::remove_file(&path);
    Ok(output)
}

fn run_dump_screen(path: &Path, full: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("zellij");
    cmd.args(["action", "dump-screen"])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    if full {
        cmd.arg("--full");
    }

    let output = cmd.output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("zellij action dump-screen failed: {}", stderr.trim()).into())
    }
}

fn dump_path() -> PathBuf {
    let filename = format!("zjctl-dump-{}.txt", uuid::Uuid::new_v4());
    std::env::temp_dir().join(filename)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn poll_interval(idle_time: f64) -> Duration {
    let mut interval = idle_time / 4.0;
    interval = interval.clamp(0.1, 1.0);
    Duration::from_secs_f64(interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_id_to_selector_parses_terminal() {
        assert_eq!(
            pane_id_to_selector("terminal:3"),
            Some("id:terminal:3".to_string())
        );
    }

    #[test]
    fn pane_id_to_selector_parses_plugin() {
        assert_eq!(
            pane_id_to_selector("plugin:7"),
            Some("id:plugin:7".to_string())
        );
    }

    #[test]
    fn pane_id_to_selector_rejects_invalid() {
        assert!(pane_id_to_selector("invalid").is_none());
        assert!(pane_id_to_selector("terminal:1:extra").is_none());
        assert!(pane_id_to_selector("other:1").is_none());
    }

    #[test]
    fn poll_interval_is_clamped() {
        assert_eq!(poll_interval(0.2), Duration::from_secs_f64(0.1));
        assert_eq!(poll_interval(8.0), Duration::from_secs_f64(1.0));
    }
}
