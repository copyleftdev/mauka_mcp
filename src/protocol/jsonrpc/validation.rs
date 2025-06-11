// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Request validation utilities for the JSON-RPC 2.0 handler.
//!
//! This module provides functions to validate JSON-RPC requests according to the specification.
//! It includes checks for protocol version, required fields, and data type validation.

use super::error::{Error, ErrorCode, JsonRpcError, Result};
use super::types::{BatchRequest, Request};
use serde_json::Value;

/// Validates a JSON-RPC 2.0 request string.
///
/// Performs the following checks:
/// - Verifies the JSON is valid
/// - Checks if it's a single request or batch
/// - For each request, validates required fields and format
///
/// Returns an error if validation fails, otherwise returns the parsed request(s).
pub fn validate_request<T: AsRef<str>>(request_str: T) -> Result<ValidatedRequest> {
    let request_str = request_str.as_ref();
    
    // Parse the JSON first
    let json: Value = serde_json::from_str(request_str)
        .map_err(|e| Error::Json(e))?;
    
    // Check if it's a batch or single request
    match json {
        Value::Array(ref arr) => {
            if arr.is_empty() {
                return Err(Error::JsonRpc("Empty batch requests are invalid".to_string()));
            }
            
            // Parse as batch
            let batch: BatchRequest = serde_json::from_value(json)
                .map_err(|e| Error::Json(e))?;
            
            // Validate each request in the batch
            for request in &batch.requests {
                validate_single_request(request)?;
            }
            
            Ok(ValidatedRequest::Batch(batch))
        },
        Value::Object(_) => {
            // Parse as single request
            let request: Request = serde_json::from_value(json)
                .map_err(|e| Error::Json(e))?;
            
            validate_single_request(&request)?;
            
            Ok(ValidatedRequest::Single(request))
        },
        _ => Err(Error::JsonRpc("Invalid JSON-RPC request, must be an object or array".to_string())),
    }
}

/// Validates a single JSON-RPC 2.0 request object.
///
/// Performs the following checks:
/// - Verifies the jsonrpc version is "2.0"
/// - Checks that method is a non-empty string
/// - Validates that params, if present, is either an object or array
/// - Ensures the id, if present, is valid (string, number, or null)
fn validate_single_request(request: &Request) -> Result<()> {
    // Check jsonrpc version
    if request.jsonrpc != "2.0" {
        return Err(Error::JsonRpc(
            format!("Invalid JSON-RPC version: {}, must be 2.0", request.jsonrpc)
        ));
    }
    
    // Check method
    if request.method.is_empty() {
        return Err(Error::JsonRpc("Method cannot be empty".to_string()));
    }
    
    // Check params if present
    if let Some(ref params) = request.params {
        if !params.is_object() && !params.is_array() && !params.is_null() {
            return Err(Error::JsonRpc(
                "Params must be an object, array, or null".to_string()
            ));
        }
    }
    
    // All checks passed
    Ok(())
}

/// The result of validating a JSON-RPC request.
///
/// Can be either a single request or a batch of requests.
#[derive(Debug, Clone)]
pub enum ValidatedRequest {
    /// A single, validated JSON-RPC request
    Single(Request),
    
    /// A batch of validated JSON-RPC requests
    Batch(BatchRequest),
}

impl ValidatedRequest {
    /// Returns true if this is a batch request
    pub fn is_batch(&self) -> bool {
        matches!(self, ValidatedRequest::Batch(_))
    }
    
    /// Returns true if this is a single request
    pub fn is_single(&self) -> bool {
        matches!(self, ValidatedRequest::Single(_))
    }
    
    /// Returns the contained batch if this is a batch request
    pub fn as_batch(&self) -> Option<&BatchRequest> {
        match self {
            ValidatedRequest::Batch(batch) => Some(batch),
            _ => None,
        }
    }
    
