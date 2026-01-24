//! Unit tests for ConfidenceScore value object
//!
//! Tests cover:
//! - Score creation and validation
//! - Threshold comparisons
//! - Arithmetic operations
//! - Edge cases and clamping

use homeflixd::domain::value_objects::ConfidenceScore;
use homeflixd::shared::error::DomainError;

#[test]
fn test_confidence_score_creation_valid() {
    let score = ConfidenceScore::new(0.5).expect("Should create score");
    assert_eq!(score.value(), 0.5);
}

#[test]
fn test_confidence_score_creation_minimum() {
    let score = ConfidenceScore::new(0.0).expect("Should create score");
    assert_eq!(score.value(), 0.0);
}

#[test]
fn test_confidence_score_creation_maximum() {
    let score = ConfidenceScore::new(1.0).expect("Should create score");
    assert_eq!(score.value(), 1.0);
}

#[test]
fn test_confidence_score_creation_below_minimum() {
    let result = ConfidenceScore::new(-0.1);
    assert!(matches!(result, Err(DomainError::InvalidInput(_))));
}

#[test]
fn test_confidence_score_creation_above_maximum() {
    let result = ConfidenceScore::new(1.1);
    assert!(matches!(result, Err(DomainError::InvalidInput(_))));
}

#[test]
fn test_is_high_true() {
    let score = ConfidenceScore::new(0.85).expect("Should create score");
    assert!(score.is_high());
}

#[test]
fn test_is_high_above_threshold() {
    let score = ConfidenceScore::new(0.90).expect("Should create score");
    assert!(score.is_high());
}

#[test]
fn test_is_high_false() {
    let score = ConfidenceScore::new(0.84).expect("Should create score");
    assert!(!score.is_high());
}

#[test]
fn test_is_medium_true() {
    let score = ConfidenceScore::new(0.75).expect("Should create score");
    assert!(score.is_medium());
}

#[test]
fn test_is_medium_lower_bound() {
    let score = ConfidenceScore::new(0.70).expect("Should create score");
    assert!(score.is_medium());
}

#[test]
fn test_is_medium_upper_bound() {
    let score = ConfidenceScore::new(0.84).expect("Should create score");
    assert!(score.is_medium());
}

#[test]
fn test_is_medium_false_below() {
    let score = ConfidenceScore::new(0.69).expect("Should create score");
    assert!(!score.is_medium());
}

#[test]
fn test_is_medium_false_above() {
    let score = ConfidenceScore::new(0.85).expect("Should create score");
    assert!(!score.is_medium());
}

#[test]
fn test_is_low_true() {
    let score = ConfidenceScore::new(0.65).expect("Should create score");
    assert!(score.is_low());
}

#[test]
fn test_is_low_lower_bound() {
    let score = ConfidenceScore::new(0.60).expect("Should create score");
    assert!(score.is_low());
}

#[test]
fn test_is_low_upper_bound() {
    let score = ConfidenceScore::new(0.69).expect("Should create score");
    assert!(score.is_low());
}

#[test]
fn test_is_low_false_below() {
    let score = ConfidenceScore::new(0.59).expect("Should create score");
    assert!(!score.is_low());
}

#[test]
fn test_is_low_false_above() {
    let score = ConfidenceScore::new(0.70).expect("Should create score");
    assert!(!score.is_low());
}

#[test]
fn test_is_very_low_true() {
    let score = ConfidenceScore::new(0.50).expect("Should create score");
    assert!(score.is_very_low());
}

#[test]
fn test_is_very_low_boundary() {
    let score = ConfidenceScore::new(0.59).expect("Should create score");
    assert!(score.is_very_low());
}

#[test]
fn test_is_very_low_false() {
    let score = ConfidenceScore::new(0.60).expect("Should create score");
    assert!(!score.is_very_low());
}

#[test]
fn test_add_positive() {
    let mut score = ConfidenceScore::new(0.5).expect("Should create score");
    score.add(0.2);
    assert_eq!(score.value(), 0.7);
}

#[test]
fn test_add_negative() {
    let mut score = ConfidenceScore::new(0.5).expect("Should create score");
    score.add(-0.2);
    assert_eq!(score.value(), 0.3);
}

#[test]
fn test_add_clamp_to_maximum() {
    let mut score = ConfidenceScore::new(0.9).expect("Should create score");
    score.add(0.2);
    assert_eq!(score.value(), 1.0);
}

