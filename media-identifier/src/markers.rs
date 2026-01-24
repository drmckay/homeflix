use crate::types::{Match, MatchCategory, Hole};

/// Conflict resolver for overlapping matches
pub struct ConflictResolver;

impl ConflictResolver {
    /// Resolve conflicts between overlapping matches
    /// Uses priority-based resolution: higher priority category wins
    /// For same priority, longer match wins
    /// For same length, higher confidence wins
    pub fn resolve(mut matches: Vec<Match>) -> Vec<Match> {
        if matches.is_empty() {
            return matches;
        }

        // Sort by start position, then by priority (desc), then by length (desc)
        matches.sort_by(|a, b| {
            a.start.cmp(&b.start)
                .then_with(|| b.category.priority().cmp(&a.category.priority()))
                .then_with(|| b.len().cmp(&a.len()))
                .then_with(|| b.confidence.cmp(&a.confidence))
        });

        let mut resolved: Vec<Match> = Vec::new();

        for current in matches {
            // Check if this match conflicts with any already-resolved match
            let has_conflict = resolved.iter().any(|existing| current.overlaps(existing));
            
            if !has_conflict {
                resolved.push(current);
            } else {
                // Check if we should replace an existing match
                // (current has higher priority and overlaps)
                let should_replace = resolved.iter().enumerate().find(|(_, existing)| {
                    current.overlaps(existing) 
                        && current.category.priority() > existing.category.priority()
                });
                
                if let Some((idx, _)) = should_replace {
                    resolved.remove(idx);
                    resolved.push(current);
                }
            }
        }

        // Re-sort by position after resolution
        resolved.sort_by_key(|m| m.start);
        resolved
    }
}

/// Hole finder - identifies unmatched regions in the input
///
/// This is part of the Rebulk-inspired algorithm. In Rebulk:
/// 1. Pattern matching finds all known patterns
/// 2. Holes are the unmatched regions between patterns
/// 3. The first hole typically contains the title
/// 4. Holes after episode markers may contain episode titles
///
/// Currently used for:
/// - Debug output (showing what wasn't matched)
/// - Testing (verifying hole detection works)
///
/// Future potential uses:
/// - Alternative title extraction algorithm
/// - Finding metadata in multiple unmatched regions
/// - Detecting unparsed content for manual review
pub struct HoleFinder;

impl HoleFinder {
    /// Find all holes (unmatched regions) in the input string.
    ///
    /// A "hole" is any non-whitespace region that isn't covered by a pattern match.
    /// Holes are useful for identifying the title and episode titles.
    ///
    /// # Example
    /// ```ignore
    /// // Input: "Dark.Matter.2015.S01E05.720p.mkv"
    /// // Matches: [Year("2015"), Episode("S01E05"), Quality("720p")]
    /// // Holes: ["Dark.Matter.", "."]
    /// //        ^^^^^^^^^^^^^^ This is the title hole
    /// ```
    pub fn find_holes(input: &str, matches: &[Match]) -> Vec<Hole> {
        let mut holes = Vec::new();
        let mut pos = 0;

        // Get sorted matches by position
        let mut sorted_matches: Vec<_> = matches.iter().collect();
        sorted_matches.sort_by_key(|m| m.start);

        for m in sorted_matches {
            if m.start > pos {
                let hole_text = &input[pos..m.start];
                if !hole_text.trim().is_empty() {
                    holes.push(Hole {
                        start: pos,
                        end: m.start,
                        text: hole_text.to_string(),
                    });
                }
            }
            pos = m.end.max(pos);
        }

        // Final hole after last match
        if pos < input.len() {
            let hole_text = &input[pos..];
            if !hole_text.trim().is_empty() {
                holes.push(Hole {
                    start: pos,
                    end: input.len(),
                    text: hole_text.to_string(),
                });
            }
        }

        holes
    }
}

/// Title extractor - determines the title from holes and match positions
pub struct TitleExtractor;

