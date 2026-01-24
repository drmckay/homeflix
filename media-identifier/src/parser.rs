use crate::markers::{ConflictResolver, HoleFinder, PostProcessor, TitleExtractor};
use crate::patterns::PatternRegistry;
use crate::tokenizer::Tokenizer;
use crate::types::{
    EpisodeInfo, Hole, Match, MatchCategory, MatchInfo, MediaType, ParsedMedia, QualityInfo,
};

/// Result of analyzing a filename, including matches and unmatched holes
#[derive(Debug)]
pub struct AnalysisResult {
    /// The input filename (without extension)
    pub input: String,
    /// All resolved pattern matches
    pub matches: Vec<Match>,
    /// Unmatched regions (holes) that may contain titles or other info
    pub holes: Vec<Hole>,
}

/// Configuration for the parser
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Include all matches in the output (for debugging)
    pub include_matches: bool,
    /// Attempt to extract episode titles
    pub extract_episode_titles: bool,
    /// Use smart tokenization (merge hyphenated words)
    pub smart_tokenize: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            include_matches: false,
            extract_episode_titles: true,
            smart_tokenize: true,
        }
    }
}

/// The main media filename parser
/// 
/// This implements a Rebulk-inspired algorithm:
/// 1. Pattern matching: Find all known patterns (year, season, quality, etc.)
/// 2. Conflict resolution: Handle overlapping matches by priority
/// 3. Hole detection: Find unmatched regions
/// 4. Title extraction: Determine title from holes and positions
/// 5. Assembly: Build the final ParsedMedia struct
pub struct MediaParser {
    config: ParserConfig,
}

impl MediaParser {
    /// Create a new parser with default configuration
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a new parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse a media filename
    pub fn parse(&self, input: &str) -> ParsedMedia {
        // Step 0: Handle file path - extract just the filename
        let filename = self.extract_filename(input);
        
        // Step 1: Strip extension if present
        let (name_without_ext, container) = self.strip_extension(&filename);

        // Step 2: Find all pattern matches
        let all_matches = PatternRegistry::find_all_matches(&name_without_ext);

        // Step 3: Resolve conflicts
        let resolved_matches = ConflictResolver::resolve(all_matches);

        // Step 4: Extract title
        let title = TitleExtractor::extract_title(&name_without_ext, &resolved_matches);

        // Step 5: Extract episode title if enabled and this is a TV show
        let episode_title = if self.config.extract_episode_titles {
            TitleExtractor::extract_episode_title(&name_without_ext, &resolved_matches)
        } else {
            None
        };

        // Step 6: Determine media type
        let media_type = self.detect_media_type(&resolved_matches);

        // Step 7: Extract structured data
        let episode_info = self.extract_episode_info(&resolved_matches, episode_title);
        let quality_info = self.extract_quality_info(&resolved_matches);
        let year = self.extract_year(&resolved_matches);
        let languages = PostProcessor::normalize_languages(&resolved_matches);
        let release_group = self.extract_release_group(&resolved_matches);

        // Step 8: Calculate confidence
        let confidence = PostProcessor::calculate_confidence(&resolved_matches, title.is_some());

        // Step 9: Build result
        ParsedMedia {
            original: input.to_string(),
            media_type,
            title,
            year,
            episode_info,
            quality: quality_info,
            languages,
            release_group,
            container,
            confidence,
            matches: if self.config.include_matches {
                resolved_matches.iter().map(MatchInfo::from).collect()
            } else {
                Vec::new()
            },
        }
    }

    /// Parse with matches included in output (for debugging)
    pub fn parse_debug(&self, input: &str) -> ParsedMedia {
        let mut config = self.config.clone();
        config.include_matches = true;
        let parser = MediaParser::with_config(config);
        parser.parse(input)
    }

    /// Analyze a filename and return debug information including holes
    ///
    /// This is useful for understanding what regions of the filename weren't
    /// matched by any pattern (potential titles, episode titles, or unrecognized metadata).
    pub fn analyze(&self, input: &str) -> AnalysisResult {
        let filename = self.extract_filename(input);
        let (name_without_ext, _) = self.strip_extension(&filename);
        let all_matches = PatternRegistry::find_all_matches(&name_without_ext);
        let resolved_matches = ConflictResolver::resolve(all_matches);
        let holes = HoleFinder::find_holes(&name_without_ext, &resolved_matches);

        AnalysisResult {
            input: name_without_ext,
            matches: resolved_matches,
            holes,
        }
    }

    /// Extract just the filename from a path
    fn extract_filename(&self, input: &str) -> String {
        // Handle both Unix and Windows paths
        input
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or(input)
            .to_string()
    }

