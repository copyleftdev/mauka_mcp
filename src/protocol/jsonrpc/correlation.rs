// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! JSON-RPC 2.0 request/response correlation.
//!
//! This module provides mechanisms to correlate JSON-RPC requests with their responses,
//! including timeout handling and correlation error detection.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;

use super::error::{Error, ErrorCode, JsonRpcError, Result};
use super::types::{Id, Request, Response};

/// Default timeout for waiting for a response.
const DEFAULT_TIMEOUT_MS: u64 = 30000; // 30 seconds

/// Error indicating a correlation issue.
#[derive(Debug, thiserror::Error)]
pub enum CorrelationError {
    /// No response was received within the timeout period.
    #[error("Request timed out waiting for response")]
    Timeout,
    
    /// The response channel was closed before a response was received.
    #[error("Response channel closed")]
    ChannelClosed,
    
    /// The request was canceled.
    #[error("Request was canceled")]
    Canceled,
}

/// A pending request awaiting correlation with a response.
#[derive(Debug)]
struct PendingRequest {
    /// The time when the request was sent.
    timestamp: Instant,
    
    /// The sender half of the oneshot channel for this request.
    response_sender: oneshot::Sender<Response>,
    
    /// Optional timeout duration for this request.
    timeout_duration: Duration,
}

/// Manages correlation between JSON-RPC requests and responses.
///
/// The correlator maintains a registry of pending requests and their IDs,
/// allowing responses to be matched to their originating requests.
#[derive(Debug, Clone)]
pub struct RequestResponseCorrelator {
    /// Map of request IDs to pending request data.
    pending_requests: Arc<RwLock<HashMap<Id, PendingRequest>>>,
    
    /// Next sequential ID to use for requests that don't specify an ID.
    next_id: Arc<Mutex<i64>>,
    
    /// Default timeout duration for requests.
    default_timeout: Duration,
}

