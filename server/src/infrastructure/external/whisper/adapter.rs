//! WhisperAdapter - Speech-to-text using whisper.cpp CLI
//!
//! Uses whisper.cpp's whisper-cli tool to transcribe audio from video files.
//! Outputs SRT format subtitles with accurate timestamps.

use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use serde::{Deserialize, Serialize};
use crate::shared::error::SpeechToTextError;

/// Transcription segment with timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Transcribed text
    pub text: String,
}

/// Full transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// All transcribed segments
    pub segments: Vec<TranscriptionSegment>,
    /// Detected or specified language code
    pub detected_language: Option<String>,
    /// Total audio duration in seconds
    pub duration_seconds: f64,
    /// Raw SRT content
    pub srt_content: String,
}

/// Whisper.cpp adapter for speech-to-text
///
/// Uses the whisper-cli binary to transcribe audio from video files.
/// Follows the same CLI wrapper pattern as FFprobeAdapter and FpcalcAdapter.
pub struct WhisperAdapter {
    /// Path to whisper model file (.bin)
    model_path: PathBuf,
    /// Path to whisper-cli binary
    cli_path: String,
    /// Timeout for transcription (can be long for full movies)
    timeout: Duration,
}

impl WhisperAdapter {
    /// Creates a new WhisperAdapter
    ///
    /// # Arguments
    /// * `model_path` - Path to the whisper model file (e.g., ggml-small.bin)
    /// * `timeout` - Timeout for transcription operations
    pub fn new(model_path: PathBuf, timeout: Duration) -> Self {
        Self {
            model_path,
            cli_path: "whisper-cli".to_string(),
            timeout,
        }
    }

    /// Creates a WhisperAdapter with custom CLI path
    pub fn with_cli_path(model_path: PathBuf, cli_path: String, timeout: Duration) -> Self {
        Self {
            model_path,
            cli_path,
            timeout,
        }
    }

    /// Checks if whisper-cli is available
    pub async fn is_available(&self) -> bool {
        Command::new(&self.cli_path)
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Checks if the model file exists
    pub fn model_exists(&self) -> bool {
        self.model_path.exists()
    }

    /// Transcribes audio from a video file
    ///
    /// # Arguments
    /// * `video_path` - Path to the video file
    /// * `audio_track_index` - Index of the audio track to transcribe (0-based)
    /// * `language` - Optional language code (e.g., "en", "hu"). None for auto-detect.
    ///
    /// # Returns
    /// TranscriptionResult containing segments, detected language, and raw SRT
    pub async fn transcribe(
        &self,
        video_path: &str,
        audio_track_index: usize,
        language: Option<&str>,
    ) -> Result<TranscriptionResult, SpeechToTextError> {
        // Extract audio track to temporary WAV file (16kHz mono for Whisper)
        let temp_audio = self.extract_audio(video_path, audio_track_index).await?;

        // Run whisper-cli
        let result = self.run_whisper(&temp_audio, language).await;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_audio).await;

        result
    }

    /// Extracts audio from video to a temporary WAV file
    ///
    /// Whisper requires 16kHz mono audio for best results.
    async fn extract_audio(
        &self,
        video_path: &str,
        audio_track_index: usize,
    ) -> Result<String, SpeechToTextError> {
        let temp_path = format!(
            "/tmp/whisper_audio_{}_{}.wav",
            uuid::Uuid::new_v4(),
            audio_track_index
        );

        let output = timeout(Duration::from_secs(300), async {
            Command::new("ffmpeg")
                .args([
                    "-i", video_path,
                    "-map", &format!("0:a:{}", audio_track_index),
                    "-ar", "16000",   // 16kHz for Whisper
                    "-ac", "1",       // Mono
                    "-c:a", "pcm_s16le",  // 16-bit PCM
                    "-y",             // Overwrite if exists
                    &temp_path,
                ])
                .output()
                .await
        })
        .await
        .map_err(|_| SpeechToTextError::Timeout("Audio extraction timed out".into()))?
        .map_err(SpeechToTextError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpeechToTextError::AudioExtractionFailed(stderr.to_string()));
        }

