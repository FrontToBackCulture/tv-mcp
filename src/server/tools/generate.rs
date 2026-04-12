// Generation Tools MCP
// Gamma (presentations) and Nanobanana (images) generation tools

use crate::core::settings::{settings_get_key, KEY_GAMMA_API, KEY_GEMINI_API};
use crate::modules::tools::gamma::{self, GammaGenerationOptions, ImageOptions, TextOptions};
use crate::modules::tools::nanobanana::{self, NanobananOptions, ReferenceImage};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define generation tools
pub fn tools() -> Vec<Tool> {
    vec![
        // Gamma (presentations)
        Tool {
            name: "gamma-generate".to_string(),
            description: "Generate a Gamma presentation from text/markdown. Returns URL to the generated presentation.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "input_text": { "type": "string", "description": "Text or markdown content for the presentation (required)" },
                    "text_mode": { "type": "string", "enum": ["generate", "condense", "preserve"], "description": "How to handle input text (default: generate)" },
                    "format": { "type": "string", "enum": ["presentation", "document", "social", "webpage"], "description": "Output format (default: presentation)" },
                    "num_cards": { "type": "integer", "description": "Target number of slides (default: 10)" },
                    "text_amount": { "type": "string", "enum": ["brief", "medium", "detailed", "extensive"], "description": "Amount of text per slide (default: medium)" },
                    "image_source": { "type": "string", "enum": ["aiGenerated", "pictographic", "unsplash", "webFreeToUse", "noImages"], "description": "Image source (default: webFreeToUse)" },
                    "theme_id": { "type": "string", "description": "Theme ID from gamma-list-themes" },
                    "folder_id": { "type": "string", "description": "Folder ID to save presentation" },
                    "additional_instructions": { "type": "string", "description": "Additional instructions for generation" }
                }),
                vec!["input_text".to_string()],
            ),
        },
        Tool {
            name: "gamma-list-themes".to_string(),
            description: "List available Gamma themes. Returns theme IDs that can be used with gamma-generate.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "query": { "type": "string", "description": "Search query for themes" },
                    "limit": { "type": "integer", "description": "Max number of themes to return" }
                }),
                vec![],
            ),
        },
        // Nanobanana (image generation)
        Tool {
            name: "nanobanana-generate".to_string(),
            description: "Generate an image from a text prompt using Gemini's image generation.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "prompt": { "type": "string", "description": "Text description of the image to generate (required)" },
                    "model": { "type": "string", "description": "Model to use (default: gemini-2.5-flash-image)" },
                    "reference_images": {
                        "type": "array",
                        "description": "Reference images (base64 encoded)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "data": { "type": "string", "description": "Base64 encoded image data" },
                                "mime_type": { "type": "string", "description": "MIME type (e.g., image/png)" }
                            }
                        }
                    }
                }),
                vec!["prompt".to_string()],
            ),
        },
        Tool {
            name: "nanobanana-generate-from-file".to_string(),
            description: "Generate an image from a markdown file with nanobanana_prompt in frontmatter.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "file_path": { "type": "string", "description": "Path to the markdown file with nanobanana_prompt (required)" },
                    "output_path": { "type": "string", "description": "Output path for the generated image (optional, defaults to same directory)" }
                }),
                vec!["file_path".to_string()],
            ),
        },
        Tool {
            name: "nanobanana-list-models".to_string(),
            description: "List available image generation models.".to_string(),
            input_schema: InputSchema::empty(),
        },
    ]
}

