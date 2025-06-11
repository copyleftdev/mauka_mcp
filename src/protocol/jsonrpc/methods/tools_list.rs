// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Implementation of the JSON-RPC "tools/list" method handler.
//!
//! This handler returns a list of available tools and their capabilities,
//! allowing clients to discover what functionality is available.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::protocol::jsonrpc::error::{ErrorCode, JsonRpcError};
use crate::protocol::jsonrpc::handler::{JsonRpcHandler, MethodContext, MethodResult};

/// Request parameters for the tools/list method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListParams {
    /// Optional filter by tool category
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    
    /// Optional filter by tool capabilities
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
    
    /// Whether to include detailed descriptions
    #[serde(default)]
    pub include_details: bool,
}

/// Represents a parameter for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    
    /// Parameter type
    pub param_type: String,
    
    /// Whether the parameter is required
    pub required: bool,
    
    /// Parameter description
    pub description: String,
    
    /// Default value if any
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    
    /// Parameter constraints
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub constraints: HashMap<String, Value>,
}

/// Represents a tool that can be invoked by clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique tool identifier
    pub id: String,
    
    /// Tool name
    pub name: String,
    
    /// Tool version
    pub version: String,
    
    /// Tool category
    pub category: String,
    
    /// Brief description
    pub description: String,
    
    /// Detailed description (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detailed_description: Option<String>,
    
    /// Tool parameters
    pub parameters: Vec<ToolParameter>,
    
    /// Tool capabilities
    pub capabilities: Vec<String>,
    
    /// Additional tool metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

/// Response for the tools/list method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    /// List of available tools
    pub tools: Vec<Tool>,
    
    /// Total number of tools available (for pagination)
    pub total_count: usize,
}

/// Registers the tools/list method handler with the JSON-RPC handler.
pub fn register_tools_list_method(handler: &mut JsonRpcHandler) {
    handler.register_method("tools/list", handle_tools_list);
}

/// Handles the tools/list method call.
///
/// Returns a list of available tools and their capabilities.
async fn handle_tools_list(params: Option<Value>, _context: MethodContext) -> MethodResult {
    // Parse parameters
    let params = match params {
        Some(params) => match serde_json::from_value::<ToolsListParams>(params) {
            Ok(params) => params,
            Err(err) => {
                return Err(JsonRpcError::new(
                    ErrorCode::InvalidParams,
                    format!("Invalid tools/list parameters: {}", err),
                ))
            }
        },
        None => ToolsListParams {
            category: None,
            capabilities: None,
            include_details: false,
        },
    };

    // Define available tools (in a real implementation, this would come from a registry)
    let mut tools = get_available_tools(params.include_details);
    
    // Apply category filter if specified
    if let Some(category) = params.category {
        tools.retain(|tool| tool.category == category);
    }
    
    // Apply capabilities filter if specified
    if let Some(capabilities) = params.capabilities {
        tools.retain(|tool| {
            capabilities.iter().all(|cap| tool.capabilities.contains(cap))
        });
    }
    
    // Create the result
    let result = ToolsListResult {
        total_count: tools.len(),
        tools,
    };
    
    // Return success response
    Ok(serde_json::to_value(result).unwrap_or_else(|_| Value::Null))
}

