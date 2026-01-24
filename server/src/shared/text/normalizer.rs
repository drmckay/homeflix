//! Title normalization utilities
//!
//! Provides functions to normalize media titles for comparison by:
//! - Removing/normalizing punctuation
//! - Handling leading articles (The, A, An)
//! - Normalizing whitespace and separators
//! - Converting number formats

use once_cell::sync::Lazy;
use regex::Regex;
use super::RomanNumeralConverter;

/// Regex to match leading articles
static LEADING_ARTICLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(the|a|an)\s+").unwrap()
});

/// Regex to match trailing articles (from TMDB format like "Avengers, The")
static TRAILING_ARTICLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i),\s*(the|a|an)$").unwrap()
});

/// Regex to match subtitles after colon or dash
static SUBTITLE_SEPARATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s*[:\-–—]\s*").unwrap()
});

/// Regex to match multiple whitespace
static MULTIPLE_SPACES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s+").unwrap()
});

/// Regex to match "Part I/1/One" suffixes (first movie in series often doesn't have this on TMDB)
static PART_ONE_SUFFIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+Part\s+(I|1|One)$").unwrap()
});

/// Regex to match common punctuation to remove
static PUNCTUATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[''""!?,;.:]"#).unwrap()
});

/// Regex to match separators to convert to spaces
static SEPARATORS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[._]").unwrap()
});

/// Title normalizer for media matching
pub struct TitleNormalizer;

impl TitleNormalizer {
    /// Basic normalization - clean up whitespace and separators
    ///
    /// This is a light normalization that preserves most of the original title
    /// structure while making it easier to read.
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(TitleNormalizer::normalize("Spider.Man"), "Spider Man");
    /// assert_eq!(TitleNormalizer::normalize("  Multiple   Spaces  "), "Multiple Spaces");
    /// ```
    pub fn normalize(title: &str) -> String {
        let mut result = title.to_string();

        // Convert separators to spaces
        result = SEPARATORS.replace_all(&result, " ").to_string();

        // Convert hyphens surrounded by spaces to space (but keep compound words like "Spider-Man")
        result = result.replace(" - ", " ");

        // Collapse multiple spaces
        result = MULTIPLE_SPACES.replace_all(&result, " ").to_string();

        // Trim whitespace
        result.trim().to_string()
    }

    /// Aggressive normalization for comparison
    ///
    /// This normalizes the title to maximize matching potential:
    /// - Removes articles (The, A, An)
    /// - Removes punctuation
    /// - Normalizes all number formats to arabic
    /// - Converts to lowercase
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     TitleNormalizer::normalize_for_comparison("The Avengers"),
    ///     "avengers"
    /// );
    /// assert_eq!(
    ///     TitleNormalizer::normalize_for_comparison("Spider-Man: Homecoming"),
    ///     "spider man homecoming"
    /// );
    /// ```
    pub fn normalize_for_comparison(title: &str) -> String {
        let mut result = title.to_string();

        // Handle trailing articles FIRST (TMDB format: "Avengers, The")
        // Must do this before removing punctuation
        result = TRAILING_ARTICLE.replace(&result, "").to_string();

        // Convert separators to spaces
        result = SEPARATORS.replace_all(&result, " ").to_string();

        // Convert hyphens to spaces (Spider-Man -> Spider Man)
        result = result.replace('-', " ");

        // Remove punctuation
        result = PUNCTUATION.replace_all(&result, "").to_string();

        // Remove leading articles
        result = LEADING_ARTICLE.replace(&result, "").to_string();

        // Convert all numbers to arabic for consistent comparison
        result = RomanNumeralConverter::to_arabic(&result);

        // Collapse multiple spaces
        result = MULTIPLE_SPACES.replace_all(&result, " ").to_string();

        // Convert to lowercase and trim
        result.to_lowercase().trim().to_string()
    }

    /// Remove subtitles after colon/dash
    ///
    /// Useful when the main title is enough for matching.
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     TitleNormalizer::remove_subtitle("Mission: Impossible - Fallout"),
    ///     "Mission"
    /// );
    /// ```
    pub fn remove_subtitle(title: &str) -> String {
        // Split on subtitle separators and take the first part
        let parts: Vec<&str> = SUBTITLE_SEPARATOR.split(title).collect();
        if let Some(first) = parts.first() {
            first.trim().to_string()
        } else {
            title.to_string()
        }
    }

    /// Extract the main title without parenthetical year or info
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     TitleNormalizer::remove_parenthetical("The Matrix (1999)"),
    ///     "The Matrix"
    /// );
    /// ```
    pub fn remove_parenthetical(title: &str) -> String {
        // Remove content in parentheses at the end
        let re = Regex::new(r"\s*\([^)]*\)\s*$").unwrap();
        re.replace(title, "").trim().to_string()
    }

