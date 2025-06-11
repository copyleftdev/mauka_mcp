// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Error types for the JSON-RPC 2.0 protocol handler.
//!
//! This module defines error codes and error types according to the
//! [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification#error_object).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standard JSON-RPC 2.0 error codes as defined in the specification.
///
/// The error codes from -32768 to -32000 are reserved for pre-defined errors.
/// The error codes -32700, -32600, -32601, -32602, and -32603 are standard JSON-RPC 2.0 errors.
/// The remaining codes in the reserved range are available for application-defined errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Parse error (-32700)
    /// Invalid JSON was received by the server.
    ParseError = -32700,
    
    /// Invalid Request (-32600)
    /// The JSON sent is not a valid Request object.
    InvalidRequest = -32600,
    
    /// Method not found (-32601)
    /// The method does not exist / is not available.
    MethodNotFound = -32601,
    
    /// Invalid params (-32602)
    /// Invalid method parameter(s).
    InvalidParams = -32602,
    
    /// Internal error (-32603)
    /// Internal JSON-RPC error.
    InternalError = -32603,
    
    /// Server error (-32000 to -32099)
    /// Reserved for implementation-defined server errors.
    ServerError = -32000,
    
    /// Application error (-32500)
    /// General application error.
    ApplicationError = -32500,
    
    /// Unauthorized (-32401)
    /// The request lacks valid authentication credentials.
    Unauthorized = -32401,
    
    /// Rate limit exceeded (-32429)
    /// Too many requests have been sent in a given amount of time.
    RateLimitExceeded = -32429,
    
    /// Request cancelled (-32800)
    /// The request was cancelled by the client.
    RequestCancelled = -32800,
}

impl ErrorCode {
    /// Returns a string description of the error code.
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::ParseError => "Parse error",
            ErrorCode::InvalidRequest => "Invalid Request",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::InvalidParams => "Invalid params",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::ServerError => "Server error",
            ErrorCode::ApplicationError => "Application error",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::RateLimitExceeded => "Rate limit exceeded",
            ErrorCode::RequestCancelled => "Request cancelled",
        }
    }
    
    /// Create an ErrorCode from a raw integer value.
    ///
    /// Returns None if the code is not a valid predefined error code.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            -32700 => Some(ErrorCode::ParseError),
            -32600 => Some(ErrorCode::InvalidRequest),
            -32601 => Some(ErrorCode::MethodNotFound),
            -32602 => Some(ErrorCode::InvalidParams),
            -32603 => Some(ErrorCode::InternalError),
            -32500 => Some(ErrorCode::ApplicationError),
            -32401 => Some(ErrorCode::Unauthorized),
            -32429 => Some(ErrorCode::RateLimitExceeded),
            -32800 => Some(ErrorCode::RequestCancelled),
            c if (-32099..=-32000).contains(&c) => Some(ErrorCode::ServerError),
            _ => None,
        }
    }
    
    /// Returns the integer error code.
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

impl From<ErrorCode> for i32 {
    fn from(code: ErrorCode) -> i32 {
        code as i32
    }
}

/// JSON-RPC error object as defined in the specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// The error code
    pub code: i32,
    
    /// A short description of the error
    pub message: String,
    
    /// Additional information about the error (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Creates a new JSON-RPC error.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }
    
    /// Creates a new JSON-RPC error with additional data.
    pub fn with_data(code: ErrorCode, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }
    
    /// Creates a standard parse error.
    pub fn parse_error() -> Self {
        Self::new(
            ErrorCode::ParseError,
            "Parse error: Invalid JSON was received",
        )
    }
    
    /// Creates a standard invalid request error.
    pub fn invalid_request() -> Self {
        Self::new(
            ErrorCode::InvalidRequest,
            "Invalid Request: The JSON sent is not a valid Request object",
        )
    }
    
    /// Creates a standard method not found error.
    pub fn method_not_found<S: Into<String>>(method: S) -> Self {
        Self::new(
            ErrorCode::MethodNotFound,
            format!("Method not found: {}", method.into()),
        )
    }
    
    /// Creates a standard invalid params error.
    pub fn invalid_params<S: Into<String>>(msg: S) -> Self {
        Self::new(
            ErrorCode::InvalidParams,
            format!("Invalid params: {}", msg.into()),
        )
    }
    
    /// Creates a standard internal error.
    pub fn internal_error<S: Into<String>>(msg: S) -> Self {
        Self::new(
            ErrorCode::InternalError,
            format!("Internal error: {}", msg.into()),
        )
    }
}

