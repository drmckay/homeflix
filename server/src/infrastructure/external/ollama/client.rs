//! OllamaClient - LLM-based subtitle translation
//!
//! Uses Ollama's HTTP API to translate subtitles between languages.
//! Processes segments in batches to maintain context while avoiding token limits.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::shared::error::TranslationError;
use crate::infrastructure::external::whisper::TranscriptionSegment;

/// Ollama API request body
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    /// How long to keep model loaded after request (0 = unload immediately)
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<i32>,
}

/// Ollama generation options
#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

/// Ollama API response
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

/// Ollama client for LLM-based translation
///
/// Uses the Ollama HTTP API to translate subtitle text.
/// Follows similar patterns to TmdbClient for HTTP operations.
pub struct OllamaClient {
    /// Ollama API base URL
    base_url: String,
    /// Model to use for translation
    model: String,
    /// HTTP client
    http_client: reqwest::Client,
    /// Request timeout
    timeout: Duration,
    /// Batch size for segment translation (maintains context)
    batch_size: usize,
}

impl OllamaClient {
    /// Creates a new OllamaClient
    ///
    /// # Arguments
    /// * `base_url` - Ollama API URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "gemma3:4b", "llama3")
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .expect("Failed to create HTTP client"),
            timeout: Duration::from_secs(300),
            batch_size: 10, // Translate 10 segments at a time for better context
        }
    }

    /// Creates OllamaClient with custom configuration
    pub fn with_config(
        base_url: &str,
        model: &str,
        timeout: Duration,
        batch_size: usize,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            http_client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to create HTTP client"),
            timeout,
            batch_size,
        }
    }

    /// Checks if Ollama is available and responding
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Checks if the specified model is available
    pub async fn model_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(models) = data.get("models").and_then(|m| m.as_array()) {
                        return models.iter().any(|m| {
                            m.get("name")
                                .and_then(|n| n.as_str())
                                .map(|n| n.starts_with(&self.model))
                                .unwrap_or(false)
                        });
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Unloads the model from VRAM
    ///
    /// This is important for systems with limited VRAM (8GB) where
    /// Whisper and Ollama compete for GPU memory.
    /// Sends a minimal request with keep_alive: 0 to force immediate unload.
    pub async fn unload_model(&self) -> Result<(), TranslationError> {
        let url = format!("{}/api/generate", self.base_url);

        // Send empty prompt with keep_alive: 0 to unload model
        let response = self.http_client
            .post(&url)
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": "",
                "keep_alive": 0
            }))
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| TranslationError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            // Ignore errors - model may not be loaded anyway
            tracing::debug!("Ollama unload returned non-success (model may not have been loaded)");
        } else {
            tracing::info!("Ollama model '{}' unloaded from VRAM", self.model);
        }

        Ok(())
    }

    /// Translates a single text string
    ///
    /// # Arguments
    /// * `text` - Text to translate
    /// * `source_lang` - Source language name (e.g., "English")
    /// * `target_lang` - Target language name (e.g., "Hungarian")
    pub async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, TranslationError> {
        let prompt = self.build_translation_prompt(text, source_lang, target_lang);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            options: Some(OllamaOptions {
                temperature: 0.3, // Lower temperature for more consistent translations
                num_predict: 2048,
            }),
            keep_alive: Some(0), // Unload immediately after to free VRAM
        };

        let url = format!("{}/api/generate", self.base_url);

        let response = self.http_client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| TranslationError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(TranslationError::ServiceUnavailable(
                format!("Ollama returned {}: {}", status, error_text)
            ));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| TranslationError::ParseError(e.to_string()))?;

        Ok(ollama_response.response.trim().to_string())
    }

    /// Translates transcription segments while preserving timestamps
    ///
    /// Processes segments in batches to maintain context for better translations.
    ///
    /// # Arguments
    /// * `segments` - Transcription segments to translate
    /// * `source_lang` - Source language name
    /// * `target_lang` - Target language name
    pub async fn translate_segments(
        &self,
        segments: Vec<TranscriptionSegment>,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<TranscriptionSegment>, TranslationError> {
        let mut translated_segments = Vec::with_capacity(segments.len());

        // Process in batches for context preservation
        for chunk in segments.chunks(self.batch_size) {
            let batch_text = chunk
                .iter()
                .enumerate()
                .map(|(i, s)| format!("[{}] {}", i + 1, s.text))
                .collect::<Vec<_>>()
                .join("\n");

            let translated_batch = self.translate_batch(&batch_text, source_lang, target_lang).await?;

            // Parse translated batch back to segments
            let translated_texts = parse_batch_response(&translated_batch, chunk.len());

            for (i, segment) in chunk.iter().enumerate() {
                translated_segments.push(TranscriptionSegment {
                    start_time: segment.start_time,
                    end_time: segment.end_time,
                    text: translated_texts.get(i)
                        .cloned()
                        .unwrap_or_else(|| segment.text.clone()),
                });
            }
        }

        Ok(translated_segments)
    }

    /// Translates a batch of numbered lines
    async fn translate_batch(
        &self,
        batch_text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, TranslationError> {
        // Build language-specific instructions for more natural output
        let style_instructions = build_style_instructions(target_lang);

        let prompt = format!(
            "You are translating movie/TV dialogue subtitles from {} to {}.\n\n\
             CRITICAL RULES:\n\
             1. Keep the [1], [2], [3] numbering exactly as is\n\
             2. Output ONLY the translations, nothing else\n\
             3. These are SPOKEN dialogues - use natural, everyday speech\n\
             4. Match the tone: casual speech stays casual, formal stays formal\n\
             5. Use contractions and colloquialisms appropriate for dialogue\n\
             6. If a segment appears NONSENSICAL or INCOMPLETE:\n\
                - Use surrounding context (previous/next segments) to understand the meaning\n\
                - Correct obvious transcription errors (misheard words that sound similar)\n\
                - Make the subtitle readable and sensible\n\
                - If truly unrecoverable, translate literally but keep it grammatical\n\n\
             {}\n\n\
             Subtitles to translate:\n{}",
            source_lang, target_lang, style_instructions, batch_text
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            options: Some(OllamaOptions {
                temperature: 0.3,
                num_predict: 4096,
            }),
            keep_alive: None, // Keep model loaded between batches for efficiency
        };

        let url = format!("{}/api/generate", self.base_url);

        let response = self.http_client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| TranslationError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(TranslationError::TranslationFailed(
                format!("Ollama returned {}: {}", status, error_text)
            ));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| TranslationError::ParseError(e.to_string()))?;

        Ok(ollama_response.response)
    }

    /// Builds the translation prompt for single text
    fn build_translation_prompt(&self, text: &str, source_lang: &str, target_lang: &str) -> String {
        let style_instructions = build_style_instructions(target_lang);
        format!(
            "Translate this {} dialogue to natural, spoken {}.\n\n\
             {}\n\n\
             Text to translate:\n{}\n\n\
             Output ONLY the translation, nothing else:",
            source_lang, target_lang, style_instructions, text
        )
    }
}

