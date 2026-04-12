// Nanobanana API Client
// Handles communication with Google Gemini's image generation API.
// Uses Gemini 2.5 Flash Image model.
//
// API Reference: https://ai.google.dev/gemini-api/docs/image-generation

use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-2.5-flash-image";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NanobananOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_images: Option<Vec<ReferenceImage>>,
}

impl Default for NanobananOptions {
    fn default() -> Self {
        Self {
            model: Some(DEFAULT_MODEL.to_string()),
            reference_images: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceImage {
    pub data: String, // base64 encoded
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NanobananaResult {
    pub image_data: String, // base64 encoded
    pub mime_type: String,
}

// Gemini API request types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: GeminiInlineData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<String>,
}

// Gemini API response types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiCandidateContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
    #[serde(rename = "inlineData")]
    inline_data: Option<GeminiResponseInlineData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponseInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiError {
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NanobananaConfig {
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub reference_images: Vec<ReferenceImage>,
}

// ============================================================================
// Commands
// ============================================================================

/// Generate an image using Gemini's image generation

pub async fn nanobanana_generate(
    api_key: String,
    prompt: String,
    options: Option<NanobananOptions>,
) -> CmdResult<NanobananaResult> {
    if api_key.is_empty() {
        return Err(CommandError::Config("Gemini API key is required".to_string()));
    }
    if prompt.trim().is_empty() {
        return Err(CommandError::Config("Image prompt is required".to_string()));
    }

    let opts = options.unwrap_or_default();
    let model = opts.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    // Build parts array - reference images first, then text prompt
    let mut parts: Vec<GeminiPart> = Vec::new();

    // Add reference images if provided
    if let Some(ref_images) = opts.reference_images {
        for img in ref_images {
            parts.push(GeminiPart::InlineData {
                inline_data: GeminiInlineData {
                    mime_type: img.mime_type,
                    data: img.data,
                },
            });
        }
    }

    // Add text prompt
    parts.push(GeminiPart::Text { text: prompt });

    let request = GeminiRequest {
        contents: vec![GeminiContent { parts }],
        generation_config: GeminiGenerationConfig {
            response_modalities: vec!["TEXT".to_string(), "IMAGE".to_string()],
        },
    };

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let gemini_response: GeminiResponse = response
        .json()
        .await?;

    // Check for API errors
    if let Some(error) = gemini_response.error {
        return Err(CommandError::Network(format!("Gemini API error: {}", error.message)));
    }

    // Extract image from response
    let candidates = gemini_response
        .candidates
        .ok_or(CommandError::Internal("No candidates in response".to_string()))?;

    if candidates.is_empty() {
        return Err(CommandError::Internal("No candidates in response".to_string()));
    }

    let content = candidates[0]
        .content
        .as_ref()
        .ok_or_else(|| {
            // Check for finish reason
            if let Some(reason) = &candidates[0].finish_reason {
                if reason == "RECITATION" {
                    return CommandError::Internal("Image generation blocked due to content policy. Please modify your prompt.".to_string());
                }
            }
            CommandError::Internal("No content in response".to_string())
        })?;

    // Find the inline data (image) part
    for part in &content.parts {
        if let Some(inline_data) = &part.inline_data {
            return Ok(NanobananaResult {
                image_data: inline_data.data.clone(),
                mime_type: inline_data.mime_type.clone(),
            });
        }
    }

    // Check if there's text explaining why image wasn't generated
    for part in &content.parts {
        if let Some(text) = &part.text {
            return Err(CommandError::Internal(format!("Image generation failed: {}", text)));
        }
    }

    Err(CommandError::Internal("No image data in response".to_string()))
}

/// Generate an image and save it to a file

pub async fn nanobanana_generate_to_file(
    api_key: String,
    prompt: String,
    output_path: String,
    options: Option<NanobananOptions>,
) -> CmdResult<String> {
    let result = nanobanana_generate(api_key, prompt, options).await?;

    // Determine file extension from mime type
    let ext = get_extension_from_mime_type(&result.mime_type);

    // Ensure output path has correct extension
    let output_path = if output_path.ends_with(&format!(".{}", ext)) {
        output_path
    } else {
        format!("{}.{}", output_path.trim_end_matches(&format!(".{}", ext)), ext)
    };

    // Decode base64 and write to file
    let image_data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &result.image_data,
    )
    .map_err(|e| CommandError::Parse(format!("Failed to decode image data: {}", e)))?;

    fs::write(&output_path, image_data)?;

    Ok(output_path)
}

/// Parse nanobanana config from markdown frontmatter or JSON

pub fn nanobanana_parse_config(content: String) -> CmdResult<NanobananaConfig> {
    // Try to parse as JSON first (for .nanobanana.json files)
    if let Ok(json_config) = serde_json::from_str::<serde_json::Value>(&content) {
        let prompt = json_config
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let model = json_config
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let reference_images = json_config
            .get("reference_images")
            .or_else(|| json_config.get("referenceImages"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let data = item.get("data")?.as_str()?.to_string();
                        let mime_type = item.get("mimeType").or_else(|| item.get("mime_type"))?.as_str()?.to_string();
                        Some(ReferenceImage { data, mime_type })
                    })
                    .collect()
            })
            .unwrap_or_default();

        return Ok(NanobananaConfig {
            prompt,
            model,
            reference_images,
        });
    }

    // Parse as markdown frontmatter
    let mut config = NanobananaConfig {
        prompt: None,
        model: None,
        reference_images: Vec::new(),
    };

    // Match frontmatter between --- markers
    let frontmatter_regex = regex::Regex::new(r"^---\s*\n([\s\S]*?)\n---")
        .map_err(|e| CommandError::Parse(format!("Regex error: {}", e)))?;

    if let Some(captures) = frontmatter_regex.captures(&content) {
        let frontmatter = &captures[1];

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some(colon_idx) = line.find(':') {
                let key = line[..colon_idx].trim();
                let mut value = line[colon_idx + 1..].trim().to_string();

                // Remove quotes
                if (value.starts_with('\'') && value.ends_with('\''))
                    || (value.starts_with('"') && value.ends_with('"'))
                {
                    value = value[1..value.len() - 1].to_string();
                }

                match key {
                    "nanobanana_prompt" => config.prompt = Some(value),
                    "nanobanana_model" => config.model = Some(value),
                    _ => {}
                }
            }
        }
    }

