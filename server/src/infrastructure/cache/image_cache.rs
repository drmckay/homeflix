//! Image Cache
//!
//! Provides filesystem-based caching for TMDB images.
//! Images are cached in data/.cache/tmdb-images/ directory with SHA256-hashed filenames.

use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use std::fs;
use tracing::{debug, warn, error};
use hex;
use crate::shared::error::FilesystemError;

/// Image cache for TMDB images
pub struct ImageCache {
    /// Base directory for cache (e.g., /data/.cache/tmdb-images/)
    cache_dir: PathBuf,
}

impl ImageCache {
    /// Creates a new image cache instance
    ///
    /// # Arguments
    /// * `data_dir` - Base data directory (e.g., /data or ./data)
    ///
    /// # Errors
    /// Returns error if cache directory cannot be created
    pub fn new(data_dir: &str) -> Result<Self, FilesystemError> {
        let data_path = Path::new(data_dir);
        let cache_dir = data_path.join(".cache").join("tmdb-images");

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)
            .map_err(|e| {
                error!("Failed to create cache directory {:?}: {}", cache_dir, e);
                FilesystemError::Io(e)
            })?;

        debug!("Image cache initialized at: {:?}", cache_dir);

        Ok(Self { cache_dir })
    }

    /// Gets the cache file path for a given TMDB image URL
    ///
    /// # Arguments
    /// * `url` - TMDB image URL (e.g., https://image.tmdb.org/t/p/w500/abc123.jpg)
    ///
    /// # Returns
    /// Cache file path (e.g., /data/.cache/tmdb-images/a1b2c3d4...jpg)
    fn get_cache_path(&self, url: &str) -> PathBuf {
        // Extract file extension from URL
        let extension = Path::new(url)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg"); // Default to jpg if no extension

        // Generate SHA256 hash of the URL
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);

        // Create filename: {hash}.{extension}
        let filename = format!("{}.{}", hash_hex, extension);
        self.cache_dir.join(filename)
    }

    /// Retrieves a cached image if it exists
    ///
    /// # Arguments
    /// * `url` - TMDB image URL
    ///
    /// # Returns
    /// * `Ok(Some(bytes))` if cached image exists
    /// * `Ok(None)` if not cached
    /// * `Err` if read error occurs
    pub fn get_cached_image(&self, url: &str) -> Result<Option<Vec<u8>>, FilesystemError> {
        let cache_path = self.get_cache_path(url);

        if !cache_path.exists() {
            debug!("Image not in cache: {}", url);
            return Ok(None);
        }

        match fs::read(&cache_path) {
            Ok(bytes) => {
                debug!("Image retrieved from cache: {} ({} bytes)", url, bytes.len());
                Ok(Some(bytes))
            }
            Err(e) => {
                warn!("Failed to read cached image {:?}: {}", cache_path, e);
                Err(FilesystemError::Io(e))
            }
        }
    }

    /// Saves an image to the cache
    ///
    /// # Arguments
    /// * `url` - TMDB image URL
    /// * `data` - Image bytes to cache
    ///
    /// # Errors
    /// Returns error if write fails
    pub fn save_cached_image(&self, url: &str, data: &[u8]) -> Result<(), FilesystemError> {
        let cache_path = self.get_cache_path(url);

        // Ensure parent directory exists (should already exist, but be safe)
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| {
                    error!("Failed to create cache parent directory: {}", e);
                    FilesystemError::Io(e)
                })?;
        }

        match fs::write(&cache_path, data) {
            Ok(_) => {
                debug!("Image cached: {} ({} bytes) -> {:?}", url, data.len(), cache_path);
                Ok(())
            }
            Err(e) => {
                error!("Failed to write cached image {:?}: {}", cache_path, e);
                Err(FilesystemError::Io(e))
            }
        }
    }

    /// Gets the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_cache_path() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ImageCache::new(temp_dir.path().to_str().unwrap()).unwrap();

        let url = "https://image.tmdb.org/t/p/w500/abc123.jpg";
        let path = cache.get_cache_path(url);

        // Check that path is in cache directory
        assert!(path.starts_with(cache.cache_dir()));

        // Check that filename contains hash and extension
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.ends_with(".jpg"));
        assert!(filename.len() > 4); // hash + extension

        // Same URL should produce same path
        let path2 = cache.get_cache_path(url);
        assert_eq!(path, path2);
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ImageCache::new(temp_dir.path().to_str().unwrap()).unwrap();

        let url = "https://image.tmdb.org/t/p/w500/test.jpg";
        let test_data = b"test image data";

        // Initially not cached
        assert!(cache.get_cached_image(url).unwrap().is_none());

        // Save to cache
        cache.save_cached_image(url, test_data).unwrap();

        // Now should be cached
        let cached = cache.get_cached_image(url).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), test_data);
    }
}
