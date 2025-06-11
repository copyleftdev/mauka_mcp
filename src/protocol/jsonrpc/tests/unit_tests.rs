// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Unit tests for the JSON-RPC 2.0 handler.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::protocol::jsonrpc::{
    BatchRequest, Error, ErrorCode, Id, JsonRpcError, JsonRpcHandler, MethodContext,
    MethodHandler, Request, Response, Result,
};

// Helper struct for testing method handlers
struct TestHandler {
    response: serde_json::Value,
    should_error: bool,
    error_code: Option<ErrorCode>,
}

#[async_trait]
impl MethodHandler for TestHandler {
    async fn handle(&self, params: Option<serde_json::Value>, _ctx: MethodContext) -> std::result::Result<serde_json::Value, JsonRpcError> {
        if self.should_error {
            let code = self.error_code.unwrap_or(ErrorCode::InternalError);
            Err(JsonRpcError::new(code, "Test error"))
        } else {
            Ok(self.response.clone())
        }
    }
}

// Creates a test handler that returns a successful response
fn success_handler(response: serde_json::Value) -> Arc<dyn MethodHandler + Send + Sync> {
    Arc::new(TestHandler {
        response,
        should_error: false,
        error_code: None,
    })
}

// Creates a test handler that returns an error response
fn error_handler(code: ErrorCode) -> Arc<dyn MethodHandler + Send + Sync> {
    Arc::new(TestHandler {
        response: json!(null),
        should_error: true,
        error_code: Some(code),
    })
}

#[tokio::test]
async fn test_handle_valid_request() {
    let mut handler = JsonRpcHandler::new();
    
    // Register a test method
    handler.register_method("add", |params, _ctx| async move {
        let obj = params.and_then(|p| p.as_object()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Expected object params")
        })?;
        
        let a = obj.get("a").and_then(|v| v.as_i64()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Missing parameter 'a'")
        })?;
        
        let b = obj.get("b").and_then(|v| v.as_i64()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Missing parameter 'b'")
        })?;
        
        Ok(json!(a + b))
    });
    
    // Test with a valid request
    let request = r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#;
    let response = handler.handle_request(request, None).await;
    
    // Parse the response and check fields
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert_eq!(response_obj.jsonrpc, "2.0");
    assert_eq!(response_obj.id, Id::Number(1));
    assert_eq!(response_obj.result, Some(json!(8)));
    assert!(response_obj.error.is_none());
}

#[tokio::test]
async fn test_handle_method_not_found() {
    let handler = JsonRpcHandler::new();
    
    // Test with a non-existent method
    let request = r#"{"jsonrpc": "2.0", "method": "non_existent", "params": {}, "id": 1}"#;
    let response = handler.handle_request(request, None).await;
    
    // Parse the response and check fields
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert_eq!(response_obj.jsonrpc, "2.0");
    assert_eq!(response_obj.id, Id::Number(1));
    assert!(response_obj.result.is_none());
    assert!(response_obj.error.is_some());
    
    let error = response_obj.error.unwrap();
    assert_eq!(error.code, ErrorCode::MethodNotFound as i32);
    assert!(error.message.contains("non_existent"));
}

#[tokio::test]
async fn test_handle_notification() {
    let mut handler = JsonRpcHandler::new();
    
    // Use a mutable counter to track if the notification was processed
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    
    // Clone the counter for the closure
    let counter_clone = counter.clone();
    
    // Register a notification handler that increments the counter
    handler.register_method("notify", move |_, _| {
        let counter = counter_clone.clone();
        async move {
            // Increment the counter
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(json!(null))
        }
    });
    
    // Send a notification (no id)
    let request = r#"{"jsonrpc": "2.0", "method": "notify", "params": {}}"#;
    let response = handler.handle_request(request, None).await;
    
    // For notifications, no response should be returned
    assert_eq!(response, "");
    
    // Wait a bit for the notification to be processed asynchronously
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Check that the counter was incremented
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_handle_batch_request() {
    let mut handler = JsonRpcHandler::new();
    
    // Register methods
    handler.register_method("add", |params, _| async move {
        let array = params.and_then(|p| p.as_array()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Expected array params")
        })?;
        
        if array.len() < 2 {
            return Err(JsonRpcError::new(
                ErrorCode::InvalidParams,
                "Expected at least 2 parameters",
            ));
        }
        
        let a = array[0].as_i64().ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "First parameter must be a number")
        })?;
        
        let b = array[1].as_i64().ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Second parameter must be a number")
        })?;
        
        Ok(json!(a + b))
    });
    
    handler.register_method("subtract", |params, _| async move {
        let obj = params.and_then(|p| p.as_object()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Expected object params")
        })?;
        
        let minuend = obj.get("minuend").and_then(|v| v.as_i64()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Missing parameter 'minuend'")
        })?;
        
        let subtrahend = obj.get("subtrahend").and_then(|v| v.as_i64()).ok_or_else(|| {
            JsonRpcError::new(ErrorCode::InvalidParams, "Missing parameter 'subtrahend'")
        })?;
        
        Ok(json!(minuend - subtrahend))
    });
    
    // Test with a batch request
    let batch = r#"[
        {"jsonrpc": "2.0", "method": "add", "params": [1, 2], "id": "1"},
        {"jsonrpc": "2.0", "method": "subtract", "params": {"minuend": 10, "subtrahend": 5}, "id": "2"},
        {"jsonrpc": "2.0", "method": "non_existent", "params": {}, "id": "3"}
    ]"#;
    
    let response = handler.handle_request(batch, None).await;
    
    // Parse the response
    let response_array: Vec<Response> = serde_json::from_str(&response).unwrap();
    assert_eq!(response_array.len(), 3);
    
    // Check each response
    assert_eq!(response_array[0].id, Id::String("1".to_string()));
    assert_eq!(response_array[0].result, Some(json!(3)));
    
    assert_eq!(response_array[1].id, Id::String("2".to_string()));
    assert_eq!(response_array[1].result, Some(json!(5)));
    
    assert_eq!(response_array[2].id, Id::String("3".to_string()));
    assert!(response_array[2].error.is_some());
    assert_eq!(response_array[2].error.as_ref().unwrap().code, -32601); // Method not found
}

