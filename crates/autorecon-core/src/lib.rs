//! AutoRecon Core Library
//!
//! This is the platform-agnostic core library for AutoRecon, a network reconnaissance
//! automation tool. It provides the business logic for scanning, service detection,
//! and pattern matching without any platform-specific I/O dependencies.
//!
//! The library uses trait-based abstractions to allow different execution environments:
//! - Native CLI (using tokio process execution)
//! - WebAssembly (using JavaScript callbacks)
//! - Python (using PyO3 callbacks)

pub mod config;
pub mod executor;
pub mod output;
pub mod patterns;
pub mod scanner;
pub mod services;

use thiserror::Error;

/// Result type alias for AutoRecon operations
pub type Result<T> = std::result::Result<T, AutoReconError>;

/// Main error type for AutoRecon
#[derive(Error, Debug)]
pub enum AutoReconError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Command execution error: {0}")]
    Execution(String),

    #[error("Service detection error: {0}")]
    ServiceDetection(String),

    #[error("Pattern matching error: {0}")]
    PatternMatching(String),

    #[error("Invalid target: {0}")]
    InvalidTarget(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Re-export commonly used types
pub use config::{Config, PortScanProfile, ServiceScan};
pub use executor::{CommandExecutor, CommandOutput};
pub use output::OutputHandler;
pub use patterns::{Pattern, PatternMatch};
pub use scanner::{ScanContext, Target};
pub use services::{DetectedService, ServiceDetector};
