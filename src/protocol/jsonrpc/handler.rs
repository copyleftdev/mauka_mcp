// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! JSON-RPC 2.0 handler implementation.
//!
//! This module provides the core handler for JSON-RPC 2.0 requests, supporting
//! method registration, request dispatching, and asynchronous execution.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use serde_json::Value;
use tokio::sync::RwLock;

use super::error::{Error, ErrorCode, JsonRpcError, Result};
use super::types::{BatchRequest, BatchResponse, Id, Request, Response};
use super::validation::{validate_request, ValidatedRequest};

/// A method handler context containing additional information about the request.
#[derive(Debug, Clone, Default)]
pub struct MethodContext {
    /// Optional metadata associated with this request
    pub metadata: HashMap<String, String>,
}

/// Type alias for method handler response.
pub type MethodResult = std::result::Result<Value, JsonRpcError>;

/// Type alias for method handler's future return type.
pub type MethodHandlerFuture = BoxFuture<'static, MethodResult>;

/// Type alias for asynchronous method handler functions.
pub type MethodHandlerFn = Arc<dyn MethodHandler + Send + Sync>;

/// Trait for method handlers to implement.
pub trait MethodHandler {
    /// Handle a method call asynchronously.
    ///
    /// # Parameters
    /// * `params` - The parameters passed to the method.
    /// * `context` - Additional context for the method call.
    ///
    /// # Returns
    /// A boxed future that resolves to a JSON-RPC result.
    fn handle(&self, params: Option<Value>, context: MethodContext) -> MethodHandlerFuture;
}

// Implement MethodHandler for async functions
impl<F, Fut> MethodHandler for F
where
    F: Send + Sync + 'static + Fn(Option<Value>, MethodContext) -> Fut,
    Fut: Future<Output = MethodResult> + Send + 'static,
{
    fn handle(&self, params: Option<Value>, context: MethodContext) -> MethodHandlerFuture {
        Box::pin((self)(params, context))
    }
}

/// Handler for JSON-RPC 2.0 requests.
///
/// This struct is responsible for:
/// - Registering method handlers
/// - Validating incoming requests
/// - Dispatching requests to appropriate handlers
/// - Collecting and formatting responses
///
/// The handler is thread-safe and supports asynchronous method execution.
#[derive(Default)]
pub struct JsonRpcHandler {
    /// Registered method handlers
    methods: Arc<RwLock<HashMap<String, MethodHandlerFn>>>,
    
    /// Optional global context provider
    context_provider: Option<Arc<dyn Fn() -> MethodContext + Send + Sync>>,
}

impl JsonRpcHandler {
    /// Creates a new JSON-RPC handler.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Registers a method handler function.
    pub fn register_method<F, Fut>(&mut self, method: impl Into<String>, handler: F)
    where
        F: Send + Sync + 'static + Fn(Option<Value>, MethodContext) -> Fut,
        Fut: Future<Output = MethodResult> + Send + 'static,
    {
        let method_name = method.into();
        let handler_fn = Arc::new(handler) as MethodHandlerFn;
        
        tokio::task::block_in_place(|| {
            futures::executor::block_on(async {
                let mut methods = self.methods.write().await;
                methods.insert(method_name, handler_fn);
            })
        });
    }
    
    /// Registers a context provider function that is called for each request.
    ///
    /// The context provider allows injecting context information into all method calls.
    pub fn register_context_provider<F>(&mut self, provider: F)
    where
        F: Fn() -> MethodContext + Send + Sync + 'static,
    {
        self.context_provider = Some(Arc::new(provider));
    }
    
