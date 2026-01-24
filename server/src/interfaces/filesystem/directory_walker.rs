// Directory Walker Interface
//
// This module defines interface for traversing directory trees.
// Enables testing with mock implementations and parallel traversal.
//
// This interface enables:
// - Testing with in-memory filesystems
// - Parallel traversal for performance
// - Filtering based on file extensions, etc.

use async_trait::async_trait;
use crate::shared::error::FilesystemError;

/// Entry result from directory walk
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalkEntry {
    /// Full path to the entry
    pub path: std::path::PathBuf,
    /// Whether this is a file
    pub is_file: bool,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Depth from root directory
    pub depth: usize,
    /// File size in bytes (None for directories)
    pub file_size: Option<u64>,
    /// File extension (None for directories)
    pub extension: Option<String>,
    /// Whether this is a symbolic link
    pub is_symlink: bool,
}

/// Directory walker interface
/// 
/// Provides methods for traversing directory trees and finding files.
/// Supports both sequential and parallel traversal.
#[async_trait]
pub trait DirectoryWalker: Send + Sync {
    /// Walk a directory tree and return all entries
    /// 
    /// # Arguments
    /// * `root` - Root directory path to walk
    /// 
    /// # Returns
    /// * `Result<Vec<WalkEntry>, FilesystemError>` - All entries in the tree
    /// 
    /// # Errors
    /// Returns error if:
    /// - Root directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn walk(&self, root: &std::path::Path) -> Result<Vec<WalkEntry>, FilesystemError>;
    
    /// Walk a directory tree with a filter function
    /// 
    /// # Arguments
    /// * `root` - Root directory path to walk
    /// * `filter` - Filter function that returns true for entries to include
    /// 
    /// # Returns
    /// * `Result<Vec<WalkEntry>, FilesystemError>` - Filtered entries
    /// 
    /// # Errors
    /// Returns error if:
    /// - Root directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn walk_with_filter(
        &self,
        root: &std::path::Path,
        filter: Box<dyn for<'a> Fn(&'a WalkEntry) -> bool + Send + Sync>,
    ) -> Result<Vec<WalkEntry>, FilesystemError>;
    
    /// Walk a directory tree in parallel
    /// 
    /// # Arguments
    /// * `root` - Root directory path to walk
    /// * `max_depth` - Maximum depth to traverse (None for unlimited)
    /// 
    /// # Returns
    /// * `Result<Vec<WalkEntry>, FilesystemError>` - All entries in the tree
    /// 
    /// # Errors
    /// Returns error if:
    /// - Root directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn walk_parallel(
        &self,
        root: &std::path::Path,
        max_depth: Option<usize>,
    ) -> Result<Vec<WalkEntry>, FilesystemError>;
    
    /// Walk a directory tree and return only video files
    ///
    /// Filters out:
    /// - Non-video files
    /// - Sample files (files in /sample/ folders or with sample- prefix)
    ///
    /// # Arguments
    /// * `root` - Root directory path to walk
    ///
    /// # Returns
    /// * `Result<Vec<WalkEntry>, FilesystemError>` - Video file entries
    ///
    /// # Errors
    /// Returns error if:
    /// - Root directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn walk_videos(&self, root: &std::path::Path) -> Result<Vec<WalkEntry>, FilesystemError> {
        self.walk_with_filter(root, Box::new(|entry| {
            entry.is_file && is_video_file(&entry.path) && !is_sample_file(&entry.path)
        }))
        .await
    }
    
    /// Walk a directory tree and return files matching extensions
    /// 
    /// # Arguments
    /// * `root` - Root directory path to walk
    /// * `extensions` - List of file extensions to include (e.g., ["mp4", "mkv"])
    /// 
    /// # Returns
    /// * `Result<Vec<WalkEntry>, FilesystemError>` - Matching file entries
    /// 
    /// # Errors
    /// Returns error if:
    /// - Root directory does not exist
    /// - Permission denied
    /// - IO error occurs
    async fn walk_by_extensions(
        &self,
        root: &std::path::Path,
        extensions: &[&str],
    ) -> Result<Vec<WalkEntry>, FilesystemError> {
        let extensions_lower: Vec<String> = extensions
            .iter()
            .map(|e| e.to_lowercase())
            .collect();
        
        self.walk_with_filter(root, Box::new(move |entry| {
            if !entry.is_file {
                return false;
            }
            if let Some(ext) = &entry.extension {
                extensions_lower.contains(&ext.to_lowercase())
            } else {
                false
            }
        }))
        .await
    }
}

/// Check if a file path has a video extension
fn is_video_file(path: &std::path::Path) -> bool {
    const VIDEO_EXTENSIONS: &[&str] = &[
        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
        "ts", "m2ts", "3gp", "ogv", "rm", "rmvb", "asf", "divx", "xvid",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if a file is a sample file that should be excluded
///
/// Sample files are detected by:
/// - Being in a folder named "sample" or "samples"
/// - Having a filename starting with "sample" or "!sample"
/// - Having "sample-" in the filename
/// - Having ".sample." in the filename (common scene release pattern)
fn is_sample_file(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Check if in a sample folder
    if path_str.contains("/sample/") || path_str.contains("/samples/") {
        return true;
    }

    // Check filename patterns
    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
        let filename_lower = filename.to_lowercase();
        if filename_lower.starts_with("sample")
            || filename_lower.starts_with("!sample")
            || filename_lower.contains("sample-")
            || filename_lower.contains(".sample.")  // Scene release pattern: movie.sample.mkv
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_sample_file_scene_pattern() {
        // Common scene release pattern: movie.sample.mkv
        assert!(is_sample_file(Path::new("/movies/Back.to.the.Future.Part.I.sample.mkv")));
        assert!(is_sample_file(Path::new("/movies/Movie.2024.1080p.BluRay.sample.mkv")));
    }

    #[test]
    fn test_is_sample_file_in_sample_folder() {
        assert!(is_sample_file(Path::new("/movies/Movie/sample/movie.mkv")));
        assert!(is_sample_file(Path::new("/movies/Movie/samples/movie.mkv")));
    }

    #[test]
    fn test_is_sample_file_prefix_patterns() {
        assert!(is_sample_file(Path::new("/movies/sample-movie.mkv")));
        assert!(is_sample_file(Path::new("/movies/sample.mkv")));
        assert!(is_sample_file(Path::new("/movies/!sample-movie.mkv")));
    }

    #[test]
    fn test_is_sample_file_not_sample() {
        // Regular movie files should not be detected as samples
        assert!(!is_sample_file(Path::new("/movies/Back.to.the.Future.mkv")));
        assert!(!is_sample_file(Path::new("/movies/Movie.2024.1080p.BluRay.mkv")));
        // Note: "Sampler.mkv" would be a false positive (starts with "sample")
        // but this is an acceptable edge case as movies named "Sampler" are rare
    }
}
