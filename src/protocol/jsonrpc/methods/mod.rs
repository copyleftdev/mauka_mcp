// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! JSON-RPC 2.0 method handlers.
//!
//! This module contains implementations of standard and custom method handlers
//! for the JSON-RPC 2.0 protocol used by Mauka MCP.

pub mod initialize;
pub mod tools_list;

// Re-exports
pub use initialize::register_initialize_method;
pub use tools_list::register_tools_list_method;
