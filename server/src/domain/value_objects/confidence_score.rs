//! ConfidenceScore value object
//!
//! Represents a confidence score from 0.0 to 1.0 for identification matches

use serde::{Deserialize, Serialize};
use std::fmt;

/// Confidence score for media identification
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceScore {
    value: f32,
}

impl ConfidenceScore {
    /// Minimum valid confidence score
    pub const MIN: f32 = 0.0;
    /// Maximum valid confidence score
    pub const MAX: f32 = 1.0;
    /// Threshold for high confidence
    pub const HIGH_THRESHOLD: f32 = 0.85;
    /// Threshold for medium confidence
    pub const MEDIUM_THRESHOLD: f32 = 0.70;
    /// Threshold for low confidence
    pub const LOW_THRESHOLD: f32 = 0.60;

    /// Creates a new confidence score
    ///
    /// # Errors
    /// Returns error if value is outside valid range [0.0, 1.0]
    pub fn new(value: f32) -> Result<Self, crate::shared::error::DomainError> {
        if value < Self::MIN || value > Self::MAX {
            return Err(crate::shared::error::DomainError::InvalidInput(format!(
                "Confidence score must be between {} and {}, got {}",
                Self::MIN,
                Self::MAX,
                value
            )));
        }
        Ok(Self { value })
    }

    /// Returns the underlying value
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Checks if confidence is high (>= 0.85)
    pub fn is_high(&self) -> bool {
        self.value >= Self::HIGH_THRESHOLD
    }

    /// Checks if confidence is medium (>= 0.70 and < 0.85)
    pub fn is_medium(&self) -> bool {
        self.value >= Self::MEDIUM_THRESHOLD && self.value < Self::HIGH_THRESHOLD
    }

    /// Checks if confidence is low (>= 0.60 and < 0.70)
    pub fn is_low(&self) -> bool {
        self.value >= Self::LOW_THRESHOLD && self.value < Self::MEDIUM_THRESHOLD
    }

    /// Checks if confidence is very low (< 0.60)
    pub fn is_very_low(&self) -> bool {
        self.value < Self::LOW_THRESHOLD
    }

    /// Adds a delta to the confidence score, clamped to valid range
    pub fn add(&mut self, delta: f32) {
        self.value = (self.value + delta).clamp(Self::MIN, Self::MAX);
    }

    /// Subtracts a delta from the confidence score, clamped to valid range
    pub fn subtract(&mut self, delta: f32) {
        self.value = (self.value - delta).clamp(Self::MIN, Self::MAX);
    }
}

impl Default for ConfidenceScore {
    fn default() -> Self {
        Self { value: 0.0 }
    }
}

impl From<f32> for ConfidenceScore {
    fn from(value: f32) -> Self {
        Self::new(value).unwrap_or_default()
    }
}

impl From<ConfidenceScore> for f32 {
    fn from(score: ConfidenceScore) -> Self {
        score.value
    }
}

impl fmt::Display for ConfidenceScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.value)
    }
}
