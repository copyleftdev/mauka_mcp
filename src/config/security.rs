//! Security configuration module.
//!
//! This module defines configuration for security features such as TLS,
//! URL validation, robots.txt compliance, and content security policy.

use super::{ConfigResult, Validate};
use crate::error::config::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SecurityConfig {
    /// TLS configuration
    pub tls: TlsConfig,

    /// URL validation configuration
    pub url_validation: UrlValidationConfig,

    /// Robots.txt compliance configuration
    pub robots: RobotsConfig,

    /// Content Security Policy configuration
    pub content_security: ContentSecurityConfig,
}


impl Validate for SecurityConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.tls.validate()?;
        self.url_validation.validate()?;
        self.robots.validate()?;
        self.content_security.validate()?;
        Ok(())
    }
}

/// TLS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Whether to verify TLS certificates
    pub verify_certificates: bool,

    /// Minimum TLS version to accept
    pub min_tls_version: String,

    /// Path to CA certificates file
    pub ca_file: Option<PathBuf>,

    /// Path to client certificate file
    pub client_cert_file: Option<PathBuf>,

    /// Path to client key file
    pub client_key_file: Option<PathBuf>,

    /// Whether to enable ALPN for HTTP/2
    pub enable_alpn: bool,

    /// List of allowed cipher suites (empty means use defaults)
    pub allowed_ciphers: Vec<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_certificates: true,
            min_tls_version: "1.2".to_string(),
            ca_file: None,
            client_cert_file: None,
            client_key_file: None,
            enable_alpn: true,
            allowed_ciphers: Vec::new(),
        }
    }
}

impl Validate for TlsConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate min_tls_version
        match self.min_tls_version.as_str() {
            "1.0" | "1.1" | "1.2" | "1.3" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid min_tls_version: {}",
                    self.min_tls_version
                )))
            }
        }

        // Validate client certificate and key files
        if (self.client_cert_file.is_some() && self.client_key_file.is_none())
            || (self.client_cert_file.is_none() && self.client_key_file.is_some())
        {
            return Err(ConfigError::ValidationError(
                "Both client_cert_file and client_key_file must be specified if one is provided"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

/// URL validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlValidationConfig {
    /// Maximum URL length in characters
    pub max_url_length: usize,

    /// Allowed URL schemes
    pub allowed_schemes: HashSet<String>,

    /// Disallowed hosts (exact matches)
    pub disallowed_hosts: HashSet<String>,

    /// Disallowed host patterns (regex)
    pub disallowed_host_patterns: Vec<String>,

    /// Whether to block private IP addresses
    pub block_private_ips: bool,

    /// Whether to block loopback addresses
    pub block_loopback: bool,

    /// Whether to validate URLs at request time
    pub validate_on_request: bool,
}

impl Default for UrlValidationConfig {
    fn default() -> Self {
        let mut allowed_schemes = HashSet::new();
        allowed_schemes.insert("http".to_string());
        allowed_schemes.insert("https".to_string());

        Self {
            max_url_length: 2048,
            allowed_schemes,
            disallowed_hosts: HashSet::new(),
            disallowed_host_patterns: Vec::new(),
            block_private_ips: true,
            block_loopback: true,
            validate_on_request: true,
        }
    }
}

impl Validate for UrlValidationConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_url_length
        if self.max_url_length == 0 {
            return Err(ConfigError::ValidationError(
                "max_url_length must be greater than 0".to_string(),
            ));
        }

        // Validate allowed_schemes
        if self.allowed_schemes.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one scheme must be allowed".to_string(),
            ));
        }

        Ok(())
    }
}

/// Robots.txt compliance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotsConfig {
    /// Whether to respect robots.txt rules
    pub respect_robots: bool,

    /// User agent to use when fetching robots.txt
    pub user_agent: String,

    /// Cache TTL for robots.txt content in seconds
    pub cache_ttl_sec: u64,
}

impl Default for RobotsConfig {
    fn default() -> Self {
        Self {
            respect_robots: true,
            user_agent: "Mauka-MCP/1.0".to_string(),
            cache_ttl_sec: 3600, // 1 hour
        }
    }
}

impl Validate for RobotsConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate user_agent
        if self.respect_robots && self.user_agent.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "user_agent cannot be empty when respect_robots is true".to_string(),
            ));
        }

        // Validate cache_ttl_sec
        if self.cache_ttl_sec == 0 {
            return Err(ConfigError::ValidationError(
                "cache_ttl_sec must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Content Security Policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSecurityConfig {
    /// Whether to validate Content-Security-Policy headers
    pub validate_csp: bool,

    /// Whether to validate the referrer policy
    pub validate_referrer: bool,

    /// Whether to validate X-Content-Type-Options
    pub validate_content_type_options: bool,

    /// Whether to validate X-Frame-Options
    pub validate_frame_options: bool,
}

impl Default for ContentSecurityConfig {
    fn default() -> Self {
        Self {
            validate_csp: true,
            validate_referrer: true,
            validate_content_type_options: true,
            validate_frame_options: true,
        }
    }
}

impl Validate for ContentSecurityConfig {
    fn validate(&self) -> ConfigResult<()> {
        // No specific validation needed for boolean flags
        Ok(())
    }
}
