// VAL Sync Monitoring - Workflow execution history & SOD table status
// Fetches monitoring data from VAL API and writes to {globalPath}/monitoring/

use super::auth;
use super::config::get_domain_config;
use super::metadata;
use super::sync::{count_items, write_json, SyncResult};
use crate::core::error::{CmdResult, CommandError};
use std::time::Instant;

// ============================================================================
// Helpers
// ============================================================================

/// Check if an HTTP status code indicates an auth error
fn is_auth_status(status: u16) -> bool {
    status == 401 || status == 403
}

/// Check if response body contains auth-related error messages
fn is_auth_body(body: &str) -> bool {
    body.contains("token not authentic")
        || body.contains("jwt expired")
        || body.contains("invalid signature")
}

/// Extract just the date portion (YYYY-MM-DD) from a datetime string
fn date_part(datetime: &str) -> &str {
    if datetime.len() >= 10 {
        &datetime[..10]
    } else {
        datetime
    }
}

// ============================================================================
// Commands
// ============================================================================

/// Sync workflow executions for a domain within a time window.
/// Fetches all pages and combines into a single JSON file.

pub async fn val_sync_workflow_executions(
    domain: String,
    from: String,
    to: String,
) -> CmdResult<SyncResult> {
    let start = Instant::now();
    let domain_config = get_domain_config(&domain)?;
    let global_path = &domain_config.global_path;
    let api_domain = domain_config.api_domain();
    let base_url = format!("https://{}.thinkval.io", api_domain);

    let from_date = date_part(&from);
    let to_date = date_part(&to);
    let file_path = format!(
        "{}/monitoring/{}/workflow_executions_{}_to_{}.json",
        global_path, from_date, from_date, to_date
    );

    // Ensure auth
    let (token, _) = auth::ensure_auth(&domain).await?;

    // Fetch with auth retry
    let data = match fetch_workflow_executions(&base_url, &token, &from, &to).await {
        Ok(data) => data,
        Err(e) if e.contains("auth") || e.contains("401") || e.contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            fetch_workflow_executions(&base_url, &new_token, &from, &to)
                .await
                .map_err(|e| CommandError::Network(format!("Workflow executions failed after reauth: {}", e)))?
        }
        Err(e) => return Err(CommandError::Network(format!("Workflow executions failed: {}", e))),
    };

    let count = count_items(&data);
    write_json(&file_path, &data)?;

    let duration_ms = start.elapsed().as_millis() as u64;
    metadata::update_artifact_sync(
        global_path,
        &domain,
        "monitoring:workflow-executions",
        count,
        "ok",
        duration_ms,
    ).await;

    Ok(SyncResult {
        domain,
        artifact_type: "monitoring:workflow-executions".to_string(),
        count,
        file_path,
        duration_ms,
        status: "ok".to_string(),
        message: format!("Synced {} workflow executions", count),
    })
}

/// Fetch all pages of workflow executions
async fn fetch_workflow_executions(
    base_url: &str,
    token: &str,
    from: &str,
    to: &str,
) -> Result<serde_json::Value, String> {
    let client = crate::HTTP_CLIENT.clone();

    let url = format!("{}/api/v1/workflow/executions", base_url);

    // Fetch page 1
    let response = client
        .get(&url)
        .query(&[
            ("uuid", "1"),
            ("token", token),
            ("from", from),
            ("to", to),
            ("page", "1"),
            ("limit", "100"),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status().as_u16();
    if is_auth_status(status) {
        return Err(format!("auth error (HTTP {})", status));
    }
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        if is_auth_body(&body) {
            return Err(format!("auth error: {}", body));
        }
        return Err(format!("HTTP {}: {}", status, body));
    }

    let page1: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Get total pages from pagination
    let total_pages = page1
        .get("pagination")
        .and_then(|p| p.get("totalPages"))
        .and_then(|t| t.as_u64())
        .unwrap_or(1) as u32;

    // Collect data from page 1
    let mut all_data: Vec<serde_json::Value> = page1
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    // Fetch remaining pages
    for page in 2..=total_pages {
        let resp = client
            .get(&url)
            .query(&[
                ("uuid", "1"),
                ("token", token),
                ("from", from),
                ("to", to),
                ("page", &page.to_string()),
                ("limit", "100"),
            ])
            .send()
            .await
            .map_err(|e| format!("Network error on page {}: {}", page, e))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP error on page {}: {}", page, body));
        }

        let page_data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error on page {}: {}", page, e))?;

        if let Some(arr) = page_data.get("data").and_then(|d| d.as_array()) {
            all_data.extend(arr.iter().cloned());
        }
    }

    Ok(serde_json::json!(all_data))
}

