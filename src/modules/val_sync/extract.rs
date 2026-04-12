// VAL Sync Extract - Transform synced JSON into structured definitions
// 6 extract operations: queries, workflows, dashboards, tables, sql, calc-fields

use super::api::val_api_fetch;
use super::auth;
use super::config::get_domain_config;
use super::metadata;
use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub domain: String,
    pub extract_type: String,
    pub count: usize,
    pub duration_ms: u64,
    pub status: String,
    pub message: String,
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Read and parse a JSON file, returning empty Value on missing/error
fn read_json(path: &str) -> CmdResult<Value> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Write JSON to file, creating parent directories
fn write_json(path: &str, value: &Value) -> CmdResult<()> {
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

/// Write text to file, creating parent directories
fn write_text(path: &str, content: &str) -> CmdResult<()> {
    let p = Path::new(path);
    if let Some(dir) = p.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }
    fs::write(p, content)?;
    Ok(())
}

/// Extract items array from flexible JSON structure
/// Tries: root array, .data array, .{key} array
fn extract_array(data: &Value, fallback_key: &str) -> Vec<Value> {
    if let Some(arr) = data.as_array() {
        return arr.clone();
    }
    if let Some(inner) = data.get("data").and_then(|d| d.as_array()) {
        return inner.clone();
    }
    if let Some(inner) = data.get(fallback_key).and_then(|d| d.as_array()) {
        return inner.clone();
    }
    vec![]
}

/// Sanitize table name for folder naming
fn sanitize_table_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Recursively extract tables from tree structure (nodes with table_name)
fn extract_tables_from_tree(node: &Value, tables: &mut Vec<Value>) {
    if node.get("table_name").and_then(|t| t.as_str()).is_some() {
        tables.push(node.clone());
    }
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child in children {
            extract_tables_from_tree(child, tables);
        }
    }
}

/// Extract SQL from a workflow plugin
fn extract_sql_from_plugin(plugin: &Value) -> Vec<(String, String)> {
    let mut queries = Vec::new();
    let plugin_name = plugin
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Unknown");
    let params = match plugin.get("params") {
        Some(p) => p,
        None => return queries,
    };

    // Type 1: sql_query field (SQLWorkflowV2Plugin, SQLQueryExecutorPlugin,
    // CrossDomainSQLWorkflowPlugin, ClearTableRecordsPlugin)
    if let Some(sql) = params.get("sql_query").and_then(|s| s.as_str()) {
        if !sql.trim().is_empty() {
            queries.push((
                sql.to_string(),
                format!("{} -> sql_query", plugin_name),
            ));
        }
    }

    // Type 2: MultiTabExcelReportPluginV2 - input.tabs[].dataSource.source
    if let Some(tabs) = params
        .get("input")
        .and_then(|i| i.get("tabs"))
        .and_then(|t| t.as_array())
    {
        for (i, tab) in tabs.iter().enumerate() {
            if let Some(ds) = tab.get("dataSource") {
                let ds_type = ds.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if ds_type == "sql" {
                    if let Some(sql) = ds.get("source").and_then(|s| s.as_str()) {
                        if !sql.trim().is_empty() {
                            let tab_name = tab
                                .get("name")
                                .or_else(|| tab.get("sheetName"))
                                .and_then(|n| n.as_str())
                                .unwrap_or(&format!("Tab {}", i + 1))
                                .to_string();
                            queries.push((
                                sql.to_string(),
                                format!("{} -> {}", plugin_name, tab_name),
                            ));
                        }
                    }
                }
            }
        }
    }

    // Type 3: MultiTabExcelReportPlugin - tabs[].dataSource.source
    if let Some(tabs) = params.get("tabs").and_then(|t| t.as_array()) {
        for (i, tab) in tabs.iter().enumerate() {
            if let Some(ds) = tab.get("dataSource") {
                let ds_type = ds.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if ds_type == "sql" {
                    if let Some(sql) = ds.get("source").and_then(|s| s.as_str()) {
                        if !sql.trim().is_empty() {
                            let tab_name = tab
                                .get("name")
                                .or_else(|| tab.get("sheetName"))
                                .and_then(|n| n.as_str())
                                .unwrap_or(&format!("Tab {}", i + 1))
                                .to_string();
                            queries.push((
                                sql.to_string(),
                                format!("{} -> {}", plugin_name, tab_name),
                            ));
                        }
                    }
                }
            }
        }
    }

    queries
}

// ============================================================================
// Extract operations
// ============================================================================