        Ok(temp_path)
    }

    /// Runs whisper-cli on an audio file
    async fn run_whisper(
        &self,
        audio_path: &str,
        language: Option<&str>,
    ) -> Result<TranscriptionResult, SpeechToTextError> {
        // Build command arguments
        let mut args = vec![
            "-m".to_string(), self.model_path.to_string_lossy().to_string(),
            "-f".to_string(), audio_path.to_string(),
            "-osrt".to_string(),  // Output SRT format
            "-of".to_string(), audio_path.to_string(),  // Output file base name
            // Anti-hallucination parameters
            "-et".to_string(), "2.4".to_string(),   // Entropy threshold (lower = stricter)
            "-lpt".to_string(), "-0.5".to_string(), // Log probability threshold (higher = stricter)
            "--max-context".to_string(), "224".to_string(), // Limit context to prevent loops
            "-bo".to_string(), "5".to_string(),     // Best-of candidates
            "-bs".to_string(), "5".to_string(),     // Beam size for beam search
        ];

        // Add language if specified
        if let Some(lang) = language {
            args.push("-l".to_string());
            args.push(lang.to_string());
        }

        let output = timeout(self.timeout, async {
            Command::new(&self.cli_path)
                .args(&args)
                .output()
                .await
        })
        .await
        .map_err(|_| SpeechToTextError::Timeout("Whisper transcription timed out".into()))?;

        let output = output.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpeechToTextError::WhisperNotFound
            } else {
                SpeechToTextError::Io(e)
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpeechToTextError::TranscriptionFailed(stderr.to_string()));
        }

        // Read the generated SRT file
        let srt_path = format!("{}.srt", audio_path);
        let srt_content = tokio::fs::read_to_string(&srt_path)
            .await
            .map_err(|e| SpeechToTextError::TranscriptionFailed(
                format!("Failed to read SRT output: {}", e)
            ))?;

        // Clean up the SRT file
        let _ = tokio::fs::remove_file(&srt_path).await;

        // Parse SRT content to segments
        let segments = parse_srt(&srt_content)?;

        // Filter out non-speech annotations like (dramatic music), [MUSIC], etc.
        let segments = filter_non_speech_annotations(segments);

        // Detect and filter hallucinated segments (repeated phrases)
        let segments = filter_hallucinations(segments);

        // Split long segments for better readability (max 4s or 70 chars)
        let segments = split_long_segments(segments);

        // Normalize capitalization (capitalize after sentence endings, not after ...)
        let segments = normalize_capitalization(segments);

        // Format text into 2 lines for better on-screen display (~38 chars/line)
        let segments = format_subtitle_lines(segments);

        // Calculate total duration from last segment
        let duration_seconds = segments
            .last()
            .map(|s| s.end_time)
            .unwrap_or(0.0);

        // Log warnings about large gaps in transcription (informational only)
        detect_gaps(&segments, duration_seconds);

        // Try to detect language from whisper output
        let detected_language = language.map(String::from).or_else(|| {
            // Whisper outputs detected language in stderr
            let stderr = String::from_utf8_lossy(&output.stderr);
            extract_detected_language(&stderr)
        });

        Ok(TranscriptionResult {
            segments,
            detected_language,
            duration_seconds,
            srt_content,
        })
    }
}

/// Parses SRT content into TranscriptionSegments
fn parse_srt(srt_content: &str) -> Result<Vec<TranscriptionSegment>, SpeechToTextError> {
    let mut segments = Vec::new();
    let mut lines = srt_content.lines().peekable();

    while lines.peek().is_some() {
        // Skip sequence number
        if let Some(line) = lines.next() {
            if line.trim().is_empty() {
                continue;
            }
            // If this is a number, it's a sequence number - skip to timestamp
            if line.trim().parse::<u32>().is_ok() {
                // Next line should be timestamp
            } else if line.contains("-->") {
                // This is actually the timestamp line
                if let Some((start, end)) = parse_timestamp_line(line) {
                    let text = collect_text_lines(&mut lines);
                    if !text.is_empty() {
                        segments.push(TranscriptionSegment {
                            start_time: start,
                            end_time: end,
                            text,
                        });
                    }
                }
                continue;
            }
        }

        // Parse timestamp line
        if let Some(timestamp_line) = lines.next() {
            if let Some((start, end)) = parse_timestamp_line(timestamp_line) {
                let text = collect_text_lines(&mut lines);
                if !text.is_empty() {
                    segments.push(TranscriptionSegment {
                        start_time: start,
                        end_time: end,
                        text,
                    });
                }
            }
        }
    }

    Ok(segments)
}

