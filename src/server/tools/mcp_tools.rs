// MCP Tools Registry — discovery + metadata

use crate::modules::mcp_tools as module;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::Value;

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "sync-mcp-tools".to_string(),
            description:
                "Sweep the in-process tv-mcp tool catalog and upsert each entry into the Supabase \
                 `mcp_tools` registry. Tools that disappear are marked status='missing' (not deleted), \
                 preserving any editable metadata. Run this after deploying a new tv-mcp build."
                    .to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "list-mcp-tools".to_string(),
            description: "List all rows in the `mcp_tools` registry, ordered by name.".to_string(),
            input_schema: InputSchema::empty(),
        },
    ]
}

pub async fn call(name: &str, _arguments: Value) -> ToolResult {
    match name {
        "sync-mcp-tools" => match module::sync_mcp_tools().await {
            Ok(result) => ToolResult::json(&result),
            Err(e) => ToolResult::error(format!("sync-mcp-tools failed: {}", e)),
        },
        "list-mcp-tools" => match module::list_mcp_tools().await {
            Ok(rows) => ToolResult::json(&rows),
            Err(e) => ToolResult::error(format!("list-mcp-tools failed: {}", e)),
        },
        _ => ToolResult::error(format!("Unknown mcp_tools tool: {}", name)),
    }
}