fn extract_queries_internal(global_path: &str) -> CmdResult<usize> {
    let input = format!("{}/schema/all_queries.json", global_path);
    let data = read_json(&input)?;
    let items = extract_array(&data, "queries");

    let output_dir = format!("{}/queries", global_path);
    let mut count = 0;

    for item in &items {
        let id = item
            .get("id")
            .or_else(|| item.get("query_id"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let path = format!("{}/query_{}/definition.json", output_dir, id);
        write_json(&path, item)?;
        count += 1;
    }

    Ok(count)
}

fn extract_workflows_internal(global_path: &str) -> CmdResult<usize> {
    let input = format!("{}/schema/all_workflows.json", global_path);
    let data = read_json(&input)?;
    let items = extract_array(&data, "workflows");

    let output_dir = format!("{}/workflows", global_path);
    let mut count = 0;

    for item in &items {
        let id = item
            .get("id")
            .or_else(|| item.get("workflow_id"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let path = format!("{}/workflow_{}/definition.json", output_dir, id);
        write_json(&path, item)?;
        count += 1;
    }

    Ok(count)
}

fn extract_dashboards_internal(global_path: &str) -> CmdResult<usize> {
    let input = format!("{}/schema/all_dashboards.json", global_path);
    let data = read_json(&input)?;
    let items = extract_array(&data, "dashboards");

    let output_dir = format!("{}/dashboards", global_path);
    let mut count = 0;

    for item in &items {
        let id = item
            .get("id")
            .or_else(|| item.get("dashboard_id"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let path = format!("{}/dashboard_{}/definition.json", output_dir, id);
        write_json(&path, item)?;
        count += 1;
    }

    Ok(count)
}

/// Tables: recursive tree traversal + per-table API fetch
async fn extract_tables_internal(domain: &str, global_path: &str) -> CmdResult<usize> {
    let input = format!("{}/schema/all_tables.json", global_path);
    let data = read_json(&input)?;

    // Recursively extract table nodes
    let mut tables = Vec::new();
    if let Some(arr) = data.as_array() {
        for item in arr {
            extract_tables_from_tree(item, &mut tables);
        }
    } else if let Some(inner) = data.get("data") {
        extract_tables_from_tree(inner, &mut tables);
    } else {
        extract_tables_from_tree(&data, &mut tables);
    }

    let domain_config = get_domain_config(domain)?;
    let base_url = format!("https://{}.thinkval.io", domain_config.api_domain());
    let (token, _) = auth::ensure_auth(domain).await?;

    let output_dir = format!("{}/data_models", global_path);
    let mut count = 0;

    for table in &tables {
        let table_name = match table.get("table_name").and_then(|t| t.as_str()) {
            Some(name) => name,
            None => continue,
        };

        let sanitized = sanitize_table_name(table_name);

        // Fetch full table definition from API
        let definition = match val_api_fetch(&base_url, &token, "data-model", Some(table_name)).await {
            Ok(def) => def,
            Err(e) => {
                // Log error but continue with other tables
                eprintln!("Failed to fetch table {}: {}", table_name, e);
                continue;
            }
        };

        let path = format!("{}/table_{}/definition.json", output_dir, sanitized);
        write_json(&path, &definition)?;
        count += 1;
    }

    Ok(count)
}

/// SQL extraction from workflow definitions
fn extract_sql_internal(global_path: &str) -> CmdResult<usize> {
    let workflows_dir = format!("{}/workflows", global_path);
    let workflows_path = Path::new(&workflows_dir);
    if !workflows_path.exists() {
        return Ok(0);
    }

    let mut count = 0;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let entries = fs::read_dir(workflows_path)?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let folder_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Extract workflow ID from folder name (workflow_{id})
        let workflow_id = match folder_name.strip_prefix("workflow_") {
            Some(id) => id.to_string(),
            None => continue,
        };

        let def_path = entry_path.join("definition.json");
        if !def_path.exists() {
            continue;
        }

        let def_str = def_path.to_string_lossy().to_string();
        let data = match read_json(&def_str) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let workflow_name = data
            .get("name")
            .or_else(|| data.get("data").and_then(|d| d.get("name")))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown");

        // Get plugins from workflow data structure
        let plugins = data
            .get("data")
            .and_then(|d| d.get("workflow"))
            .and_then(|w| w.get("plugins"))
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        let mut sql_queries = Vec::new();
        for plugin in &plugins {
            sql_queries.extend(extract_sql_from_plugin(plugin));
        }

        if sql_queries.is_empty() {
            continue;
        }

        let sql_dir = entry_path.join("sql");

        for (i, (sql, source)) in sql_queries.iter().enumerate() {
            let filename = if sql_queries.len() == 1 {
                format!("workflow_{}_definition.sql", workflow_id)
            } else {
                format!("workflow_{}_definition_{}.sql", workflow_id, i + 1)
            };

            let header = format!(
                "-- ============================================================================\n\
                 -- Workflow: {}\n\
                 -- Workflow ID: {}\n\
                 -- Source: {}\n\
                 -- Extracted: {}\n\
                 -- ============================================================================\n",
                workflow_name, workflow_id, source, today
            );

            let full_path = sql_dir.join(&filename);
            write_text(&full_path.to_string_lossy(), &format!("{}{}", header, sql))?;
            count += 1;
        }
    }

    Ok(count)
}

/// Calc fields: enrich data model definitions with ruleField
fn extract_calc_fields_internal(global_path: &str) -> CmdResult<usize> {
    let input = format!("{}/schema/all_calculated_fields.json", global_path);
    let data = match read_json(&input) {
        Ok(d) => d,
        Err(_) => return Ok(0), // No calc fields file — skip
    };

    let items = extract_array(&data, "data");

    // Build map: table_id -> Vec<(db_column_name, ruleField)>
    let mut calc_fields_by_table: HashMap<String, Vec<(String, Value)>> = HashMap::new();

    for item in &items {
        let table_id = match item.get("id").and_then(|i| i.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let rule_field = match item
            .get("settings")
            .and_then(|s| s.get("ruleField"))
        {
            Some(rf) => rf.clone(),
            None => continue,
        };

        let db_column_name = match rule_field
            .get("rules")
            .and_then(|r| r.get("db_column_name"))
            .and_then(|n| n.as_str())
        {
            Some(name) => name.to_string(),
            None => continue,
        };

        calc_fields_by_table
            .entry(table_id)
            .or_default()
            .push((db_column_name, rule_field));
    }

    let data_models_dir = format!("{}/data_models", global_path);
    let dm_path = Path::new(&data_models_dir);
    if !dm_path.exists() {
        return Ok(0);
    }

    let mut enriched_count = 0;

    let entries = fs::read_dir(dm_path)?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let folder_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Extract table ID from folder name (table_{id})
        let table_id = match folder_name.strip_prefix("table_") {
            Some(id) => id.to_string(),
            None => continue,
        };

        let calc_fields = match calc_fields_by_table.get(&table_id) {
            Some(cf) => cf,
            None => continue,
        };

        let def_path = entry_path.join("definition.json");
        if !def_path.exists() {
            continue;
        }

        let def_str = def_path.to_string_lossy().to_string();
        let mut definition = match read_json(&def_str) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Build lookup: db_column_name -> ruleField
        let cf_map: HashMap<&str, &Value> = calc_fields
            .iter()
            .map(|(name, rf)| (name.as_str(), rf))
            .collect();

        // Enrich: match field.column_name to db_column_name
        let mut modified = false;
        if let Some(fields) = definition.as_array_mut() {
            for field in fields.iter_mut() {
                if let Some(col_name) = field.get("column_name").and_then(|c| c.as_str()) {
                    if let Some(rule_field) = cf_map.get(col_name) {
                        field
                            .as_object_mut()
                            .map(|obj| obj.insert("ruleField".to_string(), (*rule_field).clone()));
                        modified = true;
                        enriched_count += 1;
                    }
                }
            }
        }

        if modified {
            write_json(&def_str, &definition)?;
        }
    }

    Ok(enriched_count)
}

// ============================================================================
// Public API (called by sync.rs)
// ============================================================================

pub async fn run_extract(domain: &str, extract_type: &str) -> CmdResult<ExtractResult> {
    let start = Instant::now();
    let domain_config = get_domain_config(domain)?;
    let global_path = &domain_config.global_path;

    let count = match extract_type {
        "queries" => extract_queries_internal(global_path)?,
        "workflows" => extract_workflows_internal(global_path)?,
        "dashboards" => extract_dashboards_internal(global_path)?,
        "tables" => extract_tables_internal(domain, global_path).await?,
        "sql" => extract_sql_internal(global_path)?,
        "calc-fields" => extract_calc_fields_internal(global_path)?,
        _ => return Err(CommandError::Internal(format!("Unknown extract type: {}", extract_type))),
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    metadata::update_extraction_sync(global_path, domain, extract_type, count, "ok", duration_ms).await;

    Ok(ExtractResult {
        domain: domain.to_string(),
        extract_type: extract_type.to_string(),
        count,
        duration_ms,
        status: "ok".to_string(),
        message: format!("Extracted {} {} items", count, extract_type),
    })
}

// ============================================================================
// Commands
// ============================================================================


pub async fn val_extract_queries(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "queries").await
}


pub async fn val_extract_workflows(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "workflows").await
}


pub async fn val_extract_dashboards(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "dashboards").await
}


pub async fn val_extract_tables(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "tables").await
}


pub async fn val_extract_sql(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "sql").await
}


pub async fn val_extract_calc_fields(domain: String) -> CmdResult<ExtractResult> {
    run_extract(&domain, "calc-fields").await
}
