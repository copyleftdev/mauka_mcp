// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Property-based tests for the JSON-RPC 2.0 handler.
//! These tests verify that the handler behaves correctly with randomly generated valid
//! and invalid inputs.

use std::collections::HashMap;
use proptest::prelude::*;
use serde_json::{json, Value};

use crate::protocol::jsonrpc::{
    JsonRpcHandler, MethodContext, JsonRpcError, ErrorCode, Id, Request, Response,
};

// Generate a valid method name (alphanumeric with underscores)
fn method_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{1,20}".prop_map(String::from)
}

// Generate a valid ID (Number, String, or Null)
fn id_strategy() -> impl Strategy<Value = Option<Id>> {
    prop_oneof![
        Just(None),
        any::<i32>().prop_map(|n| Some(Id::Number(n as i64))),
        "[a-zA-Z0-9_-]{1,10}".prop_map(|s| Some(Id::String(s)))
    ]
}

// Generate valid params (object, array, or none)
fn params_strategy() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        Just(None),
        // Object params
        prop::collection::hash_map("[a-z]{1,5}", -100i32..100, 0..5)
            .prop_map(|map| {
                let obj_map = map.into_iter()
                    .map(|(k, v)| (k, json!(v)))
                    .collect::<HashMap<_, _>>();
                Some(json!(obj_map))
            }),
        // Array params
        prop::collection::vec(any::<i32>(), 0..5)
            .prop_map(|vec| Some(json!(vec)))
    ]
}

// Generate a valid JSON-RPC request
fn request_strategy() -> impl Strategy<Value = Request> {
    (method_name_strategy(), params_strategy(), id_strategy()).prop_map(|(method, params, id)| {
        Request {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    })
}

// Generate a batch of valid JSON-RPC requests
fn batch_strategy(size: usize) -> impl Strategy<Value = Vec<Request>> {
    prop::collection::vec(request_strategy(), 1..=size)
}

// Test handler for property-based testing
fn create_test_handler() -> JsonRpcHandler {
    let mut handler = JsonRpcHandler::new();
    
    // Echo handler simply returns the params
    handler.register_method("echo", |params, _| async move {
        match params {
            Some(p) => Ok(p),
            None => Ok(json!(null)),
        }
    });
    
    // Add handler adds numbers
    handler.register_method("add", |params, _| async move {
        match params {
            Some(Value::Array(arr)) => {
                if arr.len() < 2 {
                    return Err(JsonRpcError::new(
                        ErrorCode::InvalidParams,
                        "Expected at least 2 parameters",
                    ));
                }
                
                let mut sum = 0_i64;
                for val in arr {
                    if let Some(n) = val.as_i64() {
                        sum += n;
                    } else if let Some(n) = val.as_u64() {
                        sum += n as i64;
                    } else if let Some(n) = val.as_f64() {
                        sum += n as i64;
                    } else {
                        return Err(JsonRpcError::new(
                            ErrorCode::InvalidParams,
                            "Parameters must be numbers",
                        ));
                    }
                }
                
                Ok(json!(sum))
            },
            _ => Err(JsonRpcError::new(
                ErrorCode::InvalidParams,
                "Expected array of numbers",
            )),
        }
    });
    
    // Error handler always returns an error
    handler.register_method("error", |params, _| async move {
        let code = params
            .and_then(|p| p.as_object())
            .and_then(|o| o.get("code"))
            .and_then(|c| c.as_i64())
            .unwrap_or(ErrorCode::InternalError as i64);
            
        Err(JsonRpcError::new_with_code(
            code,
            "Test error".to_string(),
            None,
        ))
    });
    
    handler
}

// Test that a valid request gets a matching response
proptest! {
    #[test]
    fn test_valid_echo_request(
        params in params_strategy(), 
        id in id_strategy().prop_filter("ID must be present", |id| id.is_some())
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let handler = create_test_handler();
            
            let request = Request {
                jsonrpc: "2.0".to_string(),
                method: "echo".to_string(),
                params: params.clone(),
                id: id.clone(),
            };
            
            // Serialize request to JSON
            let request_str = serde_json::to_string(&request).unwrap();
            
            // Send request and get response
            let response_str = handler.handle_request(request_str, None).await;
            let response: Response = serde_json::from_str(&response_str).unwrap();
            
            // Verify response
            prop_assert_eq!(response.jsonrpc, "2.0");
            prop_assert_eq!(response.id, id.unwrap());
            prop_assert_eq!(response.result, params);
            prop_assert!(response.error.is_none());
        });
    }
}

