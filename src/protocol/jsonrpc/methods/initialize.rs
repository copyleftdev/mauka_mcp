// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Implementation of the JSON-RPC "initialize" method handler.
//!
//! The initialize method is typically the first method called by a client to
//! establish capabilities and configuration with the server.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::protocol::jsonrpc::error::{ErrorCode, JsonRpcError};
use crate::protocol::jsonrpc::handler::{JsonRpcHandler, MethodContext, MethodResult};

/// Request parameters for the initialize method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// The client's name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    
    /// The client's version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    
    /// Client capabilities
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ClientCapabilities>,
    
    /// Client configuration options
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_info: Option<HashMap<String, Value>>,
    
    /// Initialization options
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initialization_options: Option<Value>,
}

/// Client capabilities communicated during initialization.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientCapabilities {
    /// Protocol version supported by the client
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    
    /// Whether the client supports batch requests
    #[serde(default)]
    pub supports_batch_requests: bool,
    
    /// Whether the client supports notifications
    #[serde(default)]
    pub supports_notifications: bool,
    
    /// Whether the client supports request cancellation
    #[serde(default)]
    pub supports_request_cancellation: bool,
    
    /// Maximum allowed request size in bytes
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,
    
    /// Optional custom capabilities
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, Value>,
}

/// Server capabilities communicated in initialize response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Protocol version supported by the server
    pub protocol_version: String,
    
    /// Server name
    pub server_name: String,
    
    /// Server version
    pub server_version: String,
    
    /// Whether the server supports batch requests
    pub supports_batch_requests: bool,
    
    /// Whether the server supports notifications
    pub supports_notifications: bool,
    
    /// Whether the server supports request cancellation
    pub supports_request_cancellation: bool,
    
    /// Maximum allowed request size in bytes
    pub max_request_size: usize,
    
    /// List of supported methods
    pub supported_methods: Vec<String>,
    
    /// Optional server extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, Value>,
}

/// Initialize response from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    
    /// Server info (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_info: Option<HashMap<String, Value>>,
}

/// Default protocol version.
fn default_protocol_version() -> String {
    "2.0".to_string()
}

/// Default maximum request size.
fn default_max_request_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

/// Registers the initialize method handler with the JSON-RPC handler.
pub fn register_initialize_method(handler: &mut JsonRpcHandler) {
    handler.register_method("initialize", handle_initialize);
}

/// Handles the initialize method call.
///
/// This is the first method called by a client to establish capabilities
/// and configuration with the server.
async fn handle_initialize(params: Option<Value>, context: MethodContext) -> MethodResult {
    // Parse parameters
    let params = match params {
        Some(params) => match serde_json::from_value::<InitializeParams>(params) {
            Ok(params) => params,
            Err(err) => {
                return Err(JsonRpcError::new(
                    ErrorCode::InvalidParams,
                    format!("Invalid initialize parameters: {}", err),
                ))
            }
        },
        None => InitializeParams {
            client_name: None,
            client_version: None,
            capabilities: None,
            client_info: None,
            initialization_options: None,
        },
    };

    // Extract client info for logging (could add actual logging here)
    let client_name = params.client_name.unwrap_or_else(|| "Unknown Client".to_string());
    let client_version = params.client_version.unwrap_or_else(|| "Unknown Version".to_string());
    
    // Create server capabilities
    let capabilities = ServerCapabilities {
        protocol_version: "2.0".to_string(),
        server_name: "Mauka MCP Server".to_string(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        supports_batch_requests: true,
        supports_notifications: true,
        supports_request_cancellation: true,
        max_request_size: 10 * 1024 * 1024, // 10 MB
        supported_methods: vec![
            "initialize".to_string(),
            "shutdown".to_string(),
            "tools/list".to_string(),
        ],
        extensions: HashMap::new(),
    };
    
    // Create additional server info
    let mut server_info = HashMap::new();
    server_info.insert("name".to_string(), Value::String("Mauka MCP Server".to_string()));
    server_info.insert("version".to_string(), Value::String(env!("CARGO_PKG_VERSION").to_string()));
    
    // Create the result
    let result = InitializeResult {
        capabilities,
        server_info: Some(server_info),
    };
    
    // Return success response
    Ok(serde_json::to_value(result).unwrap_or_else(|_| Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_initialize_with_full_params() {
        // Create full params with all fields
        let mut custom_capabilities = HashMap::new();
        custom_capabilities.insert("streaming".to_string(), json!(true));
        
        let client_capabilities = ClientCapabilities {
            protocol_version: "2.0".to_string(),
            supports_batch_requests: true,
            supports_notifications: true,
            supports_request_cancellation: false,
            max_request_size: 5 * 1024 * 1024,
            custom: custom_capabilities,
        };
        
        let mut client_info = HashMap::new();
        client_info.insert("platform".to_string(), json!("Linux"));
        
        let params = InitializeParams {
            client_name: Some("Test Client".to_string()),
            client_version: Some("1.0.0".to_string()),
            capabilities: Some(client_capabilities),
            client_info: Some(client_info),
            initialization_options: Some(json!({"trace": "verbose"})),
        };
        
        // Convert params to JSON value
        let params_json = serde_json::to_value(params).unwrap();
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_initialize(Some(params_json), context).await.unwrap();
        
        // Assert result
        let result: InitializeResult = serde_json::from_value(result).unwrap();
        assert_eq!(result.capabilities.server_name, "Mauka MCP Server");
        assert_eq!(result.capabilities.protocol_version, "2.0");
        assert!(result.capabilities.supports_batch_requests);
        assert!(result.capabilities.supports_notifications);
        assert!(result.capabilities.supports_request_cancellation);
        
        // Check supported methods
        let supported_methods = &result.capabilities.supported_methods;
        assert!(supported_methods.contains(&"initialize".to_string()));
        assert!(supported_methods.contains(&"shutdown".to_string()));
        assert!(supported_methods.contains(&"tools/list".to_string()));
        
        // Check server info
        let server_info = result.server_info.unwrap();
        assert_eq!(server_info["name"], json!("Mauka MCP Server"));
    }
    
    #[tokio::test]
    async fn test_initialize_with_no_params() {
        // Call handler with no params
        let context = MethodContext::default();
        let result = handle_initialize(None, context).await.unwrap();
        
        // Assert result
        let result: InitializeResult = serde_json::from_value(result).unwrap();
        assert_eq!(result.capabilities.server_name, "Mauka MCP Server");
    }
    
    #[tokio::test]
    async fn test_initialize_with_invalid_params() {
        // Create invalid params (missing required fields)
        let params_json = json!({ "invalid_field": true });
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_initialize(Some(params_json), context).await;
        
        // Should still succeed with default values
        assert!(result.is_ok());
    }
}
