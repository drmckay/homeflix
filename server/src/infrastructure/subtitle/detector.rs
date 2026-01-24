//! Subtitle Detector
//!
//! Discovers external subtitle files (.srt) located alongside video files.
//! Supports language detection from filename patterns.

use std::path::Path;

/// Represents an external subtitle file discovered on the filesystem.
#[derive(Debug, Clone)]
pub struct ExternalSubtitle {
    /// Absolute path to the subtitle file
    pub file_path: String,
    /// ISO 639-1 language code (e.g., "hu", "en")
    pub language: Option<String>,
    /// Human-readable language name (e.g., "Magyar", "English")
    pub language_name: Option<String>,
}

/// Discovers external subtitle files for video files.
///
/// Scans the video file's directory for .srt files that match
/// the video filename pattern.
///
/// # Supported patterns
/// - `movie.srt` - Default subtitle (no language)
/// - `movie.hu.srt`, `movie.hun.srt`, `movie.hungarian.srt` - Hungarian
/// - `movie.en.srt`, `movie.eng.srt`, `movie.english.srt` - English
/// - `movie.de.srt`, `movie.deu.srt`, `movie.ger.srt`, `movie.german.srt` - German
/// - `movie.es.srt`, `movie.spa.srt`, `movie.spanish.srt` - Spanish
/// - `movie.fr.srt`, `movie.fra.srt`, `movie.french.srt` - French
/// - `movie.it.srt`, `movie.ita.srt`, `movie.italian.srt` - Italian
#[derive(Debug, Clone)]
pub struct SubtitleDetector;

impl SubtitleDetector {
    /// Creates a new SubtitleDetector instance.
    pub fn new() -> Self {
        Self
    }

    /// Discovers all .srt subtitle files for a given video file.
    ///
    /// # Arguments
    /// * `video_path` - Path to the video file
    ///
    /// # Returns
    /// A vector of discovered external subtitles, sorted by language.
    pub fn discover(&self, video_path: &Path) -> Vec<ExternalSubtitle> {
        let mut subtitles = Vec::new();

        // Get video file stem (filename without extension)
        let video_stem = match video_path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => stem.to_lowercase(),
            None => return subtitles,
        };

        // Get parent directory
        let parent_dir = match video_path.parent() {
            Some(dir) => dir,
            None => return subtitles,
        };

