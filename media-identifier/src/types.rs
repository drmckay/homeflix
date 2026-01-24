use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of media detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Episode,
    Unknown,
}

impl Default for MediaType {
    fn default() -> Self {
        MediaType::Unknown
    }
}

/// Categories of matched patterns, ordered by typical position in filename
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum MatchCategory {
    Title,
    Year,
    Season,
    Episode,
    EpisodeTitle,
    Quality,
    Source,
    Codec,
    Audio,
    Language,
    ReleaseGroup,
    Other,
    Noise,      // Tokens to ignore (REMASTERED, PROPER, etc.)
    Container,  // File extension
}

impl MatchCategory {
    /// Priority for conflict resolution (higher wins)
    pub fn priority(&self) -> u8 {
        match self {
            MatchCategory::Season => 100,
            MatchCategory::Episode => 100,
            MatchCategory::Year => 90,
            MatchCategory::Quality => 80,
            MatchCategory::Source => 75,
            MatchCategory::Codec => 70,
            MatchCategory::Audio => 65,
            MatchCategory::Language => 60,
            MatchCategory::ReleaseGroup => 95, // High because it's usually at the end
            MatchCategory::Container => 100,
            MatchCategory::Noise => 50,
            MatchCategory::EpisodeTitle => 40,
            MatchCategory::Title => 30,
            MatchCategory::Other => 10,
        }
    }
}

/// A match found in the input string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// Start byte position in original string
    pub start: usize,
    /// End byte position (exclusive) in original string
    pub end: usize,
    /// The matched text
    pub value: String,
    /// Normalized/parsed value (e.g., "720p" -> "720p", "S01" -> season number)
    pub raw: String,
    /// Category of this match
    pub category: MatchCategory,
    /// Confidence score (0-100)
    pub confidence: u8,
}

impl Match {
    pub fn new(start: usize, end: usize, value: impl Into<String>, category: MatchCategory) -> Self {
        let value = value.into();
        Self {
            start,
            end,
            raw: value.clone(),
            value,
            category,
            confidence: 100,
        }
    }

    pub fn with_raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = raw.into();
        self
    }

    pub fn with_confidence(mut self, confidence: u8) -> Self {
        self.confidence = confidence;
        self
    }

    /// Check if this match overlaps with another
    pub fn overlaps(&self, other: &Match) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Length of the match in bytes
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}[{}..{}]='{}'", self.category, self.start, self.end, self.value)
    }
}

/// Episode information
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeInfo {
    pub season: Option<u16>,
    pub episode: Option<u16>,
    pub episode_end: Option<u16>,    // For multi-episode (E01E02 or E01-E02)
    pub episode_title: Option<String>,
    pub absolute_episode: Option<u16>, // For anime-style numbering
}

/// Quality information
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityInfo {
    pub resolution: Option<String>,  // 720p, 1080p, 2160p, 4K
    pub source: Option<String>,      // BluRay, WEB-DL, HDTV, etc.
    pub codec: Option<String>,       // x264, x265, HEVC, etc.
    pub audio: Option<String>,       // DTS, AC3, DD+5.1, etc.
}

/// The final parsed result
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedMedia {
    /// Original input filename
    pub original: String,
    
    /// Detected media type
    pub media_type: MediaType,
    
    /// Extracted title
    pub title: Option<String>,
    
    /// Release year
    pub year: Option<u16>,
    
    /// Episode information (for TV shows)
    #[serde(flatten)]
    pub episode_info: EpisodeInfo,
    
    /// Quality information
    #[serde(flatten)]
    pub quality: QualityInfo,
    
    /// Detected languages (e.g., ["Hun", "Eng"])
    pub languages: Vec<String>,
    
    /// Release group (e.g., "FULCRUM", "MaMMuT")
    pub release_group: Option<String>,
    
    /// File container/extension
    pub container: Option<String>,
    
    /// Confidence score for the overall parse (0-100)
    pub confidence: u8,
    
    /// All matches found (for debugging)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<MatchInfo>,
}

/// Simplified match info for serialization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInfo {
    pub category: MatchCategory,
    pub value: String,
    pub start: usize,
    pub end: usize,
}

impl From<&Match> for MatchInfo {
    fn from(m: &Match) -> Self {
        MatchInfo {
            category: m.category,
            value: m.value.clone(),
            start: m.start,
            end: m.end,
        }
    }
}

/// Represents a "hole" - unmatched region in the input
#[derive(Debug, Clone)]
pub struct Hole {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

impl Hole {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Token from the tokenizer
#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub separator_before: Option<char>,
    pub separator_after: Option<char>,
}

impl Token {
    pub fn new(value: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            value: value.into(),
            start,
            end,
            separator_before: None,
            separator_after: None,
        }
    }
}