#[test]
fn test_add_clamp_to_minimum() {
    let mut score = ConfidenceScore::new(0.1).expect("Should create score");
    score.add(-0.2);
    assert_eq!(score.value(), 0.0);
}

#[test]
fn test_subtract_positive() {
    let mut score = ConfidenceScore::new(0.7).expect("Should create score");
    score.subtract(0.2);
    assert_eq!(score.value(), 0.5);
}

#[test]
fn test_subtract_negative() {
    let mut score = ConfidenceScore::new(0.5).expect("Should create score");
    score.subtract(-0.2);
    assert_eq!(score.value(), 0.7);
}

#[test]
fn test_subtract_clamp_to_minimum() {
    let mut score = ConfidenceScore::new(0.1).expect("Should create score");
    score.subtract(0.2);
    assert_eq!(score.value(), 0.0);
}

#[test]
fn test_subtract_clamp_to_maximum() {
    let mut score = ConfidenceScore::new(0.9).expect("Should create score");
    score.subtract(-0.2);
    assert_eq!(score.value(), 1.0);
}

#[test]
fn test_default() {
    let score = ConfidenceScore::default();
    assert_eq!(score.value(), 0.0);
}

#[test]
fn test_from_f32_valid() {
    let score = ConfidenceScore::from(0.75);
    assert_eq!(score.value(), 0.75);
}

#[test]
fn test_from_f32_invalid_clamps() {
    let score = ConfidenceScore::from(1.5);
    assert_eq!(score.value(), 1.0);
}

#[test]
fn test_from_f32_negative_clamps() {
    let score = ConfidenceScore::from(-0.5);
    assert_eq!(score.value(), 0.0);
}

#[test]
fn test_into_f32() {
    let score = ConfidenceScore::new(0.85).expect("Should create score");
    let value: f32 = score.into();
    assert_eq!(value, 0.85);
}

#[test]
fn test_display() {
    let score = ConfidenceScore::new(0.857).expect("Should create score");
    assert_eq!(format!("{}", score), "0.86");
}

#[test]
fn test_display_rounding() {
    let score = ConfidenceScore::new(0.854).expect("Should create score");
    assert_eq!(format!("{}", score), "0.85");
}

#[test]
fn test_copy() {
    let score1 = ConfidenceScore::new(0.75).expect("Should create score");
    let score2 = score1;
    assert_eq!(score1.value(), score2.value());
}

#[test]
fn test_equality() {
    let score1 = ConfidenceScore::new(0.75).expect("Should create score");
    let score2 = ConfidenceScore::new(0.75).expect("Should create score");
    assert_eq!(score1, score2);
}

#[test]
fn test_inequality() {
    let score1 = ConfidenceScore::new(0.75).expect("Should create score");
    let score2 = ConfidenceScore::new(0.85).expect("Should create score");
    assert_ne!(score1, score2);
}

#[test]
fn test_thresholds_constants() {
    assert_eq!(ConfidenceScore::MIN, 0.0);
    assert_eq!(ConfidenceScore::MAX, 1.0);
    assert_eq!(ConfidenceScore::HIGH_THRESHOLD, 0.85);
    assert_eq!(ConfidenceScore::MEDIUM_THRESHOLD, 0.70);
    assert_eq!(ConfidenceScore::LOW_THRESHOLD, 0.60);
}

#[test]
fn test_multiple_operations() {
    let mut score = ConfidenceScore::new(0.5).expect("Should create score");
    score.add(0.2);
    score.subtract(0.1);
    score.add(0.15);
    assert_eq!(score.value(), 0.75);
}

#[test]
fn test_boundary_values() {
    // Test exact boundary values
    let high = ConfidenceScore::new(0.85).expect("Should create score");
    assert!(high.is_high());
    assert!(!high.is_medium());

    let medium_low = ConfidenceScore::new(0.70).expect("Should create score");
    assert!(medium_low.is_medium());
    assert!(!medium_low.is_low());

    let low = ConfidenceScore::new(0.60).expect("Should create score");
    assert!(low.is_low());
    assert!(!low.is_very_low());

    let very_low = ConfidenceScore::new(0.59).expect("Should create score");
    assert!(very_low.is_very_low());
}
