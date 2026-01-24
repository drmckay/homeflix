//! ConfidenceService - Multi-signal confidence scoring system
//!
//! Migrated from legacy scanner/confidence.rs with full functionality:
//! - Content type confidence (TV vs Movie)
//! - Identification confidence with similarity bonuses
//! - Season/episode confidence with validation checks

use async_trait::async_trait;
use crate::domain::value_objects::{ConfidenceScore, IdentificationResult, MatchStrategy};

/// Confidence level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// High confidence (90-100%)
    High,
    /// Medium confidence (75-89%)
    Medium,
    /// Low confidence (60-74%)
    Low,
    /// Very low confidence (<60%)
    VeryLow,
}

impl ConfidenceLevel {
    /// Creates confidence level from a score
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.90 => ConfidenceLevel::High,
            s if s >= 0.75 => ConfidenceLevel::Medium,
            s if s >= 0.60 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        }
    }

    /// Returns the verification status for this confidence level
    pub fn verification_status(&self) -> &str {
        match self {
            ConfidenceLevel::High => "verified",
            ConfidenceLevel::Medium => "unverified",
            ConfidenceLevel::Low => "unverified",
            ConfidenceLevel::VeryLow => "failed",
        }
    }

    /// Returns true if this is high confidence
    pub fn is_high(&self) -> bool {
        matches!(self, ConfidenceLevel::High)
    }

    /// Returns true if verification is recommended
    pub fn needs_review(&self) -> bool {
        matches!(self, ConfidenceLevel::Low | ConfidenceLevel::VeryLow)
    }
}

/// Service for confidence score calculations
#[async_trait]
pub trait ConfidenceService: Send + Sync {
    /// Calculates confidence score for an identification result
    async fn calculate_confidence(&self, result: &IdentificationResult) -> ConfidenceScore;

    /// Adjusts confidence based on strategy
    async fn adjust_for_strategy(&self, confidence: f32, strategy: MatchStrategy) -> ConfidenceScore;

    /// Combines multiple confidence scores
    async fn combine_scores(&self, scores: &[f32]) -> ConfidenceScore;

    /// Calculates content type confidence (TV vs Movie)
    fn calculate_content_type_score(
        &self,
        has_pattern: bool,
        has_season_folder: bool,
        nfo_confirms: bool,
        duration_matches: bool,
        tmdb_validates: bool,
    ) -> f32;

    /// Calculates identification confidence with similarity bonuses
    fn calculate_identification_score(
        &self,
        strategy: &MatchStrategy,
        levenshtein_dist: Option<usize>,
        jaro_winkler: f32,
        has_year_match: bool,
        nfo_is_xml: bool,
        tmdb_validates_episode_count: bool,
        has_multiple_candidates: bool,
    ) -> f32;

    /// Calculates season/episode confidence
    fn calculate_season_episode_score(
        &self,
        has_regex_pattern: bool,
        tmdb_title_matches: bool,
        tmdb_air_date_matches: bool,
        folder_confirms_season: bool,
        season_out_of_range: bool,
        duration_inconsistent: bool,
    ) -> f32;

    /// Gets confidence level from score
    fn get_confidence_level(&self, score: f32) -> ConfidenceLevel {
        ConfidenceLevel::from_score(score)
    }
}

/// Default implementation of confidence service
pub struct DefaultConfidenceService;

impl DefaultConfidenceService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultConfidenceService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfidenceService for DefaultConfidenceService {
    async fn calculate_confidence(&self, result: &IdentificationResult) -> ConfidenceScore {
        let mut confidence = result.strategy.confidence_weight();

        // Adjust based on year match
        if result.year.is_some() {
            confidence += 0.10;
        }

        // Adjust based on season/episode presence for TV
        if result.media_type.is_episode() {
            if result.season.is_some() && result.episode.is_some() {
                confidence += 0.15;
            }
        }

        // Adjust based on title quality
        let title_words: Vec<&str> = result.title.split_whitespace().collect();
        if title_words.len() >= 2 {
            confidence += 0.05;
        }

        // Clamp to valid range
        ConfidenceScore::new(confidence.clamp(0.0, 1.0)).unwrap_or_default()
    }

    async fn adjust_for_strategy(&self, confidence: f32, strategy: MatchStrategy) -> ConfidenceScore {
        let base = confidence * strategy.confidence_weight();
        ConfidenceScore::new(base.clamp(0.0, 1.0)).unwrap_or_default()
    }

    async fn combine_scores(&self, scores: &[f32]) -> ConfidenceScore {
        if scores.is_empty() {
            return ConfidenceScore::default();
        }

        // Weighted average - give more weight to higher scores
        let weighted_sum: f32 = scores
            .iter()
            .map(|&s| s * s)
            .sum();
        let weighted_average = (weighted_sum / scores.len() as f32).sqrt();

        ConfidenceScore::new(weighted_average.clamp(0.0, 1.0)).unwrap_or_default()
    }

    fn calculate_content_type_score(
        &self,
        has_pattern: bool,
        has_season_folder: bool,
        nfo_confirms: bool,
        duration_matches: bool,
        tmdb_validates: bool,
    ) -> f32 {
        let mut score: f32 = 0.0;

        if has_pattern {
            score += 0.40;
        }
        if has_season_folder {
            score += 0.30;
        }
        if nfo_confirms {
            score += 0.20;
        }
        if duration_matches {
            score += 0.10;
        }
        if tmdb_validates {
            score += 0.10;
        }

        score.min(1.0)
    }

