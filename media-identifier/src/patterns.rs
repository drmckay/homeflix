use crate::types::{Match, MatchCategory};
use lazy_static::lazy_static;
use regex::Regex;

/// A pattern matcher that can find matches in tokens or raw strings
pub trait Pattern: Send + Sync {
    fn name(&self) -> &'static str;
    fn category(&self) -> MatchCategory;
    
    /// Find all matches in the input string
    fn find_matches(&self, input: &str) -> Vec<Match>;
}

// ============================================================================
// REGEX-BASED PATTERNS
// ============================================================================

lazy_static! {
    // Season/Episode patterns (most critical for TV detection)
    static ref SEASON_EPISODE_PATTERNS: Vec<Regex> = vec![
        // S01E01, S01E01E02, S1E1
        Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})(?:E(\d{1,3}))?\b").unwrap(),
        // S01E01-E02 format
        Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})-E?(\d{1,3})\b").unwrap(),
        // 1x01 format
        Regex::new(r"(?i)\b(\d{1,2})x(\d{1,3})\b").unwrap(),
        // Season.01 or Season 1
        Regex::new(r"(?i)\bSeason\.?(\d{1,2})\b").unwrap(),
        // Episode.01 or Episode 1
        Regex::new(r"(?i)\bEpisode\.?(\d{1,3})\b").unwrap(),
    ];
    
    // Season-only pattern (separate so we can handle it differently)
    // This will match S01 but we'll filter out S01E in code
    static ref SEASON_ONLY_PATTERN: Regex = Regex::new(r"(?i)\bS(\d{1,2})\b").unwrap();

    // 3-digit episode pattern: 117 -> S01E17, 501 -> S05E01
    // Format: First digit is season (1-9), last two digits are episode (01-99)
    // Context: Must be preceded by separator and followed by separator
    // Excludes numbers that look like years or standalone numbers
    static ref EPISODE_3DIGIT_PATTERN: Regex = Regex::new(
        r"[.\-_\s](\d)(\d{2})[.\-_\s]"
    ).unwrap();

    // 4-digit episode pattern for high season numbers: 2401 -> S24E01
    // Format: First two digits are season (10-99), last two digits are episode (01-99)
    // This has higher risk of year conflicts so we'll be more conservative
    static ref EPISODE_4DIGIT_PATTERN: Regex = Regex::new(
        r"[.\-_\s]([1-9]\d)(\d{2})[.\-_\s]"
    ).unwrap();

    // Year pattern (4 digits, reasonable range)
    static ref YEAR_PATTERN: Regex = Regex::new(
        r"\b(19[4-9]\d|20[0-3]\d)\b"
    ).unwrap();

    // Resolution/Quality patterns
    static ref RESOLUTION_PATTERNS: Vec<(&'static str, Regex)> = vec![
        ("2160p", Regex::new(r"(?i)\b(2160p|4K|UHD)\b").unwrap()),
        ("1080p", Regex::new(r"(?i)\b1080[pi]\b").unwrap()),
        ("720p", Regex::new(r"(?i)\b720p\b").unwrap()),
        ("480p", Regex::new(r"(?i)\b480p\b").unwrap()),
        ("576p", Regex::new(r"(?i)\b576p\b").unwrap()),
    ];

    // Source patterns
    static ref SOURCE_PATTERNS: Vec<(&'static str, Regex)> = vec![
        ("Blu-ray", Regex::new(r"(?i)\b(BluRay|Blu-Ray|BDRip|BRRip|BD)\b").unwrap()),
        ("WEB-DL", Regex::new(r"(?i)\b(WEB-?DL|WEB)\b").unwrap()),
        ("WEBRip", Regex::new(r"(?i)\bWEBRip\b").unwrap()),
        ("HDTV", Regex::new(r"(?i)\bHDTV\b").unwrap()),
        ("PDTV", Regex::new(r"(?i)\bPDTV\b").unwrap()),
        ("DVDRip", Regex::new(r"(?i)\b(DVDRip|DVD)\b").unwrap()),
        ("HDRip", Regex::new(r"(?i)\bHDRip\b").unwrap()),
        ("CAM", Regex::new(r"(?i)\b(CAM|HDCAM)\b").unwrap()),
        ("TS", Regex::new(r"(?i)\b(TS|HDTS|TELESYNC)\b").unwrap()),
        ("MA", Regex::new(r"(?i)\bMA\b").unwrap()),  // Movies Anywhere
    ];

    // Codec patterns
    static ref CODEC_PATTERNS: Vec<(&'static str, Regex)> = vec![
        ("H.264", Regex::new(r"(?i)\b[Hx]\.?264\b").unwrap()),
        ("H.265", Regex::new(r"(?i)\b[Hx]\.?265\b").unwrap()),
        ("HEVC", Regex::new(r"(?i)\bHEVC\b").unwrap()),
        ("XviD", Regex::new(r"(?i)\bXviD\b").unwrap()),
        ("DivX", Regex::new(r"(?i)\bDivX\b").unwrap()),
        ("AV1", Regex::new(r"(?i)\bAV1\b").unwrap()),
        ("VP9", Regex::new(r"(?i)\bVP9\b").unwrap()),
    ];

    // Audio patterns
    static ref AUDIO_PATTERNS: Vec<(&'static str, Regex)> = vec![
        ("DTS-HD MA", Regex::new(r"(?i)\bDTS-?HD[.\s]?MA\b").unwrap()),
        ("DTS-HD", Regex::new(r"(?i)\bDTS-?HD\b").unwrap()),
        ("DTS", Regex::new(r"(?i)\bDTS\b").unwrap()),
        ("TrueHD", Regex::new(r"(?i)\bTrueHD\b").unwrap()),
        ("Atmos", Regex::new(r"(?i)\bAtmos\b").unwrap()),
        ("DD+5.1", Regex::new(r"(?i)\bDD\+?5\.1\b").unwrap()),
        ("DD5.1", Regex::new(r"(?i)\bDD5\.1\b").unwrap()),
        ("AC3", Regex::new(r"(?i)\bAC3\b").unwrap()),
        ("AAC", Regex::new(r"(?i)\bAAC\b").unwrap()),
        ("FLAC", Regex::new(r"(?i)\bFLAC\b").unwrap()),
        ("MP3", Regex::new(r"(?i)\bMP3\b").unwrap()),
        ("5.1", Regex::new(r"\b5\.1\b").unwrap()),
        ("7.1", Regex::new(r"\b7\.1\b").unwrap()),
        ("2.0", Regex::new(r"\b2\.0\b").unwrap()),
    ];

    // Language patterns
    static ref LANGUAGE_PATTERNS: Vec<(&'static str, Regex)> = vec![
        ("Hungarian", Regex::new(r"(?i)\b(HUN|Hungarian|Magyar)\b").unwrap()),
        ("English", Regex::new(r"(?i)\b(ENG|English)\b").unwrap()),
        ("German", Regex::new(r"(?i)\b(GER|German|Deutsch)\b").unwrap()),
        ("French", Regex::new(r"(?i)\b(FRE|FRA|French)\b").unwrap()),
        ("Spanish", Regex::new(r"(?i)\b(SPA|ESP|Spanish)\b").unwrap()),
        ("Italian", Regex::new(r"(?i)\b(ITA|Italian)\b").unwrap()),
        ("Russian", Regex::new(r"(?i)\b(RUS|Russian)\b").unwrap()),
        ("Japanese", Regex::new(r"(?i)\b(JPN|JAP|Japanese)\b").unwrap()),
        ("Korean", Regex::new(r"(?i)\b(KOR|Korean)\b").unwrap()),
        ("Chinese", Regex::new(r"(?i)\b(CHI|Chinese)\b").unwrap()),
        ("Multi", Regex::new(r"(?i)\bMULTi\b").unwrap()),
        ("Dual Audio", Regex::new(r"(?i)\bDual[.\s]?Audio\b").unwrap()),
    ];

    // Release group pattern (typically at the end after a hyphen)
    static ref RELEASE_GROUP_PATTERN: Regex = Regex::new(
        r"-([A-Za-z0-9]+)(?:\.[a-z]{2,4})?$"
    ).unwrap();

    // Release group at beginning of filename (scene format: GROUP-Title.Year...)
    // Matches: "fulcrum-ballerina.2025" -> captures "fulcrum"
    static ref RELEASE_GROUP_PREFIX_PATTERN: Regex = Regex::new(
        r"^([a-zA-Z][a-zA-Z0-9]{1,9})-([a-zA-Z][a-zA-Z0-9]*\.[a-zA-Z0-9])"
    ).unwrap();

    // Noise tokens (things to strip/ignore)
    static ref NOISE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\bREMASTERED\b").unwrap(),
        Regex::new(r"(?i)\bPROPER\b").unwrap(),
        Regex::new(r"(?i)\bREPACK\b").unwrap(),
        Regex::new(r"(?i)\bINTERNAL\b").unwrap(),
        Regex::new(r"(?i)\bLIMITED\b").unwrap(),
        Regex::new(r"(?i)\bREAD\.?NFO\b").unwrap(),
        Regex::new(r"(?i)\bHYBRID\b").unwrap(),
        Regex::new(r"(?i)\bCOMPLETE\b").unwrap(),
        Regex::new(r"(?i)\bUNRATED\b").unwrap(),
        Regex::new(r"(?i)\bEXTENDED\b").unwrap(),
        Regex::new(r"(?i)\bDC\b").unwrap(),  // Director's Cut
        Regex::new(r"(?i)\bSample\b").unwrap(),
    ];

    // Season range pattern (S01-S03)
    static ref SEASON_RANGE_PATTERN: Regex = Regex::new(
        r"(?i)\b[Ss](\d{1,2})-[Ss]?(\d{1,2})\b"
    ).unwrap();
}

