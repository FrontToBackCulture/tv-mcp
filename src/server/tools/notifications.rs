// Notifications MCP Tools
// Bots can check their mentions and mark them as read

use crate::modules::notifications;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-notifications".to_string(),
            description: "List notifications (mentions) for a user. Returns newest first.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "recipient": {
                        "type": "string",
                        "description": "Username to check notifications for (e.g., 'bot-mel')"
                    },
                    "unread_only": {
                        "type": "boolean",
                        "description": "Only return unread notifications (default: false)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (default: 50)"
                    }
                }),
                vec!["recipient".to_string()],
            ),
        },
        Tool {
            name: "mark-notification-read".to_string(),
            description: "Mark a notification as read.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": {
                        "type": "string",
                        "description": "Notification UUID to mark as read"
                    }
                }),
                vec!["id".to_string()],
            ),
        },
    ]
}

pub async fn call(name: &str, arguments: Value) -> ToolResult {
    match name {
        "list-notifications" => {
            let recipient = arguments["recipient"].as_str().unwrap_or("").to_string();
            let unread_only = arguments["unread_only"].as_bool();
            let limit = arguments["limit"].as_u64().map(|v| v as u32);

            match notifications::notifications_list(recipient, unread_only, limit).await {
                Ok(items) => ToolResult::json(&items),
                Err(e) => ToolResult::error(format!("Failed to list notifications: {}", e)),
            }
        }
        "mark-notification-read" => {
            let id = arguments["id"].as_str().unwrap_or("").to_string();

            match notifications::notifications_mark_read(id).await {
                Ok(()) => ToolResult::text("Notification marked as read".to_string()),
                Err(e) => ToolResult::error(format!("Failed to mark notification: {}", e)),
            }
        }
        _ => ToolResult::error(format!("Unknown notification tool: {}", name)),
    }
}