    /// Get canonical form of article placement
    ///
    /// Converts "Avengers, The" to "The Avengers"
    ///
    /// # Example
    /// ```ignore
    /// assert_eq!(
    ///     TitleNormalizer::normalize_article_placement("Avengers, The"),
    ///     "The Avengers"
    /// );
    /// ```
    pub fn normalize_article_placement(title: &str) -> String {
        if let Some(caps) = TRAILING_ARTICLE.captures(title) {
            let article = &caps[1];
            let main_title = TRAILING_ARTICLE.replace(title, "").to_string();
            format!("{} {}", article, main_title.trim())
        } else {
            title.to_string()
        }
    }

    /// Generate search variants for a title
    ///
    /// Returns multiple versions of a title that might match TMDB:
    /// - Original title
    /// - Without articles
    /// - Without subtitle
    /// - Number variants (roman/arabic/spelled)
    ///
    /// # Example
    /// ```ignore
    /// let variants = TitleNormalizer::get_search_variants("The Matrix: Reloaded");
    /// // Contains: "The Matrix: Reloaded", "Matrix: Reloaded", "The Matrix", "Matrix"
    /// ```
    pub fn get_search_variants(title: &str) -> Vec<String> {
        let mut variants = Vec::new();

        // Original (normalized)
        let normalized = Self::normalize(title);
        variants.push(normalized.clone());

        // Without leading article
        let no_article = LEADING_ARTICLE.replace(&normalized, "").to_string();
        if no_article != normalized {
            variants.push(no_article.clone());
        }

        // Without subtitle
        let no_subtitle = Self::remove_subtitle(&normalized);
        if no_subtitle != normalized && !variants.contains(&no_subtitle) {
            variants.push(no_subtitle.clone());

            // Without subtitle AND without article
            let no_subtitle_no_article = LEADING_ARTICLE.replace(&no_subtitle, "").to_string();
            if no_subtitle_no_article != no_subtitle && !variants.contains(&no_subtitle_no_article) {
                variants.push(no_subtitle_no_article);
            }
        }

        // Number variants
        for variant in &variants.clone() {
            if RomanNumeralConverter::contains_numbers(variant) {
                for num_variant in RomanNumeralConverter::get_variants(variant) {
                    if !variants.contains(&num_variant) {
                        variants.push(num_variant);
                    }
                }
            }
        }

        // Strip "Part I/1/One" suffix - first movies often don't have this on TMDB
        // e.g., "Back to the Future Part I" -> "Back to the Future"
        for variant in &variants.clone() {
            if PART_ONE_SUFFIX.is_match(variant) {
                let without_part_one = PART_ONE_SUFFIX.replace(variant, "").trim().to_string();
                if !without_part_one.is_empty() && !variants.contains(&without_part_one) {
                    variants.push(without_part_one);
                }
            }
        }

        variants
    }

