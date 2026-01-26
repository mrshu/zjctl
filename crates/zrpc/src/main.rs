//! zrpc - Zellij RPC plugin for zjctl
//!
//! Receives commands via Zellij pipes and executes pane operations.

#[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
compile_error!(
    "zjctl-zrpc is a Zellij plugin and must be built for WASI.\n\
Use: cargo build -p zjctl-zrpc --target wasm32-wasip1 --release"
);

use std::collections::BTreeMap;
use zellij_tile::prelude::*;
use zjctl_proto::{
    methods, PaneSelector, PaneType, RpcError, RpcErrorCode, RpcRequest, RpcResponse,
};

mod state;

use state::PluginState;

/// Expected pipe name for RPC messages
const RPC_PIPE_NAME: &str = "zjctl-rpc";
const CLIENT_POLL_SECS: f64 = 0.2;

register_plugin!(ZrpcPlugin);

/// Main plugin state
#[derive(Default)]
struct ZrpcPlugin {
    /// Current state snapshot
    state: PluginState,
}

impl ZellijPlugin for ZrpcPlugin {
    fn load(&mut self, _config: BTreeMap<String, String>) {
        // Hide the plugin pane - we're a background service
        hide_self();

        // Request required permissions
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::WriteToStdin,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadCliPipes,
        ]);

        // Subscribe to state updates
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ListClients,
            EventType::Timer,
            EventType::PermissionRequestResult,
        ]);

        // Prime client focus state
        list_clients();
        set_timeout(CLIENT_POLL_SECS);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PaneUpdate(manifest) => {
                self.state.update_panes(manifest);
            }
            Event::TabUpdate(tabs) => {
                self.state.update_tabs(tabs);
            }
            Event::ListClients(clients) => {
                self.state.update_clients(clients);
            }
            Event::Timer(_) => {
                list_clients();
                set_timeout(CLIENT_POLL_SECS);
            }
            Event::PermissionRequestResult(_) => {
                // After permissions are granted, we can query client focus reliably.
                list_clients();
            }
            _ => {}
        }
        false // Don't re-render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        // Only handle our RPC pipe
        if pipe_message.name != RPC_PIPE_NAME {
            return false;
        }

        // Get the pipe source - we only respond to CLI pipes
        let pipe_id = match &pipe_message.source {
            PipeSource::Cli(id) => id.clone(),
            _ => return false,
        };

        // Parse the payload as RPC request
        let payload = match pipe_message.payload {
            Some(p) => p,
            None => {
                self.send_error(
                    &pipe_id,
                    uuid::Uuid::nil(),
                    RpcErrorCode::InvalidRequest,
                    "empty payload",
                );
                return false;
            }
        };

        let request: RpcRequest = match serde_json::from_str(&payload) {
            Ok(r) => r,
            Err(e) => {
                self.send_error(
                    &pipe_id,
                    uuid::Uuid::nil(),
                    RpcErrorCode::InvalidRequest,
                    format!("invalid JSON: {}", e),
                );
                return false;
            }
        };

        // Route to handler
        self.handle_request(&pipe_id, request);

        false
    }
}

impl ZrpcPlugin {
    fn focused_pane(&self) -> Option<&state::PaneEntry> {
        if let Some(pane_id) = self.state.current_client_pane_id {
            let (is_plugin, numeric_id) = match pane_id {
                PaneId::Terminal(id) => (false, id),
                PaneId::Plugin(id) => (true, id),
            };
            if let Some(found) = self.state.panes.values().find(|p| {
                p.is_plugin == is_plugin && p.numeric_id == numeric_id && !p.suppressed
            }) {
                return Some(found);
            }
        }

        let active_tab = self.state.active_tab_index();
        if let Some(active_tab) = active_tab {
            let mut terminals: Vec<_> = self
                .state
                .panes
                .values()
                .filter(|p| p.tab_index == active_tab && p.focused && !p.is_plugin && !p.suppressed)
                .collect();
            terminals.sort_by_key(|p| p.numeric_id);
            if let Some(pane) = terminals.first() {
                return Some(*pane);
            }

            let mut any: Vec<_> = self
                .state
                .panes
                .values()
                .filter(|p| p.tab_index == active_tab && p.focused && !p.suppressed)
                .collect();
            any.sort_by_key(|p| (p.is_plugin, p.numeric_id));
            return any.first().copied();
        }

        // Fallback: pick any focused terminal pane deterministically (tab focus is per-tab).
        let mut terminals: Vec<_> = self
            .state
            .panes
            .values()
            .filter(|p| p.focused && !p.is_plugin && !p.suppressed)
            .collect();
        terminals.sort_by_key(|p| (p.tab_index, p.numeric_id));
        if let Some(pane) = terminals.first() {
            return Some(*pane);
        }

        let mut any: Vec<_> = self
            .state
            .panes
            .values()
            .filter(|p| p.focused && !p.suppressed)
            .collect();
        any.sort_by_key(|p| (p.tab_index, p.is_plugin, p.numeric_id));
        any.first().copied()
    }

