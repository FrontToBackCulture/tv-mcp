// VAL Drive - Browse files in VAL Drive (S3-backed file storage)
// Lists folders and files via VAL Drive HTTP API

use super::auth;
use super::config::{get_domain_config, val_sync_list_domains};
use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveFolder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default, rename = "type")]
    pub file_type: Option<String>,
    #[serde(default)]
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveFilesResult {
    pub files: Vec<DriveFile>,
    #[serde(default)]
    pub last_key: Option<String>,
    pub is_last_page: bool,
}

// ============================================================================
// Helpers
// ============================================================================

fn is_auth_status(status: u16) -> bool {
    status == 401 || status == 403
}

fn is_auth_body(body: &str) -> bool {
    body.contains("token not authentic")
        || body.contains("jwt expired")
        || body.contains("invalid signature")
}

// ============================================================================
// Commands
// ============================================================================

/// List folders in a VAL Drive path

pub async fn val_drive_list_folders(
    domain: String,
    folder_id: Option<String>,
) -> CmdResult<Vec<DriveFolder>> {
    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let folder = folder_id.unwrap_or_else(|| "val_drive".to_string());

    let (token, _) = auth::ensure_auth(&domain).await?;

    match fetch_folders(&base_url, &api_domain, &token, &folder).await {
        Ok(folders) => Ok(folders),
        Err(e) if e.to_string().contains("auth") || e.to_string().contains("401") || e.to_string().contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            fetch_folders(&base_url, &api_domain, &new_token, &folder)
                .await
                .map_err(|e| CommandError::Network(format!("Drive list folders failed after reauth: {}", e)))
        }
        Err(e) => Err(CommandError::Network(format!("Drive list folders failed: {}", e))),
    }
}

/// List files in a VAL Drive folder

pub async fn val_drive_list_files(
    domain: String,
    folder_id: String,
    page_size: Option<u32>,
) -> CmdResult<DriveFilesResult> {
    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let size = page_size.unwrap_or(200);

    let (token, _) = auth::ensure_auth(&domain).await?;

    match fetch_files(&base_url, &api_domain, &token, &folder_id, size).await {
        Ok(result) => Ok(result),
        Err(e) if e.to_string().contains("auth") || e.to_string().contains("401") || e.to_string().contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            fetch_files(&base_url, &api_domain, &new_token, &folder_id, size)
                .await
                .map_err(|e| CommandError::Network(format!("Drive list files failed after reauth: {}", e)))
        }
        Err(e) => Err(CommandError::Network(format!("Drive list files failed: {}", e))),
    }
}

// ============================================================================
// Fetch helpers
// ============================================================================