/// Call a generation tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        // Gamma
        "gamma-generate" => {
            // Get API key from settings
            let api_key = match settings_get_key(KEY_GAMMA_API.to_string()) {
                Ok(Some(key)) => key,
                Ok(None) => return ToolResult::error("Gamma API key not configured. Set it in tv-desktop settings.".to_string()),
                Err(e) => return ToolResult::error(format!("Failed to get API key: {}", e)),
            };

            let input_text = match args.get("input_text").and_then(|v| v.as_str()) {
                Some(text) => text.to_string(),
                None => return ToolResult::error("input_text is required".to_string()),
            };

            // Build options
            let text_options = args.get("text_amount").and_then(|v| v.as_str()).map(|amount| {
                TextOptions {
                    amount: Some(amount.to_string()),
                    language: Some("en".to_string()),
                }
            });

            let image_options = args.get("image_source").and_then(|v| v.as_str()).map(|source| {
                ImageOptions {
                    source: Some(source.to_string()),
                }
            });

            let options = GammaGenerationOptions {
                text_mode: args.get("text_mode").and_then(|v| v.as_str()).map(|s| s.to_string()),
                format: args.get("format").and_then(|v| v.as_str()).map(|s| s.to_string()),
                num_cards: args.get("num_cards").and_then(|v| v.as_i64()).map(|n| n as i32),
                text_options,
                image_options,
                theme_id: args.get("theme_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                folder_id: args.get("folder_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                additional_instructions: args.get("additional_instructions").and_then(|v| v.as_str()).map(|s| s.to_string()),
            };

            match gamma::gamma_generate(api_key, input_text, Some(options)).await {
                Ok(result) => ToolResult::json(&result),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "gamma-list-themes" => {
            let api_key = match settings_get_key(KEY_GAMMA_API.to_string()) {
                Ok(Some(key)) => key,
                Ok(None) => return ToolResult::error("Gamma API key not configured. Set it in tv-desktop settings.".to_string()),
                Err(e) => return ToolResult::error(format!("Failed to get API key: {}", e)),
            };

            let query = args.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);

            match gamma::gamma_list_themes(api_key, query, limit, None).await {
                Ok(result) => ToolResult::json(&result),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Nanobanana
        "nanobanana-generate" => {
            let api_key = match settings_get_key(KEY_GEMINI_API.to_string()) {
                Ok(Some(key)) => key,
                Ok(None) => return ToolResult::error("Gemini API key not configured. Set it in tv-desktop settings.".to_string()),
                Err(e) => return ToolResult::error(format!("Failed to get API key: {}", e)),
            };

            let prompt = match args.get("prompt").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => return ToolResult::error("prompt is required".to_string()),
            };

            let model = args.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

            let reference_images: Option<Vec<ReferenceImage>> = args.get("reference_images")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let data = item.get("data")?.as_str()?.to_string();
                            let mime_type = item.get("mime_type")?.as_str()?.to_string();
                            Some(ReferenceImage { data, mime_type })
                        })
                        .collect()
                });

            let options = NanobananOptions {
                model,
                reference_images,
            };

            match nanobanana::nanobanana_generate(api_key, prompt, Some(options)).await {
                Ok(result) => {
                    // Return just the metadata, not the full image data (too large for MCP)
                    ToolResult::json(&json!({
                        "success": true,
                        "mime_type": result.mime_type,
                        "message": "Image generated successfully. Use nanobanana-generate-to-file to save to disk."
                    }))
                }
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "nanobanana-generate-from-file" => {
            let api_key = match settings_get_key(KEY_GEMINI_API.to_string()) {
                Ok(Some(key)) => key,
                Ok(None) => return ToolResult::error("Gemini API key not configured. Set it in tv-desktop settings.".to_string()),
                Err(e) => return ToolResult::error(format!("Failed to get API key: {}", e)),
            };

            let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => return ToolResult::error("file_path is required".to_string()),
            };

            let output_path = args.get("output_path").and_then(|v| v.as_str()).map(|s| s.to_string());

            match nanobanana::nanobanana_generate_from_file(api_key, file_path, output_path, None).await {
                Ok(output_path) => ToolResult::json(&json!({
                    "success": true,
                    "output_path": output_path,
                    "message": "Image generated and saved successfully"
                })),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "nanobanana-list-models" => {
            let models = nanobanana::nanobanana_list_models();
            ToolResult::json(&models)
        }

        _ => ToolResult::error(format!("Unknown generation tool: {}", name)),
    }
}