/// Parses numbered batch response back into individual texts
fn parse_batch_response(response: &str, expected_count: usize) -> Vec<String> {
    let mut results = Vec::with_capacity(expected_count);

    // Try to parse numbered format [1], [2], etc.
    for i in 1..=expected_count {
        let current_marker = format!("[{}]", i);
        let next_marker = format!("[{}]", i + 1);

        if let Some(start) = response.find(&current_marker) {
            let text_start = start + current_marker.len();
            let text_end = if i < expected_count {
                response[text_start..].find(&next_marker)
                    .map(|pos| text_start + pos)
                    .unwrap_or(response.len())
            } else {
                response.len()
            };

            let text = response[text_start..text_end].trim().to_string();
            results.push(text);
        }
    }

    // Fallback: if parsing failed, split by newlines
    if results.len() != expected_count {
        results = response
            .lines()
            .map(|l| {
                // Remove any [N] prefix
                let line = l.trim();
                if line.starts_with('[') {
                    if let Some(end) = line.find(']') {
                        return line[end + 1..].trim().to_string();
                    }
                }
                line.to_string()
            })
            .filter(|l| !l.is_empty())
            .take(expected_count)
            .collect();
    }

    // Pad with empty strings if still not enough
    while results.len() < expected_count {
        results.push(String::new());
    }

    results
}

