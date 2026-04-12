// WhatsApp Summary MCP Tools
// List, upsert, and manage WhatsApp chat summaries linked to client initiatives

use crate::modules::whatsapp;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define WhatsApp summary tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-whatsapp-summaries".to_string(),
            description: "List WhatsApp chat summaries for a client initiative. Returns summaries in reverse chronological order. Use after_date to get only new summaries (for delta detection).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": {
                        "type": "string",
                        "description": "The initiative UUID (e.g., Client - KOI initiative ID)"
                    },
                    "after_date": {
                        "type": "string",
                        "description": "Only return summaries after this date (YYYY-MM-DD). Use for delta processing."
                    },
                    "before_date": {
                        "type": "string",
                        "description": "Only return summaries before this date (YYYY-MM-DD)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default: 100)"
                    }
                }),
                vec!["initiative_id".to_string()],
            ),
        },
        Tool {
            name: "whatsapp-latest-date".to_string(),
            description: "Get the most recent summary date for an initiative. Use this to determine what's already been processed before summarizing new messages.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": {
                        "type": "string",
                        "description": "The initiative UUID"
                    }
                }),
                vec!["initiative_id".to_string()],
            ),
        },
        Tool {
            name: "upsert-whatsapp-summary".to_string(),
            description: "Insert or update a daily WhatsApp summary for a client initiative. If a summary already exists for that initiative+date, it will be updated.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": {
                        "type": "string",
                        "description": "The initiative UUID"
                    },
                    "client_folder": {
                        "type": "string",
                        "description": "Relative path to client folder (e.g., '3_Clients/koi')"
                    },
                    "date": {
                        "type": "string",
                        "description": "The date being summarized (YYYY-MM-DD)"
                    },
                    "summary": {
                        "type": "string",
                        "description": "AI-generated summary of the day's WhatsApp messages"
                    },
                    "key_topics": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of key topics discussed (e.g., ['bank recon setup', 'Grab account'])"
                    },
                    "action_items": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Action items identified from the conversation"
                    },
                    "participants": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Names/numbers of people who sent messages that day"
                    },
                    "message_count": {
                        "type": "integer",
                        "description": "Number of messages that day"
                    },
                    "media_notes": {
                        "type": "string",
                        "description": "Description of any images/documents shared that day"
                    },
                    "source_file": {
                        "type": "string",
                        "description": "Filename of the WhatsApp export (e.g., 'WhatsApp Chat with Koi ThinkVAL - 20260326.txt')"
                    }
                }),
                vec!["initiative_id".to_string(), "client_folder".to_string(), "date".to_string(), "summary".to_string()],
            ),
        },
        Tool {
            name: "delete-whatsapp-summary".to_string(),
            description: "Delete a WhatsApp summary by ID.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": {
                        "type": "string",
                        "description": "The summary UUID"
                    }
                }),
                vec!["id".to_string()],
            ),
        },
    ]
}

/// Dispatch a WhatsApp tool call
pub async fn call(name: &str, arguments: Value) -> ToolResult {
    match name {
        "list-whatsapp-summaries" => {
            let initiative_id = arguments["initiative_id"].as_str().unwrap_or("").to_string();
            let after_date = arguments["after_date"].as_str().map(|s| s.to_string());
            let before_date = arguments["before_date"].as_str().map(|s| s.to_string());
            let limit = arguments["limit"].as_u64().map(|v| v as u32);

            match whatsapp::whatsapp_list_summaries(initiative_id, after_date, before_date, limit).await {
                Ok(summaries) => ToolResult::json(&summaries),
                Err(e) => ToolResult::error(format!("Failed to list WhatsApp summaries: {}", e)),
            }
        }
        "whatsapp-latest-date" => {
            let initiative_id = arguments["initiative_id"].as_str().unwrap_or("").to_string();

            match whatsapp::whatsapp_latest_date(initiative_id).await {
                Ok(Some(date)) => ToolResult::text(format!("Latest summary date: {}", date)),
                Ok(None) => ToolResult::text("No summaries found — this is a fresh load.".to_string()),
                Err(e) => ToolResult::error(format!("Failed to get latest date: {}", e)),
            }
        }
        "upsert-whatsapp-summary" => {
            let data = whatsapp::UpsertWhatsappSummary {
                initiative_id: arguments["initiative_id"].as_str().unwrap_or("").to_string(),
                client_folder: arguments["client_folder"].as_str().unwrap_or("").to_string(),
                date: arguments["date"].as_str().unwrap_or("").to_string(),
                summary: arguments["summary"].as_str().unwrap_or("").to_string(),
                key_topics: arguments.get("key_topics").cloned(),
                action_items: arguments.get("action_items").cloned(),
                participants: arguments.get("participants").cloned(),
                message_count: arguments["message_count"].as_i64().map(|v| v as i32),
                media_notes: arguments["media_notes"].as_str().map(|s| s.to_string()),
                source_file: arguments["source_file"].as_str().map(|s| s.to_string()),
            };

            match whatsapp::whatsapp_upsert_summary(data).await {
                Ok(summary) => ToolResult::json(&summary),
                Err(e) => ToolResult::error(format!("Failed to upsert WhatsApp summary: {}", e)),
            }
        }
        "delete-whatsapp-summary" => {
            let id = arguments["id"].as_str().unwrap_or("").to_string();

            match whatsapp::whatsapp_delete_summary(id).await {
                Ok(()) => ToolResult::text("WhatsApp summary deleted".to_string()),
                Err(e) => ToolResult::error(format!("Failed to delete WhatsApp summary: {}", e)),
            }
        }
        _ => ToolResult::error(format!("Unknown WhatsApp tool: {}", name)),
    }
}
