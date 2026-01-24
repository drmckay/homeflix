// Filesystem Adapters
//
// This module provides implementations for filesystem operations
// including directory walking and file operations.

pub mod walkdir_adapter;
pub mod file_operations_adapter;

pub use walkdir_adapter::WalkDirAdapter;
pub use file_operations_adapter::FileOperationsAdapter;