// ============================================================================
// PATTERN IMPLEMENTATIONS
// ============================================================================

/// Season/Episode pattern matcher
pub struct SeasonEpisodePattern;

impl SeasonEpisodePattern {
    pub fn find_matches(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();

        // Try each pattern
        for (idx, pattern) in SEASON_EPISODE_PATTERNS.iter().enumerate() {
            for cap in pattern.captures_iter(input) {
                let full_match = cap.get(0).unwrap();
                
                match idx {
                    0 => {
                        // S01E01 or S01E01E02
                        let season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                        let episode: u16 = cap.get(2).unwrap().as_str().parse().unwrap();
                        let episode_end: Option<u16> = cap.get(3).map(|m| m.as_str().parse().unwrap());

                        matches.push(Match {
                            start: full_match.start(),
                            end: full_match.end(),
                            value: format!("S{:02}E{:02}{}", 
                                season, episode,
                                episode_end.map(|e| format!("-E{:02}", e)).unwrap_or_default()
                            ),
                            raw: format!("{}|{}|{}", season, episode, episode_end.unwrap_or(0)),
                            category: MatchCategory::Episode,
                            confidence: 100,
                        });
                    }
                    1 => {
                        // S01E01-E02
                        let season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                        let episode: u16 = cap.get(2).unwrap().as_str().parse().unwrap();
                        let episode_end: u16 = cap.get(3).unwrap().as_str().parse().unwrap();

                        matches.push(Match {
                            start: full_match.start(),
                            end: full_match.end(),
                            value: format!("S{:02}E{:02}-E{:02}", season, episode, episode_end),
                            raw: format!("{}|{}|{}", season, episode, episode_end),
                            category: MatchCategory::Episode,
                            confidence: 100,
                        });
                    }
                    2 => {
                        // 1x01 format
                        let season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                        let episode: u16 = cap.get(2).unwrap().as_str().parse().unwrap();
                        matches.push(Match {
                            start: full_match.start(),
                            end: full_match.end(),
                            value: format!("S{:02}E{:02}", season, episode),
                            raw: format!("{}|{}|0", season, episode),
                            category: MatchCategory::Episode,
                            confidence: 95,
                        });
                    }
                    3 => {
                        // Season X
                        let season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                        matches.push(Match {
                            start: full_match.start(),
                            end: full_match.end(),
                            value: format!("S{:02}", season),
                            raw: format!("{}|0|0", season),
                            category: MatchCategory::Season,
                            confidence: 85,
                        });
                    }
                    4 => {
                        // Episode X
                        let episode: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                        matches.push(Match {
                            start: full_match.start(),
                            end: full_match.end(),
                            value: format!("E{:02}", episode),
                            raw: format!("0|{}|0", episode),
                            category: MatchCategory::Episode,
                            confidence: 80,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Handle season-only patterns (S01 not followed by E)
        // We need to check manually since Rust regex doesn't support lookahead
        for cap in SEASON_ONLY_PATTERN.captures_iter(input) {
            let full_match = cap.get(0).unwrap();
            let match_end = full_match.end();
            
            // Check if there's an 'E' immediately after (making it S01E not just S01)
            let has_episode_after = input[match_end..].chars().next()
                .map(|c| c == 'E' || c == 'e')
                .unwrap_or(false);
            
            // Only add if it's truly season-only and we don't already have a match at this position
            if !has_episode_after {
                let already_matched = matches.iter().any(|m| 
                    m.start <= full_match.start() && m.end >= full_match.end()
                );
                
                if !already_matched {
                    let season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
                    matches.push(Match {
                        start: full_match.start(),
                        end: full_match.end(),
                        value: format!("S{:02}", season),
                        raw: format!("{}|0|0", season),
                        category: MatchCategory::Season,
                        confidence: 90,
                    });
                }
            }
        }

        // Check for season range (S01-S03)
        for cap in SEASON_RANGE_PATTERN.captures_iter(input) {
            let full_match = cap.get(0).unwrap();
            let start_season: u16 = cap.get(1).unwrap().as_str().parse().unwrap();
            let end_season: u16 = cap.get(2).unwrap().as_str().parse().unwrap();

            // Only add if we don't already have a match at this position
            if !matches.iter().any(|m| m.start == full_match.start()) {
                matches.push(Match {
                    start: full_match.start(),
                    end: full_match.end(),
                    value: format!("S{:02}-S{:02}", start_season, end_season),
                    raw: format!("{}-{}|0|0", start_season, end_season),
                    category: MatchCategory::Season,
                    confidence: 95,
                });
            }
        }

        // Check for 3-digit episode format (117 -> S01E17)
        // Only if we haven't found any other episode patterns
        if !matches.iter().any(|m| m.category == MatchCategory::Episode) {
            for cap in EPISODE_3DIGIT_PATTERN.captures_iter(input) {
                let season_str = cap.get(1).unwrap().as_str();
                let episode_str = cap.get(2).unwrap().as_str();
                let season: u16 = season_str.parse().unwrap();
                let episode: u16 = episode_str.parse().unwrap();

                // Validate: season 1-9, episode 01-99, episode > 0
                if season >= 1 && season <= 9 && episode >= 1 && episode <= 99 {
                    // Calculate positions (excluding the surrounding separators)
                    let full_match = cap.get(0).unwrap();
                    let start = full_match.start() + 1; // Skip leading separator
                    let end = full_match.end() - 1;     // Skip trailing separator

                    // Check if this position is already covered by a year match
                    // The year pattern will also match, so we need context validation
                    let num_str = format!("{}{}", season_str, episode_str);
                    let as_num: u16 = num_str.parse().unwrap_or(0);

                    // Skip if it looks like a valid year (1940-2039)
                    if as_num >= 1940 && as_num <= 2039 {
                        continue;
                    }

                    // Skip if it looks like a resolution (480, 576, 720, 1080, etc.)
                    // Check if followed by 'p' or 'i'
                    let after_num = input.get(end..end+1).unwrap_or("");
                    if after_num == "p" || after_num == "i" {
                        continue;
                    }

                    // Skip common resolution numbers even without suffix
                    if matches!(as_num, 480 | 576 | 720 | 1080 | 2160) {
                        continue;
                    }

                    // Skip codec identifiers (264 from H.264, 265 from H.265/x265)
                    if matches!(as_num, 264 | 265) {
                        continue;
                    }

                    matches.push(Match {
                        start,
                        end,
                        value: format!("S{:02}E{:02}", season, episode),
                        raw: format!("{}|{}|0", season, episode),
                        category: MatchCategory::Episode,
                        confidence: 75, // Lower confidence than explicit SxxExx format
                    });
                    break; // Only take first match
                }
            }
        }

        // Check for 4-digit episode format (2401 -> S24E01) for high season shows
        // Only if we haven't found any episode patterns yet
        if !matches.iter().any(|m| m.category == MatchCategory::Episode) {
            for cap in EPISODE_4DIGIT_PATTERN.captures_iter(input) {
                let season_str = cap.get(1).unwrap().as_str();
                let episode_str = cap.get(2).unwrap().as_str();
                let season: u16 = season_str.parse().unwrap();
                let episode: u16 = episode_str.parse().unwrap();

                // Validate: season 10-99, episode 01-99
                if season >= 10 && season <= 99 && episode >= 1 && episode <= 99 {
                    let full_match = cap.get(0).unwrap();
                    let start = full_match.start() + 1;
                    let end = full_match.end() - 1;

                    // The full number
                    let num_str = format!("{}{}", season_str, episode_str);
                    let as_year: u16 = num_str.parse().unwrap_or(0);

                    // Skip if it looks like a valid year (1940-2039)
                    if as_year >= 1940 && as_year <= 2039 {
                        continue;
                    }

                    matches.push(Match {
                        start,
                        end,
                        value: format!("S{:02}E{:02}", season, episode),
                        raw: format!("{}|{}|0", season, episode),
                        category: MatchCategory::Episode,
                        confidence: 70, // Even lower confidence for 4-digit
                    });
                    break;
                }
            }
        }

        matches
    }
}

/// Year pattern matcher
pub struct YearPattern;

impl YearPattern {
    pub fn find_matches(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        
        for cap in YEAR_PATTERN.captures_iter(input) {
            let m = cap.get(1).unwrap();
            let year: u16 = m.as_str().parse().unwrap();
            
            // Confidence based on position and context
            let confidence = if m.start() > 0 {
                100
            } else {
                70  // Year at the very start is unusual
            };
            
            matches.push(Match {
                start: m.start(),
                end: m.end(),
                value: year.to_string(),
                raw: year.to_string(),
                category: MatchCategory::Year,
                confidence,
            });
        }
        
        matches
    }
}

/// Generic pattern matcher for simple regex->normalized value mappings
pub struct SimplePatternMatcher;

impl SimplePatternMatcher {
    pub fn find_matches<'a>(
        input: &str,
        patterns: &[(&'a str, Regex)],
        category: MatchCategory,
    ) -> Vec<Match> {
        let mut matches = Vec::new();
        
        for (normalized, pattern) in patterns {
            for m in pattern.find_iter(input) {
                matches.push(Match {
                    start: m.start(),
                    end: m.end(),
                    value: normalized.to_string(),
                    raw: m.as_str().to_string(),
                    category,
                    confidence: 100,
                });
            }
        }
        
        matches
    }
    
    pub fn find_noise_matches(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        
        for pattern in NOISE_PATTERNS.iter() {
            for m in pattern.find_iter(input) {
                matches.push(Match {
                    start: m.start(),
                    end: m.end(),
                    value: m.as_str().to_uppercase(),
                    raw: m.as_str().to_string(),
                    category: MatchCategory::Noise,
                    confidence: 100,
                });
            }
        }
        
        matches
    }
}

/// Release group pattern matcher
pub struct ReleaseGroupPattern;

impl ReleaseGroupPattern {
    pub fn find_matches(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();

        // Check for release group at the END of filename (most common)
        // e.g., "Movie.2023.720p.BluRay-SPARKS.mkv"
        if let Some(cap) = RELEASE_GROUP_PATTERN.captures(input) {
            let full = cap.get(0).unwrap();
            let group = cap.get(1).unwrap();

            // Filter out common false positives
            let group_str = group.as_str();
            if !Self::is_false_positive(group_str) {
                matches.push(Match {
                    start: full.start(),
                    end: full.end(),
                    value: group_str.to_string(),
                    raw: group_str.to_string(),
                    category: MatchCategory::ReleaseGroup,
                    confidence: 90,
                });
            }
        }

        // Check for release group at the BEGINNING of filename (scene format)
        // e.g., "fulcrum-ballerina.2025.bdrip.mkv"
        if matches.is_empty() {
            if let Some(cap) = RELEASE_GROUP_PREFIX_PATTERN.captures(input) {
                let group = cap.get(1).unwrap();
                let group_str = group.as_str();

                // Prefix groups are typically lowercase scene group names
                if !Self::is_false_positive(group_str) && !Self::is_common_word(group_str) {
                    matches.push(Match {
                        start: group.start(),
                        end: group.end() + 1, // Include the dash
                        value: group_str.to_string(),
                        raw: group_str.to_string(),
                        category: MatchCategory::ReleaseGroup,
                        confidence: 85, // Slightly lower confidence for prefix groups
                    });
                }
            }
        }

        matches
    }

    fn is_false_positive(group: &str) -> bool {
        let upper = group.to_uppercase();
        // Common false positives that look like release groups
        // Note: Known release groups like FULCRUM, SPARKS, No1, MaMMuT etc. should NOT be here
        matches!(
            upper.as_str(),
            "MKV" | "AVI" | "MP4" | "SRT" | "NFO" | "SFV"
            | "SAMPLE" | "SUBS" | "SUB" | "EXTRAS" | "FEATURETTES"
        )
    }

    fn is_common_word(word: &str) -> bool {
        let lower = word.to_lowercase();
        // Common words that might appear at the start but aren't release groups
        matches!(
            lower.as_str(),
            "the" | "a" | "an" | "new" | "old" | "big" | "best" | "top"
            | "my" | "our" | "this" | "that" | "one" | "two" | "first" | "last"
        )
    }
}

/// Container/extension pattern matcher
pub struct ContainerPattern;

impl ContainerPattern {
    pub fn find_match(input: &str) -> Option<Match> {
        let extensions = ["mkv", "mp4", "avi", "wmv", "mov", "m4v", "ts", "m2ts"];
        
        for ext in extensions {
            let pattern = format!(r"\.({})$", ext);
            if let Ok(re) = Regex::new(&pattern) {
                if let Some(m) = re.find(input) {
                    return Some(Match {
                        start: m.start(),
                        end: m.end(),
                        value: ext.to_string(),
                        raw: m.as_str().to_string(),
                        category: MatchCategory::Container,
                        confidence: 100,
                    });
                }
            }
        }
        None
    }
}

// ============================================================================
// MAIN PATTERN REGISTRY
// ============================================================================

/// Registry of all patterns - the Rebulk equivalent
pub struct PatternRegistry;

impl PatternRegistry {
    /// Find all matches in the input string using all registered patterns
    pub fn find_all_matches(input: &str) -> Vec<Match> {
        let mut all_matches = Vec::new();
        
        // Season/Episode (highest priority for TV detection)
        all_matches.extend(SeasonEpisodePattern::find_matches(input));
        
        // Year
        all_matches.extend(YearPattern::find_matches(input));
        
        // Quality markers
        all_matches.extend(SimplePatternMatcher::find_matches(
            input, &RESOLUTION_PATTERNS, MatchCategory::Quality
        ));
        all_matches.extend(SimplePatternMatcher::find_matches(
            input, &SOURCE_PATTERNS, MatchCategory::Source
        ));
        all_matches.extend(SimplePatternMatcher::find_matches(
            input, &CODEC_PATTERNS, MatchCategory::Codec
        ));
        all_matches.extend(SimplePatternMatcher::find_matches(
            input, &AUDIO_PATTERNS, MatchCategory::Audio
        ));
        
        // Language
        all_matches.extend(SimplePatternMatcher::find_matches(
            input, &LANGUAGE_PATTERNS, MatchCategory::Language
        ));
        
        // Noise tokens
        all_matches.extend(SimplePatternMatcher::find_noise_matches(input));
        
        // Release group
        all_matches.extend(ReleaseGroupPattern::find_matches(input));
        
        // Container
        if let Some(m) = ContainerPattern::find_match(input) {
            all_matches.push(m);
        }
        
        all_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_episode_basic() {
        let matches = SeasonEpisodePattern::find_matches("Show.S01E05.720p");
        assert!(!matches.is_empty());
        // Should have season 1, episode 5
        let m = &matches[0];
        assert_eq!(m.category, MatchCategory::Episode);
        assert!(m.raw.starts_with("1|5"));
    }

    #[test]
    fn test_season_episode_multi() {
        let matches = SeasonEpisodePattern::find_matches("Show.S01E01E02.720p");
        assert!(!matches.is_empty());
        // Should detect multi-episode
        let m = &matches[0];
        assert!(m.raw.contains("|1|") || m.raw.ends_with("|2"));
    }

    #[test]
    fn test_season_episode_range() {
        let matches = SeasonEpisodePattern::find_matches("Show.S01E01-E02.720p");
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_year() {
        let matches = YearPattern::find_matches("Movie.2023.720p");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "2023");
    }

    #[test]
    fn test_quality() {
        let matches = SimplePatternMatcher::find_matches(
            "Movie.720p.BluRay", &RESOLUTION_PATTERNS, MatchCategory::Quality
        );
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "720p");
    }

    #[test]
    fn test_release_group() {
        let matches = ReleaseGroupPattern::find_matches("Movie.720p.BluRay-SPARKS.mkv");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "SPARKS");
    }

    #[test]
    fn test_season_range() {
        let matches = SeasonEpisodePattern::find_matches("Show.S01-S03.Complete");
        // Should detect at least S01
        assert!(matches.iter().any(|m| 
            m.category == MatchCategory::Season || m.category == MatchCategory::Episode
        ));
    }
}
