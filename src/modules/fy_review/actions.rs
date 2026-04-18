// FY Review actions — invoke mgmt edge functions for capture + recognition.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{get_workspace_setting, KEY_SUPABASE_ANON_KEY, KEY_SUPABASE_URL};
use serde_json::{json, Value};

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

pub async fn fy_capture_snapshot(
    fy_code: String,
    period_start: Option<String>,
) -> CmdResult<Value> {
    let mut body = json!({ "fy_code": fy_code });
    if let Some(p) = period_start {
        body["period_start"] = Value::String(p);
    }
    invoke_edge_function("fy-capture-snapshot", body).await
}

pub async fn fy_watchdog_run(
    fy_code: Option<String>,
    period_start: Option<String>,
    threshold: Option<f64>,
) -> CmdResult<Value> {
    let mut body = json!({});
    if let Some(fy) = fy_code {
        body["fy_code"] = Value::String(fy);
    }
    if let Some(p) = period_start {
        body["period_start"] = Value::String(p);
    }
    if let Some(t) = threshold {
        body["threshold"] = Value::from(t);
    }
    invoke_edge_function("fy-watchdog", body).await
}

pub async fn fy_build_recognition(
    fy_code: Option<String>,
    orderform_code: Option<String>,
) -> CmdResult<Value> {
    let mut body = json!({});
    if let Some(fy) = fy_code {
        body["fy_code"] = Value::String(fy);
    }
    if let Some(of) = orderform_code {
        body["orderform_code"] = Value::String(of);
    }
    invoke_edge_function("fy-build-recognition", body).await
}