    /// Returns the contained single request if this is a single request
    pub fn as_single(&self) -> Option<&Request> {
        match self {
            ValidatedRequest::Single(req) => Some(req),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_validate_valid_request() {
        let request_str = r#"{"jsonrpc": "2.0", "method": "subtract", "params": {"minuend": 42, "subtrahend": 23}, "id": 1}"#;
        let result = validate_request(request_str);
        assert!(result.is_ok());
        
        // Check that it's parsed as a single request
        let validated = result.unwrap();
        assert!(validated.is_single());
        let request = validated.as_single().unwrap();
        assert_eq!(request.method, "subtract");
    }
    
    #[test]
    fn test_validate_valid_notification() {
        let notification_str = r#"{"jsonrpc": "2.0", "method": "update", "params": [1, 2, 3]}"#;
        let result = validate_request(notification_str);
        assert!(result.is_ok());
        
        // Check that it's parsed as a single request (notification)
        let validated = result.unwrap();
        assert!(validated.is_single());
        let request = validated.as_single().unwrap();
        assert_eq!(request.method, "update");
        assert!(request.is_notification());
    }
    
    #[test]
    fn test_validate_valid_batch() {
        let batch_str = r#"[
            {"jsonrpc": "2.0", "method": "sum", "params": [1,2,4], "id": "1"},
            {"jsonrpc": "2.0", "method": "notify_hello", "params": [7]},
            {"jsonrpc": "2.0", "method": "subtract", "params": {"minuend": 42, "subtrahend": 23}, "id": "3"}
        ]"#;
        
        let result = validate_request(batch_str);
        assert!(result.is_ok());
        
        // Check that it's parsed as a batch
        let validated = result.unwrap();
        assert!(validated.is_batch());
        let batch = validated.as_batch().unwrap();
        assert_eq!(batch.requests.len(), 3);
    }
    
    #[test]
    fn test_validate_invalid_version() {
        let invalid_version = r#"{"jsonrpc": "1.0", "method": "subtract", "params": {"minuend": 42, "subtrahend": 23}, "id": 1}"#;
        let result = validate_request(invalid_version);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::JsonRpc(msg) => assert!(msg.contains("Invalid JSON-RPC version")),
            _ => panic!("Expected JsonRpc error"),
        }
    }
    
    #[test]
    fn test_validate_empty_method() {
        let empty_method = r#"{"jsonrpc": "2.0", "method": "", "params": {"minuend": 42, "subtrahend": 23}, "id": 1}"#;
        let result = validate_request(empty_method);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::JsonRpc(msg) => assert_eq!(msg, "Method cannot be empty"),
            _ => panic!("Expected JsonRpc error"),
        }
    }
    
    #[test]
    fn test_validate_invalid_params() {
        let invalid_params = r#"{"jsonrpc": "2.0", "method": "test", "params": "not-an-object-or-array", "id": 1}"#;
        let result = validate_request(invalid_params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::JsonRpc(msg) => assert!(msg.contains("Params must be")),
            _ => panic!("Expected JsonRpc error"),
        }
    }
    
    #[test]
    fn test_validate_invalid_json() {
        let invalid_json = r#"{"jsonrpc": "2.0", "method": "test", "params": [1, 2,"#; // Incomplete JSON
        let result = validate_request(invalid_json);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Json(_) => {}, // Expected
            e => panic!("Expected Json error, got {:?}", e),
        }
    }
    
    #[test]
    fn test_validate_empty_batch() {
        let empty_batch = "[]";
        let result = validate_request(empty_batch);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::JsonRpc(msg) => assert!(msg.contains("Empty batch requests")),
            e => panic!("Expected JsonRpc error, got {:?}", e),
        }
    }
    
    #[test]
    fn test_validate_not_object_or_array() {
        let not_object_or_array = "42";
        let result = validate_request(not_object_or_array);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::JsonRpc(msg) => assert!(msg.contains("must be an object or array")),
            e => panic!("Expected JsonRpc error, got {:?}", e),
        }
    }
}