impl Default for RequestResponseCorrelator {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestResponseCorrelator {
    /// Creates a new request/response correlator with default settings.
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            default_timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        }
    }
    
    /// Sets the default timeout for all requests.
    pub fn with_default_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout = Duration::from_millis(timeout_ms);
        self
    }
    
    /// Assigns a new ID to a request if it doesn't already have one or has a null ID.
    ///
    /// This method ensures that every request has a unique ID for correlation purposes.
    /// If the request is a notification (no ID), it will remain a notification.
    pub fn prepare_request(&self, mut request: Request) -> Request {
        // Notifications don't need IDs as they don't expect responses
        if request.is_notification() {
            return request;
        }
        
        // If the request already has a non-null ID, use it
        if let Some(id) = &request.id {
            if !matches!(id, Id::Null) {
                return request;
            }
            // Otherwise, we'll replace the null ID with a new one
        }
        
        // Assign a new sequential ID
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        request.id = Some(Id::Number(id));
        request
    }
    /// Registers a request for correlation with its future response.
    ///
    /// Returns a future that will resolve when the corresponding response is received,
    /// or when the request times out.
    pub async fn register_request(
        &self, 
        request: &Request,
        timeout_ms: Option<u64>,
    ) -> Result<oneshot::Receiver<Response>> {
        // Skip notifications (they don't expect responses)
        if request.is_notification() {
            return Err(Error::from(JsonRpcError::new(
                ErrorCode::InvalidRequest,
                "Cannot register notification for response correlation",
            )));
        }
        
        // Ensure request has an ID
        let id = match &request.id {
            Some(id) => id.clone(),
            None => return Err(Error::from(JsonRpcError::new(
                ErrorCode::InvalidRequest,
                "Request must have an ID for correlation",
            ))),
        };
        
        // Create channel for response delivery
        let (tx, rx) = oneshot::channel();
        
        // Create pending request entry
        let timeout_duration = timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(self.default_timeout);
            
        let pending_request = PendingRequest {
            timestamp: Instant::now(),
            response_sender: tx,
            timeout_duration,
        };
        
        // Store pending request
        let mut pending_requests = self.pending_requests.write().await;
        pending_requests.insert(id, pending_request);
        
        Ok(rx)
    }
    
    /// Correlates a response with its original request.
    ///
    /// Returns `true` if the response was successfully correlated with a pending request,
    /// or `false` if no matching request was found.
    pub async fn correlate_response(&self, response: Response) -> bool {
        let id = response.id.clone();
        
        // Find and remove the pending request
        let response_sender = {
            let mut pending_requests = self.pending_requests.write().await;
            pending_requests.remove(&id).map(|req| req.response_sender)
        };
        
        // Send response if we found a matching request
        if let Some(sender) = response_sender {
            // It's okay if the receiver has been dropped
            let _ = sender.send(response);
            true
        } else {
            false
        }
    }
    
    /// Waits for a response to a specific request with timeout handling.
    ///
    /// Convenience method that registers a request and waits for its response,
    /// handling timeout errors automatically.
    pub async fn send_request_and_wait(
        &self, 
        request: &Request,
        timeout_ms: Option<u64>,
    ) -> Result<Response> {
        // Register the request
        let rx = self.register_request(request, timeout_ms).await?;
        
        // Determine timeout duration
        let timeout_duration = timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(self.default_timeout);
        
        // Wait for response with timeout
        match timeout(timeout_duration, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(Error::from(JsonRpcError::new(
                ErrorCode::InternalError,
                "Response channel closed unexpectedly",
            ))),
            Err(_) => {
                // Timeout occurred, clean up the pending request
                let id = request.id.as_ref().unwrap().clone();
                let mut pending_requests = self.pending_requests.write().await;
                pending_requests.remove(&id);
                
                Err(Error::from(JsonRpcError::new(
                    ErrorCode::InternalError,
                    "Request timed out waiting for response",
                )))
            }
        }
    }
    
    /// Cancels a pending request, causing its future to resolve with a cancellation error.
    pub async fn cancel_request(&self, id: &Id) -> bool {
        let mut pending_requests = self.pending_requests.write().await;
        if let Some(request) = pending_requests.remove(id) {
            let _ = request.response_sender.send(Response::error(
                id.clone(), 
                JsonRpcError::new(
                    ErrorCode::RequestCancelled,
                    "Request was canceled",
                ),
            ));
            true
        } else {
            false
        }
    }
    
    /// Cleans up timed-out requests from the pending requests map.
    ///
    /// Returns the number of requests that were cleaned up.
    pub async fn cleanup_timed_out_requests(&self) -> usize {
        let mut pending_requests = self.pending_requests.write().await;
        let now = Instant::now();
        
        let timed_out: Vec<Id> = pending_requests
            .iter()
            .filter_map(|(id, req)| {
                if now.duration_since(req.timestamp) > req.timeout_duration {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        
        for id in &timed_out {
            if let Some(req) = pending_requests.remove(id) {
                let _ = req.response_sender.send(Response::error(
                    id.clone(),
                    JsonRpcError::new(
                        ErrorCode::RequestCancelled,
                        "Request timed out",
                    ),
                ));
            }
        }
        
        timed_out.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_request_preparation() {
        let correlator = RequestResponseCorrelator::new();
        
        // Request with existing ID should be unchanged
        let request = Request::with_string_id("test", None, "existing-id");
        let prepared = correlator.prepare_request(request.clone());
        assert_eq!(prepared.id, request.id);
        
        // Request without ID should get a new ID
        let mut request = Request::new("test", None, None);
        request.id = Some(Id::Null); 
        let prepared = correlator.prepare_request(request);
        assert!(prepared.id.is_some());
        assert!(!matches!(prepared.id, Some(Id::Null))); 
        
        // Notification should remain a notification
        let notification = Request::notification("test", None);
        let prepared = correlator.prepare_request(notification);
        assert!(prepared.is_notification());
    }
    
    #[tokio::test]
    async fn test_correlation_success() {
        let correlator = RequestResponseCorrelator::new();
        
        // Create and register a request
        let request = Request::with_number_id("test", None, 42);
        let rx = correlator.register_request(&request, None).await.unwrap();
        
        // Create a matching response
        let response = Response::success(Id::Number(42), json!({"result": "success"}));
        
        // Correlate the response
        let success = correlator.correlate_response(response.clone()).await;
        assert!(success);
        
        // Verify we received the correct response
        let received = rx.await.unwrap();
        assert_eq!(received.id, response.id);
    }
    
    #[tokio::test]
    async fn test_correlation_timeout() {
        let correlator = RequestResponseCorrelator::with_default_timeout(
            RequestResponseCorrelator::new(), 
            100 // very short timeout
        );
        
        // Create and register a request
        let request = Request::with_number_id("test", None, 42);
        
        // Wait for the response with a very short timeout
        let result = correlator.send_request_and_wait(&request, Some(100)).await;
        
        // Should have timed out
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_cancel_request() {
        let correlator = RequestResponseCorrelator::new();
        
        // Create and register a request
        let request = Request::with_number_id("test", None, 42);
        let rx = correlator.register_request(&request, None).await.unwrap();
        
        // Cancel the request
        let canceled = correlator.cancel_request(&Id::Number(42)).await;
        assert!(canceled);
        
        // Verify we received a cancellation error
        let response = rx.await.unwrap();
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, ErrorCode::RequestCancelled as i32);
    }
    
    #[tokio::test]
    async fn test_cleanup_timed_out_requests() {
        let correlator = RequestResponseCorrelator::new();
        
        // Create and register a request with very short timeout
        let request = Request::with_number_id("test", None, 42);
        let rx = correlator.register_request(&request, Some(1)).await.unwrap();
        
        // Sleep to ensure it times out
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Clean up timed-out requests
        let cleaned_up = correlator.cleanup_timed_out_requests().await;
        assert_eq!(cleaned_up, 1);
        
        // Verify we received a timeout error
        let response = rx.await.unwrap();
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, ErrorCode::RequestCancelled as i32);
    }
}