/// Parses a timestamp line like "00:00:01,000 --> 00:00:04,500"
fn parse_timestamp_line(line: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return None;
    }

    let start = parse_timestamp(parts[0].trim())?;
    let end = parse_timestamp(parts[1].trim())?;
    Some((start, end))
}

/// Parses a single timestamp like "00:00:01,000" to seconds
fn parse_timestamp(ts: &str) -> Option<f64> {
    let ts_clean = ts.split_whitespace().next().unwrap_or(ts);
    let parts: Vec<&str> = ts_clean.split(':').collect();

    if parts.len() != 3 {
        return None;
    }

    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;

    // Handle both ',' and '.' for milliseconds separator
    let sec_parts: Vec<&str> = parts[2].split(|c| c == ',' || c == '.').collect();
    let seconds: f64 = sec_parts.first()?.parse().ok()?;
    let millis: f64 = sec_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    Some(hours * 3600.0 + minutes * 60.0 + seconds + millis / 1000.0)
}

/// Collects text lines until empty line or end of input
fn collect_text_lines<'a, I>(lines: &mut std::iter::Peekable<I>) -> String
where
    I: Iterator<Item = &'a str>,
{
    let mut text_parts = Vec::new();

    while let Some(line) = lines.peek() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            lines.next(); // consume empty line
            break;
        }
        // Stop if this looks like a sequence number for next subtitle
        if trimmed.parse::<u32>().is_ok() {
            break;
        }
        text_parts.push(trimmed.to_string());
        lines.next();
    }

    text_parts.join(" ")
}

