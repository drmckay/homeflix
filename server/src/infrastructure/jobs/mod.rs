//! Job Management Module
//!
//! Provides in-memory job tracking for long-running async operations
//! like subtitle generation and batch processing.

mod job_store;

pub use job_store::*;