#[tokio::test]
async fn test_handle_invalid_request() {
    let handler = JsonRpcHandler::new();
    
    // Test with an invalid JSON
    let request = r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1"#; // Missing closing brace
    let response = handler.handle_request(request, None).await;
    
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert!(response_obj.error.is_some());
    assert_eq!(response_obj.error.as_ref().unwrap().code, -32700); // Parse error
    
    // Test with an invalid request object
    let request = r#"{"jsonrpc": "1.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#; // Wrong version
    let response = handler.handle_request(request, None).await;
    
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert!(response_obj.error.is_some());
    assert_eq!(response_obj.error.as_ref().unwrap().code, -32600); // Invalid request
}

#[tokio::test]
async fn test_context_provider() {
    let mut handler = JsonRpcHandler::new();
    
    // Register a context-aware method
    handler.register_method("get_context", |_, context| async move {
        let user_id = context.metadata.get("user_id").cloned().unwrap_or_default();
        Ok(json!({ "user_id": user_id }))
    });
    
    // Register a context provider that adds a user_id to the context
    handler.register_context_provider(|| {
        let mut metadata = HashMap::new();
        metadata.insert("user_id".to_string(), "test-user-123".to_string());
        
        MethodContext { metadata }
    });
    
    // Test with a request that uses the context
    let request = r#"{"jsonrpc": "2.0", "method": "get_context", "id": 1}"#;
    let response = handler.handle_request(request, None).await;
    
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert_eq!(response_obj.result, Some(json!({ "user_id": "test-user-123" })));
    
    // Test with an explicitly provided context that overrides the provider
    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "override-user-456".to_string());
    let context = MethodContext { metadata };
    
    let response = handler.handle_request(request, Some(context)).await;
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert_eq!(response_obj.result, Some(json!({ "user_id": "override-user-456" })));
}

#[tokio::test]
async fn test_method_error_handling() {
    let mut handler = JsonRpcHandler::new();
    
    // Register a method that always returns an error
    handler.register_method("error_method", |_, _| async move {
        Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            "This method always fails",
        ))
    });
    
    // Test with the error method
    let request = r#"{"jsonrpc": "2.0", "method": "error_method", "params": {}, "id": 1}"#;
    let response = handler.handle_request(request, None).await;
    
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert!(response_obj.error.is_some());
    assert_eq!(response_obj.error.as_ref().unwrap().code, -32602); // Invalid params
    assert_eq!(response_obj.error.as_ref().unwrap().message, "This method always fails");
}

#[tokio::test]
async fn test_batch_with_only_notifications() {
    let mut handler = JsonRpcHandler::new();
    
    // Track notification processing with a counter
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    // Register a notification handler
    handler.register_method("notify", move |_, _| {
        let counter = counter_clone.clone();
        async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(json!(null))
        }
    });
    
    // Send a batch with only notifications (no ids)
    let batch = r#"[
        {"jsonrpc": "2.0", "method": "notify", "params": {}},
        {"jsonrpc": "2.0", "method": "notify", "params": {}}
    ]"#;
    
    let response = handler.handle_request(batch, None).await;
    
    // According to spec, we should get an empty array back for a batch with only notifications
    assert_eq!(response, "[]");
    
    // Wait a bit for the notifications to be processed asynchronously
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Check that both notifications were processed
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_empty_batch() {
    let handler = JsonRpcHandler::new();
    
    // Send an empty batch
    let batch = r#"[]"#;
    let response = handler.handle_request(batch, None).await;
    
    // Should get an invalid request error
    let response_obj: Response = serde_json::from_str(&response).unwrap();
    assert!(response_obj.error.is_some());
    assert_eq!(response_obj.error.as_ref().unwrap().code, -32600); // Invalid request
}
