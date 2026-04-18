// Trigger QBO sync by invoking the mgmt-project edge functions.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{get_workspace_setting, KEY_SUPABASE_ANON_KEY, KEY_SUPABASE_URL};
use serde_json::Value;

/// Invoke a mgmt-project edge function. Uses the anon key for Authorization
/// (same pattern as the frontend) — these functions enforce access via the
/// JWT on the call or (for qbo-connect / qbo-callback) run without JWT.
async fn invoke_edge_function(name: &str, body: Value) -> CmdResult<Value> {
    let url = get_workspace_setting(MGMT_WORKSPACE_ID, KEY_SUPABASE_URL).ok_or_else(|| {
        CommandError::Config("mgmt workspace Supabase URL not configured".into())
    })?;
    let anon_key = get_workspace_setting(MGMT_WORKSPACE_ID, KEY_SUPABASE_ANON_KEY).ok_or_else(
        || CommandError::Config("mgmt workspace Supabase anon key not configured".into()),
    )?;

    let client = crate::HTTP_CLIENT.clone();
    let res = client
        .post(format!("{}/functions/v1/{}", url, name))
        .header("Authorization", format!("Bearer {}", anon_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    Ok(res.json().await?)
}

pub async fn qbo_trigger_sync(entity: Option<String>) -> CmdResult<Value> {
    let body = serde_json::json!({
        "entity": entity.unwrap_or_else(|| "all".to_string()),
        "triggered_by": "mcp",
    });
    invoke_edge_function("qbo-sync", body).await
}

pub async fn qbo_trigger_reports_sync() -> CmdResult<Value> {
    invoke_edge_function("qbo-sync-reports", serde_json::json!({})).await
}

/// Fetch a single report for a specific date range (FY, custom period, etc.).
/// Upserts into `qbo_reports_cache` so follow-up `qbo-get-*` calls return it.
pub async fn qbo_fetch_report(
    report_type: &str,
    label: &str,
    start_date: Option<String>,
    end_date: String,
) -> CmdResult<Value> {
    let period = if let Some(start) = start_date {
        serde_json::json!({ "label": label, "start": start, "end": end_date })
    } else {
        serde_json::json!({ "label": label, "end": end_date })
    };
    let body = serde_json::json!({
        "reports": [report_type],
        "periods": [period],
    });
    invoke_edge_function("qbo-sync-reports", body).await
}
