//! Output handling abstraction
//!
//! This module defines the trait-based abstraction for output operations,
//! allowing different implementations for native, WASM, and Python environments.

use async_trait::async_trait;
use std::path::Path;

use crate::Result;

/// Trait for handling output operations (filesystem, logging, etc.)
///
/// This trait abstracts all I/O operations, allowing the core library to be
/// platform-agnostic. Different implementations can be provided for:
/// - Native environments (direct filesystem access)
/// - WASM environments (JavaScript callbacks for file operations)
/// - Python environments (Python I/O callbacks)
#[async_trait]
pub trait OutputHandler: Send + Sync {
    /// Create a directory (including parent directories)
    ///
    /// # Arguments
    /// * `path` - Path to the directory to create
    async fn create_directory(&self, path: &Path) -> Result<()>;

    /// Write content to a file (overwrite if exists)
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - Content to write
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;

    /// Append content to a file (create if doesn't exist)
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - Content to append
    async fn append_file(&self, path: &Path, content: &str) -> Result<()>;

    /// Read content from a file
    ///
    /// # Arguments
    /// * `path` - Path to the file to read
    ///
    /// # Returns
    /// The content of the file as a string
    async fn read_file(&self, path: &Path) -> Result<String>;

    /// Check if a file exists
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the file exists, false otherwise
    async fn file_exists(&self, path: &Path) -> bool;

    /// Check if a directory exists
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the directory exists, false otherwise
    async fn directory_exists(&self, path: &Path) -> bool;
}

/// Log level for output messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace-level debugging information
    Trace,
    /// Debug-level information
    Debug,
    /// Informational messages
    Info,
    /// Warning messages
    Warn,
    /// Error messages
    Error,
}

impl LogLevel {
    /// Convert log level to string
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Extension trait for formatted logging
#[async_trait]
pub trait LogHandler: OutputHandler {
    /// Log a message to a specific log file
    ///
    /// # Arguments
    /// * `log_path` - Path to the log file
    /// * `level` - Log level
    /// * `message` - Message to log
    async fn log(&self, log_path: &Path, level: LogLevel, message: &str) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let formatted = format!("[{}] [{}] {}\n", timestamp, level.as_str(), message);
        self.append_file(log_path, &formatted).await
    }
}

// Blanket implementation: any OutputHandler is also a LogHandler
impl<T: OutputHandler> LogHandler for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }
}