    fn calculate_identification_score(
        &self,
        strategy: &MatchStrategy,
        levenshtein_dist: Option<usize>,
        jaro_winkler: f32,
        has_year_match: bool,
        nfo_is_xml: bool,
        tmdb_validates_episode_count: bool,
        has_multiple_candidates: bool,
    ) -> f32 {
        let base_score: f32 = strategy.confidence_weight();
        let mut score = base_score;

        // Apply similarity bonuses if not already an ID match
        if *strategy != MatchStrategy::ImdbId && *strategy != MatchStrategy::TmdbId {
            // Exact title match (Levenshtein < 2) -> boost to 0.85 if lower
            if let Some(dist) = levenshtein_dist {
                if dist < 2 {
                    score = score.max(0.85);
                }
            }

            // Jaro-Winkler thresholds
            if jaro_winkler > 0.95 {
                score += 0.15;
            } else if jaro_winkler > 0.90 {
                score += 0.10;
            } else if jaro_winkler > 0.85 {
                score += 0.05;
            }
        }

        if has_year_match {
            score += 0.10;
        }
        if tmdb_validates_episode_count {
            score += 0.15;
        }
        if nfo_is_xml {
            score += 0.10;
        }
        if has_multiple_candidates {
            score -= 0.15;
        }

        score.min(1.0).max(0.0)
    }

    fn calculate_season_episode_score(
        &self,
        has_regex_pattern: bool,
        tmdb_title_matches: bool,
        tmdb_air_date_matches: bool,
        folder_confirms_season: bool,
        season_out_of_range: bool,
        duration_inconsistent: bool,
    ) -> f32 {
        let mut score: f32 = 0.0;

        if has_regex_pattern {
            score += 0.85;
        }
        if tmdb_title_matches {
            score += 0.15;
        }
        if tmdb_air_date_matches {
            score += 0.15;
        }
        if folder_confirms_season {
            score += 0.10;
        }
        if season_out_of_range {
            score -= 0.30;
        }
        if duration_inconsistent {
            score -= 0.10;
        }

        score.min(1.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(a: f32, b: f32) {
        assert!(
            (a - b).abs() < 0.0001,
            "assertion failed: `(left ~= right)`\n  left: `{:?}`,\n right: `{:?}`",
            a,
            b
        );
    }

    #[test]
    fn test_confidence_level_from_score() {
        assert_eq!(ConfidenceLevel::from_score(0.95), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.90), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.80), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.75), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.65), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.60), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.50), ConfidenceLevel::VeryLow);
    }

    #[test]
    fn test_confidence_level_verification_status() {
        assert_eq!(ConfidenceLevel::High.verification_status(), "verified");
        assert_eq!(ConfidenceLevel::Medium.verification_status(), "unverified");
        assert_eq!(ConfidenceLevel::Low.verification_status(), "unverified");
        assert_eq!(ConfidenceLevel::VeryLow.verification_status(), "failed");
    }

    #[test]
    fn test_calculate_content_type_score() {
        let service = DefaultConfidenceService::new();

        // Full match (capped at 1.0)
        assert_approx_eq(
            service.calculate_content_type_score(true, true, true, true, true),
            1.0,
        );

        // Pattern + Season folder
        assert_approx_eq(
            service.calculate_content_type_score(true, true, false, false, false),
            0.70,
        );

        // No match
        assert_approx_eq(
            service.calculate_content_type_score(false, false, false, false, false),
            0.0,
        );
    }

    #[test]
    fn test_calculate_identification_score() {
        let service = DefaultConfidenceService::new();

        // IMDB ID - highest confidence
        let score = service.calculate_identification_score(
            &MatchStrategy::ImdbId,
            None,
            1.0,
            true,
            false,
            false,
            false,
        );
        assert_approx_eq(score, 1.0); // 0.95 + 0.10 (year) = 1.05 -> capped at 1.0

        // FilenameOnly + NFO XML + year match
        let score = service.calculate_identification_score(
            &MatchStrategy::FilenameOnly,
            None,
            0.88,
            true,
            true,
            false,
            false,
        );
        assert_approx_eq(score, 0.85); // 0.60 + 0.05 (JW > 0.85) + 0.10 (year) + 0.10 (nfo) = 0.85
    }

    #[test]
    fn test_calculate_season_episode_score() {
        let service = DefaultConfidenceService::new();

        // Standard pattern only
        assert_approx_eq(
            service.calculate_season_episode_score(true, false, false, false, false, false),
            0.85,
        );

        // Out of range penalty
        let score = service.calculate_season_episode_score(true, false, false, false, true, false);
        assert_approx_eq(score, 0.55); // 0.85 - 0.30

        // Full confirmation
        let score = service.calculate_season_episode_score(true, true, true, true, false, false);
        assert_approx_eq(score, 1.0); // 0.85 + 0.15 + 0.15 + 0.10 = 1.25 -> capped at 1.0
    }

    #[test]
    fn test_penalty_for_multiple_candidates() {
        let service = DefaultConfidenceService::new();

        let score = service.calculate_identification_score(
            &MatchStrategy::FilenameWithYear,
            None,
            0.96,
            true,
            false,
            false,
            true, // multiple candidates
        );
        // 0.75 (base) + 0.15 (JW > 0.95) + 0.10 (year) - 0.15 (penalty) = 0.85
        assert_approx_eq(score, 0.85);
    }

    #[tokio::test]
    async fn test_combine_scores() {
        let service = DefaultConfidenceService::new();

        let combined = service.combine_scores(&[0.9, 0.8, 0.7]).await;
        // Weighted: sqrt((0.81 + 0.64 + 0.49) / 3) = sqrt(0.6467) ≈ 0.804
        assert!(combined.value() > 0.75 && combined.value() < 0.85);
    }
}
