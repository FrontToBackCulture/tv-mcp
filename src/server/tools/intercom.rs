// Intercom Tools MCP
// Publish articles to Intercom Help Center

use crate::core::settings::{settings_get_key, KEY_INTERCOM_API};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use pulldown_cmark::{html, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const INTERCOM_BASE_URL: &str = "https://api.intercom.io";
const INTERCOM_API_VERSION: &str = "2.11";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct IntercomCollection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionsResponse {
    data: Vec<IntercomCollectionRaw>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntercomCollectionRaw {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArticleResponse {
    id: String,
    title: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

// ============================================================================
// API Functions
// ============================================================================

/// List all Help Center collections
async fn list_collections(api_key: &str) -> Result<Vec<IntercomCollection>, String> {
    let client = crate::HTTP_CLIENT.clone();

    let response = client
        .get(format!("{}/help_center/collections", INTERCOM_BASE_URL))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Intercom-Version", INTERCOM_API_VERSION)
        .send()
        .await
        .map_err(|e| format!("Failed to call Intercom API: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Intercom API error {}: {}", status, text));
    }

    let data: CollectionsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data
        .data
        .into_iter()
        .map(|c| IntercomCollection {
            id: c.id,
            name: c.name,
            description: c.description,
            url: c.url,
        })
        .collect())
}

/// Publish an article to Intercom
async fn publish_article(
    api_key: &str,
    title: &str,
    body: &str,
    collection_id: &str,
    description: Option<&str>,
    state: &str,
) -> Result<ArticleResponse, String> {
    let client = crate::HTTP_CLIENT.clone();

    let mut request_body = json!({
        "title": title,
        "body": body,
        "parent_id": collection_id,
        "parent_type": "collection",
        "state": state
    });

    if let Some(desc) = description {
        request_body["description"] = json!(desc);
    }

    let response = client
        .post(format!("{}/articles", INTERCOM_BASE_URL))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Intercom-Version", INTERCOM_API_VERSION)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to call Intercom API: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Intercom API error {}: {}", status, text));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

// ============================================================================
// Markdown Parsing
// ============================================================================

/// Parse markdown content, extract title, convert to HTML
fn parse_markdown(markdown: &str, fallback_title: &str) -> (String, Option<String>, String) {
    // Try to extract YAML frontmatter
    let frontmatter_re = Regex::new(r"(?s)^---\n(.*?)\n---\n?(.*)").unwrap();

    let (frontmatter_title, frontmatter_summary, content) =
        if let Some(caps) = frontmatter_re.captures(markdown) {
            let yaml = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            // Simple YAML parsing for title and summary
            let title = extract_yaml_value(yaml, "title");
            let summary = extract_yaml_value(yaml, "summary")
                .or_else(|| extract_yaml_value(yaml, "description"));

            (title, summary, content.to_string())
        } else {
            (None, None, markdown.to_string())
        };

    // Extract title from first heading if not in frontmatter
    let title = frontmatter_title.unwrap_or_else(|| {
        let heading_re = Regex::new(r"^#\s+(.+)$").unwrap();
        if let Some(line) = content.lines().find(|l| heading_re.is_match(l)) {
            if let Some(caps) = heading_re.captures(line) {
                return caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            }
        }
        fallback_title.to_string()
    });

    // Remove first heading if it matches the title (avoid duplication)
    let body_markdown = {
        let heading_re = Regex::new(&format!(r"(?m)^#\s+{}\n+", regex::escape(&title))).unwrap();
        heading_re.replace(&content, "").to_string()
    };

    // Convert markdown to HTML
    let parser = Parser::new(&body_markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    (title, frontmatter_summary, html_output)
}

/// Simple YAML value extraction
fn extract_yaml_value(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{}:", key)) {
            let value = line[key.len() + 1..].trim();
            // Remove quotes
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

// ============================================================================
// Tool Definitions
// ============================================================================

/// Define Intercom tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-intercom-collections".to_string(),
            description: "List available Intercom Help Center collections. Use to find collection IDs for publishing articles.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "publish-to-intercom".to_string(),
            description: "Publish a markdown article to Intercom Help Center. Converts markdown to HTML, extracts title from frontmatter or first heading.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "markdown": {
                        "type": "string",
                        "description": "Markdown content to publish"
                    },
                    "collectionId": {
                        "type": "string",
                        "description": "Target collection ID (use list-intercom-collections to find)"
                    },
                    "filename": {
                        "type": "string",
                        "description": "Filename for title fallback (optional)"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["draft", "published"],
                        "description": "Article state (default: published)"
                    }
                }),
                vec!["markdown".to_string(), "collectionId".to_string()],
            ),
        },
    ]
}

// ============================================================================
// Tool Handlers
// ============================================================================

/// Call an Intercom tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    // Get API key from settings
    let api_key = match settings_get_key(KEY_INTERCOM_API.to_string()) {
        Ok(Some(key)) => key,
        Ok(None) => {
            return ToolResult::error(
                "Intercom API key not configured. Set it in tv-desktop settings or run: \
                 settings_set_key intercom_api_key YOUR_KEY"
                    .to_string(),
            )
        }
        Err(e) => return ToolResult::error(format!("Failed to get API key: {}", e)),
    };

    match name {
        "list-intercom-collections" => {
            match list_collections(&api_key).await {
                Ok(collections) => {
                    if collections.is_empty() {
                        return ToolResult::text(
                            "No collections found. Create a collection in Intercom first.".to_string(),
                        );
                    }

                    let list: Vec<String> = collections
                        .iter()
                        .map(|c| {
                            let desc = c
                                .description
                                .as_ref()
                                .map(|d| format!(" - {}", d))
                                .unwrap_or_default();
                            format!("- **{}** (ID: {}){}", c.name, c.id, desc)
                        })
                        .collect();

                    ToolResult::text(format!("## Intercom Collections\n\n{}", list.join("\n")))
                }
                Err(e) => ToolResult::error(format!("Error listing collections: {}", e)),
            }
        }

        "publish-to-intercom" => {
            let markdown = match args.get("markdown").and_then(|v| v.as_str()) {
                Some(md) => md,
                None => return ToolResult::error("markdown is required".to_string()),
            };

            let collection_id = match args.get("collectionId").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => return ToolResult::error("collectionId is required".to_string()),
            };

            let filename = args
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled.md");

            let state = args
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("published");

            // Parse markdown and convert to HTML
            let fallback_title = filename.trim_end_matches(".md");
            let (title, description, body) = parse_markdown(markdown, fallback_title);

            // Publish to Intercom
            match publish_article(
                &api_key,
                &title,
                &body,
                collection_id,
                description.as_deref(),
                state,
            )
            .await
            {
                Ok(article) => {
                    let url = article.url.unwrap_or_else(|| "N/A".to_string());
                    let article_state = article.state.unwrap_or_else(|| state.to_string());

                    ToolResult::text(format!(
                        "## Published to Intercom\n\n\
                         - **Title:** {}\n\
                         - **URL:** {}\n\
                         - **ID:** {}\n\
                         - **State:** {}",
                        article.title, url, article.id, article_state
                    ))
                }
                Err(e) => ToolResult::error(format!("Error publishing to Intercom: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown Intercom tool: {}", name)),
    }
}
