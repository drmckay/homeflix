//! NFO Parser Implementation
//!
//! Parses Kodi/XBMC .nfo files with encoding detection and multiple format support

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tracing::debug;

use super::dto::{NfoMetadata, MovieNfoRoot, EpisodeNfoRoot, TvShowNfoRoot};

// Pre-compiled regex patterns
static RE_IMDB: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"tt\d{7,8}").unwrap()
});

static RE_TMDB: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)tmdb\s*[:=]\s*(\d+)").unwrap()
});

static RE_YEAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(19|20)\d{2}\b").unwrap()
});

/// NFO file parser
pub struct NfoParser;

impl NfoParser {
    /// Parse an NFO file and extract metadata
    ///
    /// Tries multiple parsing strategies:
    /// 1. XML parsing for Kodi/XBMC format
    /// 2. Semantic/regex extraction for plain text
    ///
    /// # Arguments
    /// * `path` - Path to the .nfo file
    ///
    /// # Returns
    /// * `Ok(Some(NfoMetadata))` - If metadata was extracted
    /// * `Ok(None)` - If file doesn't exist or no useful data
    /// * `Err` - On I/O errors
    pub async fn parse(path: &Path) -> Result<Option<NfoMetadata>> {
        if !path.exists() {
            return Ok(None);
        }

        debug!("Parsing NFO file: {:?}", path);

        let mut file = tokio::fs::File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        // Encoding Detection
        // Try UTF-8, fallback to Latin-1 (ISO-8859-1)
        let content = match String::from_utf8(buffer.clone()) {
            Ok(s) => s,
            Err(_) => {
                // ISO-8859-1 decoding (map bytes to chars directly)
                buffer.iter().map(|&b| b as char).collect()
            }
        };

        // Try XML parsing first
        if let Some(meta) = Self::try_xml_parse(&content) {
            debug!("NFO parsed as XML: method={}", meta.extraction_method);
            return Ok(Some(meta));
        }

        // Fall back to semantic/regex parsing
        if let Some(meta) = Self::try_semantic_parse(&content) {
            debug!("NFO parsed semantically: has_imdb={}", meta.imdb_id.is_some());
            return Ok(Some(meta));
        }

        Ok(None)
    }

    /// Parse synchronously (for testing or non-async contexts)
    pub fn parse_sync(content: &str) -> Option<NfoMetadata> {
        Self::try_xml_parse(content).or_else(|| Self::try_semantic_parse(content))
    }

    /// Try to parse content as XML
    fn try_xml_parse(content: &str) -> Option<NfoMetadata> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with('<') {
            return None;
        }

        // Detect root element type
        let root_type = Self::detect_root_element(content);

        match root_type.as_deref() {
            Some("movie") => {
                if let Ok(m) = quick_xml::de::from_str::<MovieNfoRoot>(content) {
                    return Some(NfoMetadata {
                        title: Some(m.title),
                        original_title: m.originaltitle,
                        plot: m.plot,
                        year: m.year,
                        duration_min: m.runtime,
                        imdb_id: m.id,
                        tmdb_id: m.tmdbid,
                        genres: m.genre,
                        season: None,
                        episode: None,
                        is_xml: true,
                        credibility_score: 0.9,
                        extraction_method: "xml_movie".to_string(),
                    });
                }
            }
            Some("episodedetails") => {
                if let Ok(root) = quick_xml::de::from_str::<EpisodeNfoRoot>(content) {
                    let year = root
                        .aired
                        .as_ref()
                        .and_then(|d| d.split('-').next())
                        .and_then(|y| y.parse().ok());
                    return Some(NfoMetadata {
                        title: Some(root.title),
                        original_title: None,
                        plot: root.plot,
                        year,
                        duration_min: None,
                        imdb_id: root.id,
                        tmdb_id: root.tmdbid,
                        genres: Vec::new(),
                        season: root.season,
                        episode: root.episode,
                        is_xml: true,
                        credibility_score: 0.9,
                        extraction_method: "xml_episode".to_string(),
                    });
                }
            }
            Some("tvshow") => {
                if let Ok(root) = quick_xml::de::from_str::<TvShowNfoRoot>(content) {
                    return Some(NfoMetadata {
                        title: Some(root.title),
                        original_title: None,
                        plot: root.plot,
                        year: None,
                        duration_min: None,
                        imdb_id: root.id,
                        tmdb_id: root.tmdbid,
                        genres: Vec::new(),
                        season: None,
                        episode: None,
                        is_xml: true,
                        credibility_score: 0.9,
                        extraction_method: "xml_tvshow".to_string(),
                    });
                }
            }
            _ => {}
        }