/// Detects and filters out hallucinated segments
///
/// Whisper can hallucinate when:
/// - Audio is unclear, silent, or contains only music
/// - The model gets stuck in a loop repeating the same phrase
///
/// This function detects:
/// 1. Repeated consecutive segments (same text appearing 3+ times in a row)
/// 2. Common hallucination phrases like "Thank you for watching", "Subscribe", etc.
/// 3. Suspiciously long repetition patterns across the transcript
fn filter_hallucinations(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    if segments.is_empty() {
        return segments;
    }

    // Common hallucination phrases (case insensitive)
    // These are YouTube-style phrases that Whisper often hallucinates
    let hallucination_phrases = [
        "thank you for watching",
        "thanks for watching",
        "please subscribe",
        "like and subscribe",
        "see you next time",
        "see you in the next",
        "don't forget to subscribe",
        "hit the bell",
        "leave a comment",
        "köszönöm a figyelmet",  // Hungarian: thank you for attention
        "köszönöm hogy megnézted", // Hungarian: thanks for watching
        "iratkozz fel",  // Hungarian: subscribe
    ];

    // Normalize text for comparison
    fn normalize(text: &str) -> String {
        text.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    // Count occurrences of each normalized text
    let mut text_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for segment in &segments {
        let normalized = normalize(&segment.text);
        if !normalized.is_empty() {
            *text_counts.entry(normalized).or_insert(0) += 1;
        }
    }

    // Find texts that appear too many times (likely hallucinations)
    // If a phrase appears more than 5 times OR more than 10% of all segments, it's suspicious
    let total_segments = segments.len();
    let repetition_threshold = std::cmp::max(5, total_segments / 10);
    let repeated_texts: std::collections::HashSet<String> = text_counts
        .iter()
        .filter(|(_, &count)| count > repetition_threshold)
        .map(|(text, _)| text.clone())
        .collect();

    // Filter segments
    let original_len = segments.len();
    let mut result = Vec::with_capacity(original_len);
    let mut consecutive_same_count = 0;
    let mut last_normalized = String::new();

    for segment in segments {
        let normalized = normalize(&segment.text);

        // Skip common hallucination phrases
        if hallucination_phrases.iter().any(|phrase| normalized.contains(phrase)) {
            tracing::debug!("Filtering hallucination phrase: {}", segment.text);
            continue;
        }

        // Skip texts that appear too many times across the transcript
        if repeated_texts.contains(&normalized) {
            tracing::debug!("Filtering repeated hallucination: {}", segment.text);
            continue;
        }

        // Track consecutive same texts (allow up to 2 repeats which can be legit)
        if normalized == last_normalized && !normalized.is_empty() {
            consecutive_same_count += 1;
            if consecutive_same_count > 2 {
                tracing::debug!("Filtering consecutive repeat #{}: {}", consecutive_same_count, segment.text);
                continue;
            }
        } else {
            consecutive_same_count = 1;
            last_normalized = normalized;
        }

        result.push(segment);
    }

    // If we filtered more than 90% of segments, something is very wrong
    // This likely means the audio track has no speech - return warning segment
    if result.len() < original_len / 10 && original_len > 10 {
        tracing::warn!(
            "Filtered {}% of segments as hallucinations - audio may not contain speech",
            100 - (result.len() * 100 / original_len)
        );
    }

    result
}

/// Filters out non-speech annotations from transcription segments
///
/// Removes segments that consist entirely of non-speech annotations like:
/// - (dramatic music), (sighs), (laughs), (coughs)
/// - Hungarian: (drámai zene), (zene), (sóhaj), (nevetés)
/// - [MUSIC], [APPLAUSE], [INAUDIBLE]
/// - ♪ music notes ♪
///
/// Also cleans inline annotations from segments that have both speech and annotations.
fn filter_non_speech_annotations(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    use regex::Regex;

    // Common non-speech keywords in multiple languages
    // These are typically Whisper hallucinations for background sounds
    let non_speech_keywords = Regex::new(
        r"(?i)\b(music|zene|dramatic|drámai|sighs?|sóhaj|laughs?|nevet|coughs?|köhög|applause|taps|silence|csend|inaudible|breathing|lélegzik|footsteps|léptek|door|ajtó|phone|telefon|bell|csengő|knock|kopog|thunder|mennydörgés|rain|eső|wind|szél|screams?|sikolt|crying|sír|whispers?|suttog)\b"
    ).unwrap();

    // Pattern matches: (anything), [anything], ♪...♪, *anything*
    let annotation_pattern = Regex::new(
        r"(?i)\([^)]*\)|\[[^\]]*\]|♪[^♪]*♪|\*[^*]*\*"
    ).unwrap();

    // Common non-speech only patterns (case insensitive)
    let non_speech_only = Regex::new(
        r"(?i)^\s*(\([^)]+\)|\[[^\]]+\]|♪[^♪]*♪|\*[^*]+\*|\s)*\s*$"
    ).unwrap();

    segments
        .into_iter()
        .filter_map(|mut segment| {
            // Check if the segment is ONLY non-speech annotations
            if non_speech_only.is_match(&segment.text) {
                return None; // Remove entirely
            }

            // Remove inline annotations but keep the speech
            let cleaned = annotation_pattern.replace_all(&segment.text, "");
            let cleaned = cleaned.trim().to_string();

            // If nothing left after cleaning, skip
            if cleaned.is_empty() {
                return None;
            }

            segment.text = cleaned;
            Some(segment)
        })
        .collect()
}

/// Extracts detected language from Whisper stderr output
fn extract_detected_language(stderr: &str) -> Option<String> {
    // Whisper outputs something like "auto-detected language: en"
    for line in stderr.lines() {
        if line.contains("language:") {
            if let Some(lang) = line.split(':').last() {
                let lang = lang.trim();
                if !lang.is_empty() && lang.len() <= 3 {
                    return Some(lang.to_string());
                }
            }
        }
    }
    None
}

/// Converts TranscriptionSegments to SRT format string
pub fn segments_to_srt(segments: &[TranscriptionSegment]) -> String {
    let mut srt = String::new();

    for (i, segment) in segments.iter().enumerate() {
        // Sequence number
        srt.push_str(&format!("{}\n", i + 1));

        // Timestamp
        srt.push_str(&format!(
            "{} --> {}\n",
            format_srt_timestamp(segment.start_time),
            format_srt_timestamp(segment.end_time)
        ));

        // Text
        srt.push_str(&segment.text);
        srt.push_str("\n\n");
    }

    srt
}

/// Formats seconds to SRT timestamp format (HH:MM:SS,mmm)
fn format_srt_timestamp(total_seconds: f64) -> String {
    let hours = (total_seconds / 3600.0).floor() as u32;
    let minutes = ((total_seconds % 3600.0) / 60.0).floor() as u32;
    let seconds = (total_seconds % 60.0).floor() as u32;
    let millis = ((total_seconds % 1.0) * 1000.0).round() as u32;

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// Normalizes capitalization across segments
///
/// Rules:
/// - If previous segment ended with sentence-ending punctuation (. ! ?), capitalize next segment
/// - If previous segment ended with ellipsis (...) or comma, keep lowercase
/// - First segment is always capitalized
///
/// This fixes cases where Whisper outputs inconsistent capitalization.
fn normalize_capitalization(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    if segments.is_empty() {
        return segments;
    }

    let mut result = Vec::with_capacity(segments.len());
    let mut prev_ended_sentence = true; // First segment should be capitalized

    for mut segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            result.push(segment);
            continue;
        }

        // Capitalize first letter if previous ended a sentence
        if prev_ended_sentence {
            segment.text = capitalize_first_letter(text);
        }

        // Check how this segment ends for the next iteration
        let trimmed = segment.text.trim_end();
        prev_ended_sentence = trimmed.ends_with('.')
            || trimmed.ends_with('!')
            || trimmed.ends_with('?');

        // Ellipsis means continuation, NOT a sentence end
        if trimmed.ends_with("...") {
            prev_ended_sentence = false;
        }

        result.push(segment);
    }

    result
}

