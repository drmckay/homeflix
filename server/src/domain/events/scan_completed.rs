//! ScanCompleted event
//!
//! Emitted when a library scan completes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a library scan completes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScanCompletedEvent {
    /// Number of files processed
    pub processed_count: usize,
    /// Number of items successfully identified
    pub identified_count: usize,
    /// Number of items that failed to identify
    pub failed_count: usize,
    /// Scan duration in seconds
    pub duration_secs: u64,
    /// Path that was scanned
    pub scan_path: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl ScanCompletedEvent {
    /// Creates a new scan completed event
    pub fn new(
        processed_count: usize,
        identified_count: usize,
        failed_count: usize,
        duration_secs: u64,
        scan_path: String,
    ) -> Self {
        Self {
            processed_count,
            identified_count,
            failed_count,
            duration_secs,
            scan_path,
            timestamp: Utc::now(),
        }
    }

    /// Gets the event type name
    pub fn event_type() -> &'static str {
        "scan_completed"
    }

    /// Calculates items per second
    pub fn items_per_second(&self) -> f64 {
        if self.duration_secs == 0 {
            return 0.0;
        }
        self.processed_count as f64 / self.duration_secs as f64
    }

    /// Calculates success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.processed_count == 0 {
            return 0.0;
        }
        self.identified_count as f64 / self.processed_count as f64
    }
}

impl crate::interfaces::messaging::DomainEvent for ScanCompletedEvent {
    fn event_type(&self) -> &'static str {
        Self::event_type()
    }
}
