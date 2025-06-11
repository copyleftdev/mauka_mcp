//! Protocol module for the Mauka MCP Server.
//!
//! This module implements the MCP protocol, including JSON-RPC 2.0 handling,
//! request/response correlation, and protocol methods.

// JSON-RPC 2.0 implementation
pub mod jsonrpc;

// Re-export common protocol components
pub use self::jsonrpc::handler::{JsonRpcHandler, MethodContext, MethodHandler, MethodResult};
