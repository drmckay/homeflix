//! SRT to WebVTT Converter
//!
//! Converts SubRip (.srt) subtitle format to WebVTT (.vtt) format
//! for HTML5 video compatibility.

use crate::shared::error::SubtitleError;

/// Converts SRT subtitle content to WebVTT format.
///
/// # WebVTT vs SRT differences
/// - WebVTT requires "WEBVTT" header
/// - Timestamp uses '.' instead of ',' for milliseconds
/// - WebVTT supports additional styling (not used here)
///
/// # Arguments
/// * `srt_content` - Raw SRT file content
///
/// # Returns
/// * `Ok(String)` - WebVTT formatted content
/// * `Err(SubtitleError)` - If parsing fails
///
/// # Example
/// ```ignore
/// let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello World\n";
/// let vtt = convert_srt_to_vtt(srt)?;
/// assert!(vtt.starts_with("WEBVTT\n"));
/// ```
pub fn convert_srt_to_vtt(srt_content: &str) -> Result<String, SubtitleError> {
    let mut vtt_content = String::with_capacity(srt_content.len() + 20);

    // WebVTT header
    vtt_content.push_str("WEBVTT\n\n");

    // Process each line
    for line in srt_content.lines() {
        let trimmed = line.trim();

        // Check if this is a timestamp line
        if is_timestamp_line(trimmed) {
            // Convert SRT timestamp to VTT format
            // SRT:  00:00:01,000 --> 00:00:04,000
            // VTT:  00:00:01.000 --> 00:00:04.000
            let converted = trimmed.replace(',', ".");
            vtt_content.push_str(&converted);
        } else {
            // Keep other lines as-is (cue numbers, text, empty lines)
            vtt_content.push_str(trimmed);
        }

        vtt_content.push('\n');
    }

    Ok(vtt_content)
}

/// Checks if a line is a timestamp line.
///
/// # Arguments
/// * `line` - A single line from the SRT file
///
/// # Returns
/// `true` if the line contains SRT timestamp format
fn is_timestamp_line(line: &str) -> bool {
    // SRT timestamp pattern: 00:00:00,000 --> 00:00:00,000
    line.contains("-->") && line.contains(':')
}

/// Converts SRT subtitle content to WebVTT format with timestamp offset.
///
/// This is used when streaming starts from a position other than 0.
/// The offset is subtracted from all timestamps so subtitles sync with
/// the video element's time (which starts at 0 when seeking).
///
/// # Arguments
/// * `srt_content` - Raw SRT file content
/// * `offset_seconds` - Seconds to subtract from all timestamps
///
/// # Returns
/// * `Ok(String)` - WebVTT formatted content with adjusted timestamps
/// * `Err(SubtitleError)` - If parsing fails
pub fn convert_srt_to_vtt_with_offset(srt_content: &str, offset_seconds: f64) -> Result<String, SubtitleError> {
    let mut vtt_content = String::with_capacity(srt_content.len() + 20);

    // WebVTT header
    vtt_content.push_str("WEBVTT\n\n");

    // Process each line
    for line in srt_content.lines() {
        let trimmed = line.trim();

        // Check if this is a timestamp line
        if is_timestamp_line(trimmed) {
            // Parse and offset timestamps
            let converted = offset_timestamp_line(trimmed, offset_seconds);
            vtt_content.push_str(&converted);
        } else {
            // Keep other lines as-is (cue numbers, text, empty lines)
            vtt_content.push_str(trimmed);
        }

        vtt_content.push('\n');
    }

    Ok(vtt_content)
}

/// Offsets timestamps in a timestamp line.
///
/// Input:  "00:10:05,000 --> 00:10:08,500"
/// Offset: 600.0 (10 minutes)
/// Output: "00:00:05.000 --> 00:00:08.500"
fn offset_timestamp_line(line: &str, offset_seconds: f64) -> String {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return line.replace(',', ".");
    }

    let start = parse_timestamp(parts[0].trim());
    let end = parse_timestamp(parts[1].trim());

    let new_start = (start - offset_seconds).max(0.0);
    let new_end = (end - offset_seconds).max(0.0);

    format!("{} --> {}", format_timestamp(new_start), format_timestamp(new_end))
}

