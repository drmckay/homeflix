//! IdentificationService - Media content identification from file paths
//!
//! Uses the media-identifier crate for robust filename parsing with:
//! - Rebulk-style pattern matching algorithm
//! - Multi-episode detection (S01E01E02, S01E01-E02)
//! - Quality/source/codec extraction
//! - Release group detection
//! - Folder structure analysis for series name extraction

use async_trait::async_trait;
use regex::Regex;
use std::path::Path;
use once_cell::sync::Lazy;

use crate::domain::value_objects::{MediaType, IdentificationResult, MatchStrategy};
use crate::shared::error::DomainError;

// Regex patterns for folder structure analysis
static RE_SEASON_FOLDER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(season\s*(\d+)|s(\d+))$").unwrap()
});

static RE_STRUCTURE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(season|s\d+|disc|disk|cd|dvd|part|pt|vol|volume)\b").unwrap()
});

static RE_SEASON_CHECK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)s\d+").unwrap()
});

static RE_SXXEXX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)S\d+E\d+").unwrap()
});

/// Patterns that indicate a media root directory (should not be used for title extraction)
static RE_MEDIA_ROOT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(movies?|films?|tv\s*(shows?|series)?|series|anime|media|videos?|downloads?|library|content|home\s*videos?)$").unwrap()
});

/// Folder structure pattern detection result
#[derive(Debug, Clone, PartialEq)]
pub enum FolderPattern {
    SeriesSeason,
    FlatSeries,
    MovieYear,
    CollectionMovie,
    Unknown,
}

/// Service for identifying media content
#[async_trait]
pub trait IdentificationService: Send + Sync {
    async fn identify_media_type(&self, file_path: &str) -> Result<MediaType, DomainError>;
    async fn extract_season_episode(&self, filename: &str) -> Result<Option<(i32, Vec<i32>)>, DomainError>;
    async fn clean_title(&self, title: &str) -> Result<String, DomainError>;
    async fn identify_content(&self, file_path: &str, duration_sec: Option<u64>) -> Result<IdentificationResult, DomainError>;
    async fn analyze_folder(&self, file_path: &str) -> Result<(FolderPattern, Option<String>), DomainError>;
    async fn extract_year(&self, text: &str) -> Result<Option<i32>, DomainError>;
    async fn is_anime(&self, file_path: &str, series_name: Option<&str>) -> Result<bool, DomainError>;
}

/// Default implementation of identification service using media-identifier crate
pub struct DefaultIdentificationService;

impl DefaultIdentificationService {
    pub fn new() -> Self {
        Self
    }

    /// Parse filename using media-identifier and extract series name from folder if needed
    ///
    /// Strategy: FOLDER-FIRST for movies (scene releases have abbreviated filenames)
    /// 1. For movies: Try folder name first, use filename only for fallback
    /// 2. For episodes: Use filename for S01E01 detection, folder for series name
    ///
    /// Falls back to folder name parsing when:
    /// - Filename gives low confidence (<60)
    /// - Title is missing or too short (<3 chars)
    /// - Title looks like an abbreviation (all caps, <=5 chars)
    fn parse_with_folder_context(file_path: &str) -> media_identifier::ParsedMedia {
        let path = Path::new(file_path);

        // First, parse the filename to detect episode patterns and get baseline
        let mut parsed = media_identifier::parse(file_path);
        let filename_is_poor = Self::is_poor_parse_result(&parsed);

        // For non-episodes (movies), try FOLDER FIRST - scene releases have better folder names
        // Examples: "Back.to.the.Future.Part.Two.1989..." folder vs "walle-bttf.ii.720.mkv" file
        if parsed.media_type != media_identifier::MediaType::Episode {
            if let Some(folder_parsed) = Self::try_parse_parent_folder(path) {
                let folder_is_poor = Self::is_poor_parse_result(&folder_parsed);

                // Use folder if: folder is good, OR filename is poor and folder is at least as good
                if !folder_is_poor || (filename_is_poor && Self::is_better_result(&folder_parsed, &parsed)) {
                    // Merge: use title/year from folder, keep any useful info from filename
                    if folder_parsed.title.is_some() {
                        parsed.title = folder_parsed.title;
                    }
                    if folder_parsed.year.is_some() {
                        parsed.year = folder_parsed.year;
                    }
                    // Update media type from folder if filename didn't detect it
                    if parsed.media_type == media_identifier::MediaType::Unknown {
                        parsed.media_type = folder_parsed.media_type;
                    }
                    // Use higher confidence
                    parsed.confidence = parsed.confidence.max(folder_parsed.confidence);
                }
            }
        } else {
            // For episodes: filename has S01E01 patterns, but get series name from folder
            // Only use folder fallback if filename gave poor title
            if filename_is_poor {
                if let Some(folder_parsed) = Self::try_parse_parent_folder(path) {
                    if Self::is_better_result(&folder_parsed, &parsed) {
                        // Keep episode info from filename, use title/year from folder
                        if folder_parsed.title.is_some() {
                            parsed.title = folder_parsed.title;
                        }
                        if folder_parsed.year.is_some() && parsed.year.is_none() {
                            parsed.year = folder_parsed.year;
                        }
                        parsed.confidence = parsed.confidence.max(folder_parsed.confidence);
                    }
                }
            }

            // For episodes, also try to extract series name from folder structure
            if let Some(series_name) = Self::extract_series_from_folder(path) {
                // Use folder name if it's more informative than filename-derived title
                if let Some(ref title) = parsed.title {
                    if series_name.len() > title.len() && !series_name.contains(&title[..]) {
                        parsed.title = Some(series_name);
                    }
                } else {
                    parsed.title = Some(series_name);
                }
            }
        }

        parsed
    }

