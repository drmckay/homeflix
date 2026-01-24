//! Value Objects - Immutable objects defined by their attributes
//!
//! Value objects are identified by their attributes rather than an identity.
//! They are immutable and have no lifecycle.

pub mod audio_track;
pub mod confidence_score;
pub mod identification_result;
pub mod match_strategy;
pub mod media_type;
pub mod verification_status;
pub mod video_details;

pub use audio_track::AudioTrack;
pub use confidence_score::ConfidenceScore;
pub use identification_result::IdentificationResult;
pub use match_strategy::MatchStrategy;
pub use media_type::MediaType;
pub use verification_status::VerificationStatus;
pub use video_details::VideoDetails;
