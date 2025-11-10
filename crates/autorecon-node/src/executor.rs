//! Command executor using JavaScript callbacks via napi-rs

use autorecon_core::{
    executor::{CommandExecutor, CommandOutput},
    Result,
};
use async_trait::async_trait;
use napi::threadsafe_function::ThreadsafeFunction;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Command output from JavaScript callback
#[derive(serde::Deserialize, serde::Serialize)]
struct JsCommandOutput {
    stdout: String,
    stderr: String,
    #[serde(rename = "exitCode")]
    exit_code: i32,
    #[serde(rename = "durationMs")]
    duration_ms: u64,
}

/// Executor that calls JavaScript functions to execute commands
#[derive(Clone)]
pub struct NodeExecutor {
    /// Semaphore to limit concurrent executions
    semaphore: Arc<Semaphore>,

    /// JavaScript callback function for command execution
    execute_fn: Arc<ThreadsafeFunction<String, napi::threadsafe_function::ErrorStrategy::CalleeHandled>>,
}

impl NodeExecutor {
    /// Create a new Node executor
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent command executions
    /// * `execute_fn` - Threadsafe JavaScript function to execute commands
    pub fn new(
        max_concurrent: usize,
        execute_fn: Arc<ThreadsafeFunction<String, napi::threadsafe_function::ErrorStrategy::CalleeHandled>>,
    ) -> Self {
        NodeExecutor {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            execute_fn,
        }
    }
}

#[async_trait]
impl CommandExecutor for NodeExecutor {
    async fn execute(&self, command: &str) -> Result<CommandOutput> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!("Semaphore error: {}", e))
        })?;

        let command = command.to_string();

        // Call JavaScript function - it should return a JSON string
        let result_json: String = self.execute_fn
            .call_async(Ok(command))
            .await
            .map_err(|e| {
                autorecon_core::AutoReconError::Execution(format!(
                    "JavaScript callback error: {}",
                    e
                ))
            })?;

        // Parse the JSON result
        let js_output: JsCommandOutput = serde_json::from_str(&result_json).map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!("Failed to parse output: {}", e))
        })?;

        Ok(CommandOutput {
            stdout: js_output.stdout,
            stderr: js_output.stderr,
            exit_code: js_output.exit_code,
            duration_ms: js_output.duration_ms,
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
