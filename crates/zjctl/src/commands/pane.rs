//! Individual pane operation commands

use crate::client;
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
