//! Command executor using Python callbacks via PyO3

use autorecon_core::{
    executor::{CommandExecutor, CommandOutput},
    Result,
};
use async_trait::async_trait;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Executor that calls Python functions to execute commands
#[derive(Clone)]
pub struct PyExecutor {
    /// Semaphore to limit concurrent executions
    semaphore: Arc<Semaphore>,

    /// Python callback function for command execution
    execute_fn: PyObject,
}

impl PyExecutor {
    /// Create a new Python executor
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent command executions
    /// * `execute_fn` - Python async function to execute commands
    pub fn new(max_concurrent: usize, execute_fn: PyObject) -> Self {
        PyExecutor {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            execute_fn,
        }
    }
}

#[async_trait]
impl CommandExecutor for PyExecutor {
    async fn execute(&self, command: &str) -> Result<CommandOutput> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!("Semaphore error: {}", e))
        })?;

        let command = command.to_string();
        let execute_fn = self.execute_fn.clone();

        // Call Python async function from Rust async context
        let result = Python::with_gil(|py| -> Result<_> {
            // Call the Python function with the command
            let coroutine = execute_fn.call1(py, (command.clone(),))
                .map_err(|e| {
                    autorecon_core::AutoReconError::Execution(format!(
                        "Failed to call Python function: {}",
                        e
                    ))
                })?;

            // Convert coroutine to future and await it
            pyo3_asyncio::tokio::into_future(coroutine.as_ref(py))
                .map_err(|e| {
                    autorecon_core::AutoReconError::Execution(format!(
                        "Failed to convert Python coroutine: {}",
                        e
                    ))
                })
        })?;

        // Await the Python coroutine result
        let py_result = result.await.map_err(|e| {
            autorecon_core::AutoReconError::Execution(format!(
                "Python callback error: {}",
                e
            ))
        })?;

        // Extract the result from Python dict
        Python::with_gil(|py| {
            let dict = py_result.downcast::<PyDict>(py).map_err(|e| {
                autorecon_core::AutoReconError::Execution(format!(
                    "Expected dict result from Python callback: {}",
                    e
                ))
            })?;

            let stdout = dict
                .get_item("stdout")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<String>().ok())
                .unwrap_or_default();

            let stderr = dict
                .get_item("stderr")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<String>().ok())
                .unwrap_or_default();

            let exit_code = dict
                .get_item("exit_code")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<i32>().ok())
                .unwrap_or(1);

            let duration_ms = dict
                .get_item("duration_ms")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<u64>().ok())
                .unwrap_or(0);

            Ok(CommandOutput {
                stdout,
                stderr,
                exit_code,
                duration_ms,
            })
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