/// Capitalizes the first letter of a string, preserving the rest
fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Formats subtitle text into 2 lines for better readability
///
/// Best practices:
/// - Maximum ~35-42 characters per line (4-6 words)
/// - Maximum 2 lines per subtitle
/// - Break at natural points (after punctuation or at word boundaries)
///
/// If text is short enough for 1 line, keeps it as is.
fn format_subtitle_lines(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    const MAX_LINE_CHARS: usize = 38;  // ~4-6 words per line

    segments.into_iter().map(|mut segment| {
        let text = segment.text.trim();
        let char_count = text.chars().count();

        // If short enough for 1 line, keep as is
        if char_count <= MAX_LINE_CHARS {
            return segment;
        }

        // Find best break point near the middle
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 3 {
            return segment; // Too few words to split meaningfully
        }

        // Build lines by accumulating words
        let mut line1 = String::new();
        let mut line2_start_idx = 0;

        for (i, word) in words.iter().enumerate() {
            let potential = if line1.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", line1, word)
            };

            // Check if adding this word keeps us under the limit
            // Also prefer breaking after punctuation
            let ends_with_punct = word.ends_with(',') || word.ends_with('.')
                || word.ends_with('!') || word.ends_with('?')
                || word.ends_with(':') || word.ends_with(';');

            if potential.chars().count() <= MAX_LINE_CHARS {
                line1 = potential;
                line2_start_idx = i + 1;

                // If this word ends with punctuation and we're past halfway, break here
                if ends_with_punct && i >= words.len() / 3 && i < words.len() - 1 {
                    break;
                }
            } else {
                // This word would exceed limit, break before it
                break;
            }
        }

        // Build line 2 from remaining words
        if line2_start_idx < words.len() {
            let line2: String = words[line2_start_idx..].join(" ");

            // Only use 2 lines if line2 is not too long, otherwise keep original
            if line2.chars().count() <= MAX_LINE_CHARS + 10 {
                segment.text = format!("{}\n{}", line1, line2);
            }
            // If line2 would be too long, keep original (better than 3+ lines)
        }

        segment
    }).collect()
}

