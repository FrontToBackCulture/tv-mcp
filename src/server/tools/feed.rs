// Feed Module MCP Tools
// Home feed card management

use crate::modules::feed::{self, CreateFeedCard, UpdateFeedCard};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define Feed module tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "create-feed-card".to_string(),
            description: "Create a feed card for the Home feed. Call this during session end to surface changelog entries, new skills, releases, etc.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "card_type": { "type": "string", "enum": ["feature", "tip", "team", "skill", "platform", "release", "module", "app_tip"], "description": "Card type (required)" },
                    "category": { "type": "string", "enum": ["event", "knowledge"], "description": "Event = something happened, Knowledge = something exists (required)" },
                    "badge": { "type": "string", "description": "Display label e.g. 'What\\'s New', 'Skill Spotlight' (required)" },
                    "title": { "type": "string", "description": "Card title (required)" },
                    "body": { "type": "string", "description": "Card body text (required)" },
                    "source": { "type": "string", "description": "Source label e.g. 'Platform Updates', '22 Verified Skills' (required)" },
                    "source_detail": { "type": "string", "description": "Sub-source e.g. 'Connectivity Layer', 'Analytics'" },
                    "triggers": { "type": "array", "items": { "type": "string" }, "description": "Skill trigger phrases (for skill cards)" },
                    "chips": { "type": "array", "items": { "type": "string" }, "description": "Connector methods: 'API', 'RPA', 'Report Reader'" },
                    "stats": { "type": "array", "items": { "type": "object", "properties": { "label": { "type": "string" }, "value": { "type": "string" } } }, "description": "Stats row [{label, value}]" },
                    "features": { "type": "array", "items": { "type": "string" }, "description": "Feature bullet list (for module cards)" },
                    "author": { "type": "object", "properties": { "initials": { "type": "string" }, "name": { "type": "string" }, "role": { "type": "string" } }, "description": "Author info (for team cards)" },
                    "cta_label": { "type": "string", "description": "CTA button label e.g. 'View docs'" },
                    "cta_action": { "type": "string", "description": "Deep link route for CTA" },
                    "scheduled_date": { "type": "string", "description": "Schedule for future display (YYYY-MM-DD). Null = show immediately" },
                    "pinned": { "type": "boolean", "description": "Pin to top of feed" },
                    "created_by": { "type": "string", "description": "Bot name or user who created this" },
                    "source_ref": { "type": "string", "description": "Dedup key (changelog path, skill name, etc.)" },
                    "series_id": { "type": "string", "description": "Groups related cards into a series. Showcase card has series_order=0, detail cards 1,2,3..." },
                    "series_order": { "type": "integer", "description": "Order within a series. 0 = showcase (appears in main feed), 1+ = detail cards" }
                }),
                vec![
                    "card_type".to_string(),
                    "category".to_string(),
                    "badge".to_string(),
                    "title".to_string(),
                    "body".to_string(),
                    "source".to_string(),
                ],
            ),
        },
        Tool {
            name: "list-feed-cards".to_string(),
            description: "List feed cards. Use to check what exists before creating (dedup via source_ref).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "card_type": { "type": "string", "enum": ["feature", "tip", "team", "skill", "platform", "release", "module", "app_tip"] },
                    "category": { "type": "string", "enum": ["event", "knowledge"] },
                    "source_ref": { "type": "string", "description": "Filter by source_ref for dedup check" },
                    "include_archived": { "type": "boolean", "description": "Include archived cards (default false)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "update-feed-card".to_string(),
            description: "Update a feed card's content or archive it.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "card_id": { "type": "string", "description": "The card UUID (required)" },
                    "title": { "type": "string" },
                    "body": { "type": "string" },
                    "badge": { "type": "string" },
                    "pinned": { "type": "boolean" },
                    "archived": { "type": "boolean" },
                    "scheduled_date": { "type": "string" },
                    "cta_label": { "type": "string" },
                    "cta_action": { "type": "string" }
                }),
                vec!["card_id".to_string()],
            ),
        },
        Tool {
            name: "delete-feed-card".to_string(),
            description: "Soft-delete a feed card (sets archived=true).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "card_id": { "type": "string", "description": "The card UUID (required)" }
                }),
                vec!["card_id".to_string()],
            ),
        },
    ]
}

/// Call a Feed module tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "create-feed-card" => {
            let data: CreateFeedCard = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match feed::feed_create_card(data).await {
                Ok(card) => ToolResult::json(&card),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "list-feed-cards" => {
            let card_type = args.get("card_type").and_then(|v| v.as_str()).map(|s| s.to_string());
            let category = args.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());
            let source_ref = args.get("source_ref").and_then(|v| v.as_str()).map(|s| s.to_string());
            let include_archived = args.get("include_archived").and_then(|v| v.as_bool());
            match feed::feed_list_cards(card_type, category, source_ref, include_archived).await {
                Ok(cards) => ToolResult::json(&cards),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-feed-card" => {
            let card_id = match args.get("card_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("card_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("card_id");
            }
            let data: UpdateFeedCard = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match feed::feed_update_card(card_id, data).await {
                Ok(card) => ToolResult::json(&card),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-feed-card" => {
            let card_id = match args.get("card_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("card_id is required".to_string()),
            };
            match feed::feed_delete_card(card_id).await {
                Ok(card) => ToolResult::json(&card),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        _ => ToolResult::error(format!("Unknown feed tool: {}", name)),
    }
}
