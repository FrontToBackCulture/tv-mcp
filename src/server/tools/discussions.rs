// Discussions MCP Tools
// Universal comment system — bots can leave notes on files, deals, tasks, etc.

use crate::modules::discussions;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define discussion tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-discussions".to_string(),
            description: "List comments/discussions on any entity (file, company, task, project, campaign).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "entity_type": {
                        "type": "string",
                        "enum": ["file", "crm_company", "task", "project", "campaign", "domain", "domain_artifact"],
                        "description": "The type of entity"
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "The entity identifier (UUID for DB entities, relative path for files)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default: 100)"
                    }
                }),
                vec!["entity_type".to_string(), "entity_id".to_string()],
            ),
        },
        Tool {
            name: "add-discussion".to_string(),
            description: "Post a comment on any entity. Bots use this to leave notes on files, deals, tasks, etc.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "entity_type": {
                        "type": "string",
                        "enum": ["file", "crm_company", "task", "project", "campaign", "domain", "domain_artifact"],
                        "description": "The type of entity"
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "The entity identifier (UUID for DB entities, relative path for files)"
                    },
                    "author": {
                        "type": "string",
                        "description": "Who is posting (e.g., 'melvin', 'darren', 'bot-mel')"
                    },
                    "body": {
                        "type": "string",
                        "description": "The comment body (supports markdown)"
                    },
                    "parent_id": {
                        "type": "string",
                        "description": "UUID of parent comment for threaded replies (optional)"
                    }
                }),
                vec!["entity_type".to_string(), "entity_id".to_string(), "author".to_string(), "body".to_string()],
            ),
        },
        Tool {
            name: "update-discussion".to_string(),
            description: "Edit an existing comment.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": {
                        "type": "string",
                        "description": "The discussion UUID"
                    },
                    "body": {
                        "type": "string",
                        "description": "The updated comment body"
                    }
                }),
                vec!["id".to_string(), "body".to_string()],
            ),
        },
        Tool {
            name: "delete-discussion".to_string(),
            description: "Remove a comment.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": {
                        "type": "string",
                        "description": "The discussion UUID"
                    }
                }),
                vec!["id".to_string()],
            ),
        },
    ]
}

/// Dispatch a discussion tool call
pub async fn call(name: &str, arguments: Value) -> ToolResult {
    match name {
        "list-discussions" => {
            let entity_type = arguments["entity_type"].as_str().unwrap_or("").to_string();
            let entity_id = arguments["entity_id"].as_str().unwrap_or("").to_string();
            let limit = arguments["limit"].as_u64().map(|v| v as u32);

            match discussions::discussions_list(entity_type, entity_id, limit).await {
                Ok(items) => ToolResult::json(&items),
                Err(e) => ToolResult::error(format!("Failed to list discussions: {}", e)),
            }
        }
        "add-discussion" => {
            let entity_type = arguments["entity_type"].as_str().unwrap_or("").to_string();
            let entity_id = arguments["entity_id"].as_str().unwrap_or("").to_string();
            let author = arguments["author"].as_str().unwrap_or("").to_string();
            let body = arguments["body"].as_str().unwrap_or("").to_string();
            let parent_id = arguments["parent_id"].as_str().map(|s| s.to_string());

            match discussions::discussions_create(entity_type, entity_id, author, body, parent_id).await {
                Ok(item) => ToolResult::json(&item),
                Err(e) => ToolResult::error(format!("Failed to create discussion: {}", e)),
            }
        }
        "update-discussion" => {
            let id = arguments["id"].as_str().unwrap_or("").to_string();
            let body = arguments["body"].as_str().unwrap_or("").to_string();

            match discussions::discussions_update(id, body).await {
                Ok(item) => ToolResult::json(&item),
                Err(e) => ToolResult::error(format!("Failed to update discussion: {}", e)),
            }
        }
        "delete-discussion" => {
            let id = arguments["id"].as_str().unwrap_or("").to_string();

            match discussions::discussions_delete(id).await {
                Ok(()) => ToolResult::text("Discussion deleted".to_string()),
                Err(e) => ToolResult::error(format!("Failed to delete discussion: {}", e)),
            }
        }
        _ => ToolResult::error(format!("Unknown discussion tool: {}", name)),
    }
}