/// Sync SOD tables status for a domain on a given date.

pub async fn val_sync_sod_tables_status(
    domain: String,
    date: String,
    regenerate: bool,
) -> CmdResult<SyncResult> {
    let start = Instant::now();
    let domain_config = get_domain_config(&domain)?;
    let global_path = &domain_config.global_path;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);

    let file_path = format!(
        "{}/monitoring/{}/sod_tables_status_{}.json",
        global_path, date, date
    );

    // Ensure auth
    let (token, _) = auth::ensure_auth(&domain).await?;

    // Fetch with auth retry
    let data = match fetch_sod_tables_status(&base_url, &api_domain, &token, &date, regenerate).await {
        Ok(data) => data,
        Err(e) if e.contains("auth") || e.contains("401") || e.contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            fetch_sod_tables_status(&base_url, &api_domain, &new_token, &date, regenerate)
                .await
                .map_err(|e| CommandError::Network(format!("SOD tables status failed after reauth: {}", e)))?
        }
        Err(e) => return Err(CommandError::Network(format!("SOD tables status failed: {}", e))),
    };

    let count = count_items(&data);
    write_json(&file_path, &data)?;

    let duration_ms = start.elapsed().as_millis() as u64;
    metadata::update_artifact_sync(
        global_path,
        &domain,
        "monitoring:sod-tables",
        count,
        "ok",
        duration_ms,
    ).await;

    Ok(SyncResult {
        domain,
        artifact_type: "monitoring:sod-tables".to_string(),
        count,
        file_path,
        duration_ms,
        status: "ok".to_string(),
        message: format!("Synced {} SOD table statuses", count),
    })
}

/// Fetch notifications for a domain from the VAL notification system.
/// Returns error/system notifications stored in Redis.

pub async fn val_fetch_notifications(
    domain: String,
    max: Option<u32>,
) -> CmdResult<serde_json::Value> {
    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain();
    let base_url = format!("https://{}.thinkval.io", api_domain);

    // Ensure auth
    let (token, _) = auth::ensure_auth(&domain).await?;

    // Fetch with auth retry
    let data = match fetch_notifications(&base_url, &token, max.unwrap_or(50)).await {
        Ok(data) => data,
        Err(e) if e.contains("auth") || e.contains("401") || e.contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            fetch_notifications(&base_url, &new_token, max.unwrap_or(50))
                .await
                .map_err(|e| CommandError::Network(format!("Notifications failed after reauth: {}", e)))?
        }
        Err(e) => return Err(CommandError::Network(format!("Notifications failed: {}", e))),
    };

    Ok(data)
}

/// Fetch notifications from VAL Workspace API (notifications:stream)
/// This endpoint returns both activity and error notifications.
/// Errors have fail=true and status="fail".
async fn fetch_notifications(
    base_url: &str,
    token: &str,
    max: u32,
) -> Result<serde_json::Value, String> {
    let client = crate::HTTP_CLIENT.clone();

    // Use workspace API endpoint which reads from notifications:stream (includes errors)
    let url = format!("{}/api/v1/workspace/notifications/notifications", base_url);

    let response = client
        .get(&url)
        .query(&[
            ("token", token),
            ("max", &max.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status().as_u16();
    if is_auth_status(status) {
        return Err(format!("auth error (HTTP {})", status));
    }
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        if is_auth_body(&body) {
            return Err(format!("auth error: {}", body));
        }
        return Err(format!("HTTP {}: {}", status, body));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse notifications response: {}", e))
}

/// Fetch SOD tables status (single request, no pagination)
async fn fetch_sod_tables_status(
    base_url: &str,
    api_domain: &str,
    token: &str,
    date: &str,
    regenerate: bool,
) -> Result<serde_json::Value, String> {
    let client = crate::HTTP_CLIENT.clone();

    let url = format!("{}/api/v1/sync/sod/tables/status/{}", base_url, date);

    let response = client
        .get(&url)
        .header("sub_domain", api_domain)
        .query(&[
            ("token", token),
            ("regenerate", if regenerate { "true" } else { "false" }),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = response.status().as_u16();
    if is_auth_status(status) {
        return Err(format!("auth error (HTTP {})", status));
    }
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        if is_auth_body(&body) {
            return Err(format!("auth error: {}", body));
        }
        return Err(format!("HTTP {}: {}", status, body));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse SOD response: {}", e))
}
