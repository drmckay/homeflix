//! Audio Track Value Object
//!
//! Represents an audio track within a media file.
//! Migrated from models.rs to align with Clean Architecture.

use serde::{Deserialize, Serialize};

/// Audio track information from media file analysis
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Stream index within the container
    pub index: i32,
    /// Audio codec name (e.g., "aac", "ac3", "dts", "truehd")
    pub codec_name: String,
    /// Language code (e.g., "eng", "hun", "und" for undefined)
    pub language: Option<String>,
    /// Number of audio channels (e.g., 2 for stereo, 6 for 5.1)
    pub channels: Option<i32>,
}

impl AudioTrack {
    /// Creates a new audio track
    pub fn new(index: i32, codec_name: impl Into<String>) -> Self {
        Self {
            index,
            codec_name: codec_name.into(),
            language: None,
            channels: None,
        }
    }

    /// Sets the language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Sets the channel count
    pub fn with_channels(mut self, channels: i32) -> Self {
        self.channels = Some(channels);
        self
    }

    /// Returns a human-readable description of the track
    pub fn description(&self) -> String {
        let codec = &self.codec_name;
        let lang = self.language.as_deref().unwrap_or("Unknown");
        let channels = self.channels.map(|c| format!(" ({} ch)", c)).unwrap_or_default();
        format!("{} - {}{}", lang, codec.to_uppercase(), channels)
    }

    /// Returns true if this is a surround sound track (more than 2 channels)
    pub fn is_surround(&self) -> bool {
        self.channels.map(|c| c > 2).unwrap_or(false)
    }

    /// Returns true if this is a lossless audio codec
    pub fn is_lossless(&self) -> bool {
        let lossless_codecs = ["truehd", "dts-hd ma", "flac", "pcm", "alac"];
        let codec_lower = self.codec_name.to_lowercase();
        lossless_codecs.iter().any(|c| codec_lower.contains(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_track_creation() {
        let track = AudioTrack::new(0, "aac")
            .with_language("eng")
            .with_channels(2);

        assert_eq!(track.index, 0);
        assert_eq!(track.codec_name, "aac");
        assert_eq!(track.language, Some("eng".to_string()));
        assert_eq!(track.channels, Some(2));
    }

    #[test]
    fn test_description() {
        let track = AudioTrack::new(0, "ac3")
            .with_language("eng")
            .with_channels(6);

        assert_eq!(track.description(), "eng - AC3 (6 ch)");
    }

    #[test]
    fn test_is_surround() {
        let stereo = AudioTrack::new(0, "aac").with_channels(2);
        let surround = AudioTrack::new(1, "ac3").with_channels(6);
        let unknown = AudioTrack::new(2, "aac");

        assert!(!stereo.is_surround());
        assert!(surround.is_surround());
        assert!(!unknown.is_surround());
    }

    #[test]
    fn test_is_lossless() {
        let lossless = AudioTrack::new(0, "truehd");
        let lossy = AudioTrack::new(1, "ac3");

        assert!(lossless.is_lossless());
        assert!(!lossy.is_lossless());
    }
}