/// Splits segments that are too long for comfortable reading
///
/// Subtitle best practices (industry standard):
/// - Maximum 4 seconds display time (average viewer reads 12 words in 4 seconds)
/// - Maximum ~70 characters (~12 words, fits 2 lines)
/// - Don't split sentences if possible - prefer ending at sentence boundaries
/// - Try to keep complete thoughts together
///
/// When a segment is too long, it's split at the nearest sentence boundary
/// (period, comma, or other punctuation) and the time is distributed proportionally.
fn split_long_segments(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    const MAX_DURATION_SECS: f64 = 4.0;  // 4 seconds for comfortable reading
    const MAX_CHARS: usize = 70;          // ~12 words, 2 lines max

    let mut result = Vec::with_capacity(segments.len());

    for segment in segments {
        let duration = segment.end_time - segment.start_time;
        let char_count = segment.text.chars().count();

        // Check if segment needs splitting
        if duration <= MAX_DURATION_SECS && char_count <= MAX_CHARS {
            result.push(segment);
            continue;
        }

        // Find the best split point
        // Priority (best practice): complete sentences > clauses > commas > spaces
        let text = &segment.text;
        let min_part_len = 15; // Minimum length for each part

        // Helper to find best position for a set of split chars
        let find_split = |chars: &[char]| -> Option<usize> {
            // Look for the split point closest to middle, but with at least min_part_len on each side
            let valid_range = min_part_len..text.len().saturating_sub(min_part_len);

            for c in chars {
                // Find all positions of this char within valid range
                let positions: Vec<usize> = text.char_indices()
                    .filter(|(i, ch)| ch == c && valid_range.contains(i))
                    .map(|(i, _)| i + 1)  // +1 to include punctuation in first part
                    .collect();

                if !positions.is_empty() {
                    // Choose position closest to middle
                    let mid = text.len() / 2;
                    return positions.into_iter()
                        .min_by_key(|&pos| (pos as i64 - mid as i64).abs());
                }
            }
            None
        };

        // Try to find split points in order of preference
        let best_split = find_split(&['.', '!', '?'])           // Sentence endings (best)
            .or_else(|| find_split(&[':', ';']))                  // Clause separators
            .or_else(|| find_split(&[',']))                       // Comma
            .or_else(|| find_split(&[' ']));                      // Space (last resort)

        // If we found a split point, create two segments
        if let Some(split_pos) = best_split {
            let (first_text, second_text) = text.split_at(split_pos);
            let first_text = first_text.trim().to_string();
            let second_text = second_text.trim().to_string();

            // Skip if either part is too small
            if first_text.len() < 5 || second_text.len() < 5 {
                result.push(segment);
                continue;
            }

            // Distribute time proportionally based on text length
            let total_chars = first_text.len() + second_text.len();
            let first_ratio = first_text.len() as f64 / total_chars as f64;
            let mid_time = segment.start_time + (duration * first_ratio);

            result.push(TranscriptionSegment {
                start_time: segment.start_time,
                end_time: mid_time,
                text: first_text,
            });

            result.push(TranscriptionSegment {
                start_time: mid_time,
                end_time: segment.end_time,
                text: second_text,
            });

            tracing::debug!(
                "Split long segment ({:.1}s, {} chars) into two parts",
                duration, char_count
            );
        } else {
            // No good split point found, keep original
            result.push(segment);
        }
    }

    result
}

