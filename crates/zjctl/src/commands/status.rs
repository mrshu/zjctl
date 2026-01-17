//! Status command

use serde::Serialize;

use crate::commands::panes::{self, PaneInfo};

#[derive(Serialize)]
struct StatusReport {
    focused: Option<PaneInfo>,
    panes: Vec<PaneInfo>,
}

pub fn run(plugin: Option<&str>, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let panes = panes::list(plugin)?;
    let focused = panes.iter().find(|pane| pane.focused).cloned();

    if json {
        let report = StatusReport { focused, panes };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if let Some(focused) = focused {
        let command = focused.command.clone().unwrap_or_default();
        println!(
            "Focused: {} [{}] (tab: {}:{})",
            focused.id, focused.title, focused.tab_index, focused.tab_name
        );
        if !command.is_empty() {
            println!("Command: {}", command);
        }

        let tab_panes: Vec<PaneInfo> = panes
            .into_iter()
            .filter(|pane| pane.tab_index == focused.tab_index)
            .collect();

        panes::print_table(&tab_panes);
    } else {
        println!("Focused: none");
        panes::print_table(&panes);
    }

    Ok(())
}
