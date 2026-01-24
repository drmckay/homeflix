//! Text processing utilities for media title matching
//!
//! This module provides tools for normalizing and fuzzy matching media titles,
//! which is essential for matching user's file/folder names to TMDB entries.
//!
//! # Components
//!
//! - [`RomanNumeralConverter`] - Converts between roman numerals, arabic numbers, and spelled-out numbers
//! - [`TitleNormalizer`] - Normalizes titles for comparison (punctuation, articles, whitespace)
//! - [`FuzzyMatcher`] - Fuzzy string matching algorithms (Jaro-Winkler, Levenshtein, token-based)

mod roman_numerals;
mod normalizer;
mod fuzzy;

pub use roman_numerals::RomanNumeralConverter;
pub use normalizer::TitleNormalizer;
pub use fuzzy::{FuzzyMatcher, FuzzyMatch, FuzzyMatchConfig};