        // Read directory entries
        let entries = match std::fs::read_dir(parent_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to read directory for subtitles: {}", e);
                return subtitles;
            }
        };

        // Find matching .srt files
        for entry in entries.flatten() {
            let path = entry.path();

            // Skip if not a file or not .srt
            if !path.is_file() {
                continue;
            }
            let extension = path.extension().and_then(|e| e.to_str());
            if extension != Some("srt") {
                continue;
            }

            // Get filename without .srt extension
            let filename = match path.file_stem().and_then(|s| s.to_str()) {
                Some(name) => name.to_lowercase(),
                None => continue,
            };

            // Check if filename starts with video stem
            if !filename.starts_with(&video_stem) {
                continue;
            }

            // Extract language suffix (everything after video stem)
            let suffix = &filename[video_stem.len()..];
            let (language, language_name) = self.detect_language(suffix);

            subtitles.push(ExternalSubtitle {
                file_path: path.to_string_lossy().to_string(),
                language,
                language_name,
            });
        }

        // Sort by language (None first, then alphabetically)
        subtitles.sort_by(|a, b| match (&a.language, &b.language) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(la), Some(lb)) => la.cmp(lb),
        });

        subtitles
    }

    /// Detects language from filename suffix.
    ///
    /// # Arguments
    /// * `suffix` - The part of the filename after the video name (e.g., ".hu", ".english", ".en.forced")
    ///
    /// # Returns
    /// Tuple of (language_code, language_name), both optional.
    fn detect_language(&self, suffix: &str) -> (Option<String>, Option<String>) {
        // Remove leading dots and convert to lowercase
        let suffix = suffix.trim_start_matches('.').to_lowercase();

        if suffix.is_empty() {
            return (None, None);
        }

        // Extract just the language part (first segment before any dot)
        // This handles cases like "en.forced" -> "en"
        let lang_part = suffix.split('.').next().unwrap_or(&suffix);

        // Language patterns mapping
        let patterns: &[(&[&str], &str, &str)] = &[
            (&["hu", "hun", "hungarian"], "hu", "Magyar"),
            (&["en", "eng", "english"], "en", "English"),
            (&["de", "deu", "ger", "german"], "de", "Deutsch"),
            (&["es", "spa", "spanish"], "es", "Espanol"),
            (&["fr", "fra", "french"], "fr", "Francais"),
            (&["it", "ita", "italian"], "it", "Italiano"),
            (&["pt", "por", "portuguese"], "pt", "Portugues"),
            (&["ru", "rus", "russian"], "ru", "Russian"),
            (&["pl", "pol", "polish"], "pl", "Polski"),
            (&["nl", "dut", "dutch"], "nl", "Nederlands"),
            (&["ja", "jpn", "japanese"], "ja", "Japanese"),
            (&["ko", "kor", "korean"], "ko", "Korean"),
            (&["zh", "chi", "chinese"], "zh", "Chinese"),
            (&["ar", "ara", "arabic"], "ar", "Arabic"),
            (&["cs", "cze", "czech"], "cs", "Cesky"),
            (&["sv", "swe", "swedish"], "sv", "Svenska"),
            (&["da", "dan", "danish"], "da", "Dansk"),
            (&["fi", "fin", "finnish"], "fi", "Suomi"),
            (&["no", "nor", "norwegian"], "no", "Norsk"),
            (&["el", "gre", "greek"], "el", "Greek"),
            (&["he", "heb", "hebrew"], "he", "Hebrew"),
            (&["tr", "tur", "turkish"], "tr", "Turkce"),
            (&["th", "tha", "thai"], "th", "Thai"),
            (&["vi", "vie", "vietnamese"], "vi", "Vietnamese"),
            (&["ro", "rum", "ron", "romanian"], "ro", "Romana"),
            (&["uk", "ukr", "ukrainian"], "uk", "Ukrainian"),
            (&["bg", "bul", "bulgarian"], "bg", "Bulgarian"),
            (&["hr", "hrv", "croatian"], "hr", "Hrvatski"),
            (&["sk", "slo", "slk", "slovak"], "sk", "Slovensky"),
            (&["sl", "slv", "slovenian"], "sl", "Slovenscina"),
        ];

        for (codes, iso_code, name) in patterns {
            if codes.contains(&lang_part) {
                return (Some(iso_code.to_string()), Some(name.to_string()));
            }
        }

        // Unknown language code - return as-is
        (Some(lang_part.to_string()), None)
    }
}

impl Default for SubtitleDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_hungarian() {
        let detector = SubtitleDetector::new();

        let (code, name) = detector.detect_language(".hu");
        assert_eq!(code, Some("hu".to_string()));
        assert_eq!(name, Some("Magyar".to_string()));

        let (code, name) = detector.detect_language(".hun");
        assert_eq!(code, Some("hu".to_string()));
        assert_eq!(name, Some("Magyar".to_string()));

        let (code, name) = detector.detect_language(".hungarian");
        assert_eq!(code, Some("hu".to_string()));
        assert_eq!(name, Some("Magyar".to_string()));
    }

    #[test]
    fn test_detect_language_english() {
        let detector = SubtitleDetector::new();

        let (code, name) = detector.detect_language(".en");
        assert_eq!(code, Some("en".to_string()));
        assert_eq!(name, Some("English".to_string()));

        let (code, name) = detector.detect_language(".eng");
        assert_eq!(code, Some("en".to_string()));
        assert_eq!(name, Some("English".to_string()));
    }

    #[test]
    fn test_detect_language_empty() {
        let detector = SubtitleDetector::new();
        let (code, name) = detector.detect_language("");
        assert_eq!(code, None);
        assert_eq!(name, None);
    }

    #[test]
    fn test_detect_language_unknown() {
        let detector = SubtitleDetector::new();
        let (code, name) = detector.detect_language(".xyz");
        assert_eq!(code, Some("xyz".to_string()));
        assert_eq!(name, None);
    }

    #[test]
    fn test_detect_language_with_suffix() {
        let detector = SubtitleDetector::new();

        // Test "en.forced" pattern
        let (code, name) = detector.detect_language(".en.forced");
        assert_eq!(code, Some("en".to_string()));
        assert_eq!(name, Some("English".to_string()));

        // Test "hu.sdh" pattern
        let (code, name) = detector.detect_language(".hu.sdh");
        assert_eq!(code, Some("hu".to_string()));
        assert_eq!(name, Some("Magyar".to_string()));
    }
}
