use super::types::*;
use crate::core::error::{CommandError, CmdResult};
use crate::core::supabase::get_client;
use crate::server::tools::list_tools;
use chrono::Utc;
use serde_json::json;

/// Sweep the in-process tool catalog and upsert each entry into `mcp_tools`.
/// Tools that disappear get marked `status = 'missing'` (not deleted), which
/// preserves any editable metadata in case the tool reappears.
pub async fn sync_mcp_tools() -> CmdResult<SyncMcpToolsResult> {
    let client = get_client().await?;
    let started = Utc::now();
    let started_iso = started.to_rfc3339();

    let tools = list_tools();
    let synced = tools.len();

    for tool in tools {
        let params = json!({
            "p_slug": tool.name,
            "p_name": tool.name,
            "p_description": tool.description,
            "p_category": categorize(&tool.name),
            "p_params_schema": serde_json::to_value(&tool.input_schema)
                .unwrap_or_else(|_| json!({})),
            "p_source_file": source_file_for(&tool.name),
            "p_last_synced_at": started_iso,
        });
        let _: serde_json::Value = client
            .rpc("upsert_mcp_tool_synced", &params)
            .await
            .map_err(|e| CommandError::Internal(format!("upsert {}: {}", tool.name, e)))?;
    }

    let marked_missing: i64 = client
        .rpc(
            "mark_missing_mcp_tools",
            &json!({ "sync_started": started_iso }),
        )
        .await
        .unwrap_or(0);

    Ok(SyncMcpToolsResult {
        synced,
        marked_missing,
        started_at: started_iso,
        finished_at: Utc::now().to_rfc3339(),
    })
}

/// Read all rows from `mcp_tools`, ordered by name.
pub async fn list_mcp_tools() -> CmdResult<Vec<McpToolRow>> {
    let client = get_client().await?;
    client.select("mcp_tools", "order=name.asc").await
}

/// Best-effort prefix categorisation. Mirror of the routing logic in
/// `server/tools/mod.rs::call_tool`. When ambiguous, defaults to `misc`.
fn categorize(name: &str) -> &'static str {
    if name == "diagnostics" {
        "Diagnostics"
    } else if name == "sync-mcp-tools" || name == "list-mcp-tools" {
        "MCP Registry"
    } else if name.ends_with("-docs-page") || name.ends_with("-docs-pages") {
        "Docs Pages"
    } else if name == "generate-order-form"
        || name == "generate-proposal"
        || name == "check-document-type"
    {
        "Document Generation"
    } else if name.starts_with("list-intercom-") || name == "publish-to-intercom" {
        "Intercom"
    } else if name.starts_with("gamma-") || name.starts_with("nanobanana-") {
        "Generation"
    } else if name.starts_with("fy-") {
        "FY Review"
    } else if name.starts_with("qbo-") {
        "QuickBooks Online"
    } else if name == "create-workflow"
        || name == "update-workflow"
        || name == "execute-workflow"
        || name == "list-workflow-plugins"
        || name == "get-workflow-plugin-schema"
        || name.ends_with("-val-space")
        || name.ends_with("-val-zone")
        || name == "create-val-table"
        || name == "clone-val-table"
        || name == "assign-val-table-to-zone"
        || name == "add-val-table-field"
        || name == "add-val-table-fields"
        || name == "update-val-field"
        || name == "create-val-query"
        || name == "update-val-query"
        || name == "copy-val-query"
    {
        "VAL Admin"
    } else if name == "execute-val-sql" {
        "VAL SQL"
    } else if name == "execute-supabase-sql" {
        "Supabase SQL"
    } else if name.starts_with("sync-all-domain-")
        || name == "check-all-domain-drive-files"
        || name == "list-drive-files"
    {
        "VAL Monitoring"
    } else if name.starts_with("sync-val-") {
        "VAL Setup Sync"
    } else if name.ends_with("-whatsapp-summary")
        || name.ends_with("-whatsapp-summaries")
        || name == "get-latest-whatsapp-summary-date"
    {
        "WhatsApp"
    } else if name == "list-notifications" || name == "mark-notification-read" {
        "Notifications"
    } else if name.ends_with("-discussion") || name.ends_with("-discussions") {
        "Discussions"
    } else if name.ends_with("-blog-article") || name.ends_with("-blog-articles") {
        "Blog"
    } else if name == "apollo-search-people" {
        "Apollo"
    } else if name.contains("email") || name == "send-email" || name == "list-linked-emails" {
        "Email"
    } else if name == "list-companies"
        || name == "find-company"
        || name == "get-company"
        || name == "create-company"
        || name == "update-company"
        || name == "delete-company"
        || name == "list-contacts"
        || name == "find-contact"
        || name == "create-contact"
        || name == "update-contact"
        || name == "add-activity"
        || name == "list-activities"
        || name == "update-activity"
        || name == "delete-activity"
    {
        "CRM"
    } else if name.contains("project")
        || name.contains("task")
        || name.contains("milestone")
        || name.contains("initiative")
        || name.contains("label")
        || name.contains("skill")
        || name == "list-users"
        || name == "list-bots"
        || name == "get-pipeline"
    {
        "Work"
    } else {
        "misc"
    }
}

/// Best-effort source file pointer for the operator's reference.
fn source_file_for(name: &str) -> &'static str {
    match categorize(name) {
        "Apollo" => "src/server/tools/apollo.rs",
        "QuickBooks Online" => "src/server/tools/qbo.rs",
        "FY Review" => "src/server/tools/fy_review.rs",
        "Generation" => "src/server/tools/generate.rs",
        "VAL Setup Sync" => "src/server/tools/val_sync.rs",
        "VAL Monitoring" => "src/server/tools/val_sync.rs",
        "VAL SQL" => "src/server/tools/val_sync.rs",
        "Supabase SQL" => "src/server/tools/val_sync.rs",
        "VAL Admin" if matches!(name,
            "create-workflow" | "update-workflow" | "execute-workflow"
            | "list-workflow-plugins" | "get-workflow-plugin-schema"
        ) => "src/server/tools/workflows.rs",
        "VAL Admin" => "src/server/tools/val_admin.rs",
        "CRM" => "src/server/tools/crm.rs",
        "Email" => "src/server/tools/email.rs",
        "Document Generation" => "src/server/tools/docgen.rs",
        "Intercom" => "src/server/tools/intercom.rs",
        "Blog" => "src/server/tools/blog.rs",
        "Discussions" => "src/server/tools/discussions.rs",
        "Notifications" => "src/server/tools/notifications.rs",
        "WhatsApp" => "src/server/tools/whatsapp.rs",
        "Docs Pages" => "src/server/tools/docs.rs",
        "Work" => "src/server/tools/work.rs",
        "Diagnostics" => "src/server/tools/diagnostics.rs",
        "MCP Registry" => "src/server/tools/mcp_tools.rs",
        _ => "unknown",
    }
}
