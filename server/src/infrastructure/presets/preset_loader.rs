//! Preset Loader
//!
//! Loads collection presets from YAML files in a directory.

use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, warn, error, debug};
use serde_yaml;

use crate::domain::presets::PresetCollection;
use crate::shared::error::PresetLoadError;

/// Loads preset collections from YAML files
pub struct PresetLoader;

impl PresetLoader {
    /// Load all presets from a directory
    ///
    /// Scans the directory for `*.yaml` and `*.yml` files and attempts to parse them as presets.
    /// Returns a vector of successfully loaded presets and logs warnings for any failures.
    ///
    /// # Arguments
    /// * `presets_dir` - Path to the directory containing preset YAML files
    ///
    /// # Returns
    /// Vector of loaded presets. Empty vector if directory doesn't exist or contains no valid presets.
    pub fn load_from_directory(presets_dir: &Path) -> Result<Vec<PresetCollection>, PresetLoadError> {
        if !presets_dir.exists() {
            warn!("Presets directory does not exist: {:?}", presets_dir);
            return Ok(Vec::new());
        }

        if !presets_dir.is_dir() {
            return Err(PresetLoadError::DirectoryNotFound(
                format!("Path is not a directory: {:?}", presets_dir),
            ));
        }

        let mut presets = Vec::new();
        let mut errors = Vec::new();

        // Read directory entries
        let entries = match fs::read_dir(presets_dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(PresetLoadError::Io(e));
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();

            // Only process .yaml and .yml files
            if !path.is_file() {
                continue;
            }

            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if extension != "yaml" && extension != "yml" {
                continue;
            }

            match Self::load_preset_file(&path) {
                Ok(mut preset) => {
                    // Validate the preset
                    match preset.validate() {
                        Ok(()) => {
                            info!("Loaded preset: {}", preset.name);
                            presets.push(preset);
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Validation failed for preset file {:?}: {}",
                                path, e
                            );
                            errors.push(error_msg.clone());
                            error!("{}", error_msg);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to load preset from {:?}: {}",
                        path, e
                    );
                    errors.push(error_msg.clone());
                    warn!("{}", error_msg);
                }
            }
        }

        if !errors.is_empty() {
            warn!(
                "Loaded {} presets with {} errors. Check logs for details.",
                presets.len(),
                errors.len()
            );
        } else {
            info!("Successfully loaded {} presets from {:?}", presets.len(), presets_dir);
        }

        Ok(presets)
    }

    /// Load a single preset from a YAML file
    fn load_preset_file(path: &Path) -> Result<PresetCollection, PresetLoadError> {
        let content = fs::read_to_string(path)
            .map_err(|e| PresetLoadError::Io(e))?;

        let preset: PresetCollection = serde_yaml::from_str(&content)
            .map_err(|e| PresetLoadError::YamlParse(
                path.display().to_string(),
                e.to_string(),
            ))?;

        Ok(preset)
    }

    /// Initialize the presets directory by copying built-in presets if they don't exist
    ///
    /// # Arguments
    /// * `presets_dir` - Target directory for presets (usually in data directory)
    /// * `builtin_presets_dir` - Source directory with built-in presets (usually in server/presets)
    ///
    /// # Returns
    /// Number of presets copied
    pub fn initialize_presets_directory(
        presets_dir: &Path,
        builtin_presets_dir: &Path,
    ) -> Result<usize, PresetLoadError> {
        // Create presets directory if it doesn't exist
        if !presets_dir.exists() {
            fs::create_dir_all(presets_dir)
                .map_err(|e| PresetLoadError::Io(e))?;
            info!("Created presets directory: {:?}", presets_dir);
        }

        // Check if built-in presets directory exists
        if !builtin_presets_dir.exists() {
            warn!("Built-in presets directory not found: {:?}", builtin_presets_dir);
            return Ok(0);
        }

        let mut copied = 0;

        // Read built-in presets
        let entries = fs::read_dir(builtin_presets_dir)
            .map_err(|e| PresetLoadError::Io(e))?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read built-in preset entry: {}", e);
                    continue;
                }
            };

            let source_path = entry.path();

            if !source_path.is_file() {
                continue;
            }

            let extension = source_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if extension != "yaml" && extension != "yml" {
                continue;
            }

            // Get filename
            let filename = source_path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| PresetLoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid filename: {:?}", source_path)
                )))?;

            let target_path = presets_dir.join(filename);

            // Only copy if target doesn't exist (preserve user modifications)
            if !target_path.exists() {
                match fs::copy(&source_path, &target_path) {
                    Ok(_) => {
                        info!("Copied built-in preset: {} -> {:?}", filename, target_path);
                        copied += 1;
                    }
                    Err(e) => {
                        warn!("Failed to copy preset {}: {}", filename, e);
                    }
                }
            } else {
                debug!("Preset already exists, skipping: {:?}", target_path);
            }
        }

        Ok(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_from_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");
        
        let result = PresetLoader::load_from_directory(&nonexistent);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_load_from_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        
        let result = PresetLoader::load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
