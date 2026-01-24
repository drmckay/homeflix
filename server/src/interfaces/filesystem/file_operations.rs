// File Operations Interface
//
// This module defines interface for file system operations.
// Enables testing with mock implementations and alternative backends.
//
// This interface enables:
// - Testing with in-memory filesystems
// - Cloud storage backends
// - Permission checking and validation

use async_trait::async_trait;
use crate::shared::error::FilesystemError;

/// File metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    /// File path
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Whether file exists
    pub exists: bool,
    /// Whether path is a directory
    pub is_dir: bool,
    /// Whether path is a file
    pub is_file: bool,
    /// Whether path is a symbolic link
    pub is_symlink: bool,
    /// Last modified time (Unix timestamp)
    pub modified: Option<i64>,
    /// Created time (Unix timestamp)
    pub created: Option<i64>,
    /// File extension
    pub extension: Option<String>,
}

/// File operations interface
/// 
/// Provides methods for file system operations including:
/// - Reading and writing files
/// - Checking file existence and metadata
/// - Creating and deleting files/directories
#[async_trait]
pub trait FileOperations: Send + Sync {
    /// Check if a file or directory exists
    /// 
    /// # Arguments
    /// * `path` - Path to check
    /// 
    /// # Returns
    /// * `Result<bool, FilesystemError>` - True if path exists
    async fn exists(&self, path: &str) -> Result<bool, FilesystemError>;
    
    /// Get metadata for a file or directory
    /// 
    /// # Arguments
    /// * `path` - Path to get metadata for
    /// 
    /// # Returns
    /// * `Result<FileMetadata, FilesystemError>` - File metadata
    async fn metadata(&self, path: &str) -> Result<FileMetadata, FilesystemError>;
    
    /// Read entire file content as bytes
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// 
    /// # Returns
    /// * `Result<Vec<u8>, FilesystemError>` - File content
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn read_bytes(&self, path: &str) -> Result<Vec<u8>, FilesystemError>;
    
    /// Read entire file content as string
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// 
    /// # Returns
    /// * `Result<String, FilesystemError>` - File content as string
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    /// - File is not valid UTF-8
    async fn read_string(&self, path: &str) -> Result<String, FilesystemError>;
    
    /// Write bytes to a file
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// * `data` - Bytes to write
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn write_bytes(&self, path: &str, data: &[u8]) -> Result<(), FilesystemError>;
    
    /// Write string to a file
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// * `content` - String to write
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn write_string(&self, path: &str, content: &str) -> Result<(), FilesystemError>;
    
    /// Append bytes to a file
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// * `data` - Bytes to append
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn append_bytes(&self, path: &str, data: &[u8]) -> Result<(), FilesystemError>;
    
    /// Append string to a file
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// * `content` - String to append
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn append_string(&self, path: &str, content: &str) -> Result<(), FilesystemError>;
    
    /// Delete a file
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn delete_file(&self, path: &str) -> Result<(), FilesystemError>;
    
    /// Delete a directory and all its contents
    /// 
    /// # Arguments
    /// * `path` - Path to directory
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn delete_dir(&self, path: &str) -> Result<(), FilesystemError>;
    
    /// Delete a file or directory
    /// 
    /// # Arguments
    /// * `path` - Path to delete
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Path does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn delete(&self, path: &str) -> Result<(), FilesystemError>;
    
    /// Create a directory
    /// 
    /// # Arguments
    /// * `path` - Path to directory
    /// * `recursive` - Whether to create parent directories
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Permission denied
    /// - IO error occurs
    async fn create_dir(&self, path: &str, recursive: bool) -> Result<(), FilesystemError>;
    
    /// Create a directory if it doesn't exist
    /// 
    /// # Arguments
    /// * `path` - Path to directory
    /// * `recursive` - Whether to create parent directories
    /// 
    /// # Returns
    /// * `Result<bool, FilesystemError>` - True if directory was created
    async fn create_dir_all(&self, path: &str, recursive: bool) -> Result<bool, FilesystemError>;
    
    /// Move/rename a file or directory
    /// 
    /// # Arguments
    /// * `from` - Source path
    /// * `to` - Destination path
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Source does not exist
    /// - Destination already exists
    /// - Permission denied
    /// - IO error occurs
    async fn move_file(&self, from: &str, to: &str) -> Result<(), FilesystemError>;
    
    /// Copy a file
    /// 
    /// # Arguments
    /// * `from` - Source path
    /// * `to` - Destination path
    /// 
    /// # Returns
    /// * `Result<(), FilesystemError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Source does not exist
    /// - Destination already exists
    /// - Permission denied
    /// - IO error occurs
    async fn copy_file(&self, from: &str, to: &str) -> Result<(), FilesystemError>;
    
    /// List files in a directory (non-recursive)
    /// 
    /// # Arguments
    /// * `path` - Path to directory
    /// 
    /// # Returns
    /// * `Result<Vec<String>, FilesystemError>` - List of entry names
    /// 
    /// # Errors
    /// Returns error if:
    /// - Path is not a directory
    /// - Permission denied
    /// - IO error occurs
    async fn list_dir(&self, path: &str) -> Result<Vec<String>, FilesystemError>;
    
    /// Get file size in bytes
    /// 
    /// # Arguments
    /// * `path` - Path to file
    /// 
    /// # Returns
    /// * `Result<u64, FilesystemError>` - File size in bytes
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn file_size(&self, path: &str) -> Result<u64, FilesystemError>;
    
    /// Check if a path is a directory
    /// 
    /// # Arguments
    /// * `path` - Path to check
    /// 
    /// # Returns
    /// * `Result<bool, FilesystemError>` - True if path is a directory
    async fn is_dir(&self, path: &str) -> Result<bool, FilesystemError>;
    
    /// Check if a path is a file
    /// 
    /// # Arguments
    /// * `path` - Path to check
    /// 
    /// # Returns
    /// * `Result<bool, FilesystemError>` - True if path is a file
    async fn is_file(&self, path: &str) -> Result<bool, FilesystemError>;
}
