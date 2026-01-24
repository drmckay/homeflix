//! Video Details Value Object
//!
//! Represents video stream details from media file analysis.
//! Migrated from models.rs to align with Clean Architecture.

use serde::{Deserialize, Serialize};

/// Video stream details from FFprobe analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoDetails {
    /// Video codec name (e.g., "h264", "hevc", "vp9", "av1")
    pub codec_name: String,
    /// Frame width in pixels
    pub width: i32,
    /// Frame height in pixels
    pub height: i32,
    /// Duration in seconds
    pub duration: f64,
}

impl VideoDetails {
    /// Creates new video details
    pub fn new(codec_name: impl Into<String>, width: i32, height: i32, duration: f64) -> Self {
        Self {
            codec_name: codec_name.into(),
            width,
            height,
            duration,
        }
    }

    /// Returns the resolution label (e.g., "4K", "1080p", "720p")
    pub fn resolution_label(&self) -> &'static str {
        match self.height {
            h if h >= 2160 => "4K",
            h if h >= 1440 => "1440p",
            h if h >= 1080 => "1080p",
            h if h >= 720 => "720p",
            h if h >= 576 => "576p",
            h if h >= 480 => "480p",
            _ => "SD",
        }
    }

    /// Returns the aspect ratio as a string (e.g., "16:9", "21:9")
    pub fn aspect_ratio(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return "Unknown".to_string();
        }

        let ratio = self.width as f64 / self.height as f64;

        // Common aspect ratios - check most common first with appropriate tolerance
        if (ratio - 1.78).abs() < 0.05 {
            "16:9".to_string()
        } else if (ratio - 1.33).abs() < 0.05 {
            "4:3".to_string()
        } else if (ratio - 2.39).abs() < 0.1 {
            "21:9".to_string()
        } else if (ratio - 2.35).abs() < 0.1 {
            "2.35:1".to_string()
        } else if (ratio - 1.85).abs() < 0.05 {
            "1.85:1".to_string()
        } else {
            format!("{:.2}:1", ratio)
        }
    }

    /// Returns the duration formatted as HH:MM:SS
    pub fn duration_formatted(&self) -> String {
        let total_secs = self.duration as i64;
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Returns duration in minutes (rounded)
    pub fn duration_minutes(&self) -> i64 {
        (self.duration / 60.0).round() as i64
    }

    /// Returns true if this is a high-definition video (720p or higher)
    pub fn is_hd(&self) -> bool {
        self.height >= 720
    }

    /// Returns true if this is a 4K video
    pub fn is_4k(&self) -> bool {
        self.height >= 2160
    }

    /// Returns true if the codec is hardware-acceleratable on most devices
    pub fn is_hw_accelerated(&self) -> bool {
        let hw_codecs = ["h264", "hevc", "h265", "vp9", "av1"];
        let codec_lower = self.codec_name.to_lowercase();
        hw_codecs.iter().any(|c| codec_lower.contains(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_details_creation() {
        let details = VideoDetails::new("h264", 1920, 1080, 7200.5);

        assert_eq!(details.codec_name, "h264");
        assert_eq!(details.width, 1920);
        assert_eq!(details.height, 1080);
        assert_eq!(details.duration, 7200.5);
    }

    #[test]
    fn test_resolution_label() {
        assert_eq!(VideoDetails::new("h264", 3840, 2160, 0.0).resolution_label(), "4K");
        assert_eq!(VideoDetails::new("h264", 2560, 1440, 0.0).resolution_label(), "1440p");
        assert_eq!(VideoDetails::new("h264", 1920, 1080, 0.0).resolution_label(), "1080p");
        assert_eq!(VideoDetails::new("h264", 1280, 720, 0.0).resolution_label(), "720p");
        assert_eq!(VideoDetails::new("h264", 720, 480, 0.0).resolution_label(), "480p");
        assert_eq!(VideoDetails::new("h264", 320, 240, 0.0).resolution_label(), "SD");
    }

    #[test]
    fn test_aspect_ratio() {
        assert_eq!(VideoDetails::new("h264", 1920, 1080, 0.0).aspect_ratio(), "16:9");
        assert_eq!(VideoDetails::new("h264", 1280, 720, 0.0).aspect_ratio(), "16:9");
        assert_eq!(VideoDetails::new("h264", 640, 480, 0.0).aspect_ratio(), "4:3");
    }

    #[test]
    fn test_duration_formatted() {
        assert_eq!(VideoDetails::new("h264", 0, 0, 7200.0).duration_formatted(), "02:00:00");
        assert_eq!(VideoDetails::new("h264", 0, 0, 3661.0).duration_formatted(), "01:01:01");
        assert_eq!(VideoDetails::new("h264", 0, 0, 90.0).duration_formatted(), "01:30");
    }

    #[test]
    fn test_duration_minutes() {
        assert_eq!(VideoDetails::new("h264", 0, 0, 7200.0).duration_minutes(), 120);
        assert_eq!(VideoDetails::new("h264", 0, 0, 5430.0).duration_minutes(), 91);
    }

    #[test]
    fn test_is_hd() {
        assert!(VideoDetails::new("h264", 1920, 1080, 0.0).is_hd());
        assert!(VideoDetails::new("h264", 1280, 720, 0.0).is_hd());
        assert!(!VideoDetails::new("h264", 720, 480, 0.0).is_hd());
    }

    #[test]
    fn test_is_4k() {
        assert!(VideoDetails::new("h264", 3840, 2160, 0.0).is_4k());
        assert!(!VideoDetails::new("h264", 1920, 1080, 0.0).is_4k());
    }

    #[test]
    fn test_is_hw_accelerated() {
        assert!(VideoDetails::new("h264", 0, 0, 0.0).is_hw_accelerated());
        assert!(VideoDetails::new("hevc", 0, 0, 0.0).is_hw_accelerated());
        assert!(!VideoDetails::new("mpeg2video", 0, 0, 0.0).is_hw_accelerated());
    }
}
