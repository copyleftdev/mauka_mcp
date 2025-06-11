// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! JSON-RPC 2.0 protocol handler for the Mauka MCP Server.
//!
//! This module implements the [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification),
//! providing utilities for parsing, validating, and handling JSON-RPC requests and responses.
//! It supports both single requests/responses and batched operations as per the specification.
//!
//! # Features
//!
//! - Full JSON-RPC 2.0 specification compliance
//! - Support for requests, notifications, and responses
//! - Batch request/response handling
//! - Comprehensive error handling with standardized error codes
//! - Request validation and sanitization
//! - Zero-copy parsing where possible
//! - Asynchronous handler support
//!
//! # Example
//!
//! ```
//! use mauka_mcp::protocol::jsonrpc::{JsonRpcHandler, Request, Response, ErrorCode};
//! use std::collections::HashMap;
//!
//! // Create a new handler
//! let mut handler = JsonRpcHandler::new();
//!
//! // Register a method handler
//! handler.register_method("echo", |params, _ctx| async move {
//!     // Extract parameters
//!     let message = params.get("message").and_then(|v| v.as_str())
//!         .unwrap_or("No message provided");
//!
//!     // Return a successful response
//!     Ok(serde_json::json!({
//!         "echoed": message
//!     }))
//! });
//!
//! // Parse a JSON-RPC request
//! let request_json = r#"{
//!     "jsonrpc": "2.0",
//!     "method": "echo",
//!     "params": { "message": "Hello, world!" },
//!     "id": 1
//! }"#;
//!
//! // Handle the request (in a real application, run this in an async context)
//! let response = handler.handle_request(request_json, None).await;
//!
//! // Response will be a valid JSON-RPC 2.0 response
//! assert!(response.contains(r#""result":{"echoed":"Hello, world!"}"#));
//! assert!(response.contains(r#""id":1"#));
//! ```

pub mod error;
pub mod handler;
pub mod methods;
pub mod setup;
pub mod types;
pub mod validation;
pub mod correlation;

// Re-exports
pub use error::{Error, ErrorCode, JsonRpcError, Result};
pub use handler::JsonRpcHandler;
pub use setup::{create_handler, register_standard_methods};
pub use types::{BatchRequest, BatchResponse, Id, Notification, Request, Response};
pub use validation::validate_request;
pub use correlation::{CorrelationError, RequestResponseCorrelator};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_parse_valid_request() {
        let request_str = r#"{
            "jsonrpc": "2.0", 
            "method": "subtract", 
            "params": {"minuend": 42, "subtrahend": 23}, 
            "id": 1
        }"#;
        
        let request: Request = serde_json::from_str(request_str).unwrap();
        
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "subtract");
        assert_eq!(request.id, Some(Id::Number(1)));
        
        if let Some(params) = request.params {
            let obj = params.as_object().unwrap();
            assert_eq!(obj["minuend"], json!(42));
            assert_eq!(obj["subtrahend"], json!(23));
        } else {
            panic!("Expected params to exist");
        }
    }
    
    #[test]
    fn test_parse_notification() {
        let notification_str = r#"{
            "jsonrpc": "2.0", 
            "method": "update", 
            "params": [1, 2, 3]
        }"#;
        
        let notification: Notification = serde_json::from_str(notification_str).unwrap();
        
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "update");
        
        if let Some(params) = notification.params {
            let array = params.as_array().unwrap();
            assert_eq!(array.len(), 3);
            assert_eq!(array[0], json!(1));
            assert_eq!(array[1], json!(2));
            assert_eq!(array[2], json!(3));
        } else {
            panic!("Expected params to exist");
        }
    }
    
    #[test]
    fn test_parse_response() {
        let response_str = r#"{
            "jsonrpc": "2.0", 
            "result": 19, 
            "id": 1
        }"#;
        
        let response: Response = serde_json::from_str(response_str).unwrap();
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Id::Number(1));
        assert_eq!(response.result, Some(json!(19)));
        assert!(response.error.is_none());
    }
    
    #[test]
    fn test_parse_error_response() {
        let error_response_str = r#"{
            "jsonrpc": "2.0", 
            "error": {"code": -32601, "message": "Method not found"}, 
            "id": 1
        }"#;
        
        let response: Response = serde_json::from_str(error_response_str).unwrap();
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Id::Number(1));
        assert!(response.result.is_none());
        
        if let Some(error) = response.error {
            assert_eq!(error.code, -32601);
            assert_eq!(error.message, "Method not found");
        } else {
            panic!("Expected error to exist");
        }
    }
    
    #[test]
    fn test_batch_request() {
        let batch_str = r#"[
            {"jsonrpc": "2.0", "method": "sum", "params": [1,2,4], "id": "1"},
            {"jsonrpc": "2.0", "method": "notify_hello", "params": [7]},
            {"jsonrpc": "2.0", "method": "subtract", "params": {"minuend": 42, "subtrahend": 23}, "id": "3"}
        ]"#;
        
        let batch: BatchRequest = serde_json::from_str(batch_str).unwrap();
        
        assert_eq!(batch.requests.len(), 3);
        assert_eq!(batch.requests[0].method, "sum");
        assert_eq!(batch.requests[0].id, Some(Id::String("1".to_string())));
        
        // Second is a notification (no id)
        assert_eq!(batch.requests[1].method, "notify_hello");
        assert_eq!(batch.requests[1].id, None);
        
        assert_eq!(batch.requests[2].method, "subtract");
        assert_eq!(batch.requests[2].id, Some(Id::String("3".to_string())));
    }
}
