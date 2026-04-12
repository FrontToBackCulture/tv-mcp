// VAL Sync Operations - 6 sync operations + batch orchestrator
// Each operation fetches from VAL API and writes JSON to globalPath

use super::api::val_api_fetch;
use super::auth;
use super::config::get_domain_config;
use super::metadata;
use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub domain: String,
    pub artifact_type: String,
    pub count: usize,
    pub file_path: String,
    pub duration_ms: u64,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAllResult {
    pub domain: String,
    pub results: Vec<SyncResult>,
    pub extract_results: Vec<super::extract::ExtractResult>,
    pub stale_result: Option<serde_json::Value>,
    pub dependency_result: Option<serde_json::Value>,
    pub recency_result: Option<serde_json::Value>,
    pub total_duration_ms: u64,
    pub status: String,
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Count items in a JSON value (array length or object key count)
pub(super) fn count_items(value: &serde_json::Value) -> usize {
    if let Some(arr) = value.as_array() {
        arr.len()
    } else if let Some(data) = value.get("data") {
        if let Some(arr) = data.as_array() {
            arr.len()
        } else if let Some(obj) = data.as_object() {
            obj.len()
        } else {
            1
        }
    } else if let Some(obj) = value.as_object() {
        obj.len()
    } else {
        1
    }
}

/// Write JSON to file, creating parent directories
pub(super) fn write_json(path: &str, value: &serde_json::Value) -> CmdResult<()> {
    let p = Path::new(path);
    if let Some(dir) = p.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }
    let content = serde_json::to_string_pretty(value)?;
    fs::write(p, content)?;
    Ok(())
}

/// Generic sync operation with auth retry
async fn sync_artifact(
    domain: &str,
    artifact_type: &str,
    output_filename: &str,
) -> CmdResult<SyncResult> {
    let start = Instant::now();
    let domain_config = get_domain_config(domain)?;
    let global_path = &domain_config.global_path;
    let base_url = format!("https://{}.thinkval.io", domain_config.api_domain());
    let schema_dir = format!("{}/schema", global_path);
    let _ = fs::create_dir_all(&schema_dir);
    let file_path = format!("{}/{}", schema_dir, output_filename);

    // Ensure auth
    let (token, _) = auth::ensure_auth(domain).await?;

    // Fetch with auth retry
    let data = match val_api_fetch(&base_url, &token, artifact_type, None).await {
        Ok(data) => data,
        Err(e) if e.is_auth_error() => {
            // Retry once with fresh token
            let (new_token, _) = auth::reauth(domain).await?;
            val_api_fetch(&base_url, &new_token, artifact_type, None)
                .await
                .map_err(|e| CommandError::Network(format!("Sync {} failed after reauth: {}", artifact_type, e)))?
        }
        Err(e) => {
            return Err(CommandError::Network(format!("Sync {} failed: {}", artifact_type, e)));
        }
    };

    let count = count_items(&data);
    write_json(&file_path, &data)?;

    let duration_ms = start.elapsed().as_millis() as u64;

    metadata::update_artifact_sync(global_path, domain, artifact_type, count, "ok", duration_ms).await;

    Ok(SyncResult {
        domain: domain.to_string(),
        artifact_type: artifact_type.to_string(),
        count,
        file_path,
        duration_ms,
        status: "ok".to_string(),
        message: format!("Synced {} {} items", count, artifact_type),
    })
}

// ============================================================================
// Commands
// ============================================================================


pub async fn val_sync_fields(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "fields", "all_fields.json").await
}


pub async fn val_sync_queries(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "all-queries", "all_queries.json").await
}


pub async fn val_sync_workflows(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "all-workflows", "all_workflows.json").await
}


pub async fn val_sync_dashboards(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "all-dashboards", "all_dashboards.json").await
}


pub async fn val_sync_tables(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "all-tables", "all_tables.json").await
}


pub async fn val_sync_calc_fields(domain: String) -> CmdResult<SyncResult> {
    sync_artifact(&domain, "calc-fields", "all_calculated_fields.json").await
}

/// Full sync: all 6 sync ops + all 6 extract ops

pub async fn val_sync_all(domain: String) -> CmdResult<SyncAllResult> {
    let start = Instant::now();
    let mut results = Vec::new();
    let mut extract_results = Vec::new();
    let mut has_error = false;

    // Phase 1: Sync all aggregates
    let sync_ops: Vec<(&str, &str)> = vec![
        ("fields", "all_fields.json"),
        ("all-queries", "all_queries.json"),
        ("all-workflows", "all_workflows.json"),
        ("all-dashboards", "all_dashboards.json"),
        ("all-tables", "all_tables.json"),
        ("calc-fields", "all_calculated_fields.json"),
    ];

    for (artifact_type, filename) in &sync_ops {
        match sync_artifact(&domain, artifact_type, filename).await {
            Ok(result) => results.push(result),
            Err(e) => {
                has_error = true;
                results.push(SyncResult {
                    domain: domain.clone(),
                    artifact_type: artifact_type.to_string(),
                    count: 0,
                    file_path: String::new(),
                    duration_ms: 0,
                    status: "error".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    // Phase 2: Extract definitions
    let extract_ops = vec![
        "queries",
        "workflows",
        "dashboards",
        "tables",
        "sql",
        "calc-fields",
    ];

    for extract_type in &extract_ops {
        match super::extract::run_extract(&domain, extract_type).await {
            Ok(result) => extract_results.push(result),
            Err(e) => {
                has_error = true;
                extract_results.push(super::extract::ExtractResult {
                    domain: domain.clone(),
                    extract_type: extract_type.to_string(),
                    count: 0,
                    duration_ms: 0,
                    status: "error".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    // Phase 3-4: audit/dependencies/recency not available in standalone tv-mcp
    let stale_result: Option<serde_json::Value> = None;
    let dependency_result: Option<serde_json::Value> = None;
    let recency_result: Option<serde_json::Value> = None;

    let total_duration_ms = start.elapsed().as_millis() as u64;

    Ok(SyncAllResult {
        domain,
        results,
        extract_results,
        stale_result,
        dependency_result,
        recency_result,
        total_duration_ms,
        status: if has_error {
            "partial".to_string()
        } else {
            "ok".to_string()
        },
    })
}
