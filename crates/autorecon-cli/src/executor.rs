//! Native command executor using tokio process spawning

use autorecon_core::{
    executor::{CommandExecutor, CommandOutput},
    Result,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Semaphore;

/// Native executor that spawns system processes using tokio
#[derive(Clone)]
pub struct NativeExecutor {
    /// Semaphore to limit concurrent command executions
    semaphore: Arc<Semaphore>,
}

impl NativeExecutor {
    /// Create a new native executor
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent command executions
    pub fn new(max_concurrent: usize) -> Self {
        NativeExecutor {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
}

#[async_trait]
impl CommandExecutor for NativeExecutor {
    async fn execute(&self, command: &str) -> Result<CommandOutput> {
        // Acquire semaphore permit to limit concurrency
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!("Semaphore error: {}", e))
        })?;

        let start = Instant::now();

        // Spawn the command using sh -c for full shell compatibility
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                autorecon_core::AutoReconError::Execution(format!("Failed to spawn command: {}", e))
            })?;

        // Capture stdout and stderr
        let stdout = child.stdout.take().ok_or_else(|| {
            autorecon_core::AutoReconError::Execution("Failed to capture stdout".to_string())
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            autorecon_core::AutoReconError::Execution("Failed to capture stderr".to_string())
        })?;

        // Read output asynchronously
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        let mut stdout_content = Vec::new();
        let mut stderr_content = Vec::new();

        // Read stdout
        while let Ok(Some(line)) = stdout_lines.next_line().await {
            stdout_content.push(line);
        }

        // Read stderr
        while let Ok(Some(line)) = stderr_lines.next_line().await {
            stderr_content.push(line);
        }

        // Wait for the command to complete
        let status = child.wait().await.map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!("Failed to wait for command: {}", e))
        })?;

        let duration = start.elapsed();

        Ok(CommandOutput {
            stdout: stdout_content.join("\n"),
            stderr: stderr_content.join("\n"),
            exit_code: status.code().unwrap_or(-1),
            duration_ms: duration.as_millis() as u64,
        })
    }

    async fn execute_with_timeout(
        &self,
        command: &str,
        timeout_secs: u64,
    ) -> Result<CommandOutput> {
        let timeout_duration = tokio::time::Duration::from_secs(timeout_secs);

        match tokio::time::timeout(timeout_duration, self.execute(command)).await {
            Ok(result) => result,
            Err(_) => Err(autorecon_core::AutoReconError::Execution(format!(
                "Command timed out after {} seconds",
                timeout_secs
            ))),
        }
    }

    async fn is_tool_available(&self, tool: &str) -> bool {
        let command = format!("command -v {} >/dev/null 2>&1", tool);
        match self.execute(&command).await {
            Ok(output) => output.is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = NativeExecutor::new(5);
        let result = executor.execute("echo 'hello world'").await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.stdout.trim(), "hello world");
    }

    #[tokio::test]
    async fn test_execute_failed_command() {
        let executor = NativeExecutor::new(5);
        let result = executor.execute("false").await.unwrap();

        assert!(!result.is_success());
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_is_tool_available() {
        let executor = NativeExecutor::new(5);

        // sh should always be available
        assert!(executor.is_tool_available("sh").await);

        // This tool should not exist
        assert!(!executor.is_tool_available("definitely_not_a_real_tool_12345").await);
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let executor = NativeExecutor::new(2);

        let mut handles = Vec::new();
        for i in 0..5 {
            let exec = executor.clone();
            let handle = tokio::spawn(async move {
                exec.execute(&format!("echo 'test {}'", i)).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result.is_success());
        }
    }
}