/// Error type for JSON-RPC operations.
#[derive(Debug, Error)]
pub enum Error {
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// JSON-RPC protocol error
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    
    /// Method handler error
    #[error("Method handler error: {0}")]
    MethodHandler(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<JsonRpcError> for Error {
    fn from(error: JsonRpcError) -> Self {
        Error::JsonRpc(format!("Error {}: {}", error.code, error.message))
    }
}

impl Error {
    /// Converts the error to a JSON-RPC error.
    pub fn to_jsonrpc_error(&self) -> JsonRpcError {
        match self {
            Error::Json(_) => JsonRpcError::parse_error(),
            Error::JsonRpc(msg) => JsonRpcError::new(ErrorCode::InternalError, msg),
            Error::MethodHandler(msg) => JsonRpcError::new(ErrorCode::ApplicationError, msg),
            Error::Io(e) => JsonRpcError::new(ErrorCode::ServerError, e.to_string()),
        }
    }
}

/// Specialized Result type for JSON-RPC operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_code_descriptions() {
        assert_eq!(ErrorCode::ParseError.description(), "Parse error");
        assert_eq!(ErrorCode::InvalidRequest.description(), "Invalid Request");
        assert_eq!(ErrorCode::MethodNotFound.description(), "Method not found");
        assert_eq!(ErrorCode::InvalidParams.description(), "Invalid params");
        assert_eq!(ErrorCode::InternalError.description(), "Internal error");
    }
    
    #[test]
    fn test_error_code_from_code() {
        assert_eq!(ErrorCode::from_code(-32700), Some(ErrorCode::ParseError));
        assert_eq!(ErrorCode::from_code(-32600), Some(ErrorCode::InvalidRequest));
        assert_eq!(ErrorCode::from_code(-32601), Some(ErrorCode::MethodNotFound));
        assert_eq!(ErrorCode::from_code(-32602), Some(ErrorCode::InvalidParams));
        assert_eq!(ErrorCode::from_code(-32603), Some(ErrorCode::InternalError));
        
        // Server error range
        assert_eq!(ErrorCode::from_code(-32000), Some(ErrorCode::ServerError));
        assert_eq!(ErrorCode::from_code(-32099), Some(ErrorCode::ServerError));
        assert_eq!(ErrorCode::from_code(-32050), Some(ErrorCode::ServerError));
        
        // Invalid codes
        assert_eq!(ErrorCode::from_code(0), None);
        assert_eq!(ErrorCode::from_code(-1), None);
        assert_eq!(ErrorCode::from_code(100), None);
    }
    
    #[test]
    fn test_jsonrpc_error_creation() {
        let error = JsonRpcError::new(ErrorCode::ParseError, "Invalid JSON");
        assert_eq!(error.code, -32700);
        assert_eq!(error.message, "Invalid JSON");
        assert!(error.data.is_none());
        
        let error_with_data = JsonRpcError::with_data(
            ErrorCode::InvalidParams,
            "Invalid parameters",
            serde_json::json!({"field": "username", "issue": "required"}),
        );
        assert_eq!(error_with_data.code, -32602);
        assert_eq!(error_with_data.message, "Invalid parameters");
        assert!(error_with_data.data.is_some());
        assert_eq!(
            error_with_data.data.unwrap(),
            serde_json::json!({"field": "username", "issue": "required"})
        );
    }
    
    #[test]
    fn test_error_conversion() {
        // Create a JSON parse error by attempting to parse invalid JSON
        let json_error = Error::Json(serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err());
        let jsonrpc_error = json_error.to_jsonrpc_error();
        assert_eq!(jsonrpc_error.code, -32700);
        
        let method_error = Error::MethodHandler("Division by zero".to_string());
        let jsonrpc_error = method_error.to_jsonrpc_error();
        assert_eq!(jsonrpc_error.code, -32500);
        assert_eq!(jsonrpc_error.message, "Division by zero");
    }
    
    #[test]
    fn test_standard_errors() {
        let parse_error = JsonRpcError::parse_error();
        assert_eq!(parse_error.code, -32700);
        
        let invalid_request = JsonRpcError::invalid_request();
        assert_eq!(invalid_request.code, -32600);
        
        let method_not_found = JsonRpcError::method_not_found("sum");
        assert_eq!(method_not_found.code, -32601);
        assert!(method_not_found.message.contains("sum"));
        
        let invalid_params = JsonRpcError::invalid_params("Missing required parameter");
        assert_eq!(invalid_params.code, -32602);
        assert!(invalid_params.message.contains("Missing required parameter"));
    }
}
