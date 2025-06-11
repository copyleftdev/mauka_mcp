// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Types for the JSON-RPC 2.0 protocol.
//!
//! This module defines the core data structures for JSON-RPC 2.0 requests, responses, and
//! related types according to the [specification](https://www.jsonrpc.org/specification).

use serde::{Deserialize, Serialize};
use std::fmt;

use super::error::JsonRpcError;

/// JSON-RPC request identifier.
///
/// Can be a string, number, or null as per the JSON-RPC 2.0 specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Id {
    /// String identifier
    String(String),
    
    /// Numeric identifier
    Number(i64),
    
    /// Null identifier (not recommended but valid per spec)
    Null,
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::String(s) => write!(f, "{}", s),
            Id::Number(n) => write!(f, "{}", n),
            Id::Null => write!(f, "null"),
        }
    }
}

/// A JSON-RPC 2.0 request object.
///
/// This type represents a request for a method to be invoked on the server.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// JSON-RPC protocol version, always "2.0"
    pub jsonrpc: String,
    
    /// Name of the method to be invoked
    pub method: String,
    
    /// Method parameters, can be positional (array) or named (object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    
    /// Request identifier, if None then the request is a notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl Request {
    /// Creates a new JSON-RPC 2.0 request.
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>, id: Option<Id>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id,
        }
    }
    
    /// Returns true if this request is a notification (no id).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
    
    /// Creates a new JSON-RPC request with a string id.
    pub fn with_string_id(method: impl Into<String>, params: Option<serde_json::Value>, id: impl Into<String>) -> Self {
        Self::new(method, params, Some(Id::String(id.into())))
    }
    
    /// Creates a new JSON-RPC request with a numeric id.
    pub fn with_number_id(method: impl Into<String>, params: Option<serde_json::Value>, id: i64) -> Self {
        Self::new(method, params, Some(Id::Number(id)))
    }
    
    /// Creates a new JSON-RPC notification (no id).
    pub fn notification(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self::new(method, params, None)
    }
}

/// A JSON-RPC 2.0 notification object.
///
/// This is functionally identical to a Request without an id.
/// It's a separate type for API clarity but is serialized/deserialized the same way.
pub type Notification = Request;

/// A JSON-RPC 2.0 response object.
///
/// This type represents a response to a JSON-RPC request. It contains either a result or an error.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    /// JSON-RPC protocol version, always "2.0"
    pub jsonrpc: String,
    
    /// The result of the method invocation, if successful. Must be null if error is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    
    /// The error object, if an error occurred. Must be null if result is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    
    /// Same identifier as the request this is responding to
    pub id: Id,
}

impl Response {
    /// Creates a new successful JSON-RPC 2.0 response.
    pub fn success(id: Id, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }
    
    /// Creates a new error JSON-RPC 2.0 response.
    pub fn error(id: Id, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
    
    /// Returns true if this response contains a successful result.
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }
    
    /// Returns true if this response contains an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// A batch of JSON-RPC 2.0 requests.
///
/// This type represents a batch of requests to be processed together.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BatchRequest {
    /// The list of requests in this batch
    pub requests: Vec<Request>,
}

/// A batch of JSON-RPC 2.0 responses.
///
/// This type represents a batch of responses corresponding to a batch request.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BatchResponse {
    /// The list of responses in this batch
    pub responses: Vec<Response>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_request_serialization() {
        let request = Request::with_number_id(
            "subtract",
            Some(json!({"minuend": 42, "subtrahend": 23})),
            1,
        );
        
        let json_str = serde_json::to_string(&request).unwrap();
        let expected = r#"{"jsonrpc":"2.0","method":"subtract","params":{"minuend":42,"subtrahend":23},"id":1}"#;
        assert_eq!(json_str, expected);
        
        let deserialized: Request = serde_json::from_str(expected).unwrap();
        assert_eq!(deserialized.method, "subtract");
        assert_eq!(deserialized.id, Some(Id::Number(1)));
    }
    
    #[test]
    fn test_notification_serialization() {
        let notification = Request::notification("update", Some(json!([1, 2, 3])));
        
        let json_str = serde_json::to_string(&notification).unwrap();
        let expected = r#"{"jsonrpc":"2.0","method":"update","params":[1,2,3]}"#;
        assert_eq!(json_str, expected);
        
        assert!(notification.is_notification());
    }
    
    #[test]
    fn test_response_serialization() {
        // Success response
        let success = Response::success(Id::Number(1), json!(19));
        
        let json_str = serde_json::to_string(&success).unwrap();
        let expected = r#"{"jsonrpc":"2.0","result":19,"id":1}"#;
        assert_eq!(json_str, expected);
        
        // Error response
        let error = Response::error(
            Id::String("abc".to_string()),
            JsonRpcError::new(super::super::error::ErrorCode::MethodNotFound, "Method not found"),
        );
        
        let json_str = serde_json::to_string(&error).unwrap();
        let expected = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method not found"},"id":"abc"}"#;
        assert_eq!(json_str, expected);
    }
    
    #[test]
    fn test_batch_request_serialization() {
        let batch = BatchRequest {
            requests: vec![
                Request::with_string_id("sum", Some(json!([1, 2, 4])), "1"),
                Request::notification("notify_hello", Some(json!([7]))),
                Request::with_string_id(
                    "subtract", 
                    Some(json!({"minuend": 42, "subtrahend": 23})), 
                    "3"
                ),
            ],
        };
        
        let json_str = serde_json::to_string(&batch).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 3);
        
        // Check that the notification doesn't have an id
        assert!(parsed[1].get("id").is_none());
    }
    
    #[test]
    fn test_id_display() {
        assert_eq!(Id::String("abc".to_string()).to_string(), "abc");
        assert_eq!(Id::Number(123).to_string(), "123");
        assert_eq!(Id::Null.to_string(), "null");
    }
}
