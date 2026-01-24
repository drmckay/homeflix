//! VerificationStatus value object
//!
//! Represents the verification status of identified media

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Verification status for media identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Media has been verified with high confidence
    Verified,
    /// Media is unverified (low or medium confidence)
    Unverified,
    /// Identification failed
    Failed,
    /// Requires manual review
    ManualReview,
}

impl VerificationStatus {
    /// Returns string representation of verification status
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationStatus::Verified => "verified",
            VerificationStatus::Unverified => "unverified",
            VerificationStatus::Failed => "failed",
            VerificationStatus::ManualReview => "manual_review",
        }
    }

    /// Checks if this status indicates verified
    pub fn is_verified(&self) -> bool {
        matches!(self, VerificationStatus::Verified)
    }

    /// Checks if this status indicates failed
    pub fn is_failed(&self) -> bool {
        matches!(self, VerificationStatus::Failed)
    }

    /// Checks if this status requires manual review
    pub fn requires_manual_review(&self) -> bool {
        matches!(self, VerificationStatus::ManualReview)
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VerificationStatus {
    type Err = crate::shared::error::DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "verified" => Ok(VerificationStatus::Verified),
            "unverified" => Ok(VerificationStatus::Unverified),
            "failed" => Ok(VerificationStatus::Failed),
            "manual_review" | "manualreview" => Ok(VerificationStatus::ManualReview),
            _ => Err(crate::shared::error::DomainError::InvalidInput(format!(
                "Invalid verification status: {}",
                s
            ))),
        }
    }
}
