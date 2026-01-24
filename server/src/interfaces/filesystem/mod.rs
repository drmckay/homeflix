// Filesystem Interfaces
//
// This module defines interfaces for file system operations.
// These interfaces enable testing with mock implementations and allow for different
// filesystem backends (local, cloud, etc.).
//
// Interfaces:
// - directory_walker: Directory traversal interface
// - file_operations: File read/write operations interface

pub mod directory_walker;
pub mod file_operations;

// Re-export all filesystem traits
pub use directory_walker::{DirectoryWalker, WalkEntry};
pub use file_operations::{FileOperations, FileMetadata};
