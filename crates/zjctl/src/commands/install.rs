//! Install the zrpc plugin

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::client;

pub fn run(
    plugin: Option<&str>,
    print: bool,
    force: bool,
    load: bool,
    auto_load: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let default_url = client::default_plugin_url();
    let plugin_url = plugin.unwrap_or(default_url.as_str());
    let plugin_path = client::plugin_file_path(plugin_url)
        .ok_or_else(|| format!("install only supports file: plugin URLs (got {plugin_url})"))?;

    let (install_cmd, download_cmd, launch_cmd) =
        client::plugin_install_commands(plugin_url, &plugin_path);
    let config_path = config_file_path();
    let config_url = plugin_url_for_config(plugin_url, &plugin_path);

    if print {
        println!("install: {install_cmd}");
        println!("install: {download_cmd}");
        println!("load: {launch_cmd}");
        if auto_load {
            println!(
                "config: add to {} -> load_plugins {{ {} }}",
                config_path.display(),
                config_url
            );
        }
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

    if auto_load {
        let updated = ensure_auto_load_config(&config_path, &config_url)?;
        if updated {
            println!("config: updated {}", config_path.display());
        } else {
            println!("config: already contains plugin entry");
        }
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

fn config_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("ZELLIJ_CONFIG_FILE") {
        return PathBuf::from(path);
    }
    if let Ok(dir) = std::env::var("ZELLIJ_CONFIG_DIR") {
        return PathBuf::from(dir).join("config.kdl");
    }
    let base = if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if cfg!(windows) {
        if let Ok(dir) = std::env::var("APPDATA") {
            PathBuf::from(dir)
        } else if let Ok(dir) = std::env::var("USERPROFILE") {
            PathBuf::from(dir)
        } else {
            PathBuf::from(".")
        }
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".")
    };
    base.join("zellij").join("config.kdl")
}

fn plugin_url_for_config(plugin_url: &str, plugin_path: &Path) -> String {
    if plugin_url.contains("://") && !plugin_url.starts_with("file:") {
        return plugin_url.to_string();
    }
    let path = shorten_home(plugin_path);
    format!("file:{}", path)
}

fn shorten_home(path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(stripped) = path.strip_prefix(&home_path) {
            let rel = stripped.display().to_string();
            if rel.is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rel);
        }
    }
    path.display().to_string()
}

fn ensure_auto_load_config(
    path: &Path,
    plugin_url: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut contents = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if contents.contains(plugin_url) {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }

    contents.push_str("\nload_plugins {\n    ");
    contents.push_str(plugin_url);
    contents.push_str("\n}\n");

    fs::write(path, contents)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_auto_load_config_is_idempotent() {
        let path = std::env::temp_dir().join(format!("zjctl-config-{}.kdl", uuid::Uuid::new_v4()));
        let plugin_url = "file:/tmp/zrpc.wasm";

        let first = ensure_auto_load_config(&path, plugin_url).expect("first write");
        let second = ensure_auto_load_config(&path, plugin_url).expect("second write");

        assert!(first);
        assert!(!second);

        let _ = fs::remove_file(path);
    }
}
