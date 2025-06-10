//! Mauka MCP Server - Main entrypoint.
//!
//! This is the main entry point for the Mauka MCP Server application.
//! It initializes the logging system, loads configuration, and starts the server.

mod config;
mod error;
mod protocol;
mod utils;

#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};
use error::{set_error_reporter, MaukaError, MaukaResult, TracingErrorReporter};
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tracing::info;

/// Command line arguments for the Mauka MCP Server.
#[derive(Parser, Debug)]
#[clap(name = "Mauka MCP Server", version, author, about)]
struct Args {
    /// Path to configuration file
    #[clap(short, long, value_parser)]
    config: Option<PathBuf>,

    /// Command to execute
    #[clap(subcommand)]
    command: Option<Command>,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
enum Command {
    /// Start the server
    Start,

    /// Validate the configuration file
    Validate,

    /// Generate a default configuration file
    GenConfig {
        /// Path to output configuration file
        #[clap(short, long, value_parser)]
        output: PathBuf,
    },
}

/// Initialize the logging system.
fn init_logging() -> MaukaResult<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_line_number(true)
        .with_file(true)
        .with_thread_names(true)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| MaukaError::Custom(format!("Failed to set global tracing subscriber: {e}")))
}

/// Main entry point for the application.
fn main() -> MaukaResult<()> {
    // Initialize logging early to capture any startup errors
    init_logging()?;

    // Set up error reporter
    set_error_reporter(Arc::new(TracingErrorReporter));

    // Parse command-line arguments
    let args = <Args as clap::Parser>::parse();

    // Load configuration
    let env_prefix = "MAUKA";
    let config_loader = config::ConfigLoader::new(args.config.as_deref(), env_prefix);

    match args.command.unwrap_or(Command::Start) {
        Command::Start => {
            info!("Starting Mauka MCP Server");

            // Load and validate configuration
            let config = match config_loader.load() {
                Ok(config) => config,
                Err(e) => {
                    tracing::error!("Configuration error: {}", e);
                    process::exit(1);
                }
            };

            // Initialize global configuration
            config::init_global_config(config);

            // Log server startup information
            let config = config::get_global_config().get();
            info!(
                "Server configured with name: {}, transport: {:?}, address: {}",
                config.server.name, config.server.transport, config.server.address
            );

            // TODO: Initialize and start server components
            // This will be implemented in subsequent phases
            info!("Server initialized successfully");

            Ok(())
        }
        Command::Validate => {
            info!("Validating configuration");
            match config_loader.load() {
                Ok(_) => {
                    info!("Configuration validated successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Configuration validation error: {}", e);
                    process::exit(1);
                }
            }
        }
        Command::GenConfig { output } => {
            info!("Generating default configuration");
            let default_config = config::MaukaConfig::default();

            // Create parent directories if they don't exist
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).map_err(MaukaError::Io)?;
            }

            // Serialize to TOML
            let toml = toml::to_string_pretty(&default_config)
                .map_err(|e| MaukaError::Custom(format!("Failed to serialize config: {e}")))?;

            // Write to file
            std::fs::write(&output, toml).map_err(MaukaError::Io)?;

            info!("Default configuration written to {:?}", output);
            Ok(())
        }
    }
}
