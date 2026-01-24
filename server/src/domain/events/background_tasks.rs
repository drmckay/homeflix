//! Background Task Events
//!
//! Events emitted for background tasks and scheduled operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a background scan is scheduled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackgroundScanScheduledEvent {
    /// Scan path
    pub scan_path: String,
    /// Scheduled time
    pub scheduled_at: DateTime<Utc>,
    /// Scan interval in seconds
    pub scan_interval_secs: u64,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl BackgroundScanScheduledEvent {
    /// Creates a new background scan scheduled event
    pub fn new(scan_path: String, scheduled_at: DateTime<Utc>, scan_interval_secs: u64) -> Self {
        Self {
            scan_path,
            scheduled_at,
            scan_interval_secs,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for BackgroundScanScheduledEvent {
    fn event_type(&self) -> &'static str {
        "background_scan_scheduled"
    }
}

/// Event emitted when a background scan starts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackgroundScanStartedEvent {
    /// Scan path
    pub scan_path: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl BackgroundScanStartedEvent {
    /// Creates a new background scan started event
    pub fn new(scan_path: String) -> Self {
        Self {
            scan_path,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for BackgroundScanStartedEvent {
    fn event_type(&self) -> &'static str {
        "background_scan_started"
    }
}

/// Event emitted when a background task completes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackgroundTaskCompletedEvent {
    /// Task type
    pub task_type: String,
    /// Task identifier
    pub task_id: Option<String>,
    /// Success status
    pub success: bool,
    /// Result message (if any)
    pub result_message: Option<String>,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl BackgroundTaskCompletedEvent {
    /// Creates a new background task completed event
    pub fn new(
        task_type: String,
        task_id: Option<String>,
        success: bool,
        result_message: Option<String>,
    ) -> Self {
        Self {
            task_type,
            task_id,
            success,
            result_message,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for BackgroundTaskCompletedEvent {
    fn event_type(&self) -> &'static str {
        "background_task_completed"
    }
}
