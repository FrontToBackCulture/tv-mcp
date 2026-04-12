// VAL Sync Metadata - Tracks sync status per domain via Supabase
// Replaces the previous .sync-metadata.json file-based approach

use crate::core::error::CmdResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// Types (frontend-facing — shape unchanged)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactStatus {
    pub last_sync: String,
    pub count: usize,
    pub status: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub domain: String,
    pub created: String,
    #[serde(default)]
    pub artifacts: HashMap<String, ArtifactStatus>,
    #[serde(default)]
    pub extractions: HashMap<String, ArtifactStatus>,
    #[serde(default)]
    pub history: Vec<serde_json::Value>, // kept empty, field preserved for backwards compat
}

// ============================================================================
// Supabase row type
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusRow {
    pub domain: String,
    pub artifact: String,
    pub phase: String,
    pub count: i64,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub last_sync: String,
}

// ============================================================================
// Internal helpers
// ============================================================================

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// ============================================================================
// Public API — write to Supabase
// ============================================================================

pub async fn update_artifact_sync(
    _global_path: &str,
    domain: &str,
    artifact_type: &str,
    count: usize,
    status: &str,
    duration_ms: u64,
) {
    let row = SyncStatusRow {
        domain: domain.to_string(),
        artifact: artifact_type.to_string(),
        phase: "sync".to_string(),
        count: count as i64,
        status: status.to_string(),
        duration_ms: Some(duration_ms as i64),
        last_sync: now_iso(),
    };
    // Fire-and-forget upsert — don't block sync on metadata write
    if let Ok(client) = crate::core::supabase::get_client().await {
        let _: Result<SyncStatusRow, _> = client
            .upsert_on("domain_sync_status", &row, Some("domain,artifact,phase"))
            .await;
    }
}

pub async fn update_extraction_sync(
    _global_path: &str,
    domain: &str,
    extraction_type: &str,
    count: usize,
    status: &str,
    duration_ms: u64,
) {
    let row = SyncStatusRow {
        domain: domain.to_string(),
        artifact: extraction_type.to_string(),
        phase: "extract".to_string(),
        count: count as i64,
        status: status.to_string(),
        duration_ms: Some(duration_ms as i64),
        last_sync: now_iso(),
    };
    if let Ok(client) = crate::core::supabase::get_client().await {
        let _: Result<SyncStatusRow, _> = client
            .upsert_on("domain_sync_status", &row, Some("domain,artifact,phase"))
            .await;
    }
}

// ============================================================================
// Commands — read from Supabase
// ============================================================================

/// Get sync status/metadata for a domain (reads from Supabase)

pub async fn val_sync_get_status(domain: String) -> CmdResult<SyncMetadata> {
    let client = crate::core::supabase::get_client().await?;
    let rows: Vec<SyncStatusRow> = client
        .select(
            "domain_sync_status",
            &format!("domain=eq.{}", domain),
        )
        .await
        .unwrap_or_default();

    let mut artifacts = HashMap::new();
    let mut extractions = HashMap::new();

    for row in rows {
        let status = ArtifactStatus {
            last_sync: row.last_sync,
            count: row.count as usize,
            status: row.status,
            duration_ms: row.duration_ms.map(|d| d as u64),
        };
        match row.phase.as_str() {
            "sync" => { artifacts.insert(row.artifact, status); }
            "extract" => { extractions.insert(row.artifact, status); }
            _ => {}
        }
    }

    Ok(SyncMetadata {
        domain: domain.clone(),
        created: now_iso(),
        artifacts,
        extractions,
        history: Vec::new(),
    })
}

/// Helper for domain discovery: get latest sync timestamp and total artifact count
pub async fn read_sync_summary(domain: &str) -> (Option<String>, Option<u32>) {
    let client = match crate::core::supabase::get_client().await {
        Ok(c) => c,
        Err(_) => return (None, None),
    };

    let rows: Vec<SyncStatusRow> = client
        .select(
            "domain_sync_status",
            &format!("domain=eq.{}&phase=eq.sync", domain),
        )
        .await
        .unwrap_or_default();

    if rows.is_empty() {
        return (None, None);
    }

    let mut latest_sync: Option<String> = None;
    let mut total_count: u32 = 0;

    for row in &rows {
        total_count += row.count as u32;
        if let Some(ref current) = latest_sync {
            if row.last_sync > *current {
                latest_sync = Some(row.last_sync.clone());
            }
        } else {
            latest_sync = Some(row.last_sync.clone());
        }
    }

    (latest_sync, if total_count > 0 { Some(total_count) } else { None })
}

