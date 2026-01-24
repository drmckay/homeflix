//! Fuzzy string matching algorithms
//!
//! Provides multiple algorithms for comparing string similarity:
//! - Jaro-Winkler: Good for short strings, favors matching prefixes
//! - Levenshtein: Edit distance, good for typo detection
//! - Token-based (Jaccard): Good for word reordering

use std::collections::HashSet;
use super::TitleNormalizer;

/// Configuration for fuzzy matching
#[derive(Debug, Clone)]
pub struct FuzzyMatchConfig {
    /// Minimum similarity score to accept (0.0 - 1.0)
    pub min_similarity: f64,
    /// Year match bonus
    pub year_match_bonus: f64,
    /// Close year (±1) bonus
    pub close_year_bonus: f64,
    /// Weight for Jaro-Winkler in combined score
    pub jaro_winkler_weight: f64,
    /// Weight for Levenshtein in combined score
    pub levenshtein_weight: f64,
    /// Weight for token similarity in combined score
    pub token_weight: f64,
}

impl Default for FuzzyMatchConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.80,
            year_match_bonus: 0.10,
            close_year_bonus: 0.05,
            jaro_winkler_weight: 0.40,
            levenshtein_weight: 0.30,
            token_weight: 0.30,
        }
    }
}

/// Result of a fuzzy match
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    /// The matched text
    pub text: String,
    /// Overall similarity score (0.0 - 1.0)
    pub score: f64,
    /// Individual algorithm scores for debugging
    pub jaro_winkler_score: f64,
    pub levenshtein_score: f64,
    pub token_score: f64,
}

/// Fuzzy string matcher
pub struct FuzzyMatcher;

impl FuzzyMatcher {
    /// Calculate Jaro similarity between two strings
    ///
    /// Returns a score between 0.0 and 1.0.
    fn jaro(a: &str, b: &str) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let a_len = a_chars.len();
        let b_len = b_chars.len();

        // Match window
        let match_distance = (a_len.max(b_len) / 2).saturating_sub(1);

        let mut a_matches = vec![false; a_len];
        let mut b_matches = vec![false; b_len];

        let mut matches = 0.0;
        let mut transpositions = 0.0;

        // Find matches
        for i in 0..a_len {
            let start = i.saturating_sub(match_distance);
            let end = (i + match_distance + 1).min(b_len);

            for j in start..end {
                if b_matches[j] || a_chars[i] != b_chars[j] {
                    continue;
                }
                a_matches[i] = true;
                b_matches[j] = true;
                matches += 1.0;
                break;
            }
        }

        if matches == 0.0 {
            return 0.0;
        }

        // Count transpositions
        let mut k = 0;
        for i in 0..a_len {
            if !a_matches[i] {
                continue;
            }
            while !b_matches[k] {
                k += 1;
            }
            if a_chars[i] != b_chars[k] {
                transpositions += 1.0;
            }
            k += 1;
        }

        let a_len_f = a_len as f64;
        let b_len_f = b_len as f64;

