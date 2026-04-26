// Docs Module MCP Tools
// docs_pages management for the gated /docs portal on tv-website

use crate::modules::docs::{self, UpsertDocsPage};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define Docs module tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-docs-pages".to_string(),
            description:
                "List pages in the gated /docs portal on tv-website. Returns metadata only (no body) so listings stay light — use `get-docs-page` for the full markdown body. Optionally filter by section."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "section": {
                        "type": "string",
                        "description": "Section slug to filter by (e.g. 'bots', 'tv-client', 'tv-knowledge', 'ba-guide', 'skills'). Omit for all sections."
                    },
                    "visible_only": {
                        "type": "boolean",
                        "description": "If true, return only pages where visible=true. Default false (include hidden)."
                    }
                }),
                vec![],
            ),
        },
        Tool {
            name: "get-docs-page".to_string(),
            description:
                "Fetch a single docs page's full markdown body. Provide either `id`, or both `section` and `slug`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string", "description": "Page UUID" },
                    "section": { "type": "string", "description": "Section slug (e.g. 'bots')" },
                    "slug": { "type": "string", "description": "Page slug within the section (e.g. 'fleet-overview')" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "upsert-docs-page".to_string(),
            description:
                "Create or replace a docs page. The unique key is (section, slug) — same section + slug overwrites. Use this for both new pages and edits to existing ones."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "section": {
                        "type": "string",
                        "description": "Section slug (e.g. 'bots', 'tv-client', 'tv-knowledge', 'ba-guide'). Must already exist in docs_sections."
                    },
                    "slug": {
                        "type": "string",
                        "description": "Page slug, unique within the section (e.g. 'fleet-overview')."
                    },
                    "title": { "type": "string", "description": "Page title shown in the hero." },
                    "summary": { "type": "string", "description": "One- or two-sentence subtitle (standfirst). Optional." },
                    "body_md": {
                        "type": "string",
                        "description": "Full markdown body. Rendered with react-markdown + remark-gfm so GFM tables, fenced code, autolinks, and task lists are supported."
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tag pills shown on the page meta strip and used as the kicker on cards."
                    },
                    "sort_order": {
                        "type": "integer",
                        "description": "Lower values appear first in section listings. Default 10."
                    },
                    "visible": {
                        "type": "boolean",
                        "description": "If false, the page is hidden from the portal. Default true."
                    }
                }),
                vec!["section".to_string(), "slug".to_string(), "title".to_string()],
            ),
        },
        Tool {
            name: "delete-docs-page".to_string(),
            description:
                "Delete a docs page permanently. Provide either `id`, or both `section` and `slug`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string", "description": "Page UUID" },
                    "section": { "type": "string", "description": "Section slug" },
                    "slug": { "type": "string", "description": "Page slug within the section" }
                }),
                vec![],
            ),
        },
    ]
}

/// Call a Docs module tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "list-docs-pages" => {
            let section = args.get("section").and_then(|v| v.as_str()).map(|s| s.to_string());
            let visible_only = args.get("visible_only").and_then(|v| v.as_bool());
            match docs::list_docs_pages(section, visible_only).await {
                Ok(pages) => ToolResult::json(&pages),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "get-docs-page" => {
            let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let section = args.get("section").and_then(|v| v.as_str()).map(|s| s.to_string());
            let slug = args.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string());
            match docs::get_docs_page(id, section, slug).await {
                Ok(page) => ToolResult::json(&page),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "upsert-docs-page" => {
            let data: UpsertDocsPage = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match docs::upsert_docs_page(data).await {
                Ok(page) => ToolResult::json(&page),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-docs-page" => {
            let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let section = args.get("section").and_then(|v| v.as_str()).map(|s| s.to_string());
            let slug = args.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string());
            match docs::delete_docs_page(id, section, slug).await {
                Ok(()) => ToolResult::json(&serde_json::json!({"deleted": true})),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        _ => ToolResult::error(format!("Unknown docs tool: {}", name)),
    }
}
