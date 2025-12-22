//! Panes listing commands

use crate::client;
use serde::{Deserialize, Serialize};
use zjctl_proto::methods;

/// Pane info returned from list
#[derive(Debug, Serialize, Deserialize)]
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
}

pub fn ls(plugin: Option<&str>, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let result = client::rpc_call(plugin, methods::PANES_LIST, serde_json::json!({}))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Parse and display in table format
        let panes: Vec<PaneInfo> = serde_json::from_value(result)?;

        if panes.is_empty() {
            println!("No panes found");
            return Ok(());
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
                truncate(&pane.command.unwrap_or_default(), 13),
                flags
            );
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
