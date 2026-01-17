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
    enter: bool,
    delay_enter: f64,
    bytes: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let text = bytes.join(" ");

    let steps = build_send_steps(&text, enter, delay_enter)?;
    for step in steps {
        match step {
            SendStep::Text(text) => send_raw(plugin, selector, all, &text)?,
            SendStep::Delay(duration) => sleep(duration),
        }
    }
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

pub struct LaunchOptions<'a> {
    pub direction: Option<&'a str>,
    pub floating: bool,
    pub name: Option<&'a str>,
    pub cwd: Option<&'a str>,
    pub close_on_exit: bool,
    pub in_place: bool,
    pub start_suspended: bool,
    pub command: &'a [String],
}

pub fn launch(
    plugin: Option<&str>,
    options: LaunchOptions<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = panes::list(plugin)?;

    run_new_pane_action(&options)?;

    let after = panes::list(plugin)?;
    let pane = find_new_pane(&before, &after).map_err(|err| {
        format!("unable to identify new pane ({err}); run `zjctl panes ls` to inspect")
    })?;

    if let Some(selector) = pane_id_to_selector(&pane.id) {
        println!("{selector}");
    } else {
        println!("{}", pane.id);
    }

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

#[derive(Debug)]
enum SendStep {
    Text(String),
    Delay(Duration),
}

fn build_send_steps(
    text: &str,
    enter: bool,
    delay_enter: f64,
) -> Result<Vec<SendStep>, Box<dyn std::error::Error>> {
    if delay_enter < 0.0 {
        return Err("delay_enter must be >= 0".into());
    }

    let mut steps = Vec::new();
    if !text.is_empty() {
        steps.push(SendStep::Text(text.to_string()));
    }

    if enter {
        if delay_enter > 0.0 {
            steps.push(SendStep::Delay(Duration::from_secs_f64(delay_enter)));
        }
        steps.push(SendStep::Text("\n".to_string()));
    }

    Ok(steps)
}

fn run_new_pane_action(options: &LaunchOptions<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("zellij");
    cmd.args(["action", "new-pane"]);

    if let Some(direction) = options.direction {
        cmd.args(["--direction", direction]);
    }
    if options.floating {
        cmd.arg("--floating");
    }
    if let Some(name) = options.name {
        cmd.args(["--name", name]);
    }
    if let Some(cwd) = options.cwd {
        cmd.args(["--cwd", cwd]);
    }
    if options.close_on_exit {
        cmd.arg("--close-on-exit");
    }
    if options.in_place {
        cmd.arg("--in-place");
    }
    if options.start_suspended {
        cmd.arg("--start-suspended");
    }
    if !options.command.is_empty() {
        cmd.arg("--").args(options.command);
    }

    let status = cmd
        .status()
        .map_err(|err| format!("failed to run zellij: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("zellij action new-pane failed: {status:?}").into())
    }
}

fn find_new_pane(
    before: &[panes::PaneInfo],
    after: &[panes::PaneInfo],
) -> Result<panes::PaneInfo, String> {
    let mut new_panes: Vec<panes::PaneInfo> = after
        .iter()
        .filter(|pane| !before.iter().any(|old| old.id == pane.id))
        .cloned()
        .collect();

    match new_panes.len() {
        1 => Ok(new_panes.remove(0)),
        0 => Err("no new panes detected".to_string()),
        count => Err(format!("{count} new panes detected")),
    }
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

    fn pane(id: &str) -> panes::PaneInfo {
        panes::PaneInfo {
            id: id.to_string(),
            pane_type: "terminal".to_string(),
            title: String::new(),
            command: None,
            tab_index: 0,
            tab_name: "tab".to_string(),
            focused: false,
            floating: false,
            suppressed: false,
        }
    }

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

    #[test]
    fn build_send_steps_with_enter_and_delay() {
        let steps = build_send_steps("echo hi", true, 0.5).expect("steps");
        assert_eq!(steps.len(), 3);
        matches!(steps[0], SendStep::Text(_));
        matches!(steps[1], SendStep::Delay(_));
        matches!(steps[2], SendStep::Text(_));
    }

    #[test]
    fn build_send_steps_without_enter() {
        let steps = build_send_steps("echo hi", false, 1.0).expect("steps");
        assert_eq!(steps.len(), 1);
        matches!(steps[0], SendStep::Text(_));
    }

    #[test]
    fn build_send_steps_without_delay() {
        let steps = build_send_steps("echo hi", true, 0.0).expect("steps");
        assert_eq!(steps.len(), 2);
        matches!(steps[0], SendStep::Text(_));
        matches!(steps[1], SendStep::Text(_));
    }

    #[test]
    fn build_send_steps_rejects_negative_delay() {
        let err = build_send_steps("echo hi", true, -1.0).expect_err("error");
        assert_eq!(err.to_string(), "delay_enter must be >= 0");
    }

    #[test]
    fn find_new_pane_returns_added_pane() {
        let before = vec![pane("terminal:1")];
        let after = vec![pane("terminal:1"), pane("terminal:2")];

        let found = find_new_pane(&before, &after).expect("expected new pane");
        assert_eq!(found.id, "terminal:2");
    }

    #[test]
    fn find_new_pane_errors_when_none_added() {
        let before = vec![pane("terminal:1")];
        let after = vec![pane("terminal:1")];

        let err = find_new_pane(&before, &after).expect_err("expected error");
        assert_eq!(err, "no new panes detected");
    }

    #[test]
    fn find_new_pane_errors_when_multiple_added() {
        let before = vec![pane("terminal:1")];
        let after = vec![pane("terminal:1"), pane("terminal:2"), pane("terminal:3")];

        let err = find_new_pane(&before, &after).expect_err("expected error");
        assert_eq!(err, "2 new panes detected");
    }
}
