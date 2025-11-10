//! Native output handler using tokio filesystem operations

use autorecon_core::{
    output::OutputHandler,
    Result,
};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Native output handler that performs direct filesystem operations
#[derive(Clone)]
pub struct NativeOutputHandler {
    /// Verbosity level for logging
    verbosity: u8,
}

impl NativeOutputHandler {
    /// Create a new native output handler
    ///
    /// # Arguments
    /// * `verbosity` - Verbosity level (0 = quiet, 1 = normal, 2 = verbose)
    pub fn new(verbosity: u8) -> Self {
        NativeOutputHandler { verbosity }
    }

    /// Log to console if verbosity allows
    fn log_to_console(&self, level: u8, message: &str) {
        if self.verbosity >= level {
            println!("{}", message);
        }
    }
}

#[async_trait]
impl OutputHandler for NativeOutputHandler {
    async fn create_directory(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).await.map_err(|e| {
            autorecon_core::AutoReconError::Io(format!(
                "Failed to create directory {:?}: {}",
                path, e
            ))
        })?;

        self.log_to_console(2, &format!("Created directory: {}", path.display()));

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

        self.log_to_console(2, &format!("Wrote file: {}", path.display()));

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_directory() {
        let temp_dir = TempDir::new().unwrap();
        let handler = NativeOutputHandler::new(0);

        let test_dir = temp_dir.path().join("test/nested/dir");
        handler.create_directory(&test_dir).await.unwrap();

        assert!(handler.directory_exists(&test_dir).await);
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let handler = NativeOutputHandler::new(0);

        let test_file = temp_dir.path().join("test.txt");
        let content = "Hello, AutoRecon!";

        handler.write_file(&test_file, content).await.unwrap();

        assert!(handler.file_exists(&test_file).await);

        let read_content = handler.read_file(&test_file).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_append_file() {
        let temp_dir = TempDir::new().unwrap();
        let handler = NativeOutputHandler::new(0);

        let test_file = temp_dir.path().join("append.txt");

        handler.write_file(&test_file, "Line 1\n").await.unwrap();
        handler.append_file(&test_file, "Line 2\n").await.unwrap();
        handler.append_file(&test_file, "Line 3\n").await.unwrap();

        let content = handler.read_file(&test_file).await.unwrap();
        assert_eq!(content, "Line 1\nLine 2\nLine 3\n");
    }

    #[tokio::test]
    async fn test_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let handler = NativeOutputHandler::new(0);

        let existing_file = temp_dir.path().join("exists.txt");
        let non_existing_file = temp_dir.path().join("not_exists.txt");

        handler.write_file(&existing_file, "content").await.unwrap();

        assert!(handler.file_exists(&existing_file).await);
        assert!(!handler.file_exists(&non_existing_file).await);
    }

    #[tokio::test]
    async fn test_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let handler = NativeOutputHandler::new(0);

        let existing_dir = temp_dir.path().join("exists");
        let non_existing_dir = temp_dir.path().join("not_exists");

        handler.create_directory(&existing_dir).await.unwrap();

        assert!(handler.directory_exists(&existing_dir).await);
        assert!(!handler.directory_exists(&non_existing_dir).await);
    }
}