// ============================================================================
// Output File Status (unchanged — filesystem checks by nature)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct OutputFileStatus {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub category: String,
    pub is_folder: bool,
    pub exists: bool,
    pub modified: Option<String>,
    pub size: Option<u64>,
    pub item_count: Option<usize>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputStatusResult {
    pub domain: String,
    pub global_path: String,
    pub outputs: Vec<OutputFileStatus>,
}

fn get_expected_outputs(_global_path: &str) -> Vec<(String, String, String, bool, String)> {
    // (name, relative_path, category, is_folder, created_by)
    vec![
        // Schema Sync - aggregate JSON files from VAL API (Sync All / individual sync buttons)
        ("Fields".to_string(), "schema/all_fields.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        ("Queries".to_string(), "schema/all_queries.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        ("Workflows".to_string(), "schema/all_workflows.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        ("Dashboards".to_string(), "schema/all_dashboards.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        ("Tables".to_string(), "schema/all_tables.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        ("Calc Fields".to_string(), "schema/all_calculated_fields.json".to_string(), "Schema Sync".to_string(), false, "Sync All".to_string()),
        // Extractions - individual definition files (Sync All auto-extracts)
        ("Queries".to_string(), "queries/".to_string(), "Extractions".to_string(), true, "Sync All".to_string()),
        ("Workflows".to_string(), "workflows/".to_string(), "Extractions".to_string(), true, "Sync All".to_string()),
        ("Dashboards".to_string(), "dashboards/".to_string(), "Extractions".to_string(), true, "Sync All".to_string()),
        ("Data Models".to_string(), "data_models/".to_string(), "Extractions".to_string(), true, "Sync All".to_string()),
        // Monitoring - workflow executions and SOD status
        ("Executions".to_string(), "monitoring/".to_string(), "Monitoring".to_string(), true, "Monitoring / SOD".to_string()),
        // Analytics - error tracking
        ("Errors".to_string(), "analytics/".to_string(), "Analytics".to_string(), true, "Importer Err / Integration Err".to_string()),
    ]
}

fn check_file_status(
    global_path: &str,
    name: &str,
    relative_path: &str,
    category: &str,
    is_folder: bool,
    created_by: &str,
) -> OutputFileStatus {
    let full_path = Path::new(global_path).join(relative_path);
    let exists = full_path.exists();

    let (modified, size, item_count) = if exists {
        let metadata = fs::metadata(&full_path).ok();
        let modified = metadata.as_ref().and_then(|m| {
            m.modified().ok().map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        });
        let size = metadata.as_ref().map(|m| m.len());

        // Count items if it's a folder
        let item_count = if is_folder {
            fs::read_dir(&full_path)
                .map(|entries| entries.filter_map(|e| e.ok()).count())
                .ok()
        } else {
            None
        };

        (modified, size, item_count)
    } else {
        (None, None, None)
    };

    OutputFileStatus {
        name: name.to_string(),
        path: full_path.to_string_lossy().to_string(),
        relative_path: relative_path.to_string(),
        category: category.to_string(),
        is_folder,
        exists,
        modified,
        size,
        item_count,
        created_by: created_by.to_string(),
    }
}

/// Get status of all expected output files/folders for a domain

pub fn val_get_output_status(domain: String) -> CmdResult<OutputStatusResult> {
    let domain_config = super::config::get_domain_config(&domain)?;
    let global_path = &domain_config.global_path;

    let expected = get_expected_outputs(global_path);
    let outputs: Vec<OutputFileStatus> = expected
        .iter()
        .map(|(name, rel_path, category, is_folder, created_by)| {
            check_file_status(global_path, name, rel_path, category, *is_folder, created_by)
        })
        .collect();

    Ok(OutputStatusResult {
        domain,
        global_path: global_path.clone(),
        outputs,
    })
}