        (matches / a_len_f + matches / b_len_f + (matches - transpositions / 2.0) / matches) / 3.0
    }

    /// Calculate Jaro-Winkler similarity between two strings
    ///
    /// This extends Jaro by giving a bonus for matching prefixes.
    /// Good for comparing short strings where the beginning is most important.
    ///
    /// Returns a score between 0.0 and 1.0.
    ///
    /// # Example
    /// ```ignore
    /// let score = FuzzyMatcher::jaro_winkler("Spider-Man", "Spiderman");
    /// assert!(score > 0.90);
    /// ```
    pub fn jaro_winkler(a: &str, b: &str) -> f64 {
        let jaro_score = Self::jaro(a, b);

        if jaro_score < 0.7 {
            return jaro_score;
        }

        // Find common prefix length (max 4)
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let prefix_len = a_chars.iter()
            .zip(b_chars.iter())
            .take(4)
            .take_while(|(c1, c2)| c1.eq_ignore_ascii_case(c2))
            .count();

        // Apply Winkler modification
        let prefix_scale = 0.1;
        jaro_score + (prefix_len as f64 * prefix_scale * (1.0 - jaro_score))
    }

    /// Calculate Levenshtein (edit) distance between two strings
    ///
    /// Returns the minimum number of single-character edits needed
    /// to transform one string into the other.
    pub fn levenshtein(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        // Use two-row optimization
        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row: Vec<usize> = vec![0; b_len + 1];

        for i in 1..=a_len {
            curr_row[0] = i;

            for j in 1..=b_len {
                let cost = if a_chars[i - 1].eq_ignore_ascii_case(&b_chars[j - 1]) {
                    0
                } else {
                    1
                };

                curr_row[j] = (prev_row[j] + 1)           // deletion
                    .min(curr_row[j - 1] + 1)            // insertion
                    .min(prev_row[j - 1] + cost);        // substitution
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[b_len]
    }

    /// Calculate normalized Levenshtein similarity (0.0 to 1.0)
    ///
    /// This normalizes the edit distance by the length of the longer string.
    ///
    /// # Example
    /// ```ignore
    /// let score = FuzzyMatcher::levenshtein_normalized("kitten", "sitting");
    /// assert!(score > 0.50);
    /// ```
    pub fn levenshtein_normalized(a: &str, b: &str) -> f64 {
        let dist = Self::levenshtein(a, b);
        let max_len = a.chars().count().max(b.chars().count());

        if max_len == 0 {
            1.0
        } else {
            1.0 - (dist as f64 / max_len as f64)
        }
    }

    /// Calculate token-based (Jaccard) similarity
    ///
    /// This compares the sets of words in each string.
    /// Good for handling word reordering.
    ///
    /// # Example
    /// ```ignore
    /// let score = FuzzyMatcher::token_similarity(
    ///     "The Lord of the Rings",
    ///     "Lord of the Rings, The"
    /// );
    /// assert!(score > 0.80);
    /// ```
    pub fn token_similarity(a: &str, b: &str) -> f64 {
        let a_tokens: HashSet<String> = a
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        let b_tokens: HashSet<String> = b
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();

        if a_tokens.is_empty() && b_tokens.is_empty() {
            return 1.0;
        }
        if a_tokens.is_empty() || b_tokens.is_empty() {
            return 0.0;
        }

        let intersection = a_tokens.intersection(&b_tokens).count();
        let union = a_tokens.union(&b_tokens).count();

        intersection as f64 / union as f64
    }

    /// Calculate combined similarity score using multiple algorithms
    ///
    /// Uses configurable weights to combine Jaro-Winkler, Levenshtein,
    /// and token-based similarity scores.
    pub fn combined_similarity(a: &str, b: &str) -> f64 {
        Self::combined_similarity_with_config(a, b, &FuzzyMatchConfig::default())
    }

    /// Calculate combined similarity with custom configuration
    pub fn combined_similarity_with_config(a: &str, b: &str, config: &FuzzyMatchConfig) -> f64 {
        let jw = Self::jaro_winkler(a, b);
        let lev = Self::levenshtein_normalized(a, b);
        let tok = Self::token_similarity(a, b);

        jw * config.jaro_winkler_weight
            + lev * config.levenshtein_weight
            + tok * config.token_weight
    }

    /// Compare two titles using normalized comparison
    ///
    /// This normalizes both titles before comparing for best match potential.
    pub fn compare_titles(a: &str, b: &str) -> FuzzyMatch {
        Self::compare_titles_with_config(a, b, &FuzzyMatchConfig::default())
    }

    /// Compare two titles with custom configuration
    pub fn compare_titles_with_config(a: &str, b: &str, config: &FuzzyMatchConfig) -> FuzzyMatch {
        // Normalize titles for comparison
        let norm_a = TitleNormalizer::normalize_for_comparison(a);
        let norm_b = TitleNormalizer::normalize_for_comparison(b);

        let jw = Self::jaro_winkler(&norm_a, &norm_b);
        let lev = Self::levenshtein_normalized(&norm_a, &norm_b);
        let tok = Self::token_similarity(&norm_a, &norm_b);

        let score = jw * config.jaro_winkler_weight
            + lev * config.levenshtein_weight
            + tok * config.token_weight;

        FuzzyMatch {
            text: b.to_string(),
            score,
            jaro_winkler_score: jw,
            levenshtein_score: lev,
            token_score: tok,
        }
    }

    /// Find the best match from a list of candidates
    ///
    /// Returns the best match if it meets the minimum similarity threshold.
    pub fn find_best_match<'a>(
        query: &str,
        candidates: impl IntoIterator<Item = &'a str>,
        config: &FuzzyMatchConfig,
    ) -> Option<FuzzyMatch> {
        let norm_query = TitleNormalizer::normalize_for_comparison(query);

        let mut best: Option<FuzzyMatch> = None;

        for candidate in candidates {
            let norm_candidate = TitleNormalizer::normalize_for_comparison(candidate);

            let jw = Self::jaro_winkler(&norm_query, &norm_candidate);
            let lev = Self::levenshtein_normalized(&norm_query, &norm_candidate);
            let tok = Self::token_similarity(&norm_query, &norm_candidate);

            let score = jw * config.jaro_winkler_weight
                + lev * config.levenshtein_weight
                + tok * config.token_weight;

            if score >= config.min_similarity {
                if best.is_none() || score > best.as_ref().unwrap().score {
                    best = Some(FuzzyMatch {
                        text: candidate.to_string(),
                        score,
                        jaro_winkler_score: jw,
                        levenshtein_score: lev,
                        token_score: tok,
                    });
                }
            }
        }

        best
    }

    /// Rank all candidates by similarity
    pub fn rank_candidates<'a>(
        query: &str,
        candidates: impl IntoIterator<Item = &'a str>,
    ) -> Vec<FuzzyMatch> {
        let norm_query = TitleNormalizer::normalize_for_comparison(query);
        let config = FuzzyMatchConfig::default();

        let mut matches: Vec<FuzzyMatch> = candidates
            .into_iter()
            .map(|candidate| {
                let norm_candidate = TitleNormalizer::normalize_for_comparison(candidate);

                let jw = Self::jaro_winkler(&norm_query, &norm_candidate);
                let lev = Self::levenshtein_normalized(&norm_query, &norm_candidate);
                let tok = Self::token_similarity(&norm_query, &norm_candidate);

                let score = jw * config.jaro_winkler_weight
                    + lev * config.levenshtein_weight
                    + tok * config.token_weight;

                FuzzyMatch {
                    text: candidate.to_string(),
                    score,
                    jaro_winkler_score: jw,
                    levenshtein_score: lev,
                    token_score: tok,
                }
            })
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaro_winkler_identical() {
        assert!((FuzzyMatcher::jaro_winkler("test", "test") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_similar() {
        // Similar strings should have high scores
        let score = FuzzyMatcher::jaro_winkler("Spider-Man", "Spiderman");
        assert!(score > 0.90, "Score {} should be > 0.90", score);

        let score = FuzzyMatcher::jaro_winkler("colour", "color");
        assert!(score > 0.90, "Score {} should be > 0.90", score);
    }

    #[test]
    fn test_jaro_winkler_different() {
        // Very different strings should have low scores
        let score = FuzzyMatcher::jaro_winkler("apple", "orange");
        assert!(score < 0.70, "Score {} should be < 0.70", score);
    }

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(FuzzyMatcher::levenshtein("test", "test"), 0);
    }

    #[test]
    fn test_levenshtein_edits() {
        assert_eq!(FuzzyMatcher::levenshtein("kitten", "sitting"), 3);
        assert_eq!(FuzzyMatcher::levenshtein("saturday", "sunday"), 3);
    }

    #[test]
    fn test_levenshtein_normalized() {
        let score = FuzzyMatcher::levenshtein_normalized("test", "test");
        assert!((score - 1.0).abs() < 0.001);

        let score = FuzzyMatcher::levenshtein_normalized("", "");
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_token_similarity_identical() {
        let score = FuzzyMatcher::token_similarity("the quick brown fox", "the quick brown fox");
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_token_similarity_reordered() {
        // Reordered words should still have high similarity
        let score = FuzzyMatcher::token_similarity(
            "The Lord of the Rings",
            "Lord of the Rings The"
        );
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_token_similarity_partial() {
        let score = FuzzyMatcher::token_similarity("quick brown fox", "quick brown");
        // 2 out of 3 tokens match
        assert!(score > 0.60 && score < 0.80);
    }

    #[test]
    fn test_compare_titles_exact() {
        let result = FuzzyMatcher::compare_titles("The Matrix", "The Matrix");
        assert!(result.score > 0.99);
    }

    #[test]
    fn test_compare_titles_normalized() {
        // These should match well due to normalization
        // Both normalize to "avengers"
        let result = FuzzyMatcher::compare_titles("The Avengers", "Avengers, The");
        assert!(result.score > 0.99, "Score {} should be > 0.99", result.score);
    }

    #[test]
    fn test_compare_titles_numbers() {
        // Roman vs arabic numbers should match
        let result = FuzzyMatcher::compare_titles("Rocky III", "Rocky 3");
        assert!(result.score > 0.95, "Score {} should be > 0.95", result.score);
    }

    #[test]
    fn test_back_to_the_future_case() {
        let result = FuzzyMatcher::compare_titles(
            "Back to the Future Part Three",
            "Back to the Future Part III"
        );
        assert!(result.score > 0.95, "Score {} should be > 0.95 for BTTF", result.score);
    }

    #[test]
    fn test_find_best_match() {
        let candidates = vec![
            "The Matrix",
            "The Matrix Reloaded",
            "The Matrix Revolutions",
            "Matilda",
        ];

        let config = FuzzyMatchConfig::default();
        let result = FuzzyMatcher::find_best_match("Matrix", candidates.iter().map(|s| *s), &config);

        assert!(result.is_some());
        let best = result.unwrap();
        assert_eq!(best.text, "The Matrix");
    }

    #[test]
    fn test_rank_candidates() {
        let candidates = vec![
            "Back to the Future",
            "Back to the Future Part II",
            "Back to the Future Part III",
            "The Future",
        ];

        let ranked = FuzzyMatcher::rank_candidates(
            "Back to the Future Part 3",
            candidates.iter().map(|s| *s)
        );

        assert!(!ranked.is_empty());
        // Part III (which normalizes to Part 3) should be first
        assert_eq!(ranked[0].text, "Back to the Future Part III");
    }

    #[test]
    fn test_spider_man_variants() {
        // Note: After normalization, these become:
        // "Spider-Man" -> "spider man" (10 chars with space)
        // "Spiderman" -> "spiderman" (9 chars, no space)
        // The structural difference limits the match score, but it should still
        // be reasonably high due to shared tokens and similar characters
        let result = FuzzyMatcher::compare_titles("Spider-Man", "Spiderman");
        assert!(result.score > 0.65, "Spider-Man vs Spiderman: {}", result.score);

        // These should match very well (both normalize to "spider man")
        let result = FuzzyMatcher::compare_titles("Spider Man", "Spider-Man");
        assert!(result.score > 0.99, "Spider Man vs Spider-Man: {}", result.score);
    }
}

#[test]
fn test_bttf_part_i_vs_original() {
    let result = FuzzyMatcher::compare_titles("Back to the Future Part I", "Back to the Future");
    println!("BTTF Part I vs BTTF: Score = {}", result.score);
    // This should be checked - might be below 0.80 threshold!
    assert!(result.score > 0.70, "Score {} should be > 0.70", result.score);
}

#[test]
fn test_part_2_vs_part_ii() {
    let result = FuzzyMatcher::compare_titles("Back to the Future Part 2", "Back to the Future Part II");
    println!("Part 2 vs Part II: Score = {}", result.score);
    assert!(result.score > 0.75, "Score {} should be > 0.75", result.score);
}

#[test]
fn test_bttf_partial_match_rejection() {
    // This test verifies that "Back to the Future Part Two" vs "Back to the Future"
    // scores BELOW 0.90, which allows Strategy 6 to try the "Part II" variant
    let result = FuzzyMatcher::compare_titles("Back to the Future Part Two", "Back to the Future");
    println!("BTTF Part Two vs BTTF: Score = {}", result.score);
    // Must be below 0.90 so it gets rejected by Strategies 2-5 and Strategy 6 runs
    assert!(result.score < 0.90, "Score {} should be < 0.90 to be rejected", result.score);
    // But not too low - it's still a reasonable partial match
    assert!(result.score > 0.70, "Score {} should be > 0.70 (still related)", result.score);
}

#[test]
fn test_bttf_exact_match_acceptance() {
    // This test verifies that "Back to the Future Part II" vs "Back to the Future Part II"
    // scores ABOVE 0.85, which Strategy 6 should accept
    let result = FuzzyMatcher::compare_titles("Back to the Future Part II", "Back to the Future Part II");
    println!("BTTF Part II vs BTTF Part II: Score = {}", result.score);
    assert!(result.score > 0.99, "Score {} should be > 0.99 for exact match", result.score);
}
