//! WalkDir Adapter Implementation
//!
//! Provides WalkDir-based implementation of DirectoryWalker interface

use async_trait::async_trait;
use std::path::Path;
use crate::interfaces::filesystem::{DirectoryWalker, WalkEntry};
use crate::shared::error::FilesystemError;

/// WalkDir adapter for directory traversal
pub struct WalkDirAdapter;

impl WalkDirAdapter {
    /// Creates a new WalkDir adapter
    pub fn new() -> Self {
        Self
    }

    /// Helper function to filter entries - uses explicit loop to avoid closure lifetime issues
    fn apply_filter_loop(
        entries: Vec<WalkEntry>,
        filter: Box<dyn for<'a> Fn(&'a WalkEntry) -> bool + Send + Sync>,
    ) -> Vec<WalkEntry> {
        let mut result = Vec::new();
        for entry in entries {
            if filter(&entry) {
                result.push(entry);
            }
        }
        result
    }

    /// Creates a WalkEntry from a directory entry
    fn create_walk_entry(&self, entry: &walkdir::DirEntry) -> Result<WalkEntry, FilesystemError> {
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| FilesystemError::WalkError(e.to_string()))?;

        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let depth = path.ancestors().count();

        let file_size = if is_file {
            Some(metadata.len())
        } else {
            None
        };

        let extension = if is_file {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        let is_symlink = metadata.file_type().is_symlink();

        Ok(WalkEntry {
            path: path.to_path_buf(),
            is_file,
            is_dir,
            depth,
            file_size,
            extension,
            is_symlink,
        })
    }
}

#[async_trait]
impl DirectoryWalker for WalkDirAdapter {
    async fn walk(&self, root: &Path) -> Result<Vec<WalkEntry>, FilesystemError> {
        let mut entries = Vec::new();

        let walker = walkdir::WalkDir::new(root);

        for entry in walker.into_iter() {
            let entry = entry.map_err(|e| FilesystemError::WalkError(e.to_string()))?;

            let path = entry.path();

            let metadata = entry.metadata().map_err(|e| FilesystemError::WalkError(e.to_string()))?;

            let is_file = metadata.is_file();
            let is_dir = metadata.is_dir();
            let depth = path.ancestors().count();

            let file_size = if is_file {
                Some(metadata.len())
            } else {
                None
            };

            let extension = if is_file {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            };

            let is_symlink = metadata.file_type().is_symlink();

            entries.push(WalkEntry {
                path: path.to_path_buf(),
                is_file,
                is_dir,
                depth,
                file_size,
                extension,
                is_symlink,
            });
        }

        Ok(entries)
    }

    async fn walk_with_filter(
        &self,
        root: &Path,
        filter: Box<dyn for<'a> Fn(&'a WalkEntry) -> bool + Send + Sync>,
    ) -> Result<Vec<WalkEntry>, FilesystemError> {
        // First collect all entries
        let all_entries = self.walk(root).await?;

        // Use helper to filter - taking ownership avoids lifetime issues
        let result = Self::apply_filter_loop(all_entries, filter);

        Ok(result)
    }

    async fn walk_parallel(
        &self,
        root: &Path,
        max_depth: Option<usize>,
    ) -> Result<Vec<WalkEntry>, FilesystemError> {
        let mut entries = Vec::new();
        let max_depth = max_depth.unwrap_or(usize::MAX);

        let walker = walkdir::WalkDir::new(root).min_depth(max_depth);

        for entry in walker.into_iter() {
            let entry = entry.map_err(|e| FilesystemError::WalkError(e.to_string()))?;

            let walk_entry = self.create_walk_entry(&entry)?;

            entries.push(walk_entry);
        }

        Ok(entries)
    }
}
