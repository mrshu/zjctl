//! RPC protocol types for zjctl <-> zrpc communication.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// RPC request sent from zjctl CLI to zrpc plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// Protocol version
    pub v: u8,
    /// Request ID for correlation
    pub id: Uuid,
    /// Method to invoke
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcRequest {
    /// Create a new RPC request
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id: Uuid::new_v4(),
            method: method.into(),
            params: serde_json::Value::Null,
        }
    }

    /// Set request parameters
    pub fn with_params(mut self, params: impl Serialize) -> Result<Self, serde_json::Error> {
        self.params = serde_json::to_value(params)?;
        Ok(self)
    }
}

/// RPC response from zrpc plugin to zjctl CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Protocol version
    pub v: u8,
    /// Request ID for correlation
    pub id: Uuid,
    /// Whether the request succeeded
    pub ok: bool,
    /// Result data (if ok=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error details (if ok=false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    /// Create a success response
    pub fn success(id: Uuid, result: impl Serialize) -> Result<Self, serde_json::Error> {
        Ok(Self {
            v: PROTOCOL_VERSION,
            id,
            ok: true,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    /// Create an error response
    pub fn error(id: Uuid, error: RpcError) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

/// RPC error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code
    pub code: RpcErrorCode,
    /// Human-readable error message
    pub message: String,
}

impl RpcError {
    pub fn new(code: RpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Standard RPC error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcErrorCode {
    /// Invalid request format
    InvalidRequest,
    /// Unknown method
    MethodNotFound,
    /// Invalid parameters
    InvalidParams,
    /// Selector matched no panes
    NoMatch,
    /// Selector matched multiple panes (and --all not set)
    AmbiguousMatch,
    /// Internal error
    Internal,
}

/// RPC methods
pub mod methods {
    pub const PANES_LIST: &str = "panes.list";
    pub const PANE_SEND: &str = "pane.send";
    pub const PANE_FOCUS: &str = "pane.focus";
    pub const PANE_RENAME: &str = "pane.rename";
    pub const PANE_RESIZE: &str = "pane.resize";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = RpcRequest::new("panes.list");
        let json = serde_json::to_string(&req).unwrap();

        // Verify JSON structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["v"], 1);
        assert_eq!(parsed["method"], "panes.list");
        assert!(parsed["id"].is_string());

        // Round-trip
        let req2: RpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req2.method, "panes.list");
        assert_eq!(req2.v, PROTOCOL_VERSION);
    }

    #[test]
    fn test_request_with_params() {
        let req = RpcRequest::new("pane.send")
            .with_params(serde_json::json!({
                "selector": "focused",
                "text": "hello"
            }))
            .unwrap();

        let json = serde_json::to_string(&req).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["params"]["selector"], "focused");
        assert_eq!(parsed["params"]["text"], "hello");
    }

    #[test]
    fn test_response_success() {
        let id = Uuid::new_v4();
        let resp = RpcResponse::success(id, serde_json::json!({"count": 5})).unwrap();

        assert!(resp.ok);
        assert!(resp.error.is_none());
        assert_eq!(resp.result.as_ref().unwrap()["count"], 5);

        // Serialization round-trip
        let json = serde_json::to_string(&resp).unwrap();
        let resp2: RpcResponse = serde_json::from_str(&json).unwrap();
        assert!(resp2.ok);
        assert_eq!(resp2.id, id);
    }

    #[test]
    fn test_response_error() {
        let id = Uuid::new_v4();
        let error = RpcError::new(RpcErrorCode::NoMatch, "no panes found");
        let resp = RpcResponse::error(id, error);

        assert!(!resp.ok);
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, RpcErrorCode::NoMatch);

        // Serialization round-trip
        let json = serde_json::to_string(&resp).unwrap();
        let resp2: RpcResponse = serde_json::from_str(&json).unwrap();
        assert!(!resp2.ok);
        assert_eq!(resp2.error.unwrap().message, "no panes found");
    }

    #[test]
    fn test_error_code_serialization() {
        let error = RpcError::new(RpcErrorCode::AmbiguousMatch, "multiple matches");
        let json = serde_json::to_string(&error).unwrap();

        // Check snake_case serialization
        assert!(json.contains("ambiguous_match"));

        let error2: RpcError = serde_json::from_str(&json).unwrap();
        assert_eq!(error2.code, RpcErrorCode::AmbiguousMatch);
    }
}