    fn handle_request(&mut self, pipe_id: &str, request: RpcRequest) {
        let result = match request.method.as_str() {
            methods::PANES_LIST => self.handle_panes_list(&request),
            methods::PANE_SEND => self.handle_pane_send(&request),
            methods::PANE_FOCUS => self.handle_pane_focus(&request),
            methods::PANE_RENAME => self.handle_pane_rename(&request),
            methods::PANE_RESIZE => self.handle_pane_resize(&request),
            _ => Err(RpcError::new(
                RpcErrorCode::MethodNotFound,
                format!("unknown method: {}", request.method),
            )),
        };

        match result {
            Ok(value) => {
                let response =
                    RpcResponse::success(request.id, value).expect("failed to serialize response");
                self.send_response(pipe_id, response);
            }
            Err(error) => {
                let response = RpcResponse::error(request.id, error);
                self.send_response(pipe_id, response);
            }
        }
    }

    fn handle_panes_list(&self, _request: &RpcRequest) -> Result<serde_json::Value, RpcError> {
        let focused_id = self.focused_pane().map(|p| p.id_string());
        let panes = self.state.list_panes(focused_id.as_deref());

        serde_json::to_value(&panes).map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("serialization error: {}", e),
            )
        })
    }

    fn handle_pane_send(&self, request: &RpcRequest) -> Result<serde_json::Value, RpcError> {
        let selector_str = request.params["selector"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'selector'"))?;
        let text = request.params["text"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'text'"))?;
        let all = request.params["all"].as_bool().unwrap_or(false);

        let selector: PaneSelector = selector_str.parse().map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidParams,
                format!("invalid selector: {}", e),
            )
        })?;

        let panes = self.resolve_selector(&selector)?;

        if panes.is_empty() {
            return Err(RpcError::new(
                RpcErrorCode::NoMatch,
                "no panes match selector",
            ));
        }
        if panes.len() > 1 && !all {
            return Err(RpcError::new(
                RpcErrorCode::AmbiguousMatch,
                format!(
                    "{} panes match selector; use --all to target all",
                    panes.len()
                ),
            ));
        }

        for pane in &panes {
            write_chars_to_pane_id(text, pane.pane_id());
        }

        Ok(serde_json::json!({ "sent_to": panes.len() }))
    }

    fn handle_pane_focus(&self, request: &RpcRequest) -> Result<serde_json::Value, RpcError> {
        let selector_str = request.params["selector"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'selector'"))?;

        let selector: PaneSelector = selector_str.parse().map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidParams,
                format!("invalid selector: {}", e),
            )
        })?;

        let panes = self.resolve_selector(&selector)?;

        if panes.is_empty() {
            return Err(RpcError::new(
                RpcErrorCode::NoMatch,
                "no panes match selector",
            ));
        }
        if panes.len() > 1 {
            return Err(RpcError::new(
                RpcErrorCode::AmbiguousMatch,
                format!("{} panes match selector", panes.len()),
            ));
        }

        let pane = &panes[0];
        // should_float_if_hidden: bring floating panes to view if they're hidden
        focus_pane_with_id(pane.pane_id(), true);

        Ok(serde_json::json!({ "focused": pane.id_string() }))
    }

    fn handle_pane_rename(&self, request: &RpcRequest) -> Result<serde_json::Value, RpcError> {
        let selector_str = request.params["selector"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'selector'"))?;
        let name = request.params["name"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'name'"))?;

        let selector: PaneSelector = selector_str.parse().map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidParams,
                format!("invalid selector: {}", e),
            )
        })?;

        let panes = self.resolve_selector(&selector)?;

        if panes.is_empty() {
            return Err(RpcError::new(
                RpcErrorCode::NoMatch,
                "no panes match selector",
            ));
        }
        if panes.len() > 1 {
            return Err(RpcError::new(
                RpcErrorCode::AmbiguousMatch,
                format!("{} panes match selector", panes.len()),
            ));
        }

        let pane = &panes[0];
        rename_pane_with_id(pane.pane_id(), name);

        Ok(serde_json::json!({ "renamed": pane.id_string() }))
    }

    fn handle_pane_resize(&self, request: &RpcRequest) -> Result<serde_json::Value, RpcError> {
        let selector_str = request.params["selector"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'selector'"))?;
        let resize_type = request.params["resize_type"]
            .as_str()
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidParams, "missing 'resize_type'"))?;
        let direction = request.params["direction"].as_str();
        let step = request.params["step"].as_u64().unwrap_or(1) as usize;

        let selector: PaneSelector = selector_str.parse().map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidParams,
                format!("invalid selector: {}", e),
            )
        })?;

        let panes = self.resolve_selector(&selector)?;

        if panes.is_empty() {
            return Err(RpcError::new(
                RpcErrorCode::NoMatch,
                "no panes match selector",
            ));
        }
        if panes.len() > 1 {
            return Err(RpcError::new(
                RpcErrorCode::AmbiguousMatch,
                format!("{} panes match selector", panes.len()),
            ));
        }

        let pane = &panes[0];
        let resize = match resize_type {
            "increase" => Resize::Increase,
            "decrease" => Resize::Decrease,
            _ => {
                return Err(RpcError::new(
                    RpcErrorCode::InvalidParams,
                    "resize_type must be 'increase' or 'decrease'",
                ))
            }
        };

        let dir = match direction {
            Some("left") => Some(Direction::Left),
            Some("right") => Some(Direction::Right),
            Some("up") => Some(Direction::Up),
            Some("down") => Some(Direction::Down),
            None => None,
            Some(d) => {
                return Err(RpcError::new(
                    RpcErrorCode::InvalidParams,
                    format!("invalid direction: {}", d),
                ))
            }
        };

        let strategy = ResizeStrategy {
            resize,
            direction: dir,
            invert_on_boundaries: true,
        };

        for _ in 0..step {
            resize_pane_with_id(strategy, pane.pane_id());
        }

        Ok(serde_json::json!({ "resized": pane.id_string() }))
    }

    fn resolve_selector(
        &self,
        selector: &PaneSelector,
    ) -> Result<Vec<&state::PaneEntry>, RpcError> {
        match selector {
            PaneSelector::Focused => Ok(self.focused_pane().into_iter().collect()),
            PaneSelector::Id { pane_type, id } => {
                let is_plugin = matches!(pane_type, PaneType::Plugin);
                let found: Vec<_> = self
                    .state
                    .panes
                    .values()
                    .filter(|p| p.numeric_id == *id && p.is_plugin == is_plugin)
                    .collect();
                Ok(found)
            }
            PaneSelector::Title { pattern } => {
                let matching: Vec<_> = self
                    .state
                    .panes
                    .values()
                    .filter(|p| pattern.matches(&p.title).unwrap_or(false))
                    .collect();
                Ok(matching)
            }
            PaneSelector::Command { pattern } => {
                let matching: Vec<_> = self
                    .state
                    .panes
                    .values()
                    .filter(|p| {
                        p.command
                            .as_ref()
                            .map(|c| pattern.matches(c).unwrap_or(false))
                            .unwrap_or(false)
                    })
                    .collect();
                Ok(matching)
            }
            PaneSelector::TabIndex { tab, index } => {
                let mut panes: Vec<_> = self
                    .state
                    .panes
                    .values()
                    .filter(|p| p.tab_index == *tab)
                    .collect();
                panes.sort_by_key(|p| (p.is_plugin, p.numeric_id));
                Ok(panes.get(*index).copied().into_iter().collect())
            }
        }
    }

    fn send_response(&self, pipe_id: &str, response: RpcResponse) {
        let json = serde_json::to_string(&response).expect("failed to serialize response");
        cli_pipe_output(pipe_id, &json);
        // Signal we're done with this pipe
        unblock_cli_pipe_input(pipe_id);
    }

    fn send_error(
        &self,
        pipe_id: &str,
        id: uuid::Uuid,
        code: RpcErrorCode,
        message: impl Into<String>,
    ) {
        let error = RpcError::new(code, message);
        let response = RpcResponse::error(id, error);
        self.send_response(pipe_id, response);
    }
}