impl TitleExtractor {
    /// Extract the title from the input based on matched patterns
    ///
    /// The algorithm:
    /// 1. Check for prefix release group (e.g., "fulcrum-title...")
    /// 2. Find the first "significant" marker (year, season/episode, quality)
    /// 3. Everything between prefix and marker is likely the title
    /// 4. Clean up separators and normalize
    pub fn extract_title(input: &str, matches: &[Match]) -> Option<String> {
        if matches.is_empty() {
            // No matches - the whole thing might be a title
            return Some(Self::clean_title(input));
        }

        // Check for prefix release group at the beginning
        let prefix_release_group = matches.iter()
            .find(|m| m.category == MatchCategory::ReleaseGroup && m.start == 0);

        // Title starts after prefix release group (if any)
        let title_start = prefix_release_group
            .map(|m| m.end)
            .unwrap_or(0);

        // Find the first significant marker position (after title_start)
        let first_significant = matches.iter()
            .filter(|m| Self::is_title_boundary(m) && m.start > title_start)
            .min_by_key(|m| m.start);

        let title_end = first_significant
            .map(|m| m.start)
            .unwrap_or(input.len());

        if title_end <= title_start {
            return None;
        }

        // Extract and clean the title
        let raw_title = &input[title_start..title_end];
        let cleaned = Self::clean_title(raw_title);

        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    }

    /// Extract episode title (text between season/episode marker and quality markers)
    pub fn extract_episode_title(input: &str, matches: &[Match]) -> Option<String> {
        // Find season/episode match
        let episode_match = matches.iter()
            .find(|m| m.category == MatchCategory::Episode || m.category == MatchCategory::Season);

        // Find first quality/source/codec marker after episode
        let episode_end = episode_match.map(|m| m.end).unwrap_or(0);
        
        let next_marker = matches.iter()
            .filter(|m| {
                m.start > episode_end && matches!(
                    m.category,
                    MatchCategory::Quality | MatchCategory::Source | MatchCategory::Codec 
                    | MatchCategory::Audio | MatchCategory::Language | MatchCategory::Noise
                    | MatchCategory::ReleaseGroup
                )
            })
            .min_by_key(|m| m.start);

        if let (Some(ep), Some(next)) = (episode_match, next_marker) {
            if next.start > ep.end {
                let raw_title = &input[ep.end..next.start];
                let cleaned = Self::clean_title(raw_title);
                if !cleaned.is_empty() && cleaned.len() > 1 {
                    return Some(cleaned);
                }
            }
        }

        None
    }

    /// Determine if a match type serves as a title boundary
    fn is_title_boundary(m: &Match) -> bool {
        matches!(
            m.category,
            MatchCategory::Year 
            | MatchCategory::Season 
            | MatchCategory::Episode 
            | MatchCategory::Quality
            | MatchCategory::Source
            | MatchCategory::Codec
            | MatchCategory::Noise
        )
    }

    /// Clean up a raw title string
    fn clean_title(raw: &str) -> String {
        // Replace separators with spaces
        let mut cleaned = raw
            .replace('.', " ")
            .replace('_', " ")
            .replace('-', " ");
        
        // Handle special cases
        // Restore hyphenated words that should stay together
        cleaned = Self::restore_hyphenated_titles(&cleaned);
        
        // Trim and collapse multiple spaces
        cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        
        // Trim trailing punctuation
        cleaned = cleaned.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        
        cleaned
    }

    /// Restore known hyphenated title patterns
    fn restore_hyphenated_titles(input: &str) -> String {
        let mut result = input.to_string();
        
        // Pattern: Single capital/short word followed by space and short alphanumeric
        // This catches things like "SG 1" -> "SG-1"
        let patterns = [
            ("SG 1", "SG-1"),
            ("X Files", "X-Files"),
            ("X Men", "X-Men"),
            ("Spider Man", "Spider-Man"),
            ("Ant Man", "Ant-Man"),
            ("Iron Man", "Iron Man"),  // Not hyphenated!
        ];
        
        for (from, to) in patterns {
            result = result.replace(from, to);
        }
        
        result
    }
}

/// Post-processor for fine-tuning extracted data
pub struct PostProcessor;

