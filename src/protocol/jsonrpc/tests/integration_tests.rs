// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Integration tests for the JSON-RPC 2.0 handler.
//! These tests verify that the JSON-RPC handler correctly integrates with other
//! protocol components and the async runtime.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::channel::mpsc;
use futures::stream::StreamExt;
use serde_json::json;
use tokio::sync::Mutex;

use crate::protocol::jsonrpc::{
    JsonRpcHandler, MethodContext, MethodHandler, Request, Response, JsonRpcError, ErrorCode,
    BatchRequest, Id,
};

// A simple service that can be called via JSON-RPC
struct CounterService {
    counter: Arc<AtomicU64>,
}

impl CounterService {
    fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
        }
    }
    
    fn increment(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst) + 1
    }
    
    fn get_count(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
    
    fn reset(&self) {
        self.counter.store(0, Ordering::SeqCst);
    }
}

struct CounterHandler {
    service: Arc<CounterService>,
}

#[async_trait]
impl MethodHandler for CounterHandler {
    async fn handle(&self, params: Option<serde_json::Value>, _ctx: MethodContext) -> Result<serde_json::Value, JsonRpcError> {
        // Parse the operation from params
        let operation = params
            .and_then(|p| p.as_object())
            .and_then(|obj| obj.get("operation"))
            .and_then(|op| op.as_str())
            .ok_or_else(|| JsonRpcError::new(
                ErrorCode::InvalidParams,
                "Missing 'operation' parameter",
            ))?;
        
        match operation {
            "increment" => {
                let new_value = self.service.increment();
                Ok(json!({ "new_value": new_value }))
            },
            "get" => {
                let value = self.service.get_count();
                Ok(json!({ "value": value }))
            },
            "reset" => {
                self.service.reset();
                Ok(json!({ "status": "ok" }))
            },
            _ => Err(JsonRpcError::new(
                ErrorCode::InvalidParams,
                format!("Unknown operation: {}", operation),
            )),
        }
    }
}

// A test fixture that sets up a JsonRpcHandler with the CounterService
struct TestFixture {
    handler: JsonRpcHandler,
    service: Arc<CounterService>,
}

impl TestFixture {
    fn new() -> Self {
        let mut handler = JsonRpcHandler::new();
        let service = Arc::new(CounterService::new());
        
        handler.register_method("counter", move |params, ctx| {
            let counter_handler = CounterHandler { service: service.clone() };
            async move { counter_handler.handle(params, ctx).await }
        });
        
        Self { handler, service }
    }
    
    async fn send_request(&self, request: &str) -> Response {
        let response = self.handler.handle_request(request, None).await;
        serde_json::from_str(&response).expect("Failed to parse response")
    }
}

#[tokio::test]
async fn test_integration_counter_service() {
    let fixture = TestFixture::new();
    
    // Reset the counter
    let reset_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "reset"}, "id": 1}"#;
    let response = fixture.send_request(reset_request).await;
    assert_eq!(response.result, Some(json!({"status": "ok"})));
    
    // Increment counter
    let increment_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "increment"}, "id": 2}"#;
    let response = fixture.send_request(increment_request).await;
    assert_eq!(response.result, Some(json!({"new_value": 1})));
    
    // Get counter value
    let get_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "get"}, "id": 3}"#;
    let response = fixture.send_request(get_request).await;
    assert_eq!(response.result, Some(json!({"value": 1})));
    
    // Increment again
    let increment_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "increment"}, "id": 4}"#;
    let response = fixture.send_request(increment_request).await;
    assert_eq!(response.result, Some(json!({"new_value": 2})));
    
    // Check final value
    assert_eq!(fixture.service.get_count(), 2);
}

// Test for handling concurrent requests
#[tokio::test]
async fn test_concurrent_requests() {
    let fixture = TestFixture::new();
    
    // Reset counter
    let reset_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "reset"}, "id": 1}"#;
    fixture.send_request(reset_request).await;
    
    // Create multiple concurrent increment requests
    let mut tasks = Vec::new();
    const NUM_REQUESTS: usize = 100;
    
    for i in 0..NUM_REQUESTS {
        let handler = fixture.handler.clone();
        let request = format!(
            r#"{{"jsonrpc": "2.0", "method": "counter", "params": {{"operation": "increment"}}, "id": {}}}"#,
            i + 100
        );
        
        tasks.push(tokio::spawn(async move {
            handler.handle_request(request, None).await
        }));
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await.expect("Task failed");
    }
    
    // Verify that all increments were processed
    assert_eq!(fixture.service.get_count(), NUM_REQUESTS as u64);
}