    /// Strip the file extension and return (name, extension)
    fn strip_extension(&self, input: &str) -> (String, Option<String>) {
        if let Some((ext, pos)) = Tokenizer::extract_extension(input) {
            (input[..pos].to_string(), Some(ext))
        } else {
            (input.to_string(), None)
        }
    }

    /// Detect whether this is a movie or TV episode
    fn detect_media_type(&self, matches: &[Match]) -> MediaType {
        // If we have season/episode markers, it's a TV show
        let has_episode = matches.iter().any(|m| {
            m.category == MatchCategory::Episode || m.category == MatchCategory::Season
        });

        if has_episode {
            MediaType::Episode
        } else {
            // Has year but no episode markers - likely a movie
            let has_year = matches.iter().any(|m| m.category == MatchCategory::Year);
            if has_year {
                MediaType::Movie
            } else {
                MediaType::Unknown
            }
        }
    }

    /// Extract episode information from matches
    fn extract_episode_info(&self, matches: &[Match], episode_title: Option<String>) -> EpisodeInfo {
        let episode_match = matches.iter().find(|m| m.category == MatchCategory::Episode);
        let season_match = matches.iter().find(|m| m.category == MatchCategory::Season);

        let (mut season, episode, episode_end) = episode_match
            .map(|m| PostProcessor::parse_episode_raw(&m.raw))
            .unwrap_or((None, None, None));

        // If we only have a season match (no episode), use that
        if season.is_none() {
            if let Some(sm) = season_match {
                let (s, _, _) = PostProcessor::parse_episode_raw(&sm.raw);
                season = s;
            }
        }

        EpisodeInfo {
            season,
            episode,
            episode_end,
            episode_title,
            absolute_episode: None, // Could be added for anime support
        }
    }

    /// Extract quality information from matches
    fn extract_quality_info(&self, matches: &[Match]) -> QualityInfo {
        let resolution = matches
            .iter()
            .find(|m| m.category == MatchCategory::Quality)
            .map(|m| m.value.clone());

        let source = matches
            .iter()
            .find(|m| m.category == MatchCategory::Source)
            .map(|m| m.value.clone());

        let codec = matches
            .iter()
            .find(|m| m.category == MatchCategory::Codec)
            .map(|m| m.value.clone());

        // Combine audio matches
        let audio: Vec<String> = matches
            .iter()
            .filter(|m| m.category == MatchCategory::Audio)
            .map(|m| m.value.clone())
            .collect();

        let audio_combined = if audio.is_empty() {
            None
        } else {
            Some(audio.join(" "))
        };

        QualityInfo {
            resolution,
            source,
            codec,
            audio: audio_combined,
        }
    }

    /// Extract year from matches
    ///
    /// For TV episodes (with SxxExx markers), ignore years that appear AFTER
    /// the episode marker, as they are likely episode titles (e.g., "1969", "2001", "2010")
    fn extract_year(&self, matches: &[Match]) -> Option<u16> {
        // Find the position of the first episode/season marker
        let episode_marker_pos = matches
            .iter()
            .filter(|m| m.category == MatchCategory::Episode || m.category == MatchCategory::Season)
            .map(|m| m.start)
            .min();

        // Find year matches
        let year_match = matches
            .iter()
            .find(|m| {
                if m.category != MatchCategory::Year {
                    return false;
                }
                // If there's an episode marker, only use year if it comes BEFORE the episode marker
                if let Some(ep_pos) = episode_marker_pos {
                    m.start < ep_pos
                } else {
                    true
                }
            });

        year_match.and_then(|m| m.value.parse().ok())
    }

    /// Extract release group from matches
    fn extract_release_group(&self, matches: &[Match]) -> Option<String> {
        matches
            .iter()
            .find(|m| m.category == MatchCategory::ReleaseGroup)
            .map(|m| m.value.clone())
    }
}

impl Default for MediaParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to parse a filename
pub fn parse(input: &str) -> ParsedMedia {
    MediaParser::new().parse(input)
}

