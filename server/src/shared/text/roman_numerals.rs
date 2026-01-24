//! Roman numeral conversion utilities
//!
//! Handles conversion between:
//! - Roman numerals (I, II, III, IV, V, etc.)
//! - Arabic numbers (1, 2, 3, 4, 5, etc.)
//! - Spelled-out numbers (One, Two, Three, Four, Five, etc.)

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Roman numeral to arabic number mappings
static ROMAN_TO_ARABIC: Lazy<Vec<(&'static str, u32)>> = Lazy::new(|| {
    vec![
        ("XX", 20), ("XIX", 19), ("XVIII", 18), ("XVII", 17), ("XVI", 16),
        ("XV", 15), ("XIV", 14), ("XIII", 13), ("XII", 12), ("XI", 11),
        ("X", 10), ("IX", 9), ("VIII", 8), ("VII", 7), ("VI", 6),
        ("V", 5), ("IV", 4), ("III", 3), ("II", 2), ("I", 1),
    ]
});

/// Spelled-out number to arabic mappings (case-insensitive matching)
static SPELLED_TO_ARABIC: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // Cardinal numbers
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);
    map.insert("four", 4);
    map.insert("five", 5);
    map.insert("six", 6);
    map.insert("seven", 7);
    map.insert("eight", 8);
    map.insert("nine", 9);
    map.insert("ten", 10);
    map.insert("eleven", 11);
    map.insert("twelve", 12);
    map.insert("thirteen", 13);
    map.insert("fourteen", 14);
    map.insert("fifteen", 15);
    map.insert("sixteen", 16);
    map.insert("seventeen", 17);
    map.insert("eighteen", 18);
    map.insert("nineteen", 19);
    map.insert("twenty", 20);
    // Ordinal numbers
    map.insert("first", 1);
    map.insert("second", 2);
    map.insert("third", 3);
    map.insert("fourth", 4);
    map.insert("fifth", 5);
    map.insert("sixth", 6);
    map.insert("seventh", 7);
    map.insert("eighth", 8);
    map.insert("ninth", 9);
    map.insert("tenth", 10);
    map
});

/// Arabic to roman numeral mappings
static ARABIC_TO_ROMAN: Lazy<Vec<(u32, &'static str)>> = Lazy::new(|| {
    vec![
        (20, "XX"), (19, "XIX"), (18, "XVIII"), (17, "XVII"), (16, "XVI"),
        (15, "XV"), (14, "XIV"), (13, "XIII"), (12, "XII"), (11, "XI"),
        (10, "X"), (9, "IX"), (8, "VIII"), (7, "VII"), (6, "VI"),
        (5, "V"), (4, "IV"), (3, "III"), (2, "II"), (1, "I"),
    ]
});

/// Arabic to spelled-out number mappings
static ARABIC_TO_SPELLED: Lazy<Vec<(u32, &'static str)>> = Lazy::new(|| {
    vec![
        (1, "One"), (2, "Two"), (3, "Three"), (4, "Four"), (5, "Five"),
        (6, "Six"), (7, "Seven"), (8, "Eight"), (9, "Nine"), (10, "Ten"),
        (11, "Eleven"), (12, "Twelve"), (13, "Thirteen"), (14, "Fourteen"),
        (15, "Fifteen"), (16, "Sixteen"), (17, "Seventeen"), (18, "Eighteen"),
        (19, "Nineteen"), (20, "Twenty"),
    ]
});

/// Regex to match roman numerals as whole words (case-insensitive)
static ROMAN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(XX|XIX|XVIII|XVII|XVI|XV|XIV|XIII|XII|XI|X|IX|VIII|VII|VI|V|IV|III|II|I)\b").unwrap()
});

/// Regex to match spelled-out numbers as whole words
static SPELLED_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth)\b").unwrap()
});

/// Regex to match standalone arabic numbers (1-20)
static ARABIC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([1-9]|1[0-9]|20)\b").unwrap()
});

/// Converter for roman numerals, arabic numbers, and spelled-out numbers
pub struct RomanNumeralConverter;

impl RomanNumeralConverter {
    /// Convert a roman numeral string to arabic number
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(RomanNumeralConverter::roman_to_arabic("III"), Some(3));
    /// assert_eq!(RomanNumeralConverter::roman_to_arabic("XIV"), Some(14));
    /// ```
    pub fn roman_to_arabic(roman: &str) -> Option<u32> {
        let upper = roman.to_uppercase();
        for (r, a) in ROMAN_TO_ARABIC.iter() {
            if *r == upper {
                return Some(*a);
            }
        }
        None
    }

