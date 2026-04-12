// Gamma API Client
// Handles communication with Gamma's Generate API to create presentations
// from markdown content.
//
// API Reference: https://developers.gamma.app/docs/generate-api-parameters-explained

use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};

const GAMMA_API_BASE: &str = "https://public-api.gamma.app/v1.0";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>, // "brief" | "medium" | "detailed" | "extensive"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>, // "aiGenerated" | "pictographic" | "unsplash" | "webFreeToUse" | "noImages"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaGenerationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_mode: Option<String>, // "generate" | "condense" | "preserve"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>, // "presentation" | "document" | "social" | "webpage"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_cards: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_options: Option<TextOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_options: Option<ImageOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_instructions: Option<String>,
}

impl Default for GammaGenerationOptions {
    fn default() -> Self {
        Self {
            text_mode: Some("generate".to_string()),
            format: Some("presentation".to_string()),
            num_cards: Some(10),
            text_options: Some(TextOptions {
                amount: Some("medium".to_string()),
                language: Some("en".to_string()),
            }),
            image_options: Some(ImageOptions {
                source: Some("webFreeToUse".to_string()),
            }),
            theme_id: None,
            folder_id: None,
            additional_instructions: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaGenerationRequest {
    pub input_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_cards: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_options: Option<TextOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_options: Option<ImageOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaCreateResponse {
    pub generation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaCredits {
    pub deducted: Option<i32>,
    pub remaining: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits: Option<GammaCredits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaGenerationResult {
    pub generation_id: String,
    pub gamma_url: String,
    pub credits: Option<GammaCredits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaTheme {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaThemesResponse {
    pub data: Vec<GammaTheme>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaFolder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaFoldersResponse {
    pub data: Vec<GammaFolder>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaError {
    pub message: String,
}

// ============================================================================
// Commands
// ============================================================================

/// Create a new Gamma generation (starts async generation)

pub async fn gamma_create_generation(
    api_key: String,
    input_text: String,
    options: Option<GammaGenerationOptions>,
) -> CmdResult<String> {
    if api_key.is_empty() {
        return Err(CommandError::Config("Gamma API key is required".to_string()));
    }
    if input_text.trim().is_empty() {
        return Err(CommandError::Config("Input text is required".to_string()));
    }
    if input_text.len() > 400_000 {
        return Err(CommandError::Config("Input text exceeds maximum length of 400,000 characters".to_string()));
    }

    let defaults = GammaGenerationOptions::default();
    let opts = options.unwrap_or_default();

    // Apply defaults for required API fields when not provided
    let request = GammaGenerationRequest {
        input_text,
        text_mode: opts.text_mode.or(defaults.text_mode),
        format: opts.format.or(defaults.format),
        num_cards: opts.num_cards.or(defaults.num_cards),
        text_options: opts.text_options.or(defaults.text_options),
        image_options: opts.image_options.or(defaults.image_options),
        theme_id: opts.theme_id,
        folder_id: opts.folder_id,
        additional_instructions: opts.additional_instructions,
    };

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post(format!("{}/generations", GAMMA_API_BASE))
        .header("Content-Type", "application/json")
        .header("X-API-KEY", &api_key)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: GammaError = response
            .json()
            .await
            .unwrap_or(GammaError { message: "Unknown error".to_string() });
        return Err(CommandError::Network(format!("Gamma API error: {}", error.message)));
    }

    let result: GammaCreateResponse = response
        .json()
        .await?;

    Ok(result.generation_id)
}

/// Get the status of a Gamma generation

pub async fn gamma_get_status(
    api_key: String,
    generation_id: String,
) -> CmdResult<GammaStatusResponse> {
    if api_key.is_empty() {
        return Err(CommandError::Config("Gamma API key is required".to_string()));
    }

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .get(format!("{}/generations/{}", GAMMA_API_BASE, generation_id))
        .header("X-API-KEY", &api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: GammaError = response
            .json()
            .await
            .unwrap_or(GammaError { message: "Unknown error".to_string() });
        return Err(CommandError::Network(format!("Gamma API error: {}", error.message)));
    }

    let status: GammaStatusResponse = response
        .json()
        .await?;

    Ok(status)
}

/// Generate a Gamma presentation and poll until completion
/// Returns the final result with gamma_url

pub async fn gamma_generate(
    api_key: String,
    input_text: String,
    options: Option<GammaGenerationOptions>,
) -> CmdResult<GammaGenerationResult> {
    // Create generation
    let generation_id = gamma_create_generation(api_key.clone(), input_text, options).await?;

    // Poll for completion (max 10 minutes, 5 second intervals)
    let max_attempts = 120;
    let poll_interval = tokio::time::Duration::from_secs(5);

    for _ in 0..max_attempts {
        tokio::time::sleep(poll_interval).await;

        let status = gamma_get_status(api_key.clone(), generation_id.clone()).await?;

        match status.status.as_str() {
            "completed" => {
                return Ok(GammaGenerationResult {
                    generation_id,
                    gamma_url: status.gamma_url.unwrap_or_default(),
                    credits: status.credits,
                });
            }
            "failed" | "error" => {
                return Err(CommandError::Internal(format!(
                    "Generation failed: {}",
                    status.message.unwrap_or_else(|| "Unknown error".to_string())
                )));
            }
            _ => continue,
        }
    }

    Err(CommandError::Internal("Generation timed out after 10 minutes".to_string()))
}

/// List available Gamma themes

pub async fn gamma_list_themes(
    api_key: String,
    query: Option<String>,
    limit: Option<i32>,
    after: Option<String>,
) -> CmdResult<GammaThemesResponse> {
    if api_key.is_empty() {
        return Err(CommandError::Config("Gamma API key is required".to_string()));
    }

    let mut url = format!("{}/themes", GAMMA_API_BASE);
    let mut params = vec![];

    if let Some(q) = query {
        params.push(format!("query={}", urlencoding::encode(&q)));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(a) = after {
        params.push(format!("after={}", urlencoding::encode(&a)));
    }

    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .get(&url)
        .header("X-API-KEY", &api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: GammaError = response
            .json()
            .await
            .unwrap_or(GammaError { message: "Unknown error".to_string() });
        return Err(CommandError::Network(format!("Gamma API error: {}", error.message)));
    }

    let themes: GammaThemesResponse = response
        .json()
        .await?;

    Ok(themes)
}

/// List all Gamma themes (handles pagination)

pub async fn gamma_list_all_themes(
    api_key: String,
    query: Option<String>,
) -> CmdResult<Vec<GammaTheme>> {
    let mut all_themes = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let result = gamma_list_themes(api_key.clone(), query.clone(), Some(50), cursor).await?;
        all_themes.extend(result.data);

        if result.has_more {
            cursor = result.next_cursor;
        } else {
            break;
        }
    }

    Ok(all_themes)
}

/// List Gamma folders

pub async fn gamma_list_folders(
    api_key: String,
    query: Option<String>,
    limit: Option<i32>,
    after: Option<String>,
) -> CmdResult<GammaFoldersResponse> {
    if api_key.is_empty() {
        return Err(CommandError::Config("Gamma API key is required".to_string()));
    }

    let mut url = format!("{}/folders", GAMMA_API_BASE);
    let mut params = vec![];

    if let Some(q) = query {
        params.push(format!("query={}", urlencoding::encode(&q)));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(a) = after {
        params.push(format!("after={}", urlencoding::encode(&a)));
    }

    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .get(&url)
        .header("X-API-KEY", &api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: GammaError = response
            .json()
            .await
            .unwrap_or(GammaError { message: "Unknown error".to_string() });
        return Err(CommandError::Network(format!("Gamma API error: {}", error.message)));
    }

    let folders: GammaFoldersResponse = response
        .json()
        .await?;

    Ok(folders)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Prepare markdown for slides by adding card breaks before headings
#[allow(dead_code)]
pub fn prepare_markdown_for_slides(
    markdown: &str,
    split_on_h1: bool,
    split_on_h2: bool,
) -> String {
    let mut result = markdown.to_string();

    // Add card breaks before H1 headings
    if split_on_h1 {
        result = result
            .lines()
            .map(|line| {
                if line.starts_with("# ") {
                    format!("\n---\n{}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Add card breaks before H2 headings
    if split_on_h2 {
        let lines: Vec<&str> = result.lines().collect();
        result = lines
            .iter()
            .map(|line| {
                if line.starts_with("## ") {
                    format!("\n---\n{}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Clean up duplicate breaks
    while result.contains("\n---\n\n---\n") {
        result = result.replace("\n---\n\n---\n", "\n---\n");
    }

    // Remove leading break
    if result.starts_with("\n---\n") {
        result = result[5..].to_string();
    }

    result
}