/// Convenience function to parse with debug info
pub fn parse_debug(input: &str) -> ParsedMedia {
    MediaParser::new().parse_debug(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_basic() {
        let result = parse("Home.Alone.1990.REMASTERED.BDRip.x264.AC3.HuN-Essence.mkv");
        
        assert_eq!(result.media_type, MediaType::Movie);
        assert!(result.title.as_ref().map(|t| t.contains("Home Alone")).unwrap_or(false));
        assert_eq!(result.year, Some(1990));
        assert_eq!(result.quality.source, Some("Blu-ray".to_string()));
        assert_eq!(result.quality.codec, Some("H.264".to_string()));
        assert!(result.languages.contains(&"Hungarian".to_string()));
        assert_eq!(result.release_group, Some("Essence".to_string()));
        assert_eq!(result.container, Some("mkv".to_string()));
    }

    #[test]
    fn test_tv_show_basic() {
        let result = parse("Dark.Matter.S01E05.720p.HDTV.x264-KILLERS.mkv");
        
        assert_eq!(result.media_type, MediaType::Episode);
        assert!(result.title.as_ref().map(|t| t.contains("Dark Matter")).unwrap_or(false));
        assert_eq!(result.episode_info.season, Some(1));
        assert_eq!(result.episode_info.episode, Some(5));
        assert_eq!(result.quality.resolution, Some("720p".to_string()));
    }

    #[test]
    fn test_tv_show_with_title() {
        let result = parse("Stargate.SG-1.S09E18.Arthur's.Mantle.BDRip.x264.Hun.Eng-MaMMuT.mkv");
        
        assert_eq!(result.media_type, MediaType::Episode);
        assert!(result.title.as_ref().map(|t| t.contains("Stargate")).unwrap_or(false));
        assert_eq!(result.episode_info.season, Some(9));
        assert_eq!(result.episode_info.episode, Some(18));
    }

    #[test]
    fn test_multi_episode() {
        let result = parse("Show.S01E01E02.720p.mkv");
        
        assert_eq!(result.episode_info.season, Some(1));
        assert_eq!(result.episode_info.episode, Some(1));
        assert_eq!(result.episode_info.episode_end, Some(2));
    }

    #[test]
    fn test_episode_range() {
        let result = parse("Stargate.Atlantis.S01E01-E02.Rising.BDRip.x264.Hun.Eng-MaMMuT.mkv");
        
        assert_eq!(result.episode_info.season, Some(1));
        assert_eq!(result.episode_info.episode, Some(1));
        assert_eq!(result.episode_info.episode_end, Some(2));
    }

    #[test]
    fn test_multiple_languages() {
        let result = parse("Movie.2020.720p.BluRay.Hun.Eng-GROUP.mkv");
        
        assert!(result.languages.contains(&"Hungarian".to_string()));
        assert!(result.languages.contains(&"English".to_string()));
    }

    #[test]
    fn test_wonka_movies() {
        let wonka1 = parse("Willy.Wonka.and.the.Chocolate.Factory.1971.720p.BluRay.mkv");
        let wonka2 = parse("Wonka.2023.720p.iT.WEB-DL.mkv");
        
        assert_eq!(wonka1.year, Some(1971));
        assert!(wonka1.title.as_ref().map(|t| t.contains("Willy Wonka")).unwrap_or(false));
        
        assert_eq!(wonka2.year, Some(2023));
        assert!(wonka2.title.as_ref().map(|t| t.contains("Wonka")).unwrap_or(false));
    }

    #[test]
    fn test_season_only() {
        let result = parse("Dark.Matter.2015.S01-S03.Complete.mkv");
        
        // Should detect as Episode type due to season markers
        assert_eq!(result.media_type, MediaType::Episode);
        assert!(result.title.as_ref().map(|t| t.contains("Dark Matter")).unwrap_or(false));
        assert_eq!(result.year, Some(2015));
    }

    #[test]
    fn test_path_handling() {
        let result = parse("/media/movies/Home.Alone.1990.720p.mkv");
        assert!(result.title.as_ref().map(|t| t.contains("Home Alone")).unwrap_or(false));
        
        let result2 = parse("C:\\Users\\Media\\Home.Alone.1990.720p.mkv");
        assert!(result2.title.as_ref().map(|t| t.contains("Home Alone")).unwrap_or(false));
    }

    #[test]
    fn test_ballerina_2025() {
        let result = parse("Ballerina.2025.Hybrid.BDRip.x264.HUN-FULCRUM.mkv");

        assert_eq!(result.media_type, MediaType::Movie);
        assert!(result.title.as_ref().map(|t| t.contains("Ballerina")).unwrap_or(false));
        assert_eq!(result.year, Some(2025));
        assert_eq!(result.release_group, Some("FULCRUM".to_string()));
    }

    #[test]
    fn test_analyze_holes() {
        // Test the analyze() function that exposes holes
        let parser = MediaParser::new();
        let analysis = parser.analyze("Dark.Matter.2015.S01E05.720p.HDTV.x264-KILLERS.mkv");

        // Should have matches for Year, Episode, Quality, Source, Codec, ReleaseGroup
        assert!(!analysis.matches.is_empty());

        // Should have at least one hole (the title "Dark.Matter.")
        assert!(!analysis.holes.is_empty());
        let first_hole = &analysis.holes[0];
        assert!(first_hole.text.contains("Dark"));
        assert!(first_hole.text.contains("Matter"));
    }
}