    /// Convert an arabic number to roman numeral string
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(RomanNumeralConverter::arabic_to_roman(3), Some("III"));
    /// assert_eq!(RomanNumeralConverter::arabic_to_roman(14), Some("XIV"));
    /// ```
    pub fn arabic_to_roman(num: u32) -> Option<&'static str> {
        for (a, r) in ARABIC_TO_ROMAN.iter() {
            if *a == num {
                return Some(r);
            }
        }
        None
    }

    /// Convert a spelled-out number to arabic
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(RomanNumeralConverter::spelled_to_arabic("Three"), Some(3));
    /// assert_eq!(RomanNumeralConverter::spelled_to_arabic("third"), Some(3));
    /// ```
    pub fn spelled_to_arabic(spelled: &str) -> Option<u32> {
        SPELLED_TO_ARABIC.get(spelled.to_lowercase().as_str()).copied()
    }

    /// Convert an arabic number to spelled-out form
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(RomanNumeralConverter::arabic_to_spelled(3), Some("Three"));
    /// ```
    pub fn arabic_to_spelled(num: u32) -> Option<&'static str> {
        for (a, s) in ARABIC_TO_SPELLED.iter() {
            if *a == num {
                return Some(s);
            }
        }
        None
    }

    /// Normalize all number formats in a string to arabic numbers
    ///
    /// Converts both roman numerals and spelled-out numbers to arabic.
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     RomanNumeralConverter::to_arabic("Back to the Future Part III"),
    ///     "Back to the Future Part 3"
    /// );
    /// assert_eq!(
    ///     RomanNumeralConverter::to_arabic("Back to the Future Part Three"),
    ///     "Back to the Future Part 3"
    /// );
    /// ```
    pub fn to_arabic(text: &str) -> String {
        let mut result = text.to_string();

        // Convert roman numerals to arabic
        result = ROMAN_REGEX.replace_all(&result, |caps: &regex::Captures| {
            let roman = &caps[1];
            Self::roman_to_arabic(roman)
                .map(|n| n.to_string())
                .unwrap_or_else(|| roman.to_string())
        }).to_string();

        // Convert spelled-out numbers to arabic
        result = SPELLED_REGEX.replace_all(&result, |caps: &regex::Captures| {
            let spelled = &caps[1];
            Self::spelled_to_arabic(spelled)
                .map(|n| n.to_string())
                .unwrap_or_else(|| spelled.to_string())
        }).to_string();

        result
    }

    /// Normalize all number formats in a string to roman numerals
    ///
    /// Converts both arabic numbers and spelled-out numbers to roman.
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     RomanNumeralConverter::to_roman("Back to the Future Part 3"),
    ///     "Back to the Future Part III"
    /// );
    /// assert_eq!(
    ///     RomanNumeralConverter::to_roman("Back to the Future Part Three"),
    ///     "Back to the Future Part III"
    /// );
    /// ```
    pub fn to_roman(text: &str) -> String {
        let mut result = text.to_string();

        // Convert spelled-out numbers to arabic first (intermediate step)
        result = SPELLED_REGEX.replace_all(&result, |caps: &regex::Captures| {
            let spelled = &caps[1];
            Self::spelled_to_arabic(spelled)
                .map(|n| n.to_string())
                .unwrap_or_else(|| spelled.to_string())
        }).to_string();

        // Convert arabic numbers to roman
        result = ARABIC_REGEX.replace_all(&result, |caps: &regex::Captures| {
            let num: u32 = caps[1].parse().unwrap_or(0);
            Self::arabic_to_roman(num)
                .map(|r| r.to_string())
                .unwrap_or_else(|| num.to_string())
        }).to_string();

        result
    }

    /// Generate all number variants of a title
    ///
    /// Returns the original title plus variants with numbers converted to
    /// arabic, roman, and spelled-out forms.
    ///
    /// # Example
    /// ```ignore
    /// let variants = RomanNumeralConverter::get_variants("Part III");
    /// // Returns: ["Part III", "Part 3", "Part Three"]
    /// ```
    pub fn get_variants(text: &str) -> Vec<String> {
        let mut variants = vec![text.to_string()];

        // Check if the text contains any numbers (roman, arabic, or spelled)
        let has_roman = ROMAN_REGEX.is_match(text);
        let has_arabic = ARABIC_REGEX.is_match(text);
        let has_spelled = SPELLED_REGEX.is_match(text);

        if has_roman || has_arabic || has_spelled {
            // Generate arabic variant
            let arabic = Self::to_arabic(text);
            if arabic != text {
                variants.push(arabic.clone());
            }

            // Generate roman variant
            let roman = Self::to_roman(text);
            if roman != text && !variants.contains(&roman) {
                variants.push(roman);
            }

            // Generate spelled variant from arabic
            let arabic_version = Self::to_arabic(text);
            let spelled = ARABIC_REGEX.replace_all(&arabic_version, |caps: &regex::Captures| {
                let num: u32 = caps[1].parse().unwrap_or(0);
                Self::arabic_to_spelled(num)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| num.to_string())
            }).to_string();

            if spelled != text && !variants.contains(&spelled) {
                variants.push(spelled);
            }
        }

        variants
    }

    /// Check if a string contains any number-like patterns
    pub fn contains_numbers(text: &str) -> bool {
        ROMAN_REGEX.is_match(text) || ARABIC_REGEX.is_match(text) || SPELLED_REGEX.is_match(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roman_to_arabic() {
        assert_eq!(RomanNumeralConverter::roman_to_arabic("I"), Some(1));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("III"), Some(3));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("IV"), Some(4));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("IX"), Some(9));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("XIV"), Some(14));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("XX"), Some(20));
        // Case insensitive
        assert_eq!(RomanNumeralConverter::roman_to_arabic("iii"), Some(3));
        assert_eq!(RomanNumeralConverter::roman_to_arabic("Iv"), Some(4));
    }

    #[test]
    fn test_arabic_to_roman() {
        assert_eq!(RomanNumeralConverter::arabic_to_roman(1), Some("I"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(3), Some("III"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(4), Some("IV"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(9), Some("IX"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(14), Some("XIV"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(20), Some("XX"));
        assert_eq!(RomanNumeralConverter::arabic_to_roman(100), None);
    }

    #[test]
    fn test_spelled_to_arabic() {
        assert_eq!(RomanNumeralConverter::spelled_to_arabic("One"), Some(1));
        assert_eq!(RomanNumeralConverter::spelled_to_arabic("three"), Some(3));
        assert_eq!(RomanNumeralConverter::spelled_to_arabic("FIVE"), Some(5));
        assert_eq!(RomanNumeralConverter::spelled_to_arabic("First"), Some(1));
        assert_eq!(RomanNumeralConverter::spelled_to_arabic("third"), Some(3));
    }

    #[test]
    fn test_to_arabic_roman_numerals() {
        assert_eq!(
            RomanNumeralConverter::to_arabic("Back to the Future Part III"),
            "Back to the Future Part 3"
        );
        assert_eq!(
            RomanNumeralConverter::to_arabic("Rocky IV"),
            "Rocky 4"
        );
        assert_eq!(
            RomanNumeralConverter::to_arabic("Star Wars Episode IX"),
            "Star Wars Episode 9"
        );
    }

    #[test]
    fn test_to_arabic_spelled_numbers() {
        assert_eq!(
            RomanNumeralConverter::to_arabic("Back to the Future Part Three"),
            "Back to the Future Part 3"
        );
        assert_eq!(
            RomanNumeralConverter::to_arabic("Ocean's Eleven"),
            "Ocean's 11"
        );
    }

    #[test]
    fn test_to_roman() {
        assert_eq!(
            RomanNumeralConverter::to_roman("Back to the Future Part 3"),
            "Back to the Future Part III"
        );
        assert_eq!(
            RomanNumeralConverter::to_roman("Back to the Future Part Three"),
            "Back to the Future Part III"
        );
        assert_eq!(
            RomanNumeralConverter::to_roman("Rocky 4"),
            "Rocky IV"
        );
    }

    #[test]
    fn test_get_variants() {
        let variants = RomanNumeralConverter::get_variants("Part III");
        assert!(variants.contains(&"Part III".to_string()));
        assert!(variants.contains(&"Part 3".to_string()));
        assert!(variants.contains(&"Part Three".to_string()));

        let variants = RomanNumeralConverter::get_variants("Part 3");
        assert!(variants.contains(&"Part 3".to_string()));
        assert!(variants.contains(&"Part III".to_string()));
        assert!(variants.contains(&"Part Three".to_string()));

        let variants = RomanNumeralConverter::get_variants("Part Three");
        assert!(variants.contains(&"Part Three".to_string()));
        assert!(variants.contains(&"Part 3".to_string()));
        assert!(variants.contains(&"Part III".to_string()));
    }

    #[test]
    fn test_get_variants_no_numbers() {
        let variants = RomanNumeralConverter::get_variants("The Matrix");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], "The Matrix");
    }

    #[test]
    fn test_back_to_the_future_case() {
        // The specific case that prompted this implementation
        let local = "Back to the Future Part Three";
        let tmdb = "Back to the Future Part III";

        let local_arabic = RomanNumeralConverter::to_arabic(local);
        let tmdb_arabic = RomanNumeralConverter::to_arabic(tmdb);

        assert_eq!(local_arabic, tmdb_arabic);
    }

    #[test]
    fn test_does_not_match_words_containing_roman() {
        // "I" should not match in "Iron Man" or "Inception"
        let result = RomanNumeralConverter::to_arabic("Iron Man");
        assert_eq!(result, "Iron Man");

        let result = RomanNumeralConverter::to_arabic("Inception");
        assert_eq!(result, "Inception");

        // But standalone "I" should match
        let result = RomanNumeralConverter::to_arabic("Rocky I");
        assert_eq!(result, "Rocky 1");
    }
}