/// Returns a list of available tools.
///
/// In a real implementation, this would query a tool registry.
/// This is a mock implementation for demonstration purposes.
fn get_available_tools(include_details: bool) -> Vec<Tool> {
    vec![
        Tool {
            id: "browser_navigate".to_string(),
            name: "Browser Navigate".to_string(),
            version: "1.0.0".to_string(),
            category: "browser".to_string(),
            description: "Navigate to a URL in the browser".to_string(),
            detailed_description: if include_details {
                Some("Allows the client to navigate to a specified URL in a browser session. Supports HTTP and HTTPS protocols.".to_string())
            } else {
                None
            },
            parameters: vec![
                ToolParameter {
                    name: "url".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "The URL to navigate to".to_string(),
                    default_value: None,
                    constraints: HashMap::new(),
                }
            ],
            capabilities: vec!["browser".to_string(), "network".to_string()],
            metadata: HashMap::new(),
        },
        Tool {
            id: "browser_screenshot".to_string(),
            name: "Browser Screenshot".to_string(),
            version: "1.0.0".to_string(),
            category: "browser".to_string(),
            description: "Take a screenshot of the current browser page".to_string(),
            detailed_description: if include_details {
                Some("Captures the current state of the browser viewport or a specific element as an image. Supports various output formats.".to_string())
            } else {
                None
            },
            parameters: vec![
                ToolParameter {
                    name: "format".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Output format (png, jpeg)".to_string(),
                    default_value: Some(Value::String("png".to_string())),
                    constraints: {
                        let mut constraints = HashMap::new();
                        constraints.insert("enum".to_string(), serde_json::json!(["png", "jpeg"]));
                        constraints
                    },
                },
                ToolParameter {
                    name: "selector".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "CSS selector for capturing specific element".to_string(),
                    default_value: None,
                    constraints: HashMap::new(),
                }
            ],
            capabilities: vec!["browser".to_string(), "media".to_string()],
            metadata: HashMap::new(),
        },
        Tool {
            id: "code_completion".to_string(),
            name: "Code Completion".to_string(),
            version: "1.0.0".to_string(),
            category: "code".to_string(),
            description: "Provides code completion suggestions".to_string(),
            detailed_description: if include_details {
                Some("Generates contextually relevant code completion suggestions based on the current code context and user input.".to_string())
            } else {
                None
            },
            parameters: vec![
                ToolParameter {
                    name: "code".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Current code context".to_string(),
                    default_value: None,
                    constraints: HashMap::new(),
                },
                ToolParameter {
                    name: "language".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Programming language".to_string(),
                    default_value: None,
                    constraints: HashMap::new(),
                },
                ToolParameter {
                    name: "max_results".to_string(),
                    param_type: "integer".to_string(),
                    required: false,
                    description: "Maximum number of suggestions to return".to_string(),
                    default_value: Some(Value::Number(5.into())),
                    constraints: {
                        let mut constraints = HashMap::new();
                        constraints.insert("minimum".to_string(), serde_json::json!(1));
                        constraints.insert("maximum".to_string(), serde_json::json!(20));
                        constraints
                    },
                }
            ],
            capabilities: vec!["code".to_string(), "completion".to_string()],
            metadata: HashMap::new(),
        }
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_tools_list_no_filters() {
        // Call handler with no params
        let context = MethodContext::default();
        let result = handle_tools_list(None, context).await.unwrap();
        
        // Parse result
        let result: ToolsListResult = serde_json::from_value(result).unwrap();
        
        // Should have all tools
        assert_eq!(result.tools.len(), 3);
        assert_eq!(result.total_count, 3);
        
        // Check that tool details are not included
        for tool in &result.tools {
            assert!(tool.detailed_description.is_none());
        }
    }
    
    #[tokio::test]
    async fn test_tools_list_with_category_filter() {
        // Filter by browser category
        let params = ToolsListParams {
            category: Some("browser".to_string()),
            capabilities: None,
            include_details: false,
        };
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_tools_list(Some(serde_json::to_value(params).unwrap()), context)
            .await
            .unwrap();
        
        // Parse result
        let result: ToolsListResult = serde_json::from_value(result).unwrap();
        
        // Should have only browser tools
        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.total_count, 2);
        for tool in &result.tools {
            assert_eq!(tool.category, "browser");
        }
    }
    
    #[tokio::test]
    async fn test_tools_list_with_capabilities_filter() {
        // Filter by media capability
        let params = ToolsListParams {
            category: None,
            capabilities: Some(vec!["media".to_string()]),
            include_details: false,
        };
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_tools_list(Some(serde_json::to_value(params).unwrap()), context)
            .await
            .unwrap();
        
        // Parse result
        let result: ToolsListResult = serde_json::from_value(result).unwrap();
        
        // Should have only tools with media capability
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.total_count, 1);
        assert_eq!(result.tools[0].id, "browser_screenshot");
    }
    
    #[tokio::test]
    async fn test_tools_list_with_include_details() {
        // Include detailed descriptions
        let params = ToolsListParams {
            category: None,
            capabilities: None,
            include_details: true,
        };
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_tools_list(Some(serde_json::to_value(params).unwrap()), context)
            .await
            .unwrap();
        
        // Parse result
        let result: ToolsListResult = serde_json::from_value(result).unwrap();
        
        // All tools should have detailed descriptions
        for tool in &result.tools {
            assert!(tool.detailed_description.is_some());
        }
    }
    
    #[tokio::test]
    async fn test_tools_list_with_combined_filters() {
        // Combine category and capabilities filters
        let params = ToolsListParams {
            category: Some("browser".to_string()),
            capabilities: Some(vec!["media".to_string()]),
            include_details: false,
        };
        
        // Call handler
        let context = MethodContext::default();
        let result = handle_tools_list(Some(serde_json::to_value(params).unwrap()), context)
            .await
            .unwrap();
        
        // Parse result
        let result: ToolsListResult = serde_json::from_value(result).unwrap();
        
        // Should have only browser tools with media capability
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.total_count, 1);
        assert_eq!(result.tools[0].id, "browser_screenshot");
    }
}
