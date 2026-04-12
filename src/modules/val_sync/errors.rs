// VAL Sync Errors - Importer and Integration error sync
// Fetches error data from centralized tv domain and writes to target domain's analytics folder

use super::config::{get_domain_config, load_config_internal};
use super::metadata;
use super::sync::{write_json, SyncResult};
use crate::core::error::{CmdResult, CommandError};
use crate::modules::val_sync::auth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// Table and column mappings
const IMPORTER_ERRORS_TABLE: &str = "custom_tbl_892_1520";
const INTEGRATION_ERRORS_TABLE: &str = "custom_tbl_892_1519";
const DOMAIN_COLUMN: &str = "usr_eaea000fefface_3";
const DATE_COLUMN: &str = "usr_cccbbdad0fee0a";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct SqlQueryRequest {
    sql: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SqlQueryResponse {
    data: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct ErrorsSyncOutput {
    #[serde(rename = "syncedAt")]
    synced_at: String,
    domain: String,
    #[serde(rename = "dateRange")]
    date_range: DateRange,
    summary: ErrorsSummary,
    #[serde(rename = "dailyErrors")]
    daily_errors: HashMap<String, u32>,
    errors: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DateRange {
    from: String,
    to: String,
}

#[derive(Debug, Serialize)]
struct ErrorsSummary {
    #[serde(rename = "totalDays")]
    total_days: usize,
    #[serde(rename = "totalErrors")]
    total_errors: usize,
}

// ============================================================================
// Helpers
// ============================================================================

fn is_auth_status(status: u16) -> bool {
    status == 401 || status == 403
}

fn is_auth_body(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("token not authentic")
        || lower.contains("jwt expired")
        || lower.contains("invalid signature")
}

/// Extract date portion (YYYY-MM-DD) from a datetime string or ISO timestamp
fn extract_date(datetime: &str) -> String {
    // Handle ISO format: 2025-01-27T10:30:00.000Z
    if let Some(t_pos) = datetime.find('T') {
        return datetime[..t_pos].to_string();
    }
    // Handle space format: 2025-01-27 10:30:00
    if let Some(space_pos) = datetime.find(' ') {
        return datetime[..space_pos].to_string();
    }
    // Already just a date
    if datetime.len() >= 10 {
        return datetime[..10].to_string();
    }
    datetime.to_string()
}

/// Get today's date as YYYY-MM-DD
fn today_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Build SQL query for fetching errors
fn build_errors_query(table: &str, domain: &str, from: &str, to: &str) -> String {
    format!(
        "SELECT * FROM {} WHERE {} = '{}' AND {} >= '{}' AND {} <= '{}' ORDER BY {} DESC",
        table, DOMAIN_COLUMN, domain, DATE_COLUMN, from, DATE_COLUMN, to, DATE_COLUMN
    )
}

/// Execute SQL query against tv domain
async fn execute_tv_sql(
    token: &str,
    sql: &str,
) -> CmdResult<SqlQueryResponse> {
    let client = crate::HTTP_CLIENT.clone();

    let url = "https://tv.thinkval.io/api/v1/sqls/execute";

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .query(&[("token", token)])
        .json(&SqlQueryRequest {
            sql: sql.to_string(),
        })
        .send()
        .await?;

    let status = response.status().as_u16();
    if is_auth_status(status) {
        return Err(CommandError::Network(format!("auth error (HTTP {})", status)));
    }
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        if is_auth_body(&body) {
            return Err(CommandError::Network(format!("auth error: {}", body)));
        }
        return Err(CommandError::Http { status, body });
    }

    Ok(response.json().await?)
}

/// Fetch SQL results from tv domain
async fn fetch_all_errors(
    token: &str,
    sql: &str,
) -> CmdResult<Vec<serde_json::Value>> {
    let response = execute_tv_sql(token, sql).await?;
    Ok(response.data.unwrap_or_default())
}

/// Calculate daily breakdown from error records
fn calculate_daily_breakdown(errors: &[serde_json::Value]) -> HashMap<String, u32> {
    let mut daily: HashMap<String, u32> = HashMap::new();

    for error in errors {
        if let Some(date_value) = error.get(DATE_COLUMN) {
            let date_str = match date_value {
                serde_json::Value::String(s) => extract_date(s),
                _ => continue,
            };
            *daily.entry(date_str).or_insert(0) += 1;
        }
    }

    daily
}

/// Sync errors (shared implementation for both types)
async fn sync_errors_impl(
    domain: String,
    from: String,
    to: String,
    table: &str,
    error_type: &str,
    file_prefix: &str,
) -> CmdResult<SyncResult> {
    let start = Instant::now();

    // Get target domain config for output path
    let domain_config = get_domain_config(&domain)?;
    let global_path = &domain_config.global_path;

    // Get tv domain config for authentication
    let config = load_config_internal()?;
    let tv_domain = config
        .domains
        .iter()
        .find(|d| d.domain == "tv" || d.actual_domain.as_deref() == Some("tv"))
        .ok_or_else(|| CommandError::Config("tv domain not found in config. Error data requires tv domain access.".to_string()))?;

    // Ensure auth to tv domain
    let (token, _) = auth::ensure_auth(&tv_domain.domain).await?;

    // Build and execute query
    let sql = build_errors_query(table, &domain, &from, &to);

    let errors = match fetch_all_errors(&token, &sql).await {
        Ok(data) => data,
        Err(e) if e.to_string().contains("auth") || e.to_string().contains("401") || e.to_string().contains("403") => {
            let (new_token, _) = auth::reauth(&tv_domain.domain).await?;
            fetch_all_errors(&new_token, &sql)
                .await
                .map_err(|e| CommandError::Internal(format!("{} sync failed after reauth: {}", error_type, e)))?
        }
        Err(e) => return Err(CommandError::Internal(format!("{} sync failed: {}", error_type, e))),
    };

    // Calculate daily breakdown
    let daily_errors = calculate_daily_breakdown(&errors);

    // Build output
    let output = ErrorsSyncOutput {
        synced_at: chrono::Utc::now().to_rfc3339(),
        domain: domain.clone(),
        date_range: DateRange {
            from: from.clone(),
            to: to.clone(),
        },
        summary: ErrorsSummary {
            total_days: daily_errors.len(),
            total_errors: errors.len(),
        },
        daily_errors,
        errors: errors.clone(),
    };

    // Write to analytics folder
    let today = today_date();
    let file_path = format!(
        "{}/analytics/{}_{}.json",
        global_path, file_prefix, today
    );

    let output_value = serde_json::to_value(&output)?;

    write_json(&file_path, &output_value)?;

    let count = errors.len();
    let duration_ms = start.elapsed().as_millis() as u64;

    metadata::update_artifact_sync(
        global_path,
        &domain,
        &format!("errors:{}", error_type),
        count,
        "ok",
        duration_ms,
    ).await;

    Ok(SyncResult {
        domain,
        artifact_type: format!("errors:{}", error_type),
        count,
        file_path,
        duration_ms,
        status: "ok".to_string(),
        message: format!("Synced {} {} errors ({} days)", count, error_type, output.summary.total_days),
    })
}

// ============================================================================
// Commands
// ============================================================================

/// Sync importer errors for a domain.
/// Fetches from centralized tv domain table and saves to domain's analytics folder.

pub async fn val_sync_importer_errors(
    domain: String,
    from: String,
    to: String,
) -> CmdResult<SyncResult> {
    sync_errors_impl(
        domain,
        from,
        to,
        IMPORTER_ERRORS_TABLE,
        "importer",
        "importer_errors",
    )
    .await
}

/// Sync integration errors for a domain.
/// Fetches from centralized tv domain table and saves to domain's analytics folder.

pub async fn val_sync_integration_errors(
    domain: String,
    from: String,
    to: String,
) -> CmdResult<SyncResult> {
    sync_errors_impl(
        domain,
        from,
        to,
        INTEGRATION_ERRORS_TABLE,
        "integration",
        "integration_errors",
    )
    .await
}