// Test that batch processing works correctly for all valid requests
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn test_valid_batch_requests(
        batch in batch_strategy(5).prop_filter(
            "At least one request must have ID", 
            |reqs| reqs.iter().any(|r| r.id.is_some())
        )
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let handler = create_test_handler();
            
            // Modify all requests to use the "echo" method for simplicity
            let batch: Vec<Request> = batch.into_iter()
                .map(|mut req| {
                    req.method = "echo".to_string();
                    req
                })
                .collect();
            
            // Count requests with IDs (should get responses)
            let expected_response_count = batch.iter()
                .filter(|req| req.id.is_some())
                .count();
            
            // Serialize batch to JSON
            let batch_str = serde_json::to_string(&batch).unwrap();
            
            // Send batch and get response
            let response_str = handler.handle_request(batch_str, None).await;
            let responses: Vec<Response> = serde_json::from_str(&response_str).unwrap();
            
            // Verify batch response count
            prop_assert_eq!(responses.len(), expected_response_count);
            
            // Verify each response matches its request
            for response in &responses {
                // Find corresponding request
                let request = batch.iter()
                    .find(|req| req.id.as_ref() == Some(&response.id))
                    .unwrap();
                
                // Verify response
                prop_assert_eq!(response.jsonrpc, "2.0");
                prop_assert_eq!(response.result, request.params);
                prop_assert!(response.error.is_none());
            }
        });
    }
}

// Test handling of invalid requests
proptest! {
    #[test]
    fn test_invalid_requests(s in ".*") {
        if s.is_empty() || serde_json::from_str::<Request>(&s).is_ok() {
            // Skip empty strings and valid requests
            return Ok(());
        }
        
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let handler = create_test_handler();
            
            // Send invalid request
            let response_str = handler.handle_request(s, None).await;
            let response: Response = serde_json::from_str(&response_str).unwrap();
            
            // Verify error response
            prop_assert_eq!(response.jsonrpc, "2.0");
            prop_assert_eq!(response.id, Id::Null);
            prop_assert!(response.result.is_none());
            prop_assert!(response.error.is_some());
            
            // Error code should be parse error or invalid request
            let code = response.error.unwrap().code;
            prop_assert!(code == ErrorCode::ParseError as i32 || 
                        code == ErrorCode::InvalidRequest as i32);
        });
    }
}

// Test error responses from handler methods
proptest! {
    #[test]
    fn test_method_errors(code in -32099i64..-32000) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let handler = create_test_handler();
            
            let request = json!({
                "jsonrpc": "2.0",
                "method": "error",
                "params": { "code": code },
                "id": 1
            });
            
            // Serialize and send request
            let request_str = serde_json::to_string(&request).unwrap();
            let response_str = handler.handle_request(request_str, None).await;
            let response: Response = serde_json::from_str(&response_str).unwrap();
            
            // Verify error response
            prop_assert_eq!(response.jsonrpc, "2.0");
            prop_assert_eq!(response.id, Id::Number(1));
            prop_assert!(response.result.is_none());
            prop_assert!(response.error.is_some());
            
            // Error code should match requested code
            prop_assert_eq!(response.error.unwrap().code, code);
        });
    }
}

// Test method not found error
proptest! {
    #[test]
    fn test_method_not_found(
        method in "[a-zA-Z][a-zA-Z0-9_]{1,20}".prop_map(String::from)
            .prop_filter("Method must not exist", |m| m != "echo" && m != "add" && m != "error")
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let handler = create_test_handler();
            
            let request = json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": null,
                "id": 1
            });
            
            // Serialize and send request
            let request_str = serde_json::to_string(&request).unwrap();
            let response_str = handler.handle_request(request_str, None).await;
            let response: Response = serde_json::from_str(&response_str).unwrap();
            
            // Verify error response
            prop_assert_eq!(response.jsonrpc, "2.0");
            prop_assert_eq!(response.id, Id::Number(1));
            prop_assert!(response.result.is_none());
            prop_assert!(response.error.is_some());
            
            // Error code should be method not found
            prop_assert_eq!(response.error.unwrap().code, ErrorCode::MethodNotFound as i32);
        });
    }
}

// Test context handling
proptest! {
    #[test]
    fn test_context_handling(key in "[a-z]{1,10}", value in "[a-zA-Z0-9]{1,20}") {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut handler = create_test_handler();
            
            // Add a context handler method
            handler.register_method("get_context", move |_, ctx| {
                async move {
                    let val = ctx.metadata.get(&key).cloned().unwrap_or_default();
                    Ok(json!({ key: val }))
                }
            });
            
            // Create a context with the test value
            let mut metadata = HashMap::new();
            metadata.insert(key.clone(), value.clone());
            let context = MethodContext { metadata };
            
            // Create and send request
            let request = json!({
                "jsonrpc": "2.0",
                "method": "get_context",
                "id": 1
            });
            
            let request_str = serde_json::to_string(&request).unwrap();
            let response_str = handler.handle_request(request_str, Some(context)).await;
            let response: Response = serde_json::from_str(&response_str).unwrap();
            
            // Verify response contains context value
            prop_assert!(response.result.is_some());
            let result = response.result.unwrap();
            prop_assert!(result.is_object());
            let obj = result.as_object().unwrap();
            prop_assert!(obj.contains_key(&key));
            prop_assert_eq!(obj[&key].as_str().unwrap(), value);
        });
    }
}