impl PostProcessor {
    /// Parse the raw season/episode string into components
    pub fn parse_episode_raw(raw: &str) -> (Option<u16>, Option<u16>, Option<u16>) {
        let parts: Vec<&str> = raw.split('|').collect();
        if parts.len() != 3 {
            return (None, None, None);
        }

        let season = parts[0].parse::<u16>().ok().filter(|&n| n > 0);
        let episode = parts[1].parse::<u16>().ok().filter(|&n| n > 0);
        let episode_end = parts[2].parse::<u16>().ok().filter(|&n| n > 0);

        // Handle season range (e.g., "1-3")
        if parts[0].contains('-') {
            // Season range like "1-3"
            let range_parts: Vec<&str> = parts[0].split('-').collect();
            if range_parts.len() == 2 {
                let start = range_parts[0].parse::<u16>().ok();
                return (start, None, None); // Return start season for ranges
            }
        }

        (season, episode, episode_end)
    }

    /// Clean up language matches (deduplicate, normalize)
    pub fn normalize_languages(matches: &[Match]) -> Vec<String> {
        let mut languages: Vec<String> = matches
            .iter()
            .filter(|m| m.category == MatchCategory::Language)
            .map(|m| m.value.clone())
            .collect();
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        languages.retain(|lang| seen.insert(lang.clone()));
        
        languages
    }

    /// Determine confidence score for the overall parse
    pub fn calculate_confidence(matches: &[Match], has_title: bool) -> u8 {
        let mut confidence = 50u8;

        // Boost for having a title
        if has_title {
            confidence = confidence.saturating_add(20);
        }

        // Boost for having season/episode (strong TV indicator)
        if matches.iter().any(|m| m.category == MatchCategory::Episode) {
            confidence = confidence.saturating_add(15);
        }

        // Boost for having year (strong movie indicator if no episode)
        if matches.iter().any(|m| m.category == MatchCategory::Year) {
            confidence = confidence.saturating_add(10);
        }

        // Boost for having quality markers (indicates proper release)
        if matches.iter().any(|m| matches!(m.category, 
            MatchCategory::Quality | MatchCategory::Source | MatchCategory::Codec
        )) {
            confidence = confidence.saturating_add(5);
        }

        confidence.min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_match(start: usize, end: usize, cat: MatchCategory) -> Match {
        Match {
            start,
            end,
            value: "test".into(),
            raw: "test".into(),
            category: cat,
            confidence: 100,
        }
    }

    #[test]
    fn test_conflict_resolution_no_overlap() {
        let matches = vec![
            make_match(0, 5, MatchCategory::Title),
            make_match(10, 15, MatchCategory::Year),
        ];
        let resolved = ConflictResolver::resolve(matches);
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn test_conflict_resolution_overlap() {
        let matches = vec![
            make_match(0, 10, MatchCategory::Title),
            make_match(5, 15, MatchCategory::Year),  // Overlaps, higher priority
        ];
        let resolved = ConflictResolver::resolve(matches);
        // Year should win due to higher priority
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].category, MatchCategory::Year);
    }

    #[test]
    fn test_find_holes() {
        let input = "Hello.World.2023.720p";
        let matches = vec![
            make_match(12, 16, MatchCategory::Year),    // "2023"
            make_match(17, 21, MatchCategory::Quality), // "720p"
        ];
        let holes = HoleFinder::find_holes(input, &matches);
        // Should find holes before matches and between them
        assert!(!holes.is_empty());
        // First hole should contain "Hello.World."
        assert!(holes[0].text.contains("Hello"));
    }

    #[test]
    fn test_clean_title() {
        assert_eq!(TitleExtractor::clean_title("Dark.Matter"), "Dark Matter");
        assert_eq!(TitleExtractor::clean_title("Home_Alone"), "Home Alone");
    }

    #[test]
    fn test_episode_raw_parse() {
        let (s, e, ee) = PostProcessor::parse_episode_raw("1|5|0");
        assert_eq!(s, Some(1));
        assert_eq!(e, Some(5));
        assert_eq!(ee, None);

        let (s, e, ee) = PostProcessor::parse_episode_raw("2|1|3");
        assert_eq!(s, Some(2));
        assert_eq!(e, Some(1));
        assert_eq!(ee, Some(3));
    }
}