    /// Check if two titles match after normalization
    ///
    /// This is an exact match after aggressive normalization.
    ///
    /// # Example
    /// ```ignore
    /// assert!(TitleNormalizer::titles_match(
    ///     "The Avengers",
    ///     "Avengers, The"
    /// ));
    /// ```
    pub fn titles_match(a: &str, b: &str) -> bool {
        Self::normalize_for_comparison(a) == Self::normalize_for_comparison(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        assert_eq!(TitleNormalizer::normalize("Spider.Man"), "Spider Man");
        assert_eq!(TitleNormalizer::normalize("Spider_Man"), "Spider Man");
        assert_eq!(TitleNormalizer::normalize("  Multiple   Spaces  "), "Multiple Spaces");
        assert_eq!(TitleNormalizer::normalize("Title - With - Dashes"), "Title With Dashes");
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("The Avengers"),
            "avengers"
        );
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("Avengers, The"),
            "avengers"
        );
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("Spider-Man: Homecoming"),
            "spider man homecoming"
        );
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("A Bug's Life"),
            "bugs life"
        );
    }

    #[test]
    fn test_normalize_numbers() {
        // Roman numerals should be converted to arabic
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("Rocky III"),
            "rocky 3"
        );
        // Spelled numbers should be converted to arabic
        assert_eq!(
            TitleNormalizer::normalize_for_comparison("Ocean's Eleven"),
            "oceans 11"
        );
    }

    #[test]
    fn test_remove_subtitle() {
        assert_eq!(
            TitleNormalizer::remove_subtitle("Mission: Impossible - Fallout"),
            "Mission"
        );
        assert_eq!(
            TitleNormalizer::remove_subtitle("Star Wars: Episode IV"),
            "Star Wars"
        );
        assert_eq!(
            TitleNormalizer::remove_subtitle("The Matrix"),
            "The Matrix"
        );
    }

    #[test]
    fn test_remove_parenthetical() {
        assert_eq!(
            TitleNormalizer::remove_parenthetical("The Matrix (1999)"),
            "The Matrix"
        );
        assert_eq!(
            TitleNormalizer::remove_parenthetical("Blade Runner (Director's Cut)"),
            "Blade Runner"
        );
    }

    #[test]
    fn test_normalize_article_placement() {
        assert_eq!(
            TitleNormalizer::normalize_article_placement("Avengers, The"),
            "The Avengers"
        );
        assert_eq!(
            TitleNormalizer::normalize_article_placement("Godfather, The"),
            "The Godfather"
        );
        assert_eq!(
            TitleNormalizer::normalize_article_placement("Matrix, A"),
            "A Matrix"
        );
        // No change if no trailing article
        assert_eq!(
            TitleNormalizer::normalize_article_placement("The Matrix"),
            "The Matrix"
        );
    }

    #[test]
    fn test_get_search_variants() {
        let variants = TitleNormalizer::get_search_variants("The Matrix: Reloaded");
        assert!(variants.contains(&"The Matrix: Reloaded".to_string()));
        assert!(variants.contains(&"Matrix: Reloaded".to_string()));
        assert!(variants.contains(&"The Matrix".to_string()));
        assert!(variants.contains(&"Matrix".to_string()));
    }

    #[test]
    fn test_get_search_variants_with_numbers() {
        let variants = TitleNormalizer::get_search_variants("Rocky III");
        assert!(variants.contains(&"Rocky III".to_string()));
        assert!(variants.contains(&"Rocky 3".to_string()));
        assert!(variants.contains(&"Rocky Three".to_string()));
    }

    #[test]
    fn test_titles_match() {
        assert!(TitleNormalizer::titles_match("The Avengers", "Avengers, The"));
        // Note: Spider-Man vs Spiderman won't match exactly because
        // "spider man" != "spiderman" after normalization
        // This is handled by fuzzy matching instead
        assert!(TitleNormalizer::titles_match("Rocky III", "Rocky 3"));
        assert!(TitleNormalizer::titles_match("Ocean's Eleven", "Oceans 11"));

        // These should NOT match
        assert!(!TitleNormalizer::titles_match("The Matrix", "The Matrix Reloaded"));
        assert!(!TitleNormalizer::titles_match("Iron Man", "Iron Man 2"));
    }

    #[test]
    fn test_back_to_the_future_case() {
        assert!(TitleNormalizer::titles_match(
            "Back to the Future Part Three",
            "Back to the Future Part III"
        ));
        assert!(TitleNormalizer::titles_match(
            "Back to the Future Part Two",
            "Back to the Future Part II"
        ));
    }

    #[test]
    fn test_part_one_stripping() {
        // First movies often don't have "Part I" on TMDB
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part I");
        assert!(variants.contains(&"Back to the Future Part I".to_string()));
        assert!(variants.contains(&"Back to the Future".to_string()),
            "Should contain variant without 'Part I': {:?}", variants);

        // Also test with arabic numeral
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part 1");
        assert!(variants.contains(&"Back to the Future".to_string()),
            "Should contain variant without 'Part 1': {:?}", variants);

        // And spelled out
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part One");
        assert!(variants.contains(&"Back to the Future".to_string()),
            "Should contain variant without 'Part One': {:?}", variants);
    }

    #[test]
    fn test_bttf_all_movies_variants() {
        // BTTF Part I - TMDB title is "Back to the Future" without "Part I"
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part I");
        assert!(variants.contains(&"Back to the Future".to_string()),
            "Part I should search without 'Part I' suffix: {:?}", variants);

        // BTTF Part Two - TMDB title is "Back to the Future Part II"
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part Two");
        assert!(variants.contains(&"Back to the Future Part II".to_string()),
            "Part Two should search with roman numeral 'Part II': {:?}", variants);

        // BTTF Part Three - TMDB title is "Back to the Future Part III"
        let variants = TitleNormalizer::get_search_variants("Back to the Future Part Three");
        assert!(variants.contains(&"Back to the Future Part III".to_string()),
            "Part Three should search with roman numeral 'Part III': {:?}", variants);
    }
}

#[test]
fn test_print_bttf_variants() {
    println!("\n=== Part Two variants ===");
    let variants = TitleNormalizer::get_search_variants("Back to the Future Part Two");
    for (i, v) in variants.iter().enumerate() {
        println!("  {}: {}", i, v);
    }
    
    println!("\n=== Part Three variants ===");
    let variants = TitleNormalizer::get_search_variants("Back to the Future Part Three");
    for (i, v) in variants.iter().enumerate() {
        println!("  {}: {}", i, v);
    }
}
