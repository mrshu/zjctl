//! Protocol types for zjctl RPC communication with zrpc plugin.
//!
//! Uses newline-delimited JSON (jsonl) for transport over Zellij pipes.

mod protocol;
mod selector;

pub use protocol::*;
pub use selector::*;