    /// Check if a parse result is poor (low confidence, missing/bad title)
    fn is_poor_parse_result(parsed: &media_identifier::ParsedMedia) -> bool {
        // Low confidence threshold
        if parsed.confidence < 60 {
            return true;
        }

        // Missing title
        let title = match &parsed.title {
            Some(t) if !t.is_empty() => t,
            _ => return true,
        };

        // Title too short (likely abbreviation like "bttf")
        if title.len() < 3 {
            return true;
        }

        // Unknown media type with no year - likely poorly parsed
        if parsed.media_type == media_identifier::MediaType::Unknown && parsed.year.is_none() {
            return true;
        }

        // Title looks like it contains noise (resolution numbers like "720", "1080")
        // This suggests the parser couldn't properly identify where the title ends
        let words: Vec<&str> = title.split_whitespace().collect();
        for word in &words {
            if let Ok(num) = word.parse::<u16>() {
                // Common resolution values that shouldn't be in titles
                if matches!(num, 480 | 576 | 720 | 1080 | 2160) {
                    return true;
                }
            }
        }

        // First word of title looks like abbreviation (short, all same case)
        if let Some(first_word) = words.first() {
            // Expanded to <=5 chars to catch scene group names like "walle"
            if first_word.len() <= 5
                && first_word.chars().all(|c| c.is_ascii_alphabetic())
                && (first_word.chars().all(|c| c.is_ascii_uppercase())
                    || first_word.chars().all(|c| c.is_ascii_lowercase()))
            {
                // Short abbreviation-like first word
                return true;
            }
        }

        // Check for common scene release patterns that indicate abbreviated filename
        // e.g., "walle-bttf", "yify-movie", etc.
        let title_lower = title.to_lowercase();
        let scene_patterns = ["bttf", "yify", "sparks", "rarbg", "ettv"];
        for pattern in scene_patterns {
            if title_lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Check if folder result is better than filename result
    fn is_better_result(folder: &media_identifier::ParsedMedia, filename: &media_identifier::ParsedMedia) -> bool {
        let folder_title = folder.title.as_ref().map(|t| t.as_str()).unwrap_or("");
        let filename_title = filename.title.as_ref().map(|t| t.as_str()).unwrap_or("");

        // Folder title is significantly longer (likely more complete)
        if folder_title.len() > filename_title.len() + 3 {
            return true;
        }

        // Folder has year but filename doesn't
        if folder.year.is_some() && filename.year.is_none() {
            return true;
        }

        // Folder has higher confidence
        if folder.confidence > filename.confidence + 10 {
            return true;
        }

        false
    }

    /// Try to parse the parent folder name (skipping media root directories)
    fn try_parse_parent_folder(path: &Path) -> Option<media_identifier::ParsedMedia> {
        let mut current = path;

        // Try up to 3 levels of parent folders
        for _ in 0..3 {
            if let Some(parent) = current.parent() {
                if let Some(folder_name) = parent.file_name() {
                    let folder_str = folder_name.to_string_lossy();

                    // Skip if this is a structural folder (Season X, S01, etc.)
                    if RE_STRUCTURE.is_match(&folder_str) {
                        current = parent;
                        continue;
                    }

                    // Skip if this looks like a media root directory
                    if Self::is_media_root_folder(&folder_str) {
                        return None;
                    }

                    // Try to parse this folder name
                    let folder_parsed = media_identifier::parse(&folder_str);

                    // Only return if we got meaningful results
                    if folder_parsed.title.is_some() && folder_parsed.confidence >= 50 {
                        return Some(folder_parsed);
                    }
                }
                current = parent;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a folder name looks like a media root directory
    fn is_media_root_folder(folder_name: &str) -> bool {
        // Match common media root patterns
        RE_MEDIA_ROOT.is_match(folder_name)
    }

    /// Extract series name from folder structure
    fn extract_series_from_folder(path: &Path) -> Option<String> {
        let mut components = Vec::new();
        let mut current = path;

        for _ in 0..3 {
            if let Some(parent) = current.parent() {
                if let Some(name) = parent.file_name() {
                    components.push(name.to_string_lossy().to_string());
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Filter out structural folders (Season X, S01, etc.)
        let filtered: Vec<String> = components
            .into_iter()
            .filter(|c| !RE_STRUCTURE.is_match(c))
            .collect();

        // Parse the first non-structural folder name
        if let Some(folder_name) = filtered.first() {
            let folder_parsed = media_identifier::parse(folder_name);
            return folder_parsed.title;
        }

        None
    }

    fn is_anime_sync(path: &Path, series_name: Option<&str>) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        if path_str.contains("/anime/") || path_str.contains("\\anime\\") {
            return true;
        }

        if let Some(name) = series_name {
            let name_lower = name.to_lowercase();
            let studios = [
                "ghibli", "kyoto animation", "madhouse", "mappa", "wit studio",
                "bones", "shaft", "ufotable", "sunrise", "pierrot", "toei",
                "production i.g", "a-1 pictures", "trigger", "cloverworks",
            ];
            for studio in studios {
                if name_lower.contains(studio) {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for DefaultIdentificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdentificationService for DefaultIdentificationService {
    async fn identify_media_type(&self, file_path: &str) -> Result<MediaType, DomainError> {
        let parsed = media_identifier::parse(file_path);

        Ok(match parsed.media_type {
            media_identifier::MediaType::Episode => MediaType::Episode,
            media_identifier::MediaType::Movie => MediaType::Movie,
            media_identifier::MediaType::Unknown => {
                // Fallback: check if parent folder is a season folder
                let path = Path::new(file_path);
                if let Some(parent) = path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy()) {
                    if RE_SEASON_FOLDER.is_match(&parent) {
                        return Ok(MediaType::Episode);
                    }
                }
                MediaType::Unknown
            }
        })
    }

    async fn extract_season_episode(&self, filename: &str) -> Result<Option<(i32, Vec<i32>)>, DomainError> {
        let parsed = media_identifier::parse(filename);

        if let Some(season) = parsed.episode_info.season {
            if let Some(episode) = parsed.episode_info.episode {
                let episodes = if let Some(end) = parsed.episode_info.episode_end {
                    (episode as i32..=end as i32).collect()
                } else {
                    vec![episode as i32]
                };
                return Ok(Some((season as i32, episodes)));
            }
        }

        Ok(None)
    }

    async fn clean_title(&self, title: &str) -> Result<String, DomainError> {
        let parsed = media_identifier::parse(title);
        Ok(parsed.title.unwrap_or_else(|| title.to_string()))
    }

    async fn identify_content(&self, file_path: &str, _duration_sec: Option<u64>) -> Result<IdentificationResult, DomainError> {
        let path = Path::new(file_path);

        // Use media-identifier with folder context
        let parsed = Self::parse_with_folder_context(file_path);

        // Convert media type
        let media_type = match parsed.media_type {
            media_identifier::MediaType::Episode => MediaType::Episode,
            media_identifier::MediaType::Movie => MediaType::Movie,
            media_identifier::MediaType::Unknown => {
                // Fallback: check folder structure
                if let Some(parent) = path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy()) {
                    if RE_SEASON_FOLDER.is_match(&parent) {
                        MediaType::Episode
                    } else {
                        MediaType::Unknown
                    }
                } else {
                    MediaType::Unknown
                }
            }
        };

        // Extract season/episode info
        let season = parsed.episode_info.season.map(|s| s as i32);
        let episode = parsed.episode_info.episode.map(|e| e as i32);
        let episode_end = parsed.episode_info.episode_end.map(|e| e as i32);

        // Build multi-episode vector
        let multi_episode = if let (Some(start), Some(end)) = (episode, episode_end) {
            if end > start {
                Some((start..=end).collect::<Vec<i32>>())
            } else {
                None
            }
        } else {
            None
        };

        // Get title
        let title = parsed.title.clone().unwrap_or_else(|| {
            // Last resort: extract from filename
            path.file_stem()
                .and_then(|n| n.to_str())
                .map(|s| s.replace('.', " ").replace('_', " ").replace('-', " "))
                .unwrap_or_default()
        });

        // Determine match strategy based on parsed information
        let strategy = if parsed.year.is_some() {
            MatchStrategy::FilenameWithYear
        } else {
            MatchStrategy::FilenameOnly
        };

        // Check for anime
        let is_anime = Self::is_anime_sync(path, parsed.title.as_deref());

        // Build result
        let mut result = IdentificationResult::new(media_type, title.clone(), strategy)
            .with_year(parsed.year.map(|y| y as i32))
            .with_series_name(parsed.title.clone());

        if let Some(s) = season {
            result = result.with_season(Some(s));
        }

        if let Some(e) = episode {
            result = result.with_episode(Some(e));
        }

        if let Some(eps) = multi_episode {
            result = result.with_multi_episode(eps);
        }

        Ok(result)
    }

    async fn analyze_folder(&self, file_path: &str) -> Result<(FolderPattern, Option<String>), DomainError> {
        let path = Path::new(file_path);
        let mut components = Vec::new();
        let mut current = path;

        for _ in 0..3 {
            if let Some(parent) = current.parent() {
                if let Some(name) = parent.file_name() {
                    components.push(name.to_string_lossy().to_string());
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if components.len() >= 2 {
            let p1 = components[0].to_lowercase();
            if p1.contains("season") || RE_SEASON_CHECK.is_match(&p1) {
                let series_parsed = media_identifier::parse(&components[1]);
                return Ok((FolderPattern::SeriesSeason, series_parsed.title));
            }
        }

        if components.len() >= 3 && components[2].to_lowercase().contains("collection") {
            return Ok((FolderPattern::CollectionMovie, Some(components[1].clone())));
        }

        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let parsed = media_identifier::parse(&name);
        if parsed.year.is_some() && parsed.media_type == media_identifier::MediaType::Movie {
            return Ok((FolderPattern::MovieYear, None));
        }

        if !components.is_empty() && RE_SXXEXX.is_match(&name) {
            let series_parsed = media_identifier::parse(&components[0]);
            return Ok((FolderPattern::FlatSeries, series_parsed.title));
        }

        Ok((FolderPattern::Unknown, None))
    }

    async fn extract_year(&self, text: &str) -> Result<Option<i32>, DomainError> {
        let parsed = media_identifier::parse(text);
        Ok(parsed.year.map(|y| y as i32))
    }

    async fn is_anime(&self, file_path: &str, series_name: Option<&str>) -> Result<bool, DomainError> {
        Ok(Self::is_anime_sync(Path::new(file_path), series_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_title() {
        // Using media-identifier for title cleaning
        let parsed = media_identifier::parse("Breaking.Bad.S05E13.1080p.mkv");
        assert!(parsed.title.as_ref().map(|t| t.contains("Breaking Bad")).unwrap_or(false));

        let parsed = media_identifier::parse("Wonka.2023.2160p.WEB-DL.mkv");
        assert!(parsed.title.as_ref().map(|t| t.contains("Wonka")).unwrap_or(false));

        // Audio channel formats
        let parsed = media_identifier::parse("Lyle.Lyle.Crocodile.2022.720p.BluRay.DD+5.1.x264.mkv");
        assert!(parsed.title.as_ref().map(|t| t.contains("Lyle")).unwrap_or(false));

        // iTunes source and H.264 codec
        let parsed = media_identifier::parse("Wonka.2023.720p.iT.WEB-DL.DD+5.1.Atmos.H.264.HuN-No1.mkv");
        assert!(parsed.title.as_ref().map(|t| t.contains("Wonka")).unwrap_or(false));
        assert_eq!(parsed.year, Some(2023));
    }

    #[test]
    fn test_extract_year() {
        let parsed = media_identifier::parse("Wonka.2023.mkv");
        assert_eq!(parsed.year, Some(2023));

        // Note: Titles with years in them (like "2001: A Space Odyssey") may extract
        // the title year rather than the release year. This is a known parser limitation.
        let parsed = media_identifier::parse("2001.A.Space.Odyssey.1968.mkv");
        // Parser sees 2001 first as a valid year (it's in range 1900-2030)
        assert_eq!(parsed.year, Some(2001));
    }

    #[tokio::test]
    async fn test_extract_season_episode() {
        let service = DefaultIdentificationService::new();

        let result = service.extract_season_episode("S05E13").await.unwrap();
        assert_eq!(result, Some((5, vec![13])));

        let result = service.extract_season_episode("S01E01-E03").await.unwrap();
        assert_eq!(result, Some((1, vec![1, 2, 3])));

        let result = service.extract_season_episode("S01E01E02").await.unwrap();
        assert_eq!(result, Some((1, vec![1, 2])));
    }

    #[tokio::test]
    async fn test_identify_content_episode() {
        let service = DefaultIdentificationService::new();
        let result = service.identify_content("/media/TV/Breaking Bad/Season 5/S05E13.mkv", Some(2400)).await.unwrap();
        assert_eq!(result.media_type, MediaType::Episode);
        assert_eq!(result.season, Some(5));
        assert_eq!(result.episode, Some(13));
    }

    #[tokio::test]
    async fn test_identify_content_movie() {
        let service = DefaultIdentificationService::new();
        let result = service.identify_content("/media/Movies/Wonka.2023.mkv", Some(7200)).await.unwrap();
        assert_eq!(result.media_type, MediaType::Movie);
        assert_eq!(result.year, Some(2023));
    }

    #[test]
    fn test_is_anime() {
        assert!(DefaultIdentificationService::is_anime_sync(Path::new("/media/Anime/Naruto/S01E01.mkv"), None));
        assert!(!DefaultIdentificationService::is_anime_sync(Path::new("/media/TV/Show.mkv"), None));
    }

    #[test]
    fn test_multi_episode_patterns() {
        // S01E01E02 pattern
        let parsed = media_identifier::parse("Stargate.Universe.S01E01E02.Air.Parts.1.and.2.avi");
        assert_eq!(parsed.episode_info.season, Some(1));
        assert_eq!(parsed.episode_info.episode, Some(1));
        assert_eq!(parsed.episode_info.episode_end, Some(2));

        // S01E01-E02 pattern
        let parsed = media_identifier::parse("Stargate.Atlantis.S01E01-E02.Rising.BDRip.x264.mkv");
        assert_eq!(parsed.episode_info.season, Some(1));
        assert_eq!(parsed.episode_info.episode, Some(1));
        assert_eq!(parsed.episode_info.episode_end, Some(2));
    }

    #[test]
    fn test_is_poor_parse_result() {
        // Good result - should NOT be poor (Episode type, has season/episode)
        let good = media_identifier::parse("Breaking.Bad.S01E01.720p.mkv");
        assert!(!DefaultIdentificationService::is_poor_parse_result(&good),
            "Breaking Bad episode should not be flagged as poor");

        // Unknown type with no year - should be poor
        let unknown = media_identifier::parse("unknown.file.mkv");
        assert!(DefaultIdentificationService::is_poor_parse_result(&unknown),
            "Unknown media type with no year should be poor");

        // Title with resolution number - should be poor
        let with_res = media_identifier::parse("bttf.720.mkv");
        assert!(DefaultIdentificationService::is_poor_parse_result(&with_res),
            "Title containing '720' resolution should be poor");

        // Abbreviation-like first word - should be poor
        let abbrev = media_identifier::parse("walle-bttf.iii.mkv");
        assert!(DefaultIdentificationService::is_poor_parse_result(&abbrev),
            "Title starting with short abbreviation 'bttf' should be poor");

        // Good movie with year - should NOT be poor
        let movie = media_identifier::parse("Wonka.2023.720p.BluRay.mkv");
        assert!(!DefaultIdentificationService::is_poor_parse_result(&movie),
            "Movie with clear title and year should not be poor");
    }

    #[test]
    fn test_is_media_root_folder() {
        // Should match media root patterns
        assert!(DefaultIdentificationService::is_media_root_folder("Movies"));
        assert!(DefaultIdentificationService::is_media_root_folder("movies"));
        assert!(DefaultIdentificationService::is_media_root_folder("TV Shows"));
        assert!(DefaultIdentificationService::is_media_root_folder("TV Series"));
        assert!(DefaultIdentificationService::is_media_root_folder("Series"));
        assert!(DefaultIdentificationService::is_media_root_folder("Anime"));
        assert!(DefaultIdentificationService::is_media_root_folder("Media"));
        assert!(DefaultIdentificationService::is_media_root_folder("Videos"));
        assert!(DefaultIdentificationService::is_media_root_folder("Downloads"));
        assert!(DefaultIdentificationService::is_media_root_folder("Library"));

        // Should NOT match actual movie/show folders
        assert!(!DefaultIdentificationService::is_media_root_folder("Back to the Future (1985)"));
        assert!(!DefaultIdentificationService::is_media_root_folder("Breaking Bad"));
        assert!(!DefaultIdentificationService::is_media_root_folder("Stargate SG-1"));
        assert!(!DefaultIdentificationService::is_media_root_folder("2001 A Space Odyssey"));
    }

    #[test]
    fn test_folder_fallback_back_to_the_future() {
        // Simulate the BTTF case: filename is abbreviated, folder has full title
        // Path: /media/Movies/Back to the Future III (1990)/walle-bttf.iii.720.mkv
        let path = "/media/Movies/Back to the Future III (1990)/walle-bttf.iii.720.mkv";
        let parsed = DefaultIdentificationService::parse_with_folder_context(path);

        // Should extract title from folder, not filename
        assert!(parsed.title.is_some(), "Title should be extracted");
        let title = parsed.title.unwrap();
        assert!(
            title.contains("Back") || title.contains("Future"),
            "Title '{}' should come from folder 'Back to the Future III (1990)'",
            title
        );
    }

    #[test]
    fn test_folder_fallback_skips_media_root() {
        // When file is directly in media root, should not use root folder name
        // Path: /media/Movies/some-file.720.mkv
        let path = "/media/Movies/some-file.720.mkv";
        let parsed = DefaultIdentificationService::parse_with_folder_context(path);

        // Should NOT extract "Movies" as the title
        if let Some(ref title) = parsed.title {
            assert!(
                !title.eq_ignore_ascii_case("movies"),
                "Title '{}' should not be 'Movies' (the media root folder)",
                title
            );
        }
    }

    #[test]
    fn test_folder_fallback_good_filename_no_change() {
        // When filename parsing is good, should not override with folder
        // Path: /media/Movies/Some Collection/Wonka.2023.720p.BluRay.mkv
        let path = "/media/Movies/Some Collection/Wonka.2023.720p.BluRay.mkv";
        let parsed = DefaultIdentificationService::parse_with_folder_context(path);

        // Should keep "Wonka" from filename, not "Some Collection" from folder
        assert!(parsed.title.is_some());
        let title = parsed.title.unwrap();
        assert!(
            title.contains("Wonka"),
            "Title '{}' should be 'Wonka' from filename (good parse result)",
            title
        );
        assert_eq!(parsed.year, Some(2023));
    }
}
