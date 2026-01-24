use crate::types::Token;

/// Separators that split tokens
const SEPARATORS: &[char] = &['.', ' ', '_', '-'];

/// Characters that are valid within tokens but might look like separators
#[allow(dead_code)]
const TOKEN_INTERNAL: &[char] = &['\'', '(', ')', '[', ']', '+'];

/// Tokenizer that splits a media filename into tokens while preserving positions
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize a filename into individual tokens
    /// 
    /// This handles:
    /// - Standard separators (. _ - space)
    /// - Preserving internal punctuation (apostrophes, parentheses)
    /// - Tracking byte positions for later matching
    pub fn tokenize(input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_start = 0;
        let mut current_token = String::new();
        let mut last_separator: Option<char> = None;
        let mut chars = input.char_indices().peekable();

        while let Some((idx, ch)) = chars.next() {
            if SEPARATORS.contains(&ch) {
                // End current token if non-empty
                if !current_token.is_empty() {
                    let mut token = Token::new(
                        current_token.clone(),
                        current_start,
                        idx,
                    );
                    token.separator_before = last_separator;
                    token.separator_after = Some(ch);
                    tokens.push(token);
                    current_token.clear();
                }
                last_separator = Some(ch);
                current_start = idx + ch.len_utf8();
            } else {
                if current_token.is_empty() {
                    current_start = idx;
                }
                current_token.push(ch);
            }
        }

        // Don't forget the last token
        if !current_token.is_empty() {
            let mut token = Token::new(
                current_token,
                current_start,
                input.len(),
            );
            token.separator_before = last_separator;
            tokens.push(token);
        }

        tokens
    }

    /// Tokenize but keep groups together (e.g., "SG-1" as one token)
    /// This is useful for titles that contain hyphens
    pub fn tokenize_smart(input: &str) -> Vec<Token> {
        let basic_tokens = Self::tokenize(input);
        Self::merge_title_tokens(basic_tokens, input)
    }

    /// Try to merge tokens that likely belong together
    /// e.g., "SG" + "1" after "Stargate" should become "SG-1"
    fn merge_title_tokens(tokens: Vec<Token>, _original: &str) -> Vec<Token> {
        if tokens.is_empty() {
            return tokens;
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            let token = &tokens[i];
            
            // Check for patterns like "SG-1" (short token + hyphen + number)
            if i + 1 < tokens.len() {
                let next = &tokens[i + 1];
                
                // If connected by hyphen and looks like a title component
                if token.separator_after == Some('-') 
                    && Self::should_merge_hyphenated(token, next)
                {
                    // Merge the tokens
                    let merged_value = format!("{}-{}", token.value, next.value);
                    let merged = Token {
                        value: merged_value,
                        start: token.start,
                        end: next.end,
                        separator_before: token.separator_before,
                        separator_after: next.separator_after,
                    };
                    result.push(merged);
                    i += 2;
                    continue;
                }
            }

            result.push(token.clone());
            i += 1;
        }

        result
    }

    /// Determine if two hyphen-connected tokens should be merged
    fn should_merge_hyphenated(left: &Token, right: &Token) -> bool {
        let left_val = &left.value;
        let right_val = &right.value;

        // Pattern: XX-N (like SG-1, X-Files, etc.)
        if left_val.len() <= 3 
            && right_val.chars().all(|c| c.is_ascii_digit())
            && right_val.len() <= 2
        {
            return true;
        }

        // Pattern: Word-Word (like Spider-Man, X-Men)
        if left_val.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && right_val.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !Self::is_known_tag(left_val)
            && !Self::is_known_tag(right_val)
        {
            return true;
        }

        false
    }

    /// Check if a token is a known tag (quality, codec, etc.)
    fn is_known_tag(s: &str) -> bool {
        let upper = s.to_uppercase();
        matches!(
            upper.as_str(),
            "HDTV" | "PDTV" | "WEB" | "WEBRIP" | "WEBDL" | "BLURAY" | "BDRIP" 
            | "DVDRIP" | "HDRIP" | "X264" | "X265" | "HEVC" | "AAC" | "AC3" 
            | "DTS" | "PROPER" | "REPACK" | "REAL" | "INTERNAL" | "LIMITED"
            | "HUN" | "ENG" | "GER" | "FRE" | "SPA" | "ITA" | "RUS" | "JPN"
        )
    }

    /// Get the extension from a filename
    pub fn extract_extension(input: &str) -> Option<(String, usize)> {
        if let Some(dot_pos) = input.rfind('.') {
            let ext = &input[dot_pos + 1..];
            // Validate it's a real extension (3-4 chars, alphanumeric)
            if ext.len() >= 2 
                && ext.len() <= 4 
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
            {
                let ext_lower = ext.to_lowercase();
                if matches!(ext_lower.as_str(), 
                    "mkv" | "mp4" | "avi" | "wmv" | "mov" | "m4v" | "ts" | "m2ts"
                    | "srt" | "sub" | "idx" | "ass" | "ssa" | "nfo" | "sfv" | "jpg" | "png"
                ) {
                    return Some((ext_lower, dot_pos));
                }
            }
        }
        None
    }

    /// Remove the extension from a filename
    pub fn strip_extension(input: &str) -> &str {
        if let Some((_, pos)) = Self::extract_extension(input) {
            &input[..pos]
        } else {
            input
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenize() {
        let tokens = Tokenizer::tokenize("Dark.Matter.S01E01.720p");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].value, "Dark");
        assert_eq!(tokens[1].value, "Matter");
        assert_eq!(tokens[2].value, "S01E01");
        assert_eq!(tokens[3].value, "720p");
    }

    #[test]
    fn test_smart_tokenize_sg1() {
        let tokens = Tokenizer::tokenize_smart("Stargate.SG-1.S01E01");
        assert_eq!(tokens[1].value, "SG-1");
    }

    #[test]
    fn test_extension_extraction() {
        let result = Tokenizer::extract_extension("movie.2020.720p.mkv");
        assert!(result.is_some());
        let (ext, pos) = result.unwrap();
        assert_eq!(ext, "mkv");
        // Position is where the dot is
        assert!(pos > 10);
        
        assert_eq!(
            Tokenizer::extract_extension("movie.2020.720p"),
            None
        );
    }

    #[test]
    fn test_positions() {
        let input = "Dark.Matter.2015";
        let tokens = Tokenizer::tokenize(input);
        
        assert_eq!(&input[tokens[0].start..tokens[0].end], "Dark");
        assert_eq!(&input[tokens[1].start..tokens[1].end], "Matter");
        assert_eq!(&input[tokens[2].start..tokens[2].end], "2015");
    }
}
