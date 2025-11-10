//! Command execution abstraction
//!
//! This module defines the trait-based abstraction for command execution,
//! allowing different implementations for native, WASM, and Python environments.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Output from a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Exit code (0 = success)
    pub exit_code: i32,

    /// Duration of execution in milliseconds
    pub duration_ms: u64,
}

impl CommandOutput {
    /// Check if the command was successful (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

/// Trait for executing commands in different environments
///
/// This trait allows the core library to be platform-agnostic by abstracting
/// command execution. Different implementations can be provided for:
/// - Native environments (using tokio::process::Command)
/// - WASM environments (using JavaScript callbacks)
/// - Python environments (using Python subprocess callbacks)
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a shell command
    ///
    /// # Arguments
    /// * `command` - The shell command to execute
    ///
    /// # Returns
    /// The output of the command, including stdout, stderr, and exit code
    async fn execute(&self, command: &str) -> Result<CommandOutput>;

    /// Execute a command with a timeout
    ///
    /// # Arguments
    /// * `command` - The shell command to execute
    /// * `timeout_secs` - Maximum time to wait for completion
    ///
    /// # Returns
    /// The output of the command, or a timeout error
    async fn execute_with_timeout(
        &self,
        command: &str,
        _timeout_secs: u64,
    ) -> Result<CommandOutput> {
        // Default implementation just calls execute
        // Implementations can override for proper timeout handling
        self.execute(command).await
    }

    /// Check if a command/tool is available in the system
    ///
    /// # Arguments
    /// * `tool` - Name of the tool to check (e.g., "nmap")
    ///
    /// # Returns
    /// true if the tool is available, false otherwise
    async fn is_tool_available(&self, tool: &str) -> bool {
        // Default implementation: try to run with --version
        let command = format!("{} --version 2>/dev/null", tool);
        self.execute(&command).await.map(|o| o.is_success()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_output_is_success() {
        let output = CommandOutput {
            stdout: "output".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
            duration_ms: 100,
        };
        assert!(output.is_success());

        let output = CommandOutput {
            stdout: "".to_string(),
            stderr: "error".to_string(),
            exit_code: 1,
            duration_ms: 100,
        };
        assert!(!output.is_success());
    }

    #[test]
    fn test_combined_output() {
        let output = CommandOutput {
            stdout: "stdout content".to_string(),
            stderr: "stderr content".to_string(),
            exit_code: 0,
            duration_ms: 100,
        };
        assert_eq!(output.combined_output(), "stdout content\nstderr content");
    }
}