    /// Handles a JSON-RPC request string.
    ///
    /// This method parses the request, validates it, dispatches it to the appropriate
    /// handler, and returns a properly formatted JSON-RPC response.
    ///
    /// # Parameters
    /// * `request_str` - The JSON-RPC request string to handle.
    /// * `context` - Optional context to pass to method handlers.
    ///
    /// # Returns
    /// A JSON string containing the JSON-RPC response.
    pub async fn handle_request(
        &self,
        request_str: impl AsRef<str>,
        context: Option<MethodContext>,
    ) -> String {
        // Validate request
        let validated = match validate_request(request_str) {
            Ok(req) => req,
            Err(err) => {
                return match err {
                    Error::Json(_) => {
                        // Parse error - could not parse the JSON
                        let error = JsonRpcError::parse_error();
                        let response = Response {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(error),
                            id: Id::Null,
                        };
                        
                        serde_json::to_string(&response).unwrap_or_else(|_| {
                            r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32700,\"message\":\"Parse error\"},\"id\":null}"#.to_string()
                        })
                    },
                    Error::JsonRpc(msg) => {
                        // Invalid request format
                        let error = JsonRpcError::invalid_request();
                        let response = Response {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(error),
                            id: Id::Null,
                        };
                        
                        serde_json::to_string(&response).unwrap_or_else(|_| {
                            r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Invalid Request\"},\"id\":null}"#.to_string()
                        })
                    },
                    _ => {
                        // Other errors
                        let error = JsonRpcError::internal_error(err.to_string());
                        let response = Response {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(error),
                            id: Id::Null,
                        };
                        
                        serde_json::to_string(&response).unwrap_or_else(|_| {
                            r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"Internal error\"},\"id\":null}"#.to_string()
                        })
                    },
                };
            }
        };
        
        // Get the context for this request
        let ctx = match context {
            Some(c) => c,
            None => match &self.context_provider {
                Some(provider) => (provider)(),
                None => MethodContext::default(),
            },
        };
        
        // Dispatch request(s) to handler(s)
        match validated {
            ValidatedRequest::Single(request) => {
                let response = self.handle_single_request(request, ctx).await;
                serde_json::to_string(&response).unwrap_or_else(|_| {
                    // Error serializing response - should never happen with valid response
                    r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"Internal error: Error serializing response\"},\"id\":null}"#.to_string()
                })
            },
            ValidatedRequest::Batch(batch) => {
                let responses = self.handle_batch_request(batch, ctx).await;
                
                // Empty response array is invalid according to the spec
                if responses.responses.is_empty() {
                    return r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Invalid Request: Batch request contained only notifications\"},\"id\":null}"#.to_string();
                }
                
                serde_json::to_string(&responses.responses).unwrap_or_else(|_| {
                    r#"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"Internal error: Error serializing response\"},\"id\":null}"#.to_string()
                })
            },
        }
    }
    
    /// Handles a single JSON-RPC request.
    async fn handle_single_request(&self, request: Request, context: MethodContext) -> Response {
        // For notifications (no ID), we still process but return no response
        let id = match request.id {
            Some(id) => id,
            None => {
                // It's a notification, process it but return no response
                self.process_method_call(&request.method, request.params.clone(), context.clone()).await;
                return Response {
                    jsonrpc: "2.0".to_string(),
                    result: None, 
                    error: None,
                    id: Id::Null, // This should actually never be returned in a real scenario
                };
            }
        };
        
        // Check if method exists
        let methods = self.methods.read().await;
        if !methods.contains_key(&request.method) {
            return Response::error(
                id,
                JsonRpcError::method_not_found(&request.method),
            );
        }
        
        // Process method call
        match self.process_method_call(&request.method, request.params, context).await {
            Ok(result) => Response::success(id, result),
            Err(error) => Response::error(id, error),
        }
    }
    
    /// Handles a batch of JSON-RPC requests.
    async fn handle_batch_request(
        &self,
        batch: BatchRequest,
        context: MethodContext,
    ) -> BatchResponse {
        let mut responses = Vec::with_capacity(batch.requests.len());
        let mut futures = Vec::with_capacity(batch.requests.len());
        
        // Create futures for all requests with IDs
        for request in batch.requests.iter() {
            // Skip notifications (we process them but don't include in responses)
            if request.id.is_some() {
                let req_clone = request.clone();
                let ctx_clone = context.clone();
                let this = self.clone();
                
                futures.push(async move {
                    this.handle_single_request(req_clone, ctx_clone).await
                });
            } else {
                // Process notification in background
                let method = request.method.clone();
                let params = request.params.clone();
                let ctx_clone = context.clone();
                let this = self.clone();
                
                tokio::spawn(async move {
                    this.process_method_call(&method, params, ctx_clone).await;
                });
            }
        }
        
        // Wait for all futures to complete
        if !futures.is_empty() {
            responses = futures::future::join_all(futures).await;
        }
        
        BatchResponse { responses }
    }
    
    /// Processes a method call by dispatching it to the registered handler.
    async fn process_method_call(
        &self,
        method: &str,
        params: Option<Value>,
        context: MethodContext,
    ) -> MethodResult {
        let methods = self.methods.read().await;
        
        // Get method handler
        let handler = match methods.get(method) {
            Some(h) => h.clone(),
            None => return Err(JsonRpcError::method_not_found(method)),
        };
        
        // Call handler and return result
        handler.handle(params, context).await
    }
}

// Clone implementation for JsonRpcHandler
impl Clone for JsonRpcHandler {
    fn clone(&self) -> Self {
        Self {
            methods: self.methods.clone(),
            context_provider: self.context_provider.clone(),
        }
    }
}