/// Detects and logs warnings about large gaps in transcription
///
/// Large gaps might indicate:
/// - Music-only sections that Whisper skipped
/// - Inaudible or very quiet dialogue
/// - Processing errors
///
/// This is informational logging only - doesn't modify the segments.
fn detect_gaps(segments: &[TranscriptionSegment], video_duration: f64) {
    const GAP_THRESHOLD_SECS: f64 = 30.0;

    if segments.is_empty() {
        tracing::warn!("No transcription segments - audio may be silent or unsupported");
        return;
    }

    // Check gap at the start
    if let Some(first) = segments.first() {
        if first.start_time > GAP_THRESHOLD_SECS {
            tracing::info!(
                "Large gap at start: no dialogue until {:.1}s",
                first.start_time
            );
        }
    }

    // Check gaps between segments
    for window in segments.windows(2) {
        let gap = window[1].start_time - window[0].end_time;
        if gap > GAP_THRESHOLD_SECS {
            tracing::info!(
                "Large gap detected: {:.1}s of silence between {:.1}s and {:.1}s",
                gap, window[0].end_time, window[1].start_time
            );
        }
    }

    // Check gap at the end
    if let Some(last) = segments.last() {
        let end_gap = video_duration - last.end_time;
        if end_gap > GAP_THRESHOLD_SECS {
            tracing::info!(
                "Large gap at end: no dialogue after {:.1}s (video is {:.1}s)",
                last.end_time, video_duration
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert!((parse_timestamp("00:00:01,000").unwrap() - 1.0).abs() < 0.001);
        assert!((parse_timestamp("00:01:30,500").unwrap() - 90.5).abs() < 0.001);
        assert!((parse_timestamp("01:30:45,250").unwrap() - 5445.25).abs() < 0.001);
    }

    #[test]
    fn test_parse_srt() {
        let srt = r#"1
00:00:01,000 --> 00:00:04,000
Hello World

2
00:00:05,500 --> 00:00:08,000
Second line
"#;

        let segments = parse_srt(srt).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello World");
        assert!((segments[0].start_time - 1.0).abs() < 0.001);
        assert!((segments[0].end_time - 4.0).abs() < 0.001);
        assert_eq!(segments[1].text, "Second line");
    }

    #[test]
    fn test_segments_to_srt() {
        let segments = vec![
            TranscriptionSegment {
                start_time: 1.0,
                end_time: 4.0,
                text: "Hello World".to_string(),
            },
            TranscriptionSegment {
                start_time: 5.5,
                end_time: 8.0,
                text: "Second line".to_string(),
            },
        ];

        let srt = segments_to_srt(&segments);
        assert!(srt.contains("00:00:01,000 --> 00:00:04,000"));
        assert!(srt.contains("Hello World"));
        assert!(srt.contains("00:00:05,500 --> 00:00:08,000"));
    }

    #[test]
    fn test_format_srt_timestamp() {
        assert_eq!(format_srt_timestamp(0.0), "00:00:00,000");
        assert_eq!(format_srt_timestamp(90.5), "00:01:30,500");
        assert_eq!(format_srt_timestamp(5445.25), "01:30:45,250");
    }

    #[test]
    fn test_filter_hallucinations() {
        // Test repeated phrase detection
        let segments: Vec<TranscriptionSegment> = (0..20)
            .map(|i| TranscriptionSegment {
                start_time: i as f64,
                end_time: (i + 1) as f64,
                text: "Thank you for watching!".to_string(),
            })
            .collect();

        let filtered = filter_hallucinations(segments);
        // Should filter out all "thank you for watching" hallucinations
        assert!(filtered.is_empty(), "Should filter common hallucination phrase");

        // Test repetition detection
        let segments: Vec<TranscriptionSegment> = (0..50)
            .map(|i| TranscriptionSegment {
                start_time: i as f64,
                end_time: (i + 1) as f64,
                text: "Some random repeated text".to_string(),
            })
            .collect();

        let filtered = filter_hallucinations(segments);
        // Should filter out repeated text (appears more than 5 times)
        assert!(filtered.is_empty(), "Should filter frequently repeated text");

        // Test that legitimate varied content is preserved
        let segments = vec![
            TranscriptionSegment { start_time: 0.0, end_time: 2.0, text: "Hello there.".to_string() },
            TranscriptionSegment { start_time: 2.0, end_time: 4.0, text: "How are you?".to_string() },
            TranscriptionSegment { start_time: 4.0, end_time: 6.0, text: "I'm fine, thanks.".to_string() },
        ];

        let filtered = filter_hallucinations(segments);
        assert_eq!(filtered.len(), 3, "Should preserve legitimate varied content");
    }

    #[test]
    fn test_filter_non_speech_annotations() {
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 2.0,
                text: "(dramatic music)".to_string(),
            },
            TranscriptionSegment {
                start_time: 2.0,
                end_time: 4.0,
                text: "Hello there!".to_string(),
            },
            TranscriptionSegment {
                start_time: 4.0,
                end_time: 6.0,
                text: "[MUSIC]".to_string(),
            },
            TranscriptionSegment {
                start_time: 6.0,
                end_time: 8.0,
                text: "Nice to meet you (sighs)".to_string(),
            },
            TranscriptionSegment {
                start_time: 8.0,
                end_time: 10.0,
                text: "♪ La la la ♪".to_string(),
            },
        ];

        let filtered = filter_non_speech_annotations(segments);

        // Should keep only actual speech
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "Hello there!");
        assert_eq!(filtered[1].text, "Nice to meet you");
    }

    #[test]
    fn test_format_subtitle_lines() {
        // Short text should stay on one line
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 2.0,
                text: "Hello there!".to_string(),
            },
        ];
        let formatted = format_subtitle_lines(segments);
        assert_eq!(formatted[0].text, "Hello there!");
        assert!(!formatted[0].text.contains('\n'), "Short text should not be split");

        // Long text should be split into 2 lines
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 4.0,
                text: "This is a longer sentence that should be split into two lines".to_string(),
            },
        ];
        let formatted = format_subtitle_lines(segments);
        assert!(formatted[0].text.contains('\n'), "Long text should be split into 2 lines");

        let lines: Vec<&str> = formatted[0].text.split('\n').collect();
        assert_eq!(lines.len(), 2, "Should have exactly 2 lines");

        // Each line should be under ~40 chars (the limit is 38, but we check reasonably)
        assert!(lines[0].chars().count() <= 40, "First line should be under 40 chars, got: {}", lines[0]);
        assert!(lines[1].chars().count() <= 50, "Second line should be reasonable, got: {}", lines[1]);
    }

    #[test]
    fn test_format_subtitle_lines_with_punctuation() {
        // Should prefer breaking after punctuation
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 4.0,
                text: "Well, I think that's a great idea for the project".to_string(),
            },
        ];
        let formatted = format_subtitle_lines(segments);
        let lines: Vec<&str> = formatted[0].text.split('\n').collect();

        // Should break after "Well," if it fits
        assert!(lines[0].ends_with(',') || lines[0].len() <= 38,
            "Should break at punctuation or within limit");
    }

    #[test]
    fn test_normalize_capitalization() {
        // Capitalize after sentence endings
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 2.0,
                text: "Hello there.".to_string(),
            },
            TranscriptionSegment {
                start_time: 2.0,
                end_time: 4.0,
                text: "how are you?".to_string(), // lowercase, should be capitalized
            },
            TranscriptionSegment {
                start_time: 4.0,
                end_time: 6.0,
                text: "I'm fine.".to_string(),
            },
        ];
        let result = normalize_capitalization(segments);
        assert_eq!(result[0].text, "Hello there.");
        assert_eq!(result[1].text, "How are you?"); // Capitalized
        assert_eq!(result[2].text, "I'm fine.");

        // Don't capitalize after ellipsis
        let segments = vec![
            TranscriptionSegment {
                start_time: 0.0,
                end_time: 2.0,
                text: "Well...".to_string(),
            },
            TranscriptionSegment {
                start_time: 2.0,
                end_time: 4.0,
                text: "maybe later.".to_string(), // lowercase continuation
            },
        ];
        let result = normalize_capitalization(segments);
        assert_eq!(result[0].text, "Well...");
        assert_eq!(result[1].text, "maybe later."); // Stays lowercase (continuation)
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(capitalize_first_letter("hello"), "Hello");
        assert_eq!(capitalize_first_letter("Hello"), "Hello");
        assert_eq!(capitalize_first_letter(""), "");
        assert_eq!(capitalize_first_letter("á"), "Á"); // Unicode
    }
}