/// Parses SRT timestamp to seconds.
/// Format: HH:MM:SS,mmm or HH:MM:SS.mmm
fn parse_timestamp(ts: &str) -> f64 {
    // Remove any extra content after milliseconds (like positioning info)
    let ts_clean = ts.split_whitespace().next().unwrap_or(ts);

    let parts: Vec<&str> = ts_clean.split(':').collect();
    if parts.len() != 3 {
        return 0.0;
    }

    let hours: f64 = parts[0].parse().unwrap_or(0.0);
    let minutes: f64 = parts[1].parse().unwrap_or(0.0);

    // Handle both ',' and '.' for milliseconds separator
    let sec_parts: Vec<&str> = parts[2].split(|c| c == ',' || c == '.').collect();
    let seconds: f64 = sec_parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let millis: f64 = sec_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    hours * 3600.0 + minutes * 60.0 + seconds + millis / 1000.0
}

/// Formats seconds to VTT timestamp (HH:MM:SS.mmm)
fn format_timestamp(total_seconds: f64) -> String {
    let hours = (total_seconds / 3600.0).floor() as u32;
    let minutes = ((total_seconds % 3600.0) / 60.0).floor() as u32;
    let seconds = (total_seconds % 60.0).floor() as u32;
    let millis = ((total_seconds % 1.0) * 1000.0).round() as u32;

    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// Reads an SRT file and converts it to WebVTT format.
///
/// # Arguments
/// * `file_path` - Path to the SRT file
///
/// # Returns
/// * `Ok(String)` - WebVTT formatted content
/// * `Err(SubtitleError)` - If reading or parsing fails
pub fn read_and_convert_srt(file_path: &str) -> Result<String, SubtitleError> {
    read_and_convert_srt_with_offset(file_path, 0.0)
}

/// Reads an SRT file and converts it to WebVTT format with timestamp offset.
///
/// # Arguments
/// * `file_path` - Path to the SRT file
/// * `offset_seconds` - Seconds to subtract from all timestamps
///
/// # Returns
/// * `Ok(String)` - WebVTT formatted content with adjusted timestamps
/// * `Err(SubtitleError)` - If reading or parsing fails
pub fn read_and_convert_srt_with_offset(file_path: &str, offset_seconds: f64) -> Result<String, SubtitleError> {
    // Read file content
    let content = std::fs::read(file_path)?;

    // Try to decode as UTF-8 first, fallback to Latin-1
    let text = match String::from_utf8(content.clone()) {
        Ok(s) => s,
        Err(_) => {
            // Fallback: decode as Latin-1 (ISO-8859-1)
            content.iter().map(|&b| b as char).collect()
        }
    };

    if offset_seconds > 0.0 {
        convert_srt_to_vtt_with_offset(&text, offset_seconds)
    } else {
        convert_srt_to_vtt(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_srt() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello World\n\n2\n00:00:05,500 --> 00:00:08,000\nSecond line\n";

        let vtt = convert_srt_to_vtt(srt).unwrap();

        assert!(vtt.starts_with("WEBVTT\n"));
        assert!(vtt.contains("00:00:01.000 --> 00:00:04.000"));
        assert!(vtt.contains("00:00:05.500 --> 00:00:08.000"));
        assert!(vtt.contains("Hello World"));
        assert!(vtt.contains("Second line"));
    }

    #[test]
    fn test_convert_multiline_text() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nLine one\nLine two\n";

        let vtt = convert_srt_to_vtt(srt).unwrap();

        assert!(vtt.contains("Line one"));
        assert!(vtt.contains("Line two"));
    }

    #[test]
    fn test_is_timestamp_line() {
        assert!(is_timestamp_line("00:00:01,000 --> 00:00:04,000"));
        assert!(is_timestamp_line("01:23:45,678 --> 02:34:56,789"));
        assert!(!is_timestamp_line("1"));
        assert!(!is_timestamp_line("Hello World"));
        assert!(!is_timestamp_line(""));
    }

    #[test]
    fn test_empty_srt() {
        let srt = "";
        let vtt = convert_srt_to_vtt(srt).unwrap();
        assert_eq!(vtt, "WEBVTT\n\n");
    }
}