/// Builds language-specific style instructions for more natural translations
fn build_style_instructions(target_lang: &str) -> &'static str {
    match target_lang.to_lowercase().as_str() {
        "hungarian" | "magyar" => {
            "HUNGARIAN STYLE GUIDE:\n\
             - Use everyday spoken Hungarian, NOT literary/written style\n\
             - Prefer informal conjugations for casual dialogue (te-forma, not ön-forma)\n\
             - Avoid overly formal, archaic, or foreign-sounding structures\n\
             - Use natural Hungarian word order (topic-focus-verb)\n\
             - Keep sentences short and punchy, as people actually speak\n\
             - Drop unnecessary pronouns (én, te, ő) - Hungarian conjugation makes them clear\n\
             - Avoid 'Ő azt mondta, hogy...' - use 'Azt mondta,' instead\n\
             - Common contractions: 'nem tudom' not 'nem tudhatom'\n\n\
             Examples of good casual Hungarian:\n\
               'What are you doing?' → 'Mit csinálsz?' (NOT 'Mit teszel?')\n\
               'I don't know' → 'Nem tudom' or 'Fogalmam sincs'\n\
               'Come on!' → 'Gyerünk!' or 'Na gyere!'\n\
               'Are you crazy?' → 'Megőrültél?' (NOT 'Elment az eszed?')\n\
               'Let's go' → 'Menjünk' or 'Gyerünk'\n\
               'What the hell?' → 'Mi a fene?' or 'Mi a franc?'\n\
               'He said that...' → 'Azt mondta...' (NOT 'Ő azt mondta, hogy...')\n\
               'I think so' → 'Szerintem igen' or 'Azt hiszem'"
        }
        "german" | "deutsch" => {
            "GERMAN STYLE GUIDE:\n\
             - Use conversational German appropriate for dialogue\n\
             - Prefer du-form for casual conversations, Sie-form only when clearly formal\n\
             - Use common spoken forms and contractions\n\
             - Natural word order for dialogue, not overly formal Schriftsprache"
        }
        "spanish" | "español" => {
            "SPANISH STYLE GUIDE:\n\
             - Use natural conversational Spanish\n\
             - Prefer tú-form for casual dialogue, usted only when clearly formal\n\
             - Use common contractions and colloquial expressions\n\
             - Match the register of the original dialogue"
        }
        "french" | "français" => {
            "FRENCH STYLE GUIDE:\n\
             - Use natural spoken French, not literary style\n\
             - Prefer tu-form for casual dialogue, vous for formal contexts\n\
             - Include common spoken contractions (j'sais pas, t'as vu, etc.)\n\
             - Match the casual/formal register of the original"
        }
        _ => {
            "Use natural, conversational language appropriate for spoken dialogue.\n\
             Avoid overly formal or literary expressions."
        }
    }
}

/// Language code to full name mapping for prompts
pub fn language_code_to_name(code: &str) -> &str {
    match code.to_lowercase().as_str() {
        "en" | "eng" => "English",
        "hu" | "hun" => "Hungarian",
        "de" | "deu" | "ger" => "German",
        "es" | "spa" => "Spanish",
        "fr" | "fra" => "French",
        "it" | "ita" => "Italian",
        "pt" | "por" => "Portuguese",
        "ru" | "rus" => "Russian",
        "ja" | "jpn" => "Japanese",
        "ko" | "kor" => "Korean",
        "zh" | "zho" | "chi" => "Chinese",
        "pl" | "pol" => "Polish",
        "nl" | "nld" | "dut" => "Dutch",
        "sv" | "swe" => "Swedish",
        "cs" | "ces" | "cze" => "Czech",
        "ro" | "ron" | "rum" => "Romanian",
        _ => code,
    }
}

/// Language name to code mapping
pub fn language_name_to_code(name: &str) -> &str {
    match name.to_lowercase().as_str() {
        "english" => "en",
        "hungarian" | "magyar" => "hu",
        "german" | "deutsch" => "de",
        "spanish" | "español" => "es",
        "french" | "français" => "fr",
        "italian" | "italiano" => "it",
        "portuguese" | "português" => "pt",
        "russian" | "русский" => "ru",
        "japanese" | "日本語" => "ja",
        "korean" | "한국어" => "ko",
        "chinese" | "中文" => "zh",
        "polish" | "polski" => "pl",
        "dutch" | "nederlands" => "nl",
        "swedish" | "svenska" => "sv",
        "czech" | "čeština" => "cs",
        "romanian" | "română" => "ro",
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_batch_response_numbered() {
        let response = "[1] Szia világ\n[2] Második sor\n[3] Harmadik sor";
        let results = parse_batch_response(response, 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "Szia világ");
        assert_eq!(results[1], "Második sor");
        assert_eq!(results[2], "Harmadik sor");
    }

    #[test]
    fn test_parse_batch_response_fallback() {
        let response = "Szia világ\nMásodik sor\nHarmadik sor";
        let results = parse_batch_response(response, 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "Szia világ");
        assert_eq!(results[1], "Második sor");
        assert_eq!(results[2], "Harmadik sor");
    }

    #[test]
    fn test_language_code_to_name() {
        assert_eq!(language_code_to_name("en"), "English");
        assert_eq!(language_code_to_name("hu"), "Hungarian");
        assert_eq!(language_code_to_name("de"), "German");
        assert_eq!(language_code_to_name("unknown"), "unknown");
    }

    #[test]
    fn test_language_name_to_code() {
        assert_eq!(language_name_to_code("English"), "en");
        assert_eq!(language_name_to_code("Hungarian"), "hu");
        assert_eq!(language_name_to_code("German"), "de");
    }
}
