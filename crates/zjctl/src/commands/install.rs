//! Install the zrpc plugin

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::client;

pub fn run(
    plugin: Option<&str>,
    print: bool,
    force: bool,
    load: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let default_url = client::default_plugin_url();
    let plugin_url = plugin.unwrap_or(default_url.as_str());
    let plugin_path = client::plugin_file_path(plugin_url).ok_or_else(|| {
        format!("install only supports file: plugin URLs (got {plugin_url})")
    })?;

    let (install_cmd, download_cmd, launch_cmd) =
        client::plugin_install_commands(plugin_url, &plugin_path);

    if print {
        println!("install: {install_cmd}");
        println!("install: {download_cmd}");
        println!("load: {launch_cmd}");
        return Ok(());
    }

    if let Some(parent) = plugin_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if plugin_path.exists() && !force {
        println!("plugin file exists: {}", plugin_path.display());
    } else {
        download_plugin(&plugin_path)?;
        println!("plugin installed: {}", plugin_path.display());
    }

    if load {
        let launch_url = client::plugin_launch_url(plugin_url, Some(&plugin_path));
        let status = Command::new("zellij")
            .args(["action", "launch-plugin", &launch_url])
            .status()
            .map_err(|err| format!("failed to run zellij: {err}"))?;
        if !status.success() {
            return Err(format!("zellij action launch-plugin failed: {status:?}").into());
        }
    } else {
        println!("load: {launch_cmd}");
    }

    Ok(())
}

fn download_plugin(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("curl")
        .args(["-L", client::DEFAULT_PLUGIN_DOWNLOAD_URL, "-o"])
        .arg(path)
        .status()
        .map_err(|err| format!("failed to run curl: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("curl failed with status: {status:?}").into())
    }
}
