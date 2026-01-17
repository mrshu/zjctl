//! RPC client for communicating with zrpc plugin via Zellij pipes.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;
use zjctl_proto::{RpcRequest, RpcResponse};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("zellij not found in PATH (install Zellij 0.43+)")]
    ZellijMissing,
    #[error("failed to spawn zellij pipe: {0}")]
    Spawn(std::io::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize request: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error(
        "zrpc plugin not found at {path}\n\nInstall:\n  {install_cmd}\n  {download_cmd}\n\nLoad it in Zellij:\n  {launch_cmd}\n  # or add to config.kdl load_plugins\n\nRun `zjctl doctor` for more checks."
    )]
    PluginNotInstalled {
        path: String,
        install_cmd: String,
        download_cmd: String,
        launch_cmd: String,
    },
    #[error(
        "no response from zrpc plugin\n\nMake sure it is loaded in your Zellij session:\n  {launch_cmd}\n  # or add to config.kdl load_plugins\n\nIf prompted, accept ReadCliPipes permissions.\nRun `zjctl doctor` for more checks."
    )]
    PluginNotLoaded { launch_cmd: String },
    #[error("zellij pipe exited with error{exit_note}\n{stderr}\n\nRun `zjctl doctor` for more checks.")]
    PipeError { exit_note: String, stderr: String },
    #[error("RPC error: {0}")]
    RpcError(String),
}

pub fn default_plugin_url() -> String {
    format!("file:{}", default_plugin_path().display())
}

pub fn default_plugin_path() -> PathBuf {
    let rel = Path::new("zellij").join("plugins").join("zrpc.wasm");

    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join(rel);
    }

    if cfg!(windows) {
        if let Ok(dir) = std::env::var("APPDATA") {
            return PathBuf::from(dir).join(rel);
        }
        if let Ok(dir) = std::env::var("USERPROFILE") {
            return PathBuf::from(dir).join(rel);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join(rel);
    }

    PathBuf::from("zrpc.wasm")
}

/// Send an RPC request to the zrpc plugin and wait for response
pub fn call(request: &RpcRequest, plugin_path: Option<&str>) -> Result<RpcResponse, ClientError> {
    let default_url = default_plugin_url();
    let plugin_url = plugin_path.unwrap_or(default_url.as_str());
    let request_json = serde_json::to_string(request)?;

    if let Some(path) = plugin_file_path(plugin_url) {
        if !path.is_file() {
            let (install_cmd, download_cmd, launch_cmd) =
                plugin_install_commands(plugin_url, &path);
            return Err(ClientError::PluginNotInstalled {
                path: path.display().to_string(),
                install_cmd,
                download_cmd,
                launch_cmd,
            });
        }
    }

    // Use zellij pipe to send message to plugin
    // The plugin name in the pipe message will match the payload we send
    let mut child = Command::new("zellij")
        .args(["pipe", "--plugin", plugin_url, "--name", "zjctl-rpc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => ClientError::ZellijMissing,
            _ => ClientError::Spawn(err),
        })?;

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", request_json)?;
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let exit_note = match output.status.code() {
            Some(code) => format!(" (exit code: {code})"),
            None => " (terminated by signal)".to_string(),
        };
        let stderr_note = if stderr.trim().is_empty() {
            "no stderr output".to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(ClientError::PipeError {
            exit_note,
            stderr: stderr_note,
        });
    }

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(resp) = serde_json::from_str::<RpcResponse>(line) {
            if resp.id == request.id {
                return Ok(resp);
            }
        }
    }

    let launch_cmd = plugin_launch_command(plugin_url, plugin_file_path(plugin_url).as_deref());
    Err(ClientError::PluginNotLoaded { launch_cmd })
}

/// Helper to create and send a request
pub fn rpc_call(
    plugin: Option<&str>,
    method: &str,
    params: impl serde::Serialize,
) -> Result<serde_json::Value, ClientError> {
    let request = RpcRequest::new(method).with_params(params)?;

    let response = call(&request, plugin)?;

    if response.ok {
        Ok(response.result.unwrap_or(serde_json::Value::Null))
    } else {
        let err = response
            .error
            .map(|e| e.message)
            .unwrap_or_else(|| "unknown error".to_string());
        Err(ClientError::RpcError(err))
    }
}

pub fn plugin_file_path(plugin_url: &str) -> Option<PathBuf> {
    if plugin_url.contains("://") && !plugin_url.starts_with("file:") {
        return None;
    }
    let raw = plugin_url.strip_prefix("file:").unwrap_or(plugin_url);
    if raw.is_empty() {
        return None;
    }
    Some(expand_tilde(raw))
}

fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub(crate) fn plugin_launch_command(plugin_url: &str, plugin_path: Option<&Path>) -> String {
    let launch_url = plugin_launch_url(plugin_url, plugin_path);
    format!("zellij action launch-plugin \"{}\"", launch_url)
}

fn plugin_launch_url(plugin_url: &str, plugin_path: Option<&Path>) -> String {
    if plugin_url.contains("://") && !plugin_url.starts_with("file:") {
        return plugin_url.to_string();
    }
    if let Some(path) = plugin_path {
        return format!("file:{}", path.display());
    }
    if plugin_url.starts_with("file:") {
        plugin_url.to_string()
    } else {
        format!("file:{}", plugin_url)
    }
}

pub(crate) fn plugin_install_commands(
    plugin_url: &str,
    plugin_path: &Path,
) -> (String, String, String) {
    let dir = plugin_path.parent().unwrap_or_else(|| Path::new("."));
    let install_cmd = format!("mkdir -p \"{}\"", dir.display());
    let download_cmd = format!(
        "curl -L https://github.com/mrshu/zjctl/releases/latest/download/zrpc.wasm -o \"{}\"",
        plugin_path.display()
    );
    let launch_cmd = plugin_launch_command(plugin_url, Some(plugin_path));
    (install_cmd, download_cmd, launch_cmd)
}
