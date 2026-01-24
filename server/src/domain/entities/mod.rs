//! Domain Entities - Core business objects with identity
//!
//! Entities are objects that have an identity (ID) and lifecycle.

pub mod collection;
pub mod episode;
pub mod media;
pub mod season;
pub mod series;

pub use collection::{Collection, CollectionItem};
pub use episode::Episode;
pub use media::Media;
pub use season::Season;
pub use series::Series;
