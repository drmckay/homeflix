//! File Operations Adapter Implementation
//!
//! Provides std::fs-based implementation of FileOperations interface

use async_trait::async_trait;
use std::path::Path;
use crate::interfaces::filesystem::{FileOperations, FileMetadata};
use crate::shared::error::FilesystemError;

/// Std::fs adapter for file operations
pub struct FileOperationsAdapter;

impl FileOperationsAdapter {
    /// Creates a new file operations adapter
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FileOperations for FileOperationsAdapter {
    async fn exists(&self, path: &str) -> Result<bool, FilesystemError> {
        let p = Path::new(path);
        Ok(p.exists())
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata, FilesystemError> {
        let p = Path::new(path);
        let metadata = p.metadata()
            .map_err(FilesystemError::Io)?;

        let modified = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        let created = metadata.created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let is_symlink = metadata.file_type().is_symlink();

        let extension = if is_file {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        Ok(FileMetadata {
            path: path.to_string(),
            size: metadata.len(),
            exists: true,
            is_dir,
            is_file,
            is_symlink,
            modified,
            created,
            extension,
        })
    }

    async fn read_bytes(&self, path: &str) -> Result<Vec<u8>, FilesystemError> {
        let p = Path::new(path);
        std::fs::read(p)
            .map_err(FilesystemError::Io)
    }

    async fn read_string(&self, path: &str) -> Result<String, FilesystemError> {
        let p = Path::new(path);
        let bytes = std::fs::read(p)
            .map_err(FilesystemError::Io)?;

        String::from_utf8(bytes)
            .map_err(|e| FilesystemError::Utf8Error(e.to_string()))
    }

    async fn write_bytes(&self, path: &str, data: &[u8]) -> Result<(), FilesystemError> {
        let p = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        std::fs::write(p, data)
            .map_err(FilesystemError::Io)
    }

    async fn write_string(&self, path: &str, content: &str) -> Result<(), FilesystemError> {
        let p = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        std::fs::write(p, content)
            .map_err(FilesystemError::Io)
    }

    async fn append_bytes(&self, path: &str, data: &[u8]) -> Result<(), FilesystemError> {
        use std::io::Write;
        let p = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        // Append to file
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(p)
            .map_err(FilesystemError::Io)?;

        file.write_all(data)
            .map_err(FilesystemError::Io)?;

        Ok(())
    }

    async fn append_string(&self, path: &str, content: &str) -> Result<(), FilesystemError> {
        use std::io::Write;
        let p = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(p)
            .map_err(FilesystemError::Io)?;

        file.write_all(content.as_bytes())
            .map_err(FilesystemError::Io)?;

        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<(), FilesystemError> {
        let p = Path::new(path);
        std::fs::remove_file(p)
            .map_err(FilesystemError::Io)
    }

    async fn delete_dir(&self, path: &str) -> Result<(), FilesystemError> {
        let p = Path::new(path);
        std::fs::remove_dir_all(p)
            .map_err(FilesystemError::Io)
    }

    async fn delete(&self, path: &str) -> Result<(), FilesystemError> {
        let p = Path::new(path);

        if p.is_dir() {
            self.delete_dir(path).await
        } else {
            self.delete_file(path).await
        }
    }

    async fn create_dir(&self, path: &str, recursive: bool) -> Result<(), FilesystemError> {
        let p = Path::new(path);

        if recursive {
            std::fs::create_dir_all(p)
                .map_err(FilesystemError::Io)
        } else {
            std::fs::create_dir(p)
                .map_err(FilesystemError::Io)
        }
    }

    async fn create_dir_all(&self, path: &str, recursive: bool) -> Result<bool, FilesystemError> {
        let p = Path::new(path);

        let created = if recursive {
            std::fs::create_dir_all(p)
        } else {
            std::fs::create_dir(p)
        };

        created.map(|_| true).map_err(FilesystemError::Io)
    }

    async fn move_file(&self, from: &str, to: &str) -> Result<(), FilesystemError> {
        let src = Path::new(from);
        let dst = Path::new(to);

        // Create parent directory if it doesn't exist
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        std::fs::rename(&src, &dst)
            .map_err(FilesystemError::Io)
    }

    async fn copy_file(&self, from: &str, to: &str) -> Result<(), FilesystemError> {
        let src = Path::new(from);
        let dst = Path::new(to);

        // Create parent directory if it doesn't exist
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(FilesystemError::Io)?;
        }

        std::fs::copy(&src, &dst)
            .map(|_| ())
            .map_err(FilesystemError::Io)
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<String>, FilesystemError> {
        let p = Path::new(path);

        let entries = std::fs::read_dir(p)
            .map_err(FilesystemError::Io)?;

        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(FilesystemError::Io)?;
            let name = entry.file_name()
                .to_string_lossy()
                .to_string();
            names.push(name);
        }

        Ok(names)
    }

    async fn file_size(&self, path: &str) -> Result<u64, FilesystemError> {
        let p = Path::new(path);
        p.metadata()
            .map(|m| m.len())
            .map_err(FilesystemError::Io)
    }

    async fn is_dir(&self, path: &str) -> Result<bool, FilesystemError> {
        let p = Path::new(path);
        Ok(p.is_dir())
    }

    async fn is_file(&self, path: &str) -> Result<bool, FilesystemError> {
        let p = Path::new(path);
        Ok(p.is_file())
    }
}
