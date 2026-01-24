//! GPU Coordination Module
//!
//! Provides coordination for GPU-intensive tasks to prevent resource conflicts.
//! Ensures that Whisper (speech-to-text) and Ollama (translation) don't run simultaneously
//! since they share the same GPU.

mod coordinator;

pub use coordinator::*;