    Ok(config)
}

/// Load reference images from file paths, returning base64-encoded ReferenceImage structs.
/// Supports comma-separated paths in a single string or a YAML-style list.
pub fn load_reference_images_from_paths(
    paths_str: &str,
    base_dir: &Path,
) -> Vec<ReferenceImage> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let paths: Vec<&str> = paths_str
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'' || c == '[' || c == ']'))
        .filter(|s| !s.is_empty())
        .collect();

    let mut images = Vec::new();
    for path_str in paths {
        let path = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            base_dir.join(path_str)
        };

        if let Ok(bytes) = fs::read(&path) {
            let data = STANDARD.encode(&bytes);
            let mime_type = match path.extension().and_then(|e| e.to_str()) {
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("webp") => "image/webp",
                _ => "image/png",
            }
            .to_string();
            images.push(ReferenceImage { data, mime_type });
        }
    }
    images
}

/// Generate image from a markdown file with nanobanana_prompt in frontmatter.
/// Supports `nanobanana_references` field with comma-separated file paths
/// that are automatically loaded and base64-encoded as reference images.

pub async fn nanobanana_generate_from_file(
    api_key: String,
    file_path: String,
    output_path: Option<String>,
    options: Option<NanobananOptions>,
) -> CmdResult<String> {
    let content = fs::read_to_string(&file_path)?;
    let base_dir = Path::new(&file_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let config = nanobanana_parse_config(content.clone())?;

    let prompt = config
        .prompt
        .ok_or(CommandError::Config("No nanobanana_prompt found in frontmatter".to_string()))?;

    // Merge options
    let mut merged_options = options.unwrap_or_default();
    if merged_options.model.is_none() {
        merged_options.model = config.model;
    }
    if merged_options.reference_images.is_none() && !config.reference_images.is_empty() {
        merged_options.reference_images = Some(config.reference_images);
    }

    // Load reference images from file paths (nanobanana_references frontmatter)
    if merged_options.reference_images.is_none() || merged_options.reference_images.as_ref().map_or(true, |v| v.is_empty()) {
        // Parse nanobanana_references from frontmatter
        let frontmatter_regex = regex::Regex::new(r"^---\s*\n([\s\S]*?)\n---")
            .map_err(|e| CommandError::Parse(format!("Regex error: {}", e)))?;
        if let Some(captures) = frontmatter_regex.captures(&content) {
            let frontmatter = &captures[1];
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some(colon_idx) = line.find(':') {
                    let key = line[..colon_idx].trim();
                    let value = line[colon_idx + 1..].trim().trim_matches(|c| c == '"' || c == '\'');
                    if key == "nanobanana_references" && !value.is_empty() {
                        let ref_images = load_reference_images_from_paths(value, &base_dir);
                        if !ref_images.is_empty() {
                            merged_options.reference_images = Some(ref_images);
                        }
                    }
                }
            }
        }
    }

    // Determine output path
    let output = output_path.unwrap_or_else(|| {
        let path = Path::new(&file_path);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let parent = path.parent().unwrap_or(Path::new("."));
        parent
            .join(format!("{}_nanobanana", stem))
            .to_string_lossy()
            .to_string()
    });

    nanobanana_generate_to_file(api_key, prompt, output, Some(merged_options)).await
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get file extension from MIME type
pub fn get_extension_from_mime_type(mime_type: &str) -> &str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

/// List available models for image generation

pub fn nanobanana_list_models() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "Gemini 2.5 Flash Image (Default)",
            "value": "gemini-2.5-flash-image"
        }),
        serde_json::json!({
            "name": "Gemini 2.0 Flash (Fast)",
            "value": "gemini-2.0-flash-exp"
        }),
        serde_json::json!({
            "name": "Gemini 2.0 Flash Thinking (Quality)",
            "value": "gemini-2.0-flash-thinking-exp-01-21"
        }),
    ]
}