async fn fetch_folders(
    base_url: &str,
    api_domain: &str,
    token: &str,
    folder_id: &str,
) -> CmdResult<Vec<DriveFolder>> {
    let client = crate::HTTP_CLIENT.clone();

    let url = format!("{}/api/v1/val_drive/folders", base_url);

    let response = client
        .get(&url)
        .header("sub_domain", api_domain)
        .query(&[("folderId", folder_id), ("token", token)])
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

    let body: serde_json::Value = response.json().await?;

    // API returns { data: [...] } or just [...]
    let items = if let Some(arr) = body.get("data").and_then(|d| d.as_array()) {
        arr.clone()
    } else if let Some(arr) = body.as_array() {
        arr.clone()
    } else {
        return Ok(vec![]);
    };

    let folders: Vec<DriveFolder> = items
        .into_iter()
        .filter_map(|item| {
            let name = item
                .get("name")
                .or_else(|| item.get("folderName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let id = item
                .get("id")
                .or_else(|| item.get("folderId"))
                .or_else(|| item.get("prefix"))
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            let last_modified = item
                .get("lastModified")
                .or_else(|| item.get("last_modified"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if name.is_empty() && id.is_empty() {
                return None;
            }

            Some(DriveFolder {
                id,
                name,
                last_modified,
            })
        })
        .collect();

    Ok(folders)
}

async fn fetch_files(
    base_url: &str,
    api_domain: &str,
    token: &str,
    folder_id: &str,
    page_size: u32,
) -> CmdResult<DriveFilesResult> {
    let client = crate::HTTP_CLIENT.clone();

    // URL-encode the folder_id for path usage
    let encoded_folder = urlencoding::encode(folder_id);
    let url = format!("{}/api/v1/val_drive/folders/{}/files", base_url, encoded_folder);

    let response = client
        .get(&url)
        .header("sub_domain", api_domain)
        .query(&[
            ("token", token),
            ("pageSize", &page_size.to_string()),
        ])
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

    let body: serde_json::Value = response.json().await?;

    // Parse files from response
    let items = if let Some(arr) = body.get("data").and_then(|d| d.as_array()) {
        arr.clone()
    } else if let Some(arr) = body.get("files").and_then(|d| d.as_array()) {
        arr.clone()
    } else if let Some(arr) = body.as_array() {
        arr.clone()
    } else {
        vec![]
    };

    let files: Vec<DriveFile> = items
        .into_iter()
        .filter_map(|item| {
            let name = item
                .get("name")
                .or_else(|| item.get("fileName"))
                .or_else(|| item.get("key"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                return None;
            }

            let id = item
                .get("id")
                .or_else(|| item.get("fileId"))
                .or_else(|| item.get("key"))
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();

            let size = item
                .get("size")
                .or_else(|| item.get("fileSize"))
                .and_then(|v| v.as_u64());

            let file_type = item
                .get("type")
                .or_else(|| item.get("contentType"))
                .or_else(|| item.get("mimeType"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let last_modified = item
                .get("lastModified")
                .or_else(|| item.get("last_modified"))
                .or_else(|| item.get("uploadedAt"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Some(DriveFile {
                id,
                name,
                size,
                file_type,
                last_modified,
            })
        })
        .collect();

    let last_key = body
        .get("lastKey")
        .or_else(|| body.get("nextToken"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_last_page = body
        .get("isLastPage")
        .or_else(|| body.get("isTruncated").map(|v| {
            // isTruncated=true means NOT last page, so we negate
            // But we return as-is and handle below
            v
        }))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // If we got isTruncated instead of isLastPage, the logic is inverted
    let is_last = if body.get("isTruncated").is_some() {
        !body.get("isTruncated").and_then(|v| v.as_bool()).unwrap_or(false)
    } else {
        is_last_page
    };

    Ok(DriveFilesResult {
        files,
        last_key,
        is_last_page: is_last,
    })
}

// ============================================================================
// Workflow folder config (parsed from all_workflows.json on disk)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveWorkflowFolder {
    pub folder_path: String,
    pub move_to_processed: bool,
    pub workflow_count: usize,
}

/// Parse all_workflows.json for a domain and extract VALDriveToVALInsertPlugin folder configs.
/// Returns which Drive folders have workflows and whether they move files to processed/.
pub fn parse_workflow_drive_folders(global_path: &str) -> Vec<DriveWorkflowFolder> {
    let wf_path = std::path::Path::new(global_path).join("schema/all_workflows.json");
    let content = match std::fs::read_to_string(&wf_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let data: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let workflows = match data.get("data").and_then(|d| d.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    // Collect folder_path -> (move_to_processed, count) mapping
    let mut folder_map: std::collections::HashMap<String, (bool, usize)> =
        std::collections::HashMap::new();

    for wf in workflows {
        let plugins = wf
            .get("data")
            .and_then(|d| d.get("workflow"))
            .and_then(|w| w.get("plugins"))
            .and_then(|p| p.as_array());

        if let Some(plugins) = plugins {
            for plugin in plugins {
                let name = plugin.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if name != "VALDriveToVALInsertPlugin" {
                    continue;
                }
                let params = match plugin.get("params") {
                    Some(p) => p,
                    None => continue,
                };
                let folder_path = params
                    .get("folderPath")
                    .and_then(|f| f.as_str())
                    .unwrap_or("")
                    .to_string();
                if folder_path.is_empty() {
                    continue;
                }
                let move_flag = params
                    .get("moveFileToProcessedFolder")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let entry = folder_map.entry(folder_path).or_insert((move_flag, 0));
                entry.1 += 1;
                // If ANY workflow for this folder moves to processed, mark it true
                if move_flag {
                    entry.0 = true;
                }
            }
        }
    }

    folder_map
        .into_iter()
        .map(|(folder_path, (move_to_processed, workflow_count))| DriveWorkflowFolder {
            folder_path,
            move_to_processed,
            workflow_count,
        })
        .collect()
}

/// Tauri command: get Drive workflow folder configs for a domain

pub async fn val_drive_workflow_folders(domain: String) -> CmdResult<Vec<DriveWorkflowFolder>> {
    let domain_config = get_domain_config(&domain)?;
    Ok(parse_workflow_drive_folders(&domain_config.global_path))
}

/// Get workflow folder configs for all production domains (used by MCP tool)
pub fn get_all_domain_workflow_folders() -> HashMap<String, Vec<DriveWorkflowFolder>> {
    let domains = match val_sync_list_domains() {
        Ok(d) => d,
        Err(_) => return HashMap::new(),
    };
    let excluded = ["documentation", "lab", "templates"];
    let mut result = HashMap::new();
    for d in domains {
        if excluded.contains(&d.domain.to_lowercase().as_str()) {
            continue;
        }
        let folders = parse_workflow_drive_folders(&d.global_path);
        if !folders.is_empty() {
            result.insert(d.domain.clone(), folders);
        }
    }
    result
}

// ============================================================================
// Upload files to VAL Drive
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileResult {
    pub name: String,
    pub status: String, // "uploaded" | "error"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub total: usize,
    pub uploaded: usize,
    pub failed: usize,
    pub files: Vec<UploadFileResult>,
}

/// Upload local files to a VAL Drive folder.
/// Reads files from disk and POSTs them as multipart form data to the VAL Drive API.

pub async fn val_drive_upload_files(
    domain: String,
    folder_path: String,
    file_paths: Vec<String>,
) -> CmdResult<UploadResult> {
    if file_paths.is_empty() {
        return Ok(UploadResult {
            total: 0,
            uploaded: 0,
            failed: 0,
            files: vec![],
        });
    }

    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let (token, _) = auth::ensure_auth(&domain).await?;

    let mut results: Vec<UploadFileResult> = Vec::new();
    let mut uploaded = 0usize;
    let mut failed = 0usize;

    // Upload in batches of 10 files
    let batch_size = 10;
    for chunk in file_paths.chunks(batch_size) {
        match upload_batch(&base_url, &api_domain, &token, &folder_path, chunk).await {
            Ok(batch_results) => {
                for r in batch_results {
                    if r.status == "uploaded" {
                        uploaded += 1;
                    } else {
                        failed += 1;
                    }
                    results.push(r);
                }
            }
            Err(e) => {
                // If auth error, retry once with reauth
                if e.to_string().contains("auth") || e.to_string().contains("401") || e.to_string().contains("403") {
                    let (new_token, _) = auth::reauth(&domain).await?;
                    match upload_batch(&base_url, &api_domain, &new_token, &folder_path, chunk).await {
                        Ok(batch_results) => {
                            for r in batch_results {
                                if r.status == "uploaded" {
                                    uploaded += 1;
                                } else {
                                    failed += 1;
                                }
                                results.push(r);
                            }
                        }
                        Err(e2) => {
                            for path in chunk {
                                let name = std::path::Path::new(path)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.clone());
                                failed += 1;
                                results.push(UploadFileResult {
                                    name,
                                    status: "error".to_string(),
                                    error: Some(format!("Upload failed after reauth: {}", e2)),
                                });
                            }
                        }
                    }
                } else {
                    for path in chunk {
                        let name = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());
                        failed += 1;
                        results.push(UploadFileResult {
                            name,
                            status: "error".to_string(),
                            error: Some(format!("Upload failed: {}", e)),
                        });
                    }
                }
            }
        }

        // Small delay between batches
        if chunk.len() == batch_size {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    Ok(UploadResult {
        total: file_paths.len(),
        uploaded,
        failed,
        files: results,
    })
}

async fn upload_batch(
    base_url: &str,
    api_domain: &str,
    token: &str,
    folder_path: &str,
    file_paths: &[String],
) -> CmdResult<Vec<UploadFileResult>> {
    let client = crate::HTTP_CLIENT.clone();
    let url = format!("{}/api/v1/val_drive/filesAsync", base_url);

    let mut form = reqwest::multipart::Form::new();
    let mut file_names: Vec<String> = Vec::new();

    for path_str in file_paths {
        let path = std::path::Path::new(path_str);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let content = tokio::fs::read(path).await.map_err(|e| {
            CommandError::Io(format!("Failed to read file {}: {}", path_str, e))
        })?;

        let part = reqwest::multipart::Part::bytes(content)
            .file_name(file_name.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        form = form.part("file[]", part);
        file_names.push(file_name);
    }

    let response = client
        .post(&url)
        .header("sub_domain", api_domain)
        .query(&[
            ("uuid", "1"),
            ("token", token),
            ("folderPath", folder_path),
        ])
        .multipart(form)
        .timeout(std::time::Duration::from_secs(300))
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

    // API returns 200 but files may still be processing async — we mark as uploaded
    Ok(file_names
        .into_iter()
        .map(|name| UploadFileResult {
            name,
            status: "uploaded".to_string(),
            error: None,
        })
        .collect())
}

// ============================================================================
// Trigger VAL workflow execution
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRerunResult {
    pub workflow_id: u64,
    pub execution_id: u64,
    pub status: String,
}

/// Trigger a VAL workflow rerun on a domain.
/// Used to run dataLoad workflows after uploading files to VAL Drive.

pub async fn val_workflow_rerun(
    domain: String,
    workflow_id: u64,
) -> CmdResult<WorkflowRerunResult> {
    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let (token, _) = auth::ensure_auth(&domain).await?;

    match rerun_workflow(&base_url, &api_domain, &token, workflow_id).await {
        Ok(r) => Ok(r),
        Err(e) if e.to_string().contains("auth") || e.to_string().contains("401") || e.to_string().contains("403") => {
            let (new_token, _) = auth::reauth(&domain).await?;
            rerun_workflow(&base_url, &api_domain, &new_token, workflow_id)
                .await
                .map_err(|e| CommandError::Network(format!("Workflow rerun failed after reauth: {}", e)))
        }
        Err(e) => Err(e),
    }
}

async fn rerun_workflow(
    base_url: &str,
    api_domain: &str,
    token: &str,
    workflow_id: u64,
) -> CmdResult<WorkflowRerunResult> {
    let client = crate::HTTP_CLIENT.clone();
    let url = format!("{}/api/v1/workflow/{}/rerun", base_url, workflow_id);

    let response = client
        .post(&url)
        .header("sub_domain", api_domain)
        .query(&[("uuid", "1"), ("token", token)])
        .json(&serde_json::json!({}))
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

    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::json!({}));
    let exec_id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|id| id.as_u64())
        .or_else(|| body.get("id").and_then(|id| id.as_u64()))
        .unwrap_or(0);

    Ok(WorkflowRerunResult {
        workflow_id,
        execution_id: exec_id,
        status: "triggered".to_string(),
    })
}

// ============================================================================
// Insert rows into VAL tables
// ============================================================================

/// Insert rows into a VAL table via the CRUD middleware (/addCustomData → records/create).

pub async fn val_table_insert_rows(
    domain: String,
    table_name: String,
    zone: String,
    pk: String,
    columns: Vec<serde_json::Value>,   // [{ column_name, data_type }]
    rows: Vec<Vec<String>>,            // each row is values matching columns order
) -> CmdResult<serde_json::Value> {
    if rows.is_empty() {
        return Ok(serde_json::json!({ "inserted": 0 }));
    }

    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let (token, _) = auth::ensure_auth(&domain).await?;

    let client = crate::HTTP_CLIENT.clone();
    // The public endpoint is /db/crud/addCustomData (proxied to crud-service /api/v1/records/create)
    let url = format!("{}/db/crud/addCustomData", base_url);

    // Build column_names and data_type strings
    let col_names: Vec<String> = columns.iter()
        .filter_map(|c| c.get("column_name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    let data_types: Vec<String> = columns.iter()
        .filter_map(|c| c.get("data_type").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    let column_names_str = col_names.join(",");
    let data_type_str = data_types.join(",");
    let date_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut inserted = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for row_values in &rows {
        // Build object_data with values
        let object_data: Vec<serde_json::Value> = columns.iter().enumerate().map(|(i, c)| {
            let col = c.get("column_name").and_then(|v| v.as_str()).unwrap_or("");
            let dt = c.get("data_type").and_then(|v| v.as_str()).unwrap_or("");
            let val = row_values.get(i).cloned().unwrap_or_default();
            serde_json::json!({
                "column_name": col,
                "data_type": dt,
                "type": "",
                "value": val,
            })
        }).collect();

        let body = serde_json::json!({
            "tableid": table_name,
            "zone": zone,
            "pk": pk,
            "curr_date": date_now,
            "created_date": date_now,
            "value_array": row_values,
            "column_names": column_names_str,
            "data_type": data_type_str,
            "object_data": object_data,
            "user": "tv-client",
            "name": table_name,
        });

        let response = client
            .post(&url)
            .header("sub_domain", &api_domain)
            .query(&[("uuid", "1"), ("token", &token)])
            .json(&body)
            .send()
            .await?;

        let resp_status = response.status().as_u16();
        let resp_body = response.text().await.unwrap_or_default();
        if resp_status >= 200 && resp_status < 300 {
            inserted += 1;
        } else {
            // Include request body snippet for debugging
            let body_preview = serde_json::to_string(&body).unwrap_or_default();
            errors.push(format!(
                "HTTP {}: {} | Sent: {}",
                resp_status,
                &resp_body[..resp_body.len().min(500)],
                &body_preview[..body_preview.len().min(500)]
            ));
        }
    }

    Ok(serde_json::json!({
        "inserted": inserted,
        "failed": errors.len(),
        "errors": errors,
    }))
}

/// Check the latest execution status for a specific workflow ID.
/// Queries recent executions and finds the most recent one for this workflow.

pub async fn val_workflow_execution_status(
    domain: String,
    workflow_id: u64,
) -> CmdResult<serde_json::Value> {
    let domain_config = get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();
    let base_url = format!("https://{}.thinkval.io", api_domain);
    let (token, _) = auth::ensure_auth(&domain).await?;

    let client = crate::HTTP_CLIENT.clone();
    // Use the executions list endpoint with a short time window
    let now = chrono::Utc::now();
    let from = (now - chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%S").to_string();
    let to = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let url = format!("{}/api/v1/workflow/executions", base_url);
    let response = client
        .get(&url)
        .query(&[
            ("uuid", "1"),
            ("token", token.as_str()),
            ("from", from.as_str()),
            ("to", to.as_str()),
            ("page", "1"),
            ("limit", "20"),
        ])
        .send()
        .await?;

    let status = response.status().as_u16();
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::json!({}));
    let executions = body.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default();

    // Find the most recent execution for this workflow ID
    // Try multiple field names: workflowId, workflow_id, job_id
    let matching = executions.iter().find(|e| {
        for field in &["workflowId", "workflow_id", "job_id"] {
            if let Some(val) = e.get(field) {
                if val.as_u64() == Some(workflow_id) { return true; }
                if val.as_str().map(|s| s == workflow_id.to_string()).unwrap_or(false) { return true; }
            }
        }
        false
    });

    match matching {
        Some(exec) => Ok(exec.clone()),
        None => {
            // Return first execution as fallback with debug info
            if let Some(first) = executions.first() {
                let keys: Vec<String> = first.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
                Ok(serde_json::json!({
                    "status": "unknown",
                    "_debug_keys": keys,
                    "_debug_workflow_id": workflow_id,
                    "_debug_first_exec": first,
                    "_debug_total": executions.len()
                }))
            } else {
                Ok(serde_json::json!({ "status": "unknown", "_debug": "no executions found" }))
            }
        }
    }
}

// ============================================================================
// AI outlet matching — maps data outlet names to scope outlet codes
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutletMatch {
    pub data_name: String,
    pub scope_code: String,  // empty if no match
    pub confidence: String,  // "high", "medium", "low"
}

/// Use Claude to match outlet names from data files to scope outlet codes.

pub async fn ai_match_outlets(
    scope_outlets: Vec<serde_json::Value>,  // [{ entity, outlet }]
    data_outlets: Vec<String>,              // store names from CSV
) -> CmdResult<Vec<OutletMatch>> {
    use crate::core::settings::{load_settings, KEY_ANTHROPIC_API};

    let settings = load_settings()?;
    let api_key = settings
        .keys
        .get(KEY_ANTHROPIC_API)
        .ok_or_else(|| CommandError::Config("Anthropic API key not configured".to_string()))?
        .clone();

    // Build the prompt
    let scope_list: Vec<String> = scope_outlets
        .iter()
        .map(|o| {
            format!(
                "{} ({})",
                o.get("outlet").and_then(|v| v.as_str()).unwrap_or(""),
                o.get("entity").and_then(|v| v.as_str()).unwrap_or("")
            )
        })
        .collect();

    let prompt = format!(
        "Match each data outlet name to the most likely scope outlet code. \
        Scope outlets (code + entity):\n{}\n\n\
        Data outlet names:\n{}\n\n\
        Return ONLY a JSON array, no markdown, no explanation. Each element: \
        {{\"data_name\": \"...\", \"scope_code\": \"...\", \"confidence\": \"high|medium|low\"}}. \
        If no match, use scope_code: \"\". Be strict — only match if clearly the same location.",
        scope_list.join("\n"),
        data_outlets.join("\n")
    );

    let client = crate::HTTP_CLIENT.clone();
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 2048,
        "temperature": 0.0,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| CommandError::Network(format!("Anthropic API failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    let result: serde_json::Value = resp.json().await?;
    let text = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("[]");

    // Parse the JSON array from the response
    let matches: Vec<OutletMatch> = serde_json::from_str(text).unwrap_or_else(|_| {
        // Try to extract JSON from markdown code block
        let trimmed = text.trim();
        let json_str = if trimmed.starts_with("```") {
            trimmed
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            trimmed
        };
        serde_json::from_str(json_str).unwrap_or_default()
    });

    Ok(matches)
}

// ============================================================================
// Drive Scan Config — persisted folder list per domain
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveScanFolder {
    pub folder_path: String,
    pub enabled: bool,
    pub move_to_processed: bool,
    pub source: String, // "workflow" | "manual"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainScanConfig {
    pub folders: Vec<DriveScanFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveScanConfig {
    pub domains: HashMap<String, DomainScanConfig>,
}

fn scan_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tv-desktop")
        .join("drive-scan-config.json")
}

/// Load scan config from disk. Returns empty config if file doesn't exist.
pub fn load_scan_config() -> DriveScanConfig {
    let path = scan_config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(DriveScanConfig {
            domains: HashMap::new(),
        }),
        Err(_) => DriveScanConfig {
            domains: HashMap::new(),
        },
    }
}

fn save_scan_config_to_disk(config: &DriveScanConfig) -> CmdResult<()> {
    let path = scan_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Seed scan config from workflow configs, merging with existing user edits
pub fn seed_scan_config() -> CmdResult<DriveScanConfig> {
    let mut config = load_scan_config();
    let all_wf = get_all_domain_workflow_folders();

    for (domain, wf_folders) in &all_wf {
        let domain_config = config
            .domains
            .entry(domain.clone())
            .or_insert_with(|| DomainScanConfig {
                folders: Vec::new(),
            });

        // Index existing folders by path
        let existing_paths: HashMap<String, usize> = domain_config
            .folders
            .iter()
            .enumerate()
            .map(|(i, f)| (f.folder_path.clone(), i))
            .collect();

        // Merge workflow folders
        let mut new_folders: Vec<DriveScanFolder> = Vec::new();
        for wf in wf_folders {
            if let Some(&idx) = existing_paths.get(&wf.folder_path) {
                // Already exists — update move_to_processed from workflow, keep user's enabled state
                domain_config.folders[idx].move_to_processed = wf.move_to_processed;
            } else {
                // New workflow folder — add as enabled
                new_folders.push(DriveScanFolder {
                    folder_path: wf.folder_path.clone(),
                    enabled: true,
                    move_to_processed: wf.move_to_processed,
                    source: "workflow".to_string(),
                });
            }
        }
        domain_config.folders.extend(new_folders);

        // Sort folders by path for consistent ordering
        domain_config
            .folders
            .sort_by(|a, b| a.folder_path.cmp(&b.folder_path));
    }

    save_scan_config_to_disk(&config)?;
    Ok(config)
}


pub async fn val_drive_scan_config_load() -> CmdResult<DriveScanConfig> {
    Ok(load_scan_config())
}


pub async fn val_drive_scan_config_save(config: DriveScanConfig) -> CmdResult<()> {
    save_scan_config_to_disk(&config)
}


pub async fn val_drive_scan_config_seed() -> CmdResult<DriveScanConfig> {
    seed_scan_config()
}

// ============================================================================
// Drive Scan Results — persisted last scan output
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultFile {
    pub folder: String,
    pub name: String,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainScanResult {
    pub domain: String,
    pub status: String, // "clean" | "has-files" | "stale" | "error"
    pub files: Vec<ScanResultFile>,
    pub stale_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedScanResults {
    pub last_scan_at: String,
    pub results: Vec<DomainScanResult>,
}

fn scan_results_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tv-desktop")
        .join("drive-scan-results.json")
}


pub async fn val_drive_scan_results_load() -> CmdResult<Option<PersistedScanResults>> {
    let path = scan_results_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let results: PersistedScanResults = serde_json::from_str(&content)?;
            Ok(Some(results))
        }
        Err(_) => Ok(None),
    }
}


pub async fn val_drive_scan_results_save(results: PersistedScanResults) -> CmdResult<()> {
    let path = scan_results_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(&path, json)?;
    Ok(())
}
