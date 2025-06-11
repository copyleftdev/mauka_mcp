// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Setup and initialization utilities for the JSON-RPC handler.
//!
//! This module provides functions to register method handlers and configure
//! the JSON-RPC handler for use in the Mauka MCP server.

use crate::protocol::jsonrpc::handler::JsonRpcHandler;
use crate::protocol::jsonrpc::methods::{
    register_initialize_method, register_tools_list_method,
};

/// Registers all standard method handlers with the JSON-RPC handler.
///
/// This function should be called once during server initialization to
/// set up all the standard JSON-RPC method handlers.
pub fn register_standard_methods(handler: &mut JsonRpcHandler) {
    // Register core protocol methods
    register_initialize_method(handler);
    register_tools_list_method(handler);
    
    // Future method handlers will be registered here
    // register_shutdown_method(handler);
}

/// Creates a fully configured JSON-RPC handler with all standard methods.
///
/// This is a convenience function for creating a handler with all methods
/// pre-registered, ready for use in the Mauka MCP server.
pub fn create_handler() -> JsonRpcHandler {
    let mut handler = JsonRpcHandler::new();
    register_standard_methods(&mut handler);
    handler
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::types::{Id, Request, Response};
    use serde_json::{json, Value};
    
    #[tokio::test(flavor = "multi_thread")]
    async fn test_initialize_method_registered() {
        // Create handler with standard methods
        let handler = create_handler();
        
        // Create initialize request
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: Some(Id::Number(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "client_name": "Test Client",
                "client_version": "1.0.0"
            })),
        };
        
        // Serialize request to string
        let request_str = serde_json::to_string(&request).unwrap();
        
        // Handle request
        let response_str = handler.handle_request(request_str, None).await;
        
        // Deserialize response
        let response: Response = serde_json::from_str(&response_str).unwrap();
        
        // Check that response is successful
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        assert_eq!(response.id, Id::Number(1));
        
        // Check that capabilities field exists in result
        let result = response.result.unwrap();
        assert!(result.is_object());
        let result_obj = result.as_object().unwrap();
        assert!(result_obj.contains_key("capabilities"));
    }
    
    #[tokio::test(flavor = "multi_thread")]
    async fn test_tools_list_method_registered() {
        // Create handler with standard methods
        let handler = create_handler();
        
        // Create tools/list request
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: Some(Id::Number(2)),
            method: "tools/list".to_string(),
            params: Some(json!({
                "include_details": true
            })),
        };
        
        // Serialize request to string
        let request_str = serde_json::to_string(&request).unwrap();
        
        // Handle request
        let response_str = handler.handle_request(request_str, None).await;
        
        // Deserialize response
        let response: Response = serde_json::from_str(&response_str).unwrap();
        
        // Check that response is successful
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        assert_eq!(response.id, Id::Number(2));
        
        // Check that tools field exists in result
        let result = response.result.unwrap();
        assert!(result.is_object());
        let result_obj = result.as_object().unwrap();
        assert!(result_obj.contains_key("tools"));
        assert!(result_obj.contains_key("total_count"));
    }
    
    #[tokio::test(flavor = "multi_thread")]
    async fn test_unknown_method_returns_error() {
        // Create handler with standard methods
        let handler = create_handler();
        
        // Create request with unknown method
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: Some(Id::Number(3)),
            method: "unknown_method".to_string(),
            params: None,
        };
        
        // Serialize request to string
        let request_str = serde_json::to_string(&request).unwrap();
        
        // Handle request
        let response_str = handler.handle_request(request_str, None).await;
        
        // Deserialize response
        let response: Response = serde_json::from_str(&response_str).unwrap();
        
        // Check that response is an error with method not found
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.id, Id::Number(3));
        assert_eq!(response.error.unwrap().code, -32601); // Method not found
    }
}
