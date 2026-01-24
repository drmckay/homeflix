//! ScanFailed event
//!
//! Emitted when a library scan fails

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a library scan fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScanFailedEvent {
    /// Path that was being scanned
    pub scan_path: String,
    /// Error message
    pub error_message: String,
    /// Number of items processed before failure
    pub processed_count: usize,
    /// Number of items identified before failure
    pub identified_count: usize,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl ScanFailedEvent {
    /// Creates a new scan failed event
    pub fn new(
        scan_path: String,
        error_message: String,
        processed_count: usize,
        identified_count: usize,
    ) -> Self {
        Self {
            scan_path,
            error_message,
            processed_count,
            identified_count,
            timestamp: Utc::now(),
        }
    }

    /// Gets the event type name
    pub fn event_type() -> &'static str {
        "scan_failed"
    }
}

impl crate::interfaces::messaging::DomainEvent for ScanFailedEvent {
    fn event_type(&self) -> &'static str {
        Self::event_type()
    }
}
