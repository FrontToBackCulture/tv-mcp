// Intercom API Commands
// Publish, update, and delete articles in Intercom Help Center

use crate::core::error::{CmdResult, CommandError};
use pulldown_cmark::{html, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

const INTERCOM_BASE_URL: &str = "https://api.intercom.io";
const INTERCOM_API_VERSION: &str = "2.11";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercomCollection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercomArticle {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CollectionsResponse {
    data: Vec<CollectionRaw>,
}

#[derive(Debug, Deserialize)]
struct CollectionRaw {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

// ============================================================================
// Markdown helpers
// ============================================================================

/// Parse markdown content, extract title from frontmatter or first heading, convert body to HTML
fn parse_markdown(markdown: &str, fallback_title: &str) -> (String, Option<String>, String) {
    let frontmatter_re = Regex::new(r"(?s)^---\n(.*?)\n---\n?(.*)").unwrap();

    let (fm_title, fm_summary, body) = if let Some(caps) = frontmatter_re.captures(markdown) {
        let yaml = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let title = extract_yaml_value(yaml, "title");
        let summary = extract_yaml_value(yaml, "summary")
            .or_else(|| extract_yaml_value(yaml, "description"));
        (title, summary, content.to_string())
    } else {
        (None, None, markdown.to_string())
    };

    // Title from frontmatter or first heading
    let title = fm_title.unwrap_or_else(|| {
        let heading_re = Regex::new(r"^#\s+(.+)$").unwrap();
        for line in body.lines() {
            if let Some(caps) = heading_re.captures(line) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
        fallback_title.to_string()
    });

    // Strip first heading if it matches title
    let body_clean = {
        let re = Regex::new(&format!(r"(?m)^#\s+{}\n+", regex::escape(&title))).unwrap();
        re.replace(&body, "").to_string()
    };

    let parser = Parser::new(&body_clean);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    (title, fm_summary, html_output)
}

fn extract_yaml_value(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{}:", key)) {
            let value = line[key.len() + 1..].trim();
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

// ============================================================================
// Tauri commands
// ============================================================================

/// List all Intercom Help Center collections

pub async fn intercom_list_collections(
    api_key: String,
) -> CmdResult<Vec<IntercomCollection>> {
    let client = crate::HTTP_CLIENT.clone();

    let response = client
        .get(format!("{}/help_center/collections", INTERCOM_BASE_URL))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Intercom-Version", INTERCOM_API_VERSION)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status: status.as_u16(), body: text });
    }

    let data: CollectionsResponse = response
        .json()
        .await?;

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

/// Publish a new article to Intercom Help Center (markdown -> HTML conversion done here)

pub async fn intercom_publish_article(
    api_key: String,
    markdown: String,
    filename: String,
    collection_id: String,
    state: Option<String>,
) -> CmdResult<IntercomArticle> {
    let fallback_title = filename.trim_end_matches(".md");
    let (title, description, body_html) = parse_markdown(&markdown, fallback_title);
    let state = state.unwrap_or_else(|| "published".to_string());

    let client = crate::HTTP_CLIENT.clone();

    let mut request_body = json!({
        "title": title,
        "body": body_html,
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
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status: status.as_u16(), body: text });
    }

    Ok(response.json().await?)
}

/// Update an existing Intercom article

pub async fn intercom_update_article(
    api_key: String,
    article_id: String,
    markdown: String,
    filename: String,
    collection_id: Option<String>,
    state: Option<String>,
) -> CmdResult<IntercomArticle> {
    let fallback_title = filename.trim_end_matches(".md");
    let (title, description, body_html) = parse_markdown(&markdown, fallback_title);

    let client = crate::HTTP_CLIENT.clone();

    let mut request_body = json!({
        "title": title,
        "body": body_html,
    });

    if let Some(cid) = collection_id {
        request_body["parent_id"] = json!(cid);
        request_body["parent_type"] = json!("collection");
    }

    if let Some(desc) = description {
        request_body["description"] = json!(desc);
    }

    if let Some(s) = state {
        request_body["state"] = json!(s);
    }

    let response = client
        .put(format!("{}/articles/{}", INTERCOM_BASE_URL, article_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Intercom-Version", INTERCOM_API_VERSION)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status: status.as_u16(), body: text });
    }

    Ok(response.json().await?)
}

/// Delete an Intercom article

pub async fn intercom_delete_article(
    api_key: String,
    article_id: String,
) -> CmdResult<()> {
    let client = crate::HTTP_CLIENT.clone();

    let response = client
        .delete(format!("{}/articles/{}", INTERCOM_BASE_URL, article_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Intercom-Version", INTERCOM_API_VERSION)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status: status.as_u16(), body: text });
    }

    Ok(())
}