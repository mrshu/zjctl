//! Panes listing commands

use crate::client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use zjctl_proto::methods;

/// Pane info returned from list
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaneInfo {
    pub id: String,
    pub pane_type: String,
    pub title: String,
    pub command: Option<String>,
    pub tab_index: usize,
    pub tab_name: String,
    pub focused: bool,
    pub floating: bool,
    pub suppressed: bool,
    #[serde(default)]
    pub rows: usize,
    #[serde(default)]
    pub cols: usize,
}

pub fn list(plugin: Option<&str>) -> Result<Vec<PaneInfo>, Box<dyn std::error::Error>> {
    // When zrpc is auto-launched via `zellij pipe`, it can receive the pipe message before it has
    // processed its first PaneUpdate. In that window, panes.list can return an incomplete snapshot.
    // We poll briefly until the pane IDs stabilize.
    let start = Instant::now();
    let timeout = Duration::from_millis(500);
    let interval = Duration::from_millis(50);

    let mut panes = list_once(plugin)?;
    let mut ids: Vec<String> = panes.iter().map(|p| p.id.clone()).collect();
    ids.sort();

    loop {
        if start.elapsed() >= timeout {
            return Ok(panes);
        }

        std::thread::sleep(interval);

        let next = list_once(plugin)?;
        let mut next_ids: Vec<String> = next.iter().map(|p| p.id.clone()).collect();
        next_ids.sort();

        if next_ids == ids {
            return Ok(next);
        }

        panes = next;
        ids = next_ids;
    }
}

fn list_once(plugin: Option<&str>) -> Result<Vec<PaneInfo>, Box<dyn std::error::Error>> {
    let result = client::rpc_call(plugin, methods::PANES_LIST, serde_json::json!({}))?;
    let panes: Vec<PaneInfo> = serde_json::from_value(result)?;
    Ok(panes)
}

pub fn ls(plugin: Option<&str>, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        let panes = list(plugin)?;
        println!("{}", serde_json::to_string_pretty(&panes)?);
    } else {
        let panes = list(plugin)?;
        print_table(&panes);
    }

    Ok(())
}

pub fn print_table(panes: &[PaneInfo]) {
    if panes.is_empty() {
        println!("No panes found");
        return;
    }

    println!(
        "{:<20} {:<10} {:<30} {:<15} {:<8}",
        "ID", "TAB", "TITLE", "COMMAND", "FLAGS"
    );
    println!("{}", "-".repeat(90));

    for pane in panes {
        let flags = format!(
            "{}{}{}",
            if pane.focused { "F" } else { "-" },
            if pane.floating { "f" } else { "-" },
            if pane.suppressed { "s" } else { "-" }
        );
        println!(
            "{:<20} {:<10} {:<30} {:<15} {:<8}",
            pane.id,
            pane.tab_name,
            truncate(&pane.title, 28),
            truncate(&pane.command.clone().unwrap_or_default(), 13),
            flags
        );
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
