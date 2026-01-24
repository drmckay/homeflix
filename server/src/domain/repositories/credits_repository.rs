//! CreditsRepository trait
//!
//! Repository interface for media credits (cast/crew) data access

use async_trait::async_trait;
use crate::shared::error::RepositoryError;

/// A single credit entry (cast or crew member)
#[derive(Debug, Clone)]
pub struct CreditEntry {
    pub person_id: i64,
    pub person_name: String,
    pub role: String,
    pub character_name: Option<String>,
    pub department: Option<String>,
    pub profile_url: Option<String>,
    pub credit_order: i32,
    pub credit_type: CreditType,
}

/// Type of credit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditType {
    Cast,
    Crew,
}

impl CreditType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreditType::Cast => "cast",
            CreditType::Crew => "crew",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cast" => CreditType::Cast,
            _ => CreditType::Crew,
        }
    }
}

/// Repository for media credits data access
#[async_trait]
pub trait CreditsRepository: Send + Sync {
    /// Gets all credits for a media item
    async fn get_credits(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError>;

    /// Gets cast members for a media item
    async fn get_cast(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError>;

    /// Gets crew members for a media item
    async fn get_crew(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError>;

    /// Saves credits for a media item (replaces existing)
    async fn save_credits(&self, media_id: i64, credits: &[CreditEntry]) -> Result<(), RepositoryError>;

    /// Checks if credits exist for a media item
    async fn has_credits(&self, media_id: i64) -> Result<bool, RepositoryError>;

    /// Deletes all credits for a media item
    async fn delete_credits(&self, media_id: i64) -> Result<(), RepositoryError>;
}