        None
    }

    /// Detect the root element type from XML content
    fn detect_root_element(content: &str) -> Option<String> {
        // Skip XML declaration and whitespace, find root element
        let content = content.trim_start();

        // Skip <?xml ...?>
        let content = if content.starts_with("<?xml") {
            content.find("?>")
                .map(|pos| content[pos + 2..].trim_start())
                .unwrap_or(content)
        } else {
            content
        };

        // Find the first opening tag
        if let Some(start) = content.find('<') {
            let after_bracket = &content[start + 1..];
            // Find end of tag name (space, >, /)
            let end = after_bracket
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(after_bracket.len());
            let tag_name = &after_bracket[..end];
            if !tag_name.is_empty() && !tag_name.starts_with('!') && !tag_name.starts_with('?') {
                return Some(tag_name.to_lowercase());
            }
        }
        None
    }

    /// Try semantic/regex parsing for plain text NFOs
    fn try_semantic_parse(content: &str) -> Option<NfoMetadata> {
        let mut meta = NfoMetadata {
            is_xml: false,
            credibility_score: 0.0,
            extraction_method: "semantic".to_string(),
            ..Default::default()
        };

        // Extract IMDB ID
        if let Some(caps) = RE_IMDB.captures(content) {
            meta.imdb_id = Some(caps.get(0).unwrap().as_str().to_string());
            meta.credibility_score = 0.7; // ID found
        }

        // Extract TMDB ID (tmdb:12345 or tmdb=12345)
        if let Some(caps) = RE_TMDB.captures(content) {
            meta.tmdb_id = caps.get(1).and_then(|s| s.as_str().parse().ok());
            meta.credibility_score = 0.7;
        }

        // Extract Year
        if let Some(caps) = RE_YEAR.captures(content) {
            meta.year = caps.get(0).unwrap().as_str().parse().ok();
            if meta.credibility_score == 0.0 {
                meta.credibility_score = 0.2;
            }
        }

        // Only return if we found at least an ID
        if meta.imdb_id.is_some() || meta.tmdb_id.is_some() {
            return Some(meta);
        }

        None
    }

    /// Find the NFO file for a given media file
    ///
    /// Looks for:
    /// 1. Same name as video with .nfo extension
    /// 2. movie.nfo in same directory
    /// 3. tvshow.nfo in parent directory (for episodes)
    pub async fn find_nfo_for_media(media_path: &Path) -> Option<std::path::PathBuf> {
        // Same name with .nfo extension
        let nfo_path = media_path.with_extension("nfo");
        if nfo_path.exists() {
            return Some(nfo_path);
        }

        // movie.nfo in same directory
        if let Some(parent) = media_path.parent() {
            let movie_nfo = parent.join("movie.nfo");
            if movie_nfo.exists() {
                return Some(movie_nfo);
            }

            // tvshow.nfo in parent directory
            if let Some(grandparent) = parent.parent() {
                let tvshow_nfo = grandparent.join("tvshow.nfo");
                if tvshow_nfo.exists() {
                    return Some(tvshow_nfo);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_movie_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<movie>
    <title>Inception</title>
    <originaltitle>Inception</originaltitle>
    <year>2010</year>
    <plot>A thief who steals corporate secrets through dream-sharing technology.</plot>
    <runtime>148</runtime>
    <id>tt1375666</id>
    <tmdbid>27205</tmdbid>
</movie>"#;

        let meta = NfoParser::parse_sync(xml).expect("Should parse");
        assert_eq!(meta.title, Some("Inception".to_string()));
        assert_eq!(meta.year, Some(2010));
        assert_eq!(meta.imdb_id, Some("tt1375666".to_string()));
        assert_eq!(meta.tmdb_id, Some(27205));
        assert_eq!(meta.duration_min, Some(148));
        assert!(meta.is_xml);
        assert_eq!(meta.extraction_method, "xml_movie");
    }

    #[test]
    fn test_parse_episode_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<episodedetails>
    <title>Pilot</title>
    <season>1</season>
    <episode>1</episode>
    <plot>The beginning of the story.</plot>
    <aired>2008-01-20</aired>
    <id>tt0959621</id>
</episodedetails>"#;

        let meta = NfoParser::parse_sync(xml).expect("Should parse");
        assert_eq!(meta.title, Some("Pilot".to_string()));
        assert_eq!(meta.season, Some(1));
        assert_eq!(meta.episode, Some(1));
        assert_eq!(meta.year, Some(2008));
        assert_eq!(meta.extraction_method, "xml_episode");
    }

    #[test]
    fn test_parse_tvshow_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tvshow>
    <title>Breaking Bad</title>
    <plot>A high school chemistry teacher turned meth maker.</plot>
    <id>tt0903747</id>
    <tmdbid>1396</tmdbid>
</tvshow>"#;

        let meta = NfoParser::parse_sync(xml).expect("Should parse");
        assert_eq!(meta.title, Some("Breaking Bad".to_string()));
        assert_eq!(meta.imdb_id, Some("tt0903747".to_string()));
        assert_eq!(meta.tmdb_id, Some(1396));
        assert_eq!(meta.extraction_method, "xml_tvshow");
    }

    #[test]
    fn test_parse_semantic_imdb_only() {
        let text = r#"
        RELEASE INFO
        ============
        Movie: Inception (2010)
        IMDB: https://www.imdb.com/title/tt1375666/
        "#;

        let meta = NfoParser::parse_sync(text).expect("Should parse");
        assert_eq!(meta.imdb_id, Some("tt1375666".to_string()));
        assert!(!meta.is_xml);
        assert_eq!(meta.extraction_method, "semantic");
        assert!(meta.credibility_score > 0.5);
    }

    #[test]
    fn test_parse_semantic_tmdb() {
        let text = "tmdb: 27205\nYear: 2010";

        let meta = NfoParser::parse_sync(text).expect("Should parse");
        assert_eq!(meta.tmdb_id, Some(27205));
        assert_eq!(meta.year, Some(2010));
    }

    #[test]
    fn test_no_ids_returns_none() {
        let text = "Just some random text without any useful information";
        assert!(NfoParser::parse_sync(text).is_none());
    }
}
