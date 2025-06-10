//! Tests for the configuration module.
//!
//! This module contains tests for configuration loading, validation, and usage.

// We only need fs and tempdir here
use crate::config::{
    cache::CacheConfig, http::HttpConfig, limits::LimitsConfig, security::SecurityConfig,
    server::TransportType, ConfigLoader, MaukaConfig, Validate,
};
use std::fs;
use tempfile::tempdir;

/// Test that default configuration can be created and is valid.
#[test]
fn test_default_config_is_valid() {
    let config = MaukaConfig::default();
    assert!(config.validate().is_ok());
}

/// Test that configuration validation catches invalid values.
#[test]
fn test_config_validation() {
    let mut config = MaukaConfig::default();

    // Invalid server configuration
    config.server.worker_threads = 0;
    assert!(config.validate().is_err());

    // Fix and test another invalid value
    config.server.worker_threads = 4;
    config.http.rate_limiter.initial_rate = -1.0;
    assert!(config.validate().is_err());

    // Fix and test another invalid value
    config.http.rate_limiter.initial_rate = 50.0;
    config.cache.memory.p_value = 1.5;
    assert!(config.validate().is_err());
}

/// Test loading configuration from a file.
#[test]
fn test_load_config_from_file() {
    // Clean environment variables that might affect this test
    std::env::remove_var("TEST_FILE__SERVER__NAME");
    std::env::remove_var("TEST_FILE__HTTP__CLIENT__USER_AGENT");

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config_file_test.toml");

    // Create a minimal valid configuration file
    let config_content = r#"
    [server]
    name = "test-server"
    worker_threads = 2
    
    [http.client]
    user_agent = "Test-Agent/1.0"
    
    [security.tls]
    allowed_ciphers = ["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
    min_version = "TLS1.2"
    
    [security.url_validation]
    disallowed_hosts = ["blocked.example.com"]
    disallowed_host_patterns = [".*\\.evil\\.com$"]
    "#;

    fs::write(&config_path, config_content).unwrap();

    // Load the configuration with a unique prefix
    let loader = ConfigLoader::new(Some(&config_path), "TEST_FILE");
    let config = loader.load().unwrap();

    // Verify values were loaded correctly
    assert_eq!(config.server.name, "test-server");
    assert_eq!(config.server.worker_threads, 2);
    assert_eq!(config.http.client.user_agent, "Test-Agent/1.0");

    // Other values should be defaults
    assert_eq!(config.server.transport, TransportType::Both);
}

/// Test loading configuration with environment variable overrides.
#[test]
fn test_env_var_override() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config_env_test.toml");

    // Create a minimal valid configuration file
    let config_content = r#"
    [server]
    name = "test-server"
    worker_threads = 2
    
    [security.tls]
    allowed_ciphers = ["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
    min_version = "TLS1.2"
    
    [security.url_validation]
    disallowed_hosts = ["blocked.example.com"]
    disallowed_host_patterns = [".*\\.evil\\.com$"]
    "#;

    fs::write(&config_path, config_content).unwrap();

    // Set environment variables with a unique prefix
    std::env::set_var("TEST_ENV__SERVER__NAME", "env-server");
    std::env::set_var("TEST_ENV__HTTP__CLIENT__USER_AGENT", "Env-Agent/1.0");

    // Load the configuration with a unique prefix
    let loader = ConfigLoader::new(Some(&config_path), "TEST_ENV");
    let config = loader.load().unwrap();

    // Verify environment variables took precedence
    assert_eq!(config.server.name, "env-server");
    assert_eq!(config.http.client.user_agent, "Env-Agent/1.0");

    // Clean up environment variables
    std::env::remove_var("TEST_ENV__SERVER__NAME");
    std::env::remove_var("TEST_ENV__HTTP__CLIENT__USER_AGENT");
}

/// Test that loading an invalid configuration file returns an error.
#[test]
fn test_load_invalid_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("invalid.toml");

    // Create an invalid TOML file
    let config_content = r#"
    [server
    name = test-server"
    "#;

    fs::write(&config_path, config_content).unwrap();

    // Try to load the configuration with a unique prefix
    let loader = ConfigLoader::new(Some(&config_path), "TEST_INVALID");
    assert!(loader.load().is_err());
}

/// Test that validation fails for various invalid configurations.
#[test]
fn test_specific_validation_rules() {
    // Test HTTP client validation
    let mut http_config = HttpConfig::default();
    http_config.client.user_agent = String::new();
    assert!(http_config.validate().is_err());

    // Test cache policy validation
    let mut cache_config = CacheConfig::default();
    cache_config.policy.min_size_bytes = 1000;
    cache_config.policy.max_size_bytes = 100;
    assert!(cache_config.validate().is_err());

    // Test security validation
    let mut security_config = SecurityConfig::default();
    security_config.url_validation.allowed_schemes.clear();
    assert!(security_config.validate().is_err());

    // Test limits validation
    let mut limits_config = LimitsConfig::default();
    limits_config.memory.warning_threshold = 1.5;
    assert!(limits_config.validate().is_err());
}
