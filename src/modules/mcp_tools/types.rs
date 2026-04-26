use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Row read from the `mcp_tools` table (all fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolRow {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub params_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_seen_at: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default)]
    pub examples: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// Upsert payload — only the synced fields. Editable fields are preserved
/// because we omit them from the upsert body.
#[derive(Debug, Clone, Serialize)]
pub struct McpToolSyncRow {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub params_schema: Value,
    pub source_file: String,
    pub last_synced_at: String,
}

/// Summary returned from a sync run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMcpToolsResult {
    pub synced: usize,
    pub marked_missing: i64,
    pub started_at: String,
    pub finished_at: String,
}
