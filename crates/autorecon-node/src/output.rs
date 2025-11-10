//! Output handler using native Node.js filesystem operations

use autorecon_core::{output::OutputHandler, Result};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Output handler that uses Node.js filesystem directly
#[derive(Clone)]
pub struct NodeOutputHandler {
    /// Verbosity level for logging
    verbosity: u8,
}

impl NodeOutputHandler {
    /// Create a new Node output handler
    ///
    /// # Arguments
    /// * `verbosity` - Verbosity level (0 = quiet, 1 = normal, 2 = verbose)
    pub fn new(verbosity: u8) -> Self {
        NodeOutputHandler { verbosity }
    }
}

#[async_trait]
impl OutputHandler for NodeOutputHandler {
    async fn create_directory(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!(
                "Failed to create directory {:?}: {}",
                path, e
            ))
        })?;

        if self.verbosity >= 2 {
            println!("Created directory: {}", path.display());
        }

        Ok(())
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_directory(parent).await?;
        }

        let mut file = fs::File::create(path).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!("Failed to create file {:?}: {}", path, e))
        })?;

        file.write_all(content.as_bytes()).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!("Failed to write to file {:?}: {}", path, e))
        })?;

        file.flush().await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!("Failed to flush file {:?}: {}", path, e))
        })?;

        if self.verbosity >= 2 {
            println!("Wrote file: {}", path.display());
        }

        Ok(())
    }

    async fn append_file(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_directory(parent).await?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| {
                autorecon_core::AutoReconError::Io(format!(
                    "Failed to open file for append {:?}: {}",
                    path, e
                ))
            })?;

        file.write_all(content.as_bytes()).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!(
                "Failed to append to file {:?}: {}",
                path, e
            ))
        })?;

        file.flush().await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!("Failed to flush file {:?}: {}", path, e))
        })?;

        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!("Failed to read file {:?}: {}", path, e))
        })?;

        Ok(content)
    }

    async fn file_exists(&self, path: &Path) -> bool {
        match fs::metadata(path).await {
            Ok(metadata) => metadata.is_file(),
            Err(_) => false,
        }
    }

    async fn directory_exists(&self, path: &Path) -> bool {
        match fs::metadata(path).await {
            Ok(metadata) => metadata.is_dir(),
            Err(_) => false,
        }
    }
}
