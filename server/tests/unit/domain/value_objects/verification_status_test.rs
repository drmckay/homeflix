//! Unit tests for VerificationStatus value object
//!
//! Tests cover:
//! - Status creation and comparison
//! - Status checking methods
//! - String conversion

use homeflixd::domain::value_objects::VerificationStatus;

#[test]
fn test_verification_status_unverified() {
    let status = VerificationStatus::Unverified;
    assert_eq!(status.as_str(), "unverified");
    assert!(!status.is_verified());
    assert!(!status.is_failed());
}

#[test]
fn test_verification_status_verified() {
    let status = VerificationStatus::Verified;
    assert_eq!(status.as_str(), "verified");
    assert!(status.is_verified());
    assert!(!status.is_failed());
}

#[test]
fn test_verification_status_failed() {
    let status = VerificationStatus::Failed;
    assert_eq!(status.as_str(), "failed");
    assert!(!status.is_verified());
    assert!(status.is_failed());
}

#[test]
fn test_verification_status_from_str_unverified() {
    let status = VerificationStatus::from_str("unverified").expect("Should parse unverified");
    assert_eq!(status, VerificationStatus::Unverified);
}

#[test]
fn test_verification_status_from_str_verified() {
    let status = VerificationStatus::from_str("verified").expect("Should parse verified");
    assert_eq!(status, VerificationStatus::Verified);
}

#[test]
fn test_verification_status_from_str_failed() {
    let status = VerificationStatus::from_str("failed").expect("Should parse failed");
    assert_eq!(status, VerificationStatus::Failed);
}

#[test]
fn test_verification_status_from_str_invalid() {
    let result = VerificationStatus::from_str("invalid");
    assert!(result.is_err());
}

#[test]
fn test_verification_status_serialization() {
    let status = VerificationStatus::Verified;
    let serialized = serde_json::to_string(&status).expect("Should serialize");
    assert_eq!(serialized, "\"verified\"");
}

#[test]
fn test_verification_status_deserialization() {
    let json = "\"failed\"";
    let status: VerificationStatus = serde_json::from_str(json).expect("Should deserialize");
    assert_eq!(status, VerificationStatus::Failed);
}

#[test]
fn test_verification_status_clone() {
    let status1 = VerificationStatus::Verified;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

#[test]
fn test_verification_status_debug() {
    let status = VerificationStatus::Verified;
    let debug_str = format!("{:?}", status);
    assert!(debug_str.contains("Verified"));
}

#[test]
fn test_verification_status_partial_eq() {
    let status1 = VerificationStatus::Verified;
    let status2 = VerificationStatus::Verified;
    assert_eq!(status1, status2);
}

#[test]
fn test_verification_status_partial_ne() {
    let status1 = VerificationStatus::Verified;
    let status2 = VerificationStatus::Failed;
    assert_ne!(status1, status2);
}