// Test for handling streaming requests and responses
#[tokio::test]
async fn test_request_response_stream() {
    let fixture = Arc::new(TestFixture::new());
    
    // Reset counter
    let reset_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "reset"}, "id": 1}"#;
    fixture.send_request(reset_request).await;
    
    // Create channels for request-response streaming
    let (tx, mut rx) = mpsc::channel(10);
    let tx = Arc::new(Mutex::new(tx));
    
    // Spawn a task to process requests
    let fixture_clone = fixture.clone();
    let processor = tokio::spawn(async move {
        while let Some(request) = rx.next().await {
            let response = fixture_clone.handler.handle_request(request, None).await;
            // In a real system, the response would be sent back through another channel
            // For testing purposes, we just let it drop
        }
    });
    
    // Send a series of increment requests
    for i in 0..10 {
        let request = format!(
            r#"{{"jsonrpc": "2.0", "method": "counter", "params": {{"operation": "increment"}}, "id": {}}}"#,
            i + 200
        );
        
        tx.lock().await.try_send(request).expect("Failed to send request");
        
        // Small delay to simulate realistic timing
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    // Give time for all requests to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Check that all increments were processed
    assert_eq!(fixture.service.get_count(), 10);
    
    // Clean up
    drop(tx);
    let _ = processor.await;
}

// Test for error propagation
#[tokio::test]
async fn test_error_propagation() {
    let fixture = TestFixture::new();
    
    // Send request with invalid operation
    let invalid_request = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "invalid"}, "id": 1}"#;
    let response = fixture.send_request(invalid_request).await;
    
    // Verify error response
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::InvalidParams as i32);
    assert!(error.message.contains("Unknown operation"));
    
    // Send request with missing params
    let missing_params = r#"{"jsonrpc": "2.0", "method": "counter", "id": 2}"#;
    let response = fixture.send_request(missing_params).await;
    
    // Verify error response
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::InvalidParams as i32);
}

// Test for request correlation
#[tokio::test]
async fn test_request_correlation() {
    let fixture = TestFixture::new();
    
    // Reset counter
    fixture.send_request(r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "reset"}, "id": 1}"#).await;
    
    // Send two increment requests with different IDs
    let req1 = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "increment"}, "id": "req-1"}"#;
    let req2 = r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "increment"}, "id": "req-2"}"#;
    
    let resp1 = fixture.send_request(req1).await;
    let resp2 = fixture.send_request(req2).await;
    
    // Verify that response IDs match request IDs
    assert_eq!(resp1.id, Id::String("req-1".to_string()));
    assert_eq!(resp2.id, Id::String("req-2".to_string()));
}

// Test for batch request handling with mixed results (success and errors)
#[tokio::test]
async fn test_mixed_batch_requests() {
    let fixture = TestFixture::new();
    
    // Reset counter first
    fixture.send_request(r#"{"jsonrpc": "2.0", "method": "counter", "params": {"operation": "reset"}, "id": 0}"#).await;
    
    // Create a batch with mix of valid and invalid requests
    let batch = r#"[
        {"jsonrpc": "2.0", "method": "counter", "params": {"operation": "increment"}, "id": "req-1"},
        {"jsonrpc": "2.0", "method": "counter", "params": {"operation": "invalid"}, "id": "req-2"},
        {"jsonrpc": "2.0", "method": "counter", "params": {"operation": "get"}, "id": "req-3"},
        {"jsonrpc": "2.0", "method": "non_existent", "params": {}, "id": "req-4"}
    ]"#;
    
    let response_str = fixture.handler.handle_request(batch, None).await;
    let responses: Vec<Response> = serde_json::from_str(&response_str).expect("Failed to parse batch response");
    
    // Verify we got 4 responses
    assert_eq!(responses.len(), 4);
    
    // First response should be success (increment)
    assert_eq!(responses[0].id, Id::String("req-1".to_string()));
    assert_eq!(responses[0].result, Some(json!({"new_value": 1})));
    assert!(responses[0].error.is_none());
    
    // Second response should be error (invalid operation)
    assert_eq!(responses[1].id, Id::String("req-2".to_string()));
    assert!(responses[1].result.is_none());
    assert!(responses[1].error.is_some());
    assert_eq!(responses[1].error.as_ref().unwrap().code, ErrorCode::InvalidParams as i32);
    
    // Third response should be success (get count)
    assert_eq!(responses[2].id, Id::String("req-3".to_string()));
    assert_eq!(responses[2].result, Some(json!({"value": 1})));
    assert!(responses[2].error.is_none());
    
    // Fourth response should be error (method not found)
    assert_eq!(responses[3].id, Id::String("req-4".to_string()));
    assert!(responses[3].result.is_none());
    assert!(responses[3].error.is_some());
    assert_eq!(responses[3].error.as_ref().unwrap().code, ErrorCode::MethodNotFound as i32);
}
