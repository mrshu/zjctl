//! RPC client for communicating with zrpc plugin via Zellij pipes.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use thiserror::Error;
use zjctl_proto::{RpcRequest, RpcResponse};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("failed to spawn zellij pipe: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("failed to serialize request: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("zellij pipe exited with error: {0}")]
    PipeError(String),
    #[error("no response received from plugin")]
    NoResponse,
    #[error("RPC error: {0}")]
    RpcError(String),
}

const DEFAULT_PLUGIN_URL: &str = "file:~/.config/zellij/plugins/zrpc.wasm";

/// Send an RPC request to the zrpc plugin and wait for response
pub fn call(request: &RpcRequest, plugin_path: Option<&str>) -> Result<RpcResponse, ClientError> {
    let plugin_url = plugin_path.unwrap_or(DEFAULT_PLUGIN_URL);
    let request_json = serde_json::to_string(request)?;

    // Use zellij pipe to send message to plugin
    // The plugin name in the pipe message will match the payload we send
    let mut child = Command::new("zellij")
        .args(["pipe", "--plugin", plugin_url, "--name", "zjctl-rpc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        writeln!(stdin, "{}", request_json)?;
    }

    // Read response from stdout
    let stdout = child.stdout.take().ok_or(ClientError::NoResponse)?;
    let reader = BufReader::new(stdout);

    let mut response: Option<RpcResponse> = None;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        // Try to parse as RpcResponse
        if let Ok(resp) = serde_json::from_str::<RpcResponse>(&line) {
            if resp.id == request.id {
                response = Some(resp);
                break;
            }
        }
    }

    // Wait for child to exit
    let status = child.wait()?;
    if !status.success() {
        // Try to read stderr
        return Err(ClientError::PipeError(format!(
            "exit code: {:?}",
            status.code()
        )));
    }

    response.ok_or(ClientError::NoResponse)
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
