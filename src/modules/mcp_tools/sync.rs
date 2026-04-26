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
    if name.starts_with("apollo-") {
        "sales"
    } else if name.starts_with("qbo-") || name.starts_with("fy-") {
        "finance"
    } else if name.starts_with("gamma-") || name.starts_with("nanobanana-") {
        "generation"
    } else if name.starts_with("sync-val-")
        || name.starts_with("sync-all-domain-")
        || name == "execute-val-sql"
        || name == "execute-supabase-sql"
        || name == "list-drive-files"
        || name == "check-all-domain-drive-files"
    {
        "val_sync"
    } else if name == "list-companies" || name == "find-company" || name == "get-company"
        || name == "create-company" || name == "update-company" || name == "delete-company"
        || name == "list-contacts" || name == "find-contact" || name == "create-contact"
        || name == "update-contact"
        || name == "add-activity"
        || name == "list-activities"
        || name == "update-activity"
        || name == "delete-activity"
    {
        "crm"
    } else if name.contains("email") || name == "send-email"
        || name == "list-linked-emails"
    {
        "email"
    } else if name.starts_with("generate-order-form")
        || name.starts_with("generate-proposal")
        || name == "check-document-type"
    {
        "docgen"
    } else if name.starts_with("list-intercom-") || name == "publish-to-intercom" {
        "content"
    } else if name.ends_with("-blog-article") || name.ends_with("-blog-articles") {
        "content"
    } else if name.ends_with("-discussion") || name.ends_with("-discussions") {
        "communication"
    } else if name.ends_with("-notification")
        || name.ends_with("-notifications")
        || name.starts_with("mark-notification-")
    {
        "communication"
    } else if name.ends_with("-whatsapp-summary")
        || name.ends_with("-whatsapp-summaries")
        || name == "get-latest-whatsapp-summary-date"
    {
        "communication"
    } else if name.contains("triage") {
        "communication"
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
        "work"
    } else if name == "diagnostics" {
        "system"
    } else if name == "sync-mcp-tools" || name == "list-mcp-tools" {
        "registry"
    } else {
        "misc"
    }
}

/// Best-effort source file pointer for the operator's reference.
fn source_file_for(name: &str) -> &'static str {
    match categorize(name) {
        "sales" => "src/server/tools/apollo.rs",
        "finance" if name.starts_with("qbo-") => "src/server/tools/qbo.rs",
        "finance" => "src/server/tools/fy_review.rs",
        "generation" => "src/server/tools/generate.rs",
        "val_sync" => "src/server/tools/val_sync.rs",
        "crm" => "src/server/tools/crm.rs",
        "email" => "src/server/tools/email.rs",
        "docgen" => "src/server/tools/docgen.rs",
        "content" if name.contains("intercom") => "src/server/tools/intercom.rs",
        "content" => "src/server/tools/blog.rs",
        "communication" if name.contains("discussion") => "src/server/tools/discussions.rs",
        "communication" if name.contains("notification") => "src/server/tools/notifications.rs",
        "communication" if name.contains("whatsapp") => "src/server/tools/whatsapp.rs",
        "communication" if name.contains("triage") => "src/server/tools/work.rs",
        "work" => "src/server/tools/work.rs",
        "system" => "src/server/tools/diagnostics.rs",
        "registry" => "src/server/tools/mcp_tools.rs",
        _ => "unknown",
    }
}
