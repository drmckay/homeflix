//! FpcalcAdapter - Audio fingerprint generation using fpcalc CLI
//!
//! Uses Chromaprint's fpcalc tool to generate audio fingerprints for media files.
//! Fingerprints can be used to:
//! - Track which audio tracks have been transcribed
//! - Avoid duplicate transcription work
//! - Link subtitles to specific audio tracks

use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use serde::{Deserialize, Serialize};
use crate::shared::error::FingerprintError;

/// Audio fingerprint result from fpcalc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFingerprint {
    /// Raw fingerprint data (array of unsigned 32-bit integers)
    /// Note: fpcalc outputs unsigned integers that can exceed i32 max
    pub fingerprint: Vec<u32>,
    /// Duration in seconds
    pub duration: f64,
    /// Audio track index this fingerprint belongs to
    pub audio_track_index: usize,
}

/// fpcalc JSON output structure
#[derive(Debug, Deserialize)]
struct FpcalcOutput {
    fingerprint: Vec<u32>,
    duration: f64,
}

/// fpcalc adapter for audio fingerprinting
///
/// Uses the fpcalc CLI tool (part of libchromaprint-tools) to generate
/// audio fingerprints from video files.
pub struct FpcalcAdapter {
    /// Path to fpcalc binary
    cli_path: String,
    /// Timeout for fpcalc execution
    timeout: Duration,
}

impl FpcalcAdapter {
    /// Creates a new FpcalcAdapter
    ///
    /// # Arguments
    /// * `timeout` - Timeout for fpcalc execution
    pub fn new(timeout: Duration) -> Self {
        Self {
            cli_path: "fpcalc".to_string(),
            timeout,
        }
    }

    /// Creates an FpcalcAdapter with custom CLI path
    pub fn with_cli_path(cli_path: String, timeout: Duration) -> Self {
        Self { cli_path, timeout }
    }

    /// Checks if fpcalc is available
    pub async fn is_available(&self) -> bool {
        Command::new(&self.cli_path)
            .arg("-version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Generates audio fingerprint for a specific audio track in a video file
    ///
    /// # Arguments
    /// * `video_path` - Path to the video file
    /// * `audio_track_index` - Index of the audio track to fingerprint (0-based)
    ///
    /// # Returns
    /// AudioFingerprint containing the fingerprint data and duration
    pub async fn fingerprint(
        &self,
        video_path: &str,
        audio_track_index: usize,
    ) -> Result<AudioFingerprint, FingerprintError> {
        // Extract the audio track to a temporary WAV file
        let temp_audio = self.extract_audio_track(video_path, audio_track_index).await?;

        // Run fpcalc on the extracted audio
        let result = self.run_fpcalc(&temp_audio).await;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_audio).await;

        let output = result?;

        Ok(AudioFingerprint {
            fingerprint: output.fingerprint,
            duration: output.duration,
            audio_track_index,
        })
    }

    /// Extracts a specific audio track from video to a temporary WAV file
    async fn extract_audio_track(
        &self,
        video_path: &str,
        track_index: usize,
    ) -> Result<String, FingerprintError> {
        let temp_path = format!(
            "/tmp/audio_fp_{}_{}.wav",
            uuid::Uuid::new_v4(),
            track_index
        );

        let output = timeout(self.timeout, async {
            Command::new("ffmpeg")
                .args([
                    "-i", video_path,
                    "-map", &format!("0:a:{}", track_index),
                    "-ar", "44100",  // 44.1kHz for Chromaprint
                    "-ac", "2",      // Stereo
                    "-t", "120",     // Only first 120 seconds for fingerprint
                    "-y",            // Overwrite if exists
                    &temp_path,
                ])
                .output()
                .await
        })
        .await
        .map_err(|_| FingerprintError::Timeout("Audio extraction timed out".into()))?
        .map_err(FingerprintError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FingerprintError::AudioExtractionFailed(stderr.to_string()));
        }

        Ok(temp_path)
    }

    /// Runs fpcalc on an audio file and returns the fingerprint
    async fn run_fpcalc(&self, audio_path: &str) -> Result<FpcalcOutput, FingerprintError> {
        let output = timeout(self.timeout, async {
            Command::new(&self.cli_path)
                .args([
                    "-raw",          // Output raw fingerprint as integers
                    "-json",         // JSON output format
                    "-length", "120", // Fingerprint length (seconds)
                    audio_path,
                ])
                .output()
                .await
        })
        .await
        .map_err(|_| FingerprintError::Timeout("fpcalc execution timed out".into()))?;

        let output = output.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FingerprintError::FpcalcNotFound
            } else {
                FingerprintError::Io(e)
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FingerprintError::FingerprintFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout)
            .map_err(|e| FingerprintError::ParseError(e.to_string()))
    }

    /// Compares two fingerprints and returns a similarity score (0.0 - 1.0)
    ///
    /// Uses bit-level comparison of Chromaprint integers.
    /// Higher score means more similar audio content.
    pub fn compare_fingerprints(fp1: &AudioFingerprint, fp2: &AudioFingerprint) -> f64 {
        if fp1.fingerprint.is_empty() || fp2.fingerprint.is_empty() {
            return 0.0;
        }

        // Compare using Hamming distance
        let len = fp1.fingerprint.len().min(fp2.fingerprint.len());
        let mut matching_bits = 0u64;
        let total_bits = (len * 32) as u64;

        for i in 0..len {
            let xor: u32 = fp1.fingerprint[i] ^ fp2.fingerprint[i];
            // Count non-matching bits
            matching_bits += 32 - xor.count_ones() as u64;
        }

        matching_bits as f64 / total_bits as f64
    }

    /// Converts fingerprint to hex string for storage
    pub fn fingerprint_to_hex(fingerprint: &AudioFingerprint) -> String {
        fingerprint
            .fingerprint
            .iter()
            .map(|n| format!("{:08x}", n))
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for FpcalcAdapter {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_identical_fingerprints() {
        let fp1 = AudioFingerprint {
            fingerprint: vec![1, 2, 3, 4, 5],
            duration: 120.0,
            audio_track_index: 0,
        };
        let fp2 = fp1.clone();

        let similarity = FpcalcAdapter::compare_fingerprints(&fp1, &fp2);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compare_different_fingerprints() {
        let fp1 = AudioFingerprint {
            fingerprint: vec![0, 0, 0, 0],
            duration: 120.0,
            audio_track_index: 0,
        };
        let fp2 = AudioFingerprint {
            fingerprint: vec![u32::MAX, u32::MAX, u32::MAX, u32::MAX], // All bits set
            duration: 120.0,
            audio_track_index: 1,
        };

        let similarity = FpcalcAdapter::compare_fingerprints(&fp1, &fp2);
        assert!(similarity < 0.5);
    }

    #[test]
    fn test_fingerprint_to_hex() {
        let fp = AudioFingerprint {
            fingerprint: vec![255, 256, 65535],
            duration: 60.0,
            audio_track_index: 0,
        };

        let hex = FpcalcAdapter::fingerprint_to_hex(&fp);
        assert_eq!(hex, "000000ff000001000000ffff");
    }
}
