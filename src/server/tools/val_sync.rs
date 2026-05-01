// VAL Sync MCP Tools
// 16 tools for syncing VAL platform data via Claude Code

use crate::modules::val_sync::{config, drive, errors, extract, metadata, monitoring, sql, sync};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use chrono::Timelike;
use serde_json::{json, Value};

macro_rules! require_domain {
    ($args:expr) => {
        match $args.get("domain").and_then(|d| d.as_str()) {
            Some(d) => d.to_string(),
            None => return ToolResult::error("'domain' parameter is required".to_string()),
        }
    };
}

// ============================================================================
// Tool Definitions
// ============================================================================

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "sync-val-list-domains".to_string(),
            description: "List all configured VAL domains with their global paths. Use this to see which domains are available for syncing.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "sync-val-fields".to_string(),
            description: "Sync field definitions from a VAL domain. Downloads all field metadata to schema/all_fields.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-queries".to_string(),
            description: "Sync query definitions from a VAL domain. Downloads all queries to schema/all_queries.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-workflows".to_string(),
            description: "Sync workflow definitions from a VAL domain. Downloads all workflows to schema/all_workflows.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-dashboards".to_string(),
            description: "Sync dashboard definitions from a VAL domain. Downloads all dashboards to schema/all_dashboards.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-tables".to_string(),
            description: "Sync table/data model definitions from a VAL domain. Downloads the admin tree to schema/all_tables.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-calc-fields".to_string(),
            description: "Sync calculated field definitions from a VAL domain. Downloads to schema/all_calculated_fields.json.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-all".to_string(),
            description: "Full sync + extract for a VAL domain. Runs all 6 sync operations followed by all 6 extract operations. This is the recommended way to fully sync a domain.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-extract".to_string(),
            description: "Run extract operations on already-synced data. Extracts individual definitions from aggregated JSON files. Types: queries, workflows, dashboards, tables, sql, calc-fields. Omit type to run all extracts.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    },
                    "type": {
                        "type": "string",
                        "description": "Extract type: queries, workflows, dashboards, tables, sql, calc-fields. Omit to run all.",
                        "enum": ["queries", "workflows", "dashboards", "tables", "sql", "calc-fields"]
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "sync-val-status".to_string(),
            description: "Get sync status and metadata for a domain. Shows last sync times, item counts, and recent history.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name. If omitted, shows status for all domains."
                    }
                }),
                vec![],
            ),
        },
        Tool {
            name: "sync-all-domain-workflows".to_string(),
            description: "Sync workflow definitions for ALL production domains. Downloads workflow metadata for each domain. Takes time to complete.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "sync-all-domain-monitoring".to_string(),
            description: "Sync workflow execution/monitoring data for ALL production domains. Fetches recent workflow execution history (11pm yesterday to now) from VAL API. Takes a few minutes to complete.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "sync-all-domain-importers".to_string(),
            description: "Sync custom importer error logs for ALL production domains. Fetches from centralized tv domain and saves to each domain's analytics folder. Takes time to complete.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "sync-all-domain-integration-errors".to_string(),
            description: "Sync integration/API error logs (POS, bank, delivery platforms) for ALL production domains. Fetches from centralized tv domain and saves to each domain's analytics folder. Takes time to complete.".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "sync-all-domain-sod-tables".to_string(),
            description: "Sync SOD (Start of Day) table calculation status for eligible domains (dapaolo, saladstop, spaespritgroup, grain). Shows completed/incomplete/errored tables.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "date": {
                        "type": "string",
                        "description": "Date in YYYY-MM-DD format (defaults to today SGT)"
                    }
                }),
                vec![],
            ),
        },
        Tool {
            name: "execute-val-sql".to_string(),
            description: "Execute a SQL query on a VAL domain. Provide SQL directly or as a file path. Returns summary and data. Only SELECT queries are allowed.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'suntec', 'koi', 'tryval', 'jfh')"
                    },
                    "sql": {
                        "type": "string",
                        "description": "SQL query (SELECT only) OR path to a .sql file"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Max rows to return (default: 1000)"
                    }
                }),
                vec!["domain".to_string(), "sql".to_string()],
            ),
        },
        Tool {
            name: "execute-supabase-sql".to_string(),
            description: "Execute a read-only SQL query on the Supabase database. Only SELECT queries are allowed. Use this for querying Supabase tables (val_health_issues, val_health_summary, val_sync_runs, skills, tasks, projects, etc.) or calling Supabase functions like score_production_health().".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "sql": {
                        "type": "string",
                        "description": "SQL SELECT query to execute against Supabase"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Max rows to return (default: 100)"
                    }
                }),
                vec!["sql".to_string()],
            ),
        },
    ]
}

// ============================================================================
// Tool Dispatch
// ============================================================================

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "sync-val-list-domains" => {
            match config::val_sync_list_domains() {
                Ok(domains) => {
                    if domains.is_empty() {
                        return ToolResult::text(
                            "No domains configured. Import config with val_sync_import_config or add domains manually.".to_string(),
                        );
                    }
                    let list: Vec<String> = domains
                        .iter()
                        .map(|d| {
                            let alias = if d.has_actual_domain {
                                " (has API alias)"
                            } else {
                                ""
                            };
                            format!("- **{}**{}: `{}`", d.domain, alias, d.global_path)
                        })
                        .collect();
                    ToolResult::text(format!(
                        "## VAL Domains ({} configured)\n\n{}",
                        domains.len(),
                        list.join("\n")
                    ))
                }
                Err(e) => ToolResult::error(format!("Failed to list domains: {}", e)),
            }
        }

        "sync-val-fields" => {
            let domain = require_domain!(args);
            match sync::val_sync_fields(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync fields failed: {}", e)),
            }
        }

        "sync-val-queries" => {
            let domain = require_domain!(args);
            match sync::val_sync_queries(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync queries failed: {}", e)),
            }
        }

        "sync-val-workflows" => {
            let domain = require_domain!(args);
            match sync::val_sync_workflows(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync workflows failed: {}", e)),
            }
        }

        "sync-val-dashboards" => {
            let domain = require_domain!(args);
            match sync::val_sync_dashboards(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync dashboards failed: {}", e)),
            }
        }

        "sync-val-tables" => {
            let domain = require_domain!(args);
            match sync::val_sync_tables(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync tables failed: {}", e)),
            }
        }

        "sync-val-calc-fields" => {
            let domain = require_domain!(args);
            match sync::val_sync_calc_fields(domain.clone()).await {
                Ok(r) => ToolResult::text(format_sync_result(&r)),
                Err(e) => ToolResult::error(format!("Sync calc fields failed: {}", e)),
            }
        }

        "sync-val-all" => {
            let domain = require_domain!(args);
            match sync::val_sync_all(domain.clone()).await {
                Ok(r) => {
                    let mut lines = vec![format!(
                        "## Full Sync: {} ({})\n",
                        r.domain, r.status
                    )];

                    lines.push("### Sync Results".to_string());
                    for sr in &r.results {
                        lines.push(format!(
                            "- **{}**: {} items ({}ms) [{}]",
                            sr.artifact_type, sr.count, sr.duration_ms, sr.status
                        ));
                    }

                    lines.push("\n### Extract Results".to_string());
                    for er in &r.extract_results {
                        lines.push(format!(
                            "- **{}**: {} items ({}ms) [{}]",
                            er.extract_type, er.count, er.duration_ms, er.status
                        ));
                    }

                    lines.push(format!("\n**Total: {}ms**", r.total_duration_ms));
                    ToolResult::text(lines.join("\n"))
                }
                Err(e) => ToolResult::error(format!("Full sync failed: {}", e)),
            }
        }

        "sync-val-extract" => {
            let domain = require_domain!(args);
            let extract_type = args.get("type").and_then(|t| t.as_str());

            match extract_type {
                Some(t) => {
                    match extract::run_extract(&domain, t).await {
                        Ok(r) => ToolResult::text(format!(
                            "Extracted **{}** {}: {} items in {}ms",
                            r.domain, r.extract_type, r.count, r.duration_ms
                        )),
                        Err(e) => ToolResult::error(format!("Extract {} failed: {}", t, e)),
                    }
                }
                None => {
                    // Run all extracts
                    let types = ["queries", "workflows", "dashboards", "tables", "sql", "calc-fields"];
                    let mut lines = vec![format!("## Extract All: {}\n", domain)];

                    for t in &types {
                        match extract::run_extract(&domain, t).await {
                            Ok(r) => lines.push(format!(
                                "- **{}**: {} items ({}ms) [{}]",
                                r.extract_type, r.count, r.duration_ms, r.status
                            )),
                            Err(e) => lines.push(format!("- **{}**: ERROR - {}", t, e)),
                        }
                    }

                    ToolResult::text(lines.join("\n"))
                }
            }
        }

        "sync-val-status" => {
            let domain = args.get("domain").and_then(|d| d.as_str());

            match domain {
                Some(d) => {
                    match metadata::val_sync_get_status(d.to_string()).await {
                        Ok(meta) => ToolResult::json(&meta),
                        Err(e) => ToolResult::error(format!("Failed to get status: {}", e)),
                    }
                }
                None => {
                    // Show status for all domains
                    match config::val_sync_list_domains() {
                        Ok(domains) => {
                            let mut lines = vec!["## Sync Status (All Domains)\n".to_string()];
                            for d in &domains {
                                match metadata::val_sync_get_status(d.domain.clone()).await {
                                    Ok(meta) => {
                                        let artifact_count = meta.artifacts.len();
                                        let last_sync = meta
                                            .artifacts
                                            .values()
                                            .map(|a| a.last_sync.as_str())
                                            .max()
                                            .unwrap_or("never");
                                        lines.push(format!(
                                            "- **{}**: {} artifact types synced, last: {}",
                                            d.domain, artifact_count, last_sync
                                        ));
                                    }
                                    Err(_) => {
                                        lines.push(format!("- **{}**: no sync data", d.domain));
                                    }
                                }
                            }
                            ToolResult::text(lines.join("\n"))
                        }
                        Err(e) => ToolResult::error(format!("Failed to list domains: {}", e)),
                    }
                }
            }
        }

        "execute-val-sql" => {
            let domain = require_domain!(args);
            let sql_query = match args.get("sql").and_then(|s| s.as_str()) {
                Some(s) => s.to_string(),
                None => return ToolResult::error("'sql' parameter is required".to_string()),
            };
            let limit = args.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize);

            match sql::val_execute_sql(domain.clone(), sql_query, limit).await {
                Ok(result) => {
                    if let Some(err) = &result.error {
                        return ToolResult::error(format!("SQL error: {}", err));
                    }

                    let mut lines = vec![
                        format!("## SQL Results: {} ({})", domain, result.row_count),
                        format!("Columns: {}", result.columns.join(", ")),
                        String::new(),
                    ];

                    if result.truncated {
                        lines.push(format!("*Results truncated to {} rows*\n", result.data.len()));
                    }

                    // Format data as markdown table
                    if !result.data.is_empty() && !result.columns.is_empty() {
                        // Header
                        lines.push(format!("| {} |", result.columns.join(" | ")));
                        lines.push(format!("| {} |", result.columns.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));

                        // Rows (max 500 for display)
                        for row in result.data.iter().take(500) {
                            let cells: Vec<String> = result.columns.iter().map(|col| {
                                row.get(col)
                                    .map(|v| {
                                        if v.is_null() {
                                            "NULL".to_string()
                                        } else if let Some(s) = v.as_str() {
                                            s.to_string()
                                        } else {
                                            v.to_string()
                                        }
                                    })
                                    .unwrap_or_default()
                            }).collect();
                            lines.push(format!("| {} |", cells.join(" | ")));
                        }

                        if result.data.len() > 500 {
                            lines.push(format!("\n*...and {} more rows*", result.data.len() - 500));
                        }
                    } else {
                        lines.push("No rows returned.".to_string());
                    }

                    ToolResult::text(lines.join("\n"))
                }
                Err(e) => ToolResult::error(format!("SQL execution failed: {}", e)),
            }
        }

        "execute-supabase-sql" => {
            let sql_query = match args.get("sql").and_then(|s| s.as_str()) {
                Some(s) => s.to_string(),
                None => return ToolResult::error("'sql' parameter is required".to_string()),
            };
            let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(100) as usize;

            // Validate: must start with SELECT
            let normalized = sql_query.trim().to_uppercase();
            if !normalized.starts_with("SELECT") {
                return ToolResult::error("Only SELECT queries are allowed".to_string());
            }

            // Block mutation keywords
            let dangerous = ["INSERT", "UPDATE", "DELETE", "DROP", "TRUNCATE", "ALTER", "CREATE", "GRANT", "REVOKE"];
            for kw in &dangerous {
                let pattern = format!(r"\b{}\b", kw);
                if regex::Regex::new(&pattern).map(|r| r.is_match(&normalized)).unwrap_or(false) {
                    return ToolResult::error(format!("Keyword '{}' is not allowed", kw));
                }
            }

            let client = match crate::core::supabase::get_client().await {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("Failed to get Supabase client: {}", e)),
            };

            let result: Result<serde_json::Value, _> = client.rpc(
                "execute_readonly_sql",
                &json!({ "query": sql_query }),
            ).await;

            match result {
                Ok(data) => {
                    let rows = match data.as_array() {
                        Some(arr) => arr.clone(),
                        None => vec![data],
                    };

                    let total = rows.len();
                    let limited: Vec<&serde_json::Value> = rows.iter().take(limit).collect();
                    let columns: Vec<String> = if let Some(first) = limited.first() {
                        first.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
                    } else {
                        vec![]
                    };

                    let mut lines = vec![
                        format!("## Supabase SQL Results ({}{})", total, if total > limit { " rows, truncated" } else { " rows" }),
                        String::new(),
                    ];

                    if !limited.is_empty() && !columns.is_empty() {
                        lines.push(format!("| {} |", columns.join(" | ")));
                        lines.push(format!("| {} |", columns.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));

                        for row in &limited {
                            let cells: Vec<String> = columns.iter().map(|col| {
                                row.get(col)
                                    .map(|v| {
                                        if v.is_null() {
                                            "NULL".to_string()
                                        } else if let Some(s) = v.as_str() {
                                            s.to_string()
                                        } else {
                                            v.to_string()
                                        }
                                    })
                                    .unwrap_or_default()
                            }).collect();
                            lines.push(format!("| {} |", cells.join(" | ")));
                        }

                        if total > limit {
                            lines.push(format!("\n*...and {} more rows*", total - limit));
                        }
                    } else {
                        lines.push("No rows returned.".to_string());
                    }

                    ToolResult::text(lines.join("\n"))
                }
                Err(e) => ToolResult::error(format!("Supabase SQL error: {}", e)),
            }
        }

        "sync-all-domain-workflows" => {
            handle_sync_all_domain_workflows().await
        }

        "sync-all-domain-monitoring" => {
            handle_sync_all_domain_monitoring().await
        }

        "sync-all-domain-importers" => {
            handle_sync_all_domain_errors("importer").await
        }

        "sync-all-domain-integration-errors" => {
            handle_sync_all_domain_errors("integration").await
        }

        "sync-all-domain-sod-tables" => {
            let date = args
                .get("date")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    chrono::Utc::now()
                        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                        .format("%Y-%m-%d")
                        .to_string()
                });
            handle_sync_all_domain_sod_tables(&date).await
        }

        _ => ToolResult::error(format!("Unknown val-sync tool: {}", name)),
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn format_sync_result(r: &sync::SyncResult) -> String {
    format!(
        "Synced **{}** {}: {} items in {}ms\nFile: `{}`",
        r.domain, r.artifact_type, r.count, r.duration_ms, r.file_path
    )
}

/// Get default date range for error queries: 11pm yesterday to now (SGT)
fn get_default_error_date_range() -> (String, String) {
    let sgt = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now = chrono::Utc::now().with_timezone(&sgt);
    let yesterday_11pm = now - chrono::Duration::hours(now.hour() as i64 + 1);
    let yesterday_11pm = yesterday_11pm
        .date_naive()
        .and_hms_opt(23, 0, 0)
        .unwrap();
    let from = yesterday_11pm.format("%Y-%m-%d %H:%M:%S").to_string();
    let to = now.format("%Y-%m-%d %H:%M:%S").to_string();
    (from, to)
}

/// Filter config to production domains (exclude documentation, lab, templates)
fn get_production_domains() -> Result<Vec<config::DomainSummary>, crate::core::error::CommandError> {
    let domains = config::val_sync_list_domains()?;
    let excluded = ["documentation", "lab", "templates"];
    Ok(domains
        .into_iter()
        .filter(|d| !excluded.contains(&d.domain.to_lowercase().as_str()))
        .collect())
}

/// Sync importer or integration errors across all production domains
/// Sync workflow definitions across all production domains
async fn handle_sync_all_domain_workflows() -> ToolResult {
    let domains = match get_production_domains() {
        Ok(d) => d,
        Err(e) => return ToolResult::error(format!("Failed to list domains: {}", e)),
    };

    if domains.is_empty() {
        return ToolResult::error("No production domains found in config".to_string());
    }

    let mut results = Vec::new();
    let mut success_count = 0u32;
    let mut failed_count = 0u32;
    let mut total_workflows = 0usize;

    for d in &domains {
        match sync::val_sync_workflows(d.domain.clone()).await {
            Ok(r) => {
                success_count += 1;
                total_workflows += r.count;
                results.push(format!("{}: {} workflows", d.domain, r.count));
            }
            Err(e) => {
                failed_count += 1;
                let err_msg = e.to_string();
                let short_err = if err_msg.len() > 100 { &err_msg[..100] } else { &err_msg };
                results.push(format!("{}: FAILED - {}", d.domain, short_err));
            }
        }
    }

    let status = if failed_count == 0 {
        "All domains synced successfully"
    } else {
        "Completed with errors"
    };

    let mut lines = vec![
        "## Sync All Domain Workflows".to_string(),
        String::new(),
        format!("**Status:** {}", status),
        format!("**Domains processed:** {}", domains.len()),
        format!("**Successful:** {}", success_count),
    ];
    if failed_count > 0 {
        lines.push(format!("**Failed:** {}", failed_count));
    }
    lines.push(format!("**Total workflows:** {}", total_workflows));
    lines.push(String::new());
    lines.push("**Results:**".to_string());
    for r in &results {
        lines.push(format!("- {}", r));
    }

    ToolResult::text(lines.join("\n"))
}

/// Sync workflow execution monitoring data across all production domains
/// Default window: 11pm yesterday to now (SGT)
async fn handle_sync_all_domain_monitoring() -> ToolResult {
    let domains = match get_production_domains() {
        Ok(d) => d,
        Err(e) => return ToolResult::error(format!("Failed to list domains: {}", e)),
    };

    if domains.is_empty() {
        return ToolResult::error("No production domains found in config".to_string());
    }

    // Default window: 11pm yesterday to now (SGT)
    let (from, to) = get_default_error_date_range();

    let mut results = Vec::new();
    let mut success_count = 0u32;
    let mut failed_count = 0u32;
    let mut total_executions = 0usize;

    for d in &domains {
        match monitoring::val_sync_workflow_executions(
            d.domain.clone(),
            from.clone(),
            to.clone(),
        )
        .await
        {
            Ok(r) => {
                success_count += 1;
                total_executions += r.count;
                results.push(format!("{}: {} executions", d.domain, r.count));
            }
            Err(e) => {
                failed_count += 1;
                let err_msg = e.to_string();
                let short_err = if err_msg.len() > 100 { &err_msg[..100] } else { &err_msg };
                results.push(format!("{}: FAILED - {}", d.domain, short_err));
            }
        }
    }

    let status = if failed_count == 0 {
        "All domains synced successfully"
    } else {
        "Completed with errors"
    };

    let mut lines = vec![
        "## Sync All Domain Monitoring".to_string(),
        String::new(),
        format!("**Status:** {}", status),
        format!("**Window:** {} to {}", from, to),
        format!("**Domains processed:** {}", domains.len()),
        format!("**Successful:** {}", success_count),
    ];
    if failed_count > 0 {
        lines.push(format!("**Failed:** {}", failed_count));
    }
    lines.push(format!("**Total executions:** {}", total_executions));
    lines.push(String::new());
    lines.push("**Results:**".to_string());
    for r in &results {
        lines.push(format!("- {}", r));
    }

    ToolResult::text(lines.join("\n"))
}

async fn handle_sync_all_domain_errors(error_type: &str) -> ToolResult {
    let domains = match get_production_domains() {
        Ok(d) => d,
        Err(e) => return ToolResult::error(format!("Failed to list domains: {}", e)),
    };

    if domains.is_empty() {
        return ToolResult::error("No production domains found in config".to_string());
    }

    let (from, to) = get_default_error_date_range();
    let mut results = Vec::new();
    let mut success_count = 0u32;
    let mut failed_count = 0u32;
    let mut total_errors = 0usize;

    for d in &domains {
        let result = if error_type == "importer" {
            errors::val_sync_importer_errors(d.domain.clone(), from.clone(), to.clone()).await
        } else {
            errors::val_sync_integration_errors(d.domain.clone(), from.clone(), to.clone()).await
        };

        match result {
            Ok(r) => {
                success_count += 1;
                total_errors += r.count;
                results.push(format!("{}: {} errors", d.domain, r.count));
            }
            Err(e) => {
                failed_count += 1;
                let err_msg = e.to_string();
                let short_err = if err_msg.len() > 100 { &err_msg[..100] } else { &err_msg };
                results.push(format!("{}: FAILED - {}", d.domain, short_err));
            }
        }
    }

    let status = if failed_count == 0 {
        "All domains synced successfully"
    } else {
        "Completed with errors"
    };

    let label = if error_type == "importer" {
        "Importer Errors"
    } else {
        "Integration Errors"
    };

    let mut lines = vec![
        format!("## Sync All Domain {}", label),
        String::new(),
        format!("**Status:** {}", status),
        format!("**Domains processed:** {}", domains.len()),
        format!("**Successful:** {}", success_count),
    ];
    if failed_count > 0 {
        lines.push(format!("**Failed:** {}", failed_count));
    }
    lines.push(format!("**Total errors found:** {}", total_errors));
    lines.push(String::new());
    lines.push("**Results:**".to_string());
    for r in &results {
        lines.push(format!("- {}", r));
    }

    ToolResult::text(lines.join("\n"))
}

/// Sync SOD table status across eligible domains
async fn handle_sync_all_domain_sod_tables(date: &str) -> ToolResult {
    const SOD_ELIGIBLE: &[&str] = &["dapaolo", "saladstop", "spaespritgroup", "grain"];

    let all_domains = match config::val_sync_list_domains() {
        Ok(d) => d,
        Err(e) => return ToolResult::error(format!("Failed to list domains: {}", e)),
    };

    let domains: Vec<_> = all_domains
        .into_iter()
        .filter(|d| SOD_ELIGIBLE.contains(&d.domain.to_lowercase().as_str()))
        .collect();

    if domains.is_empty() {
        return ToolResult::error(format!(
            "No SOD-eligible domains found in config. SOD tables only apply to: {}",
            SOD_ELIGIBLE.join(", ")
        ));
    }

    let mut results = Vec::new();
    let mut issue_results = Vec::new();
    let mut _success_count = 0u32;
    let mut failed_count = 0u32;
    let mut total_tables = 0usize;
    let mut total_completed = 0usize;
    let mut total_started = 0usize;
    let mut total_errored = 0usize;

    for d in &domains {
        match monitoring::val_sync_sod_tables_status(d.domain.clone(), date.to_string(), false)
            .await
        {
            Ok(r) => {
                _success_count += 1;

                // Read the output file to parse status breakdown
                let status_counts = parse_sod_status_from_file(&r.file_path);
                let completed = status_counts.get("completed").copied().unwrap_or(0);
                let started = status_counts.get("started").copied().unwrap_or(0);
                let errored = status_counts.get("errored").copied().unwrap_or(0);
                let table_count = r.count;

                total_tables += table_count;
                total_completed += completed;
                total_started += started;
                total_errored += errored;

                if started > 0 || errored > 0 {
                    let mut issues = Vec::new();
                    if started > 0 {
                        issues.push(format!("{} incomplete", started));
                    }
                    if errored > 0 {
                        issues.push(format!("{} errored", errored));
                    }
                    issue_results.push(format!(
                        "**{}**: {} ({}/{} completed)",
                        d.domain,
                        issues.join(", "),
                        completed,
                        table_count
                    ));
                }
                results.push(format!(
                    "{}: {}/{} completed{}{}",
                    d.domain,
                    completed,
                    table_count,
                    if started > 0 {
                        format!(", {} incomplete", started)
                    } else {
                        String::new()
                    },
                    if errored > 0 {
                        format!(", {} errored", errored)
                    } else {
                        String::new()
                    }
                ));
            }
            Err(e) => {
                failed_count += 1;
                let err_msg = e.to_string();
                let short_err = if err_msg.len() > 100 { &err_msg[..100] } else { &err_msg };
                results.push(format!("{}: FAILED - {}", d.domain, short_err));
            }
        }
    }

    let has_issues = total_started > 0 || total_errored > 0 || failed_count > 0;
    let status = if !has_issues {
        "All SOD calculations completed"
    } else if total_started > 0 {
        "Some SOD calculations still running/incomplete"
    } else if total_errored > 0 {
        "Some SOD calculations errored"
    } else {
        "Some syncs failed"
    };

    let mut lines = vec![
        "## Sync All Domain SOD Tables".to_string(),
        String::new(),
        format!("**Status:** {}", status),
        format!("**Date:** {}", date),
        format!("**Domains processed:** {}", domains.len()),
        String::new(),
        "### Summary".to_string(),
        format!("- **Completed:** {}", total_completed),
    ];
    if total_started > 0 {
        lines.push(format!("- **Incomplete (started):** {}", total_started));
    }
    if total_errored > 0 {
        lines.push(format!("- **Errored:** {}", total_errored));
    }
    lines.push(format!("- **Total tables:** {}", total_tables));
    lines.push(String::new());

    if !issue_results.is_empty() {
        lines.push("### Domains Needing Attention".to_string());
        for r in &issue_results {
            lines.push(format!("- {}", r));
        }
        lines.push(String::new());
    }

    lines.push("### All Domains".to_string());
    for r in &results {
        lines.push(format!("- {}", r));
    }

    ToolResult::text(lines.join("\n"))
}

/// Check Drive files across all production domains
/// Recursively scans val_drive folders and reports unprocessed files with age
pub async fn handle_check_all_domain_drive_files_public() -> ToolResult {
    handle_check_all_domain_drive_files().await
}

async fn handle_check_all_domain_drive_files() -> ToolResult {
    let domains = match get_production_domains() {
        Ok(d) => d,
        Err(e) => return ToolResult::error(format!("Failed to list domains: {}", e)),
    };

    if domains.is_empty() {
        return ToolResult::error("No production domains found in config".to_string());
    }

    // Load persisted scan config; fall back to workflow discovery if empty
    let scan_config = drive::load_scan_config();
    let has_persisted_config = !scan_config.domains.is_empty();
    let all_wf_folders = if has_persisted_config {
        std::collections::HashMap::new() // won't be used
    } else {
        drive::get_all_domain_workflow_folders()
    };

    let sgt = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now = chrono::Utc::now().with_timezone(&sgt);
    let stale_threshold = chrono::Duration::hours(24);

    let mut lines = vec![
        "## Drive Files Check — All Domains".to_string(),
        String::new(),
    ];

    let mut domains_with_issues: Vec<String> = Vec::new();
    let mut domains_clean: Vec<String> = Vec::new();
    let mut domains_failed: Vec<String> = Vec::new();
    let mut total_stale = 0usize;
    let mut total_unprocessed = 0usize;

    for d in &domains {
        // Build effective folder list from persisted config or workflow discovery
        let effective_folders: Vec<(String, bool)> = if has_persisted_config {
            if let Some(domain_config) = scan_config.domains.get(&d.domain) {
                domain_config
                    .folders
                    .iter()
                    .filter(|f| f.enabled)
                    .map(|f| (f.folder_path.clone(), f.move_to_processed))
                    .collect()
            } else {
                vec![]
            }
        } else {
            all_wf_folders
                .get(&d.domain)
                .map(|folders| {
                    folders
                        .iter()
                        .map(|f| (f.folder_path.clone(), f.move_to_processed))
                        .collect()
                })
                .unwrap_or_default()
        };

        // Helper: check if a folder path has moveFileToProcessedFolder=true
        let expects_processed = |folder_path: &str| -> bool {
            effective_folders
                .iter()
                .any(|(fp, mtp)| fp == folder_path && *mtp)
        };

        // Helper: check if a folder path is in the effective config at all
        let has_workflow = |folder_path: &str| -> bool {
            effective_folders.iter().any(|(fp, _)| fp == folder_path)
        };

        // List top-level folders in val_drive
        let top_folders = match drive::val_drive_list_folders(
            d.domain.clone(),
            Some("val_drive".to_string()),
        )
        .await
        {
            Ok(f) => f,
            Err(e) => {
                let err_msg = e.to_string();
                let short = if err_msg.len() > 80 { &err_msg[..80] } else { &err_msg };
                domains_failed.push(format!("{}: {}", d.domain, short));
                continue;
            }
        };

        // Also check if the files endpoint returns folder-like entries
        let top_files = drive::val_drive_list_files(
            d.domain.clone(),
            "val_drive".to_string(),
            Some(200),
        )
        .await
        .ok();

        // Merge folder sources: explicit folders + folder-like file entries (name ends with /)
        let mut folder_ids: Vec<(String, String)> = top_folders
            .iter()
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();

        if let Some(ref tf) = top_files {
            for f in &tf.files {
                if f.name.ends_with('/') {
                    let clean = f.name.trim_end_matches('/').to_string();
                    let fid = format!("val_drive/{}", clean);
                    if !folder_ids.iter().any(|(_, n)| n == &clean) {
                        folder_ids.push((fid, clean));
                    }
                }
            }
        }

        if folder_ids.is_empty() {
            domains_clean.push(d.domain.clone());
            continue;
        }

        // Scan each subfolder for files (1 level deep — source report folders)
        let mut domain_issues: Vec<String> = Vec::new();
        let mut domain_stale = 0usize;
        let mut domain_unprocessed = 0usize;

        for (folder_id, folder_name) in &folder_ids {
            // Skip hidden and test folders
            if folder_name.starts_with('.') || folder_name == "Test" {
                continue;
            }

            // List sub-folders (e.g., 01_SourceReports, 02_OutputReports)
            let sub_folders = match drive::val_drive_list_folders(
                d.domain.clone(),
                Some(folder_id.clone()),
            )
            .await
            {
                Ok(f) => f,
                Err(_) => continue,
            };

            // Also get folder-like entries from files
            let sub_files = drive::val_drive_list_files(
                d.domain.clone(),
                folder_id.clone(),
                Some(200),
            )
            .await
            .ok();

            let mut sub_ids: Vec<(String, String)> = sub_folders
                .iter()
                .map(|f| (f.id.clone(), f.name.clone()))
                .collect();

            if let Some(ref sf) = sub_files {
                for f in &sf.files {
                    if f.name.ends_with('/') {
                        let clean = f.name.trim_end_matches('/').to_string();
                        let fid = format!("{}/{}", folder_id, clean);
                        if !sub_ids.iter().any(|(_, n)| n == &clean) {
                            sub_ids.push((fid, clean));
                        }
                    }
                }
                // Check for actual files at this level too (not just subfolders)
                // Only flag if this folder has a workflow with moveFileToProcessedFolder=true
                let this_folder_path = folder_id.clone();
                if expects_processed(&this_folder_path) {
                    for f in &sf.files {
                        if !f.name.ends_with('/') && f.name.contains('.') {
                            domain_unprocessed += 1;
                            let age = file_age_str(&f.last_modified, &now);
                            let is_stale = is_file_stale(&f.last_modified, &now, &stale_threshold);
                            if is_stale {
                                domain_stale += 1;
                            }
                            domain_issues.push(format!(
                                "  {}/{}: **{}** ({}){}",
                                folder_name,
                                f.name,
                                format_file_size(f.size),
                                age,
                                if is_stale { " ⚠" } else { "" }
                            ));
                        }
                    }
                }
            }

            // Check each sub-folder for unprocessed files (skip "processed" and output folders)
            for (sub_id, sub_name) in &sub_ids {
                let sub_lower = sub_name.to_lowercase();
                if sub_lower == "processed" || sub_lower.contains("output") {
                    continue;
                }

                // Build the full folder path to check against workflow config
                let full_folder_path = format!("{}/{}", folder_id, sub_name);

                // Only flag files as unprocessed if this folder has a workflow
                // with moveFileToProcessedFolder=true
                if !expects_processed(&full_folder_path) && !expects_processed(folder_id) {
                    // No workflow expects files to move — check if there's any workflow at all
                    if has_workflow(&full_folder_path) || has_workflow(folder_id) {
                        // Workflow exists but doesn't move to processed — files here are normal
                        continue;
                    }
                    // No workflow at all for this folder — skip (not a monitored folder)
                    continue;
                }

                let files_result = match drive::val_drive_list_files(
                    d.domain.clone(),
                    sub_id.clone(),
                    Some(200),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                // Filter to actual files (not folder-like entries)
                let actual_files: Vec<_> = files_result
                    .files
                    .iter()
                    .filter(|f| !f.name.ends_with('/') && f.name.contains('.'))
                    .collect();

                for f in &actual_files {
                    domain_unprocessed += 1;
                    let age = file_age_str(&f.last_modified, &now);
                    let is_stale = is_file_stale(&f.last_modified, &now, &stale_threshold);
                    if is_stale {
                        domain_stale += 1;
                    }
                    domain_issues.push(format!(
                        "  {}/{}/{}: **{}** ({}){}",
                        folder_name,
                        sub_name,
                        f.name,
                        format_file_size(f.size),
                        age,
                        if is_stale { " ⚠" } else { "" }
                    ));
                }
            }
        }

        if domain_issues.is_empty() {
            domains_clean.push(d.domain.clone());
        } else {
            total_stale += domain_stale;
            total_unprocessed += domain_unprocessed;
            domains_with_issues.push(format!(
                "### {} — {} unprocessed{}\n{}",
                d.domain,
                domain_unprocessed,
                if domain_stale > 0 {
                    format!(" ({} stale >24h)", domain_stale)
                } else {
                    String::new()
                },
                domain_issues.join("\n")
            ));
        }
    }

    // Build summary
    let has_issues = !domains_with_issues.is_empty() || !domains_failed.is_empty();
    let status = if !has_issues {
        "All domains clean — no unprocessed Drive files"
    } else if total_stale > 0 {
        "Stale files detected (>24h unprocessed)"
    } else {
        "Unprocessed files found"
    };

    lines.push(format!("**Status:** {}", status));
    lines.push(format!("**Checked at:** {}", now.format("%Y-%m-%d %H:%M SGT")));
    lines.push(format!(
        "**Domains:** {} checked, {} with files, {} clean, {} failed",
        domains.len(),
        domains_with_issues.len(),
        domains_clean.len(),
        domains_failed.len()
    ));

    if total_unprocessed > 0 {
        lines.push(format!(
            "**Total unprocessed:** {} files ({} stale >24h)",
            total_unprocessed, total_stale
        ));
    }
    lines.push(String::new());

    // Domains with issues
    if !domains_with_issues.is_empty() {
        for section in &domains_with_issues {
            lines.push(section.clone());
            lines.push(String::new());
        }
    }

    // Failed domains
    if !domains_failed.is_empty() {
        lines.push("### Failed to Check".to_string());
        for f in &domains_failed {
            lines.push(format!("- {}", f));
        }
        lines.push(String::new());
    }

    // Clean domains
    if !domains_clean.is_empty() {
        lines.push(format!(
            "### Clean Domains ({})",
            domains_clean.len()
        ));
        lines.push(
            domains_clean
                .iter()
                .map(|d| format!("**{}** ✓", d))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    ToolResult::text(lines.join("\n"))
}

/// Helper: format file size for MCP output
fn format_file_size(size: Option<u64>) -> String {
    match size {
        Some(s) if s < 1024 => format!("{} B", s),
        Some(s) if s < 1024 * 1024 => format!("{} KB", s / 1024),
        Some(s) => format!("{:.1} MB", s as f64 / (1024.0 * 1024.0)),
        None => "? B".to_string(),
    }
}

/// Helper: get relative age string from optional ISO timestamp
fn file_age_str(
    last_modified: &Option<String>,
    now: &chrono::DateTime<chrono::FixedOffset>,
) -> String {
    match last_modified.as_deref() {
        Some(lm) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(lm)
                .or_else(|_| chrono::DateTime::parse_from_str(lm, "%Y-%m-%dT%H:%M:%S%.fZ"))
                .or_else(|_| chrono::DateTime::parse_from_str(lm, "%Y-%m-%d %H:%M:%S%:z"));
            match parsed {
                Ok(dt) => {
                    let mins = (*now - dt).num_minutes();
                    if mins < 1 {
                        "just now".to_string()
                    } else if mins < 60 {
                        format!("{}m ago", mins)
                    } else if mins < 24 * 60 {
                        format!("{}h ago", mins / 60)
                    } else {
                        format!("{}d ago", mins / (24 * 60))
                    }
                }
                Err(_) => lm.to_string(),
            }
        }
        None => "unknown age".to_string(),
    }
}

/// Helper: check if file is older than threshold
fn is_file_stale(
    last_modified: &Option<String>,
    now: &chrono::DateTime<chrono::FixedOffset>,
    threshold: &chrono::Duration,
) -> bool {
    match last_modified.as_deref() {
        Some(lm) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(lm)
                .or_else(|_| chrono::DateTime::parse_from_str(lm, "%Y-%m-%dT%H:%M:%S%.fZ"))
                .or_else(|_| chrono::DateTime::parse_from_str(lm, "%Y-%m-%d %H:%M:%S%:z"));
            match parsed {
                Ok(dt) => (*now - dt) > *threshold,
                Err(_) => false,
            }
        }
        None => false,
    }
}

/// List Drive files and folders for a domain
pub async fn handle_list_drive_files_public(domain: &str, folder: &str) -> ToolResult {
    handle_list_drive_files(domain, folder).await
}

async fn handle_list_drive_files(domain: &str, folder: &str) -> ToolResult {
    let mut lines = vec![format!("## Drive: {} / {}\n", domain, folder)];

    // List folders
    match drive::val_drive_list_folders(domain.to_string(), Some(folder.to_string())).await {
        Ok(folders) => {
            if !folders.is_empty() {
                lines.push(format!("### Folders ({})", folders.len()));
                for f in &folders {
                    lines.push(format!("- **{}**/", f.name));
                }
                lines.push(String::new());
            }
        }
        Err(e) => {
            lines.push(format!("*Folders error: {}*\n", e));
        }
    }

    // List files
    match drive::val_drive_list_files(domain.to_string(), folder.to_string(), Some(200)).await {
        Ok(result) => {
            if result.files.is_empty() {
                lines.push("No files in this folder.".to_string());
            } else {
                lines.push(format!("### Files ({})", result.files.len()));
                lines.push("| Name | Size | Age |".to_string());
                lines.push("| --- | --- | --- |".to_string());

                for file in &result.files {
                    let size = file
                        .size
                        .map(|s| {
                            if s < 1024 {
                                format!("{} B", s)
                            } else if s < 1024 * 1024 {
                                format!("{} KB", s / 1024)
                            } else {
                                format!("{:.1} MB", s as f64 / (1024.0 * 1024.0))
                            }
                        })
                        .unwrap_or_else(|| "—".to_string());

                    let age = file
                        .last_modified
                        .as_deref()
                        .map(|lm| format_age_from_iso(lm))
                        .unwrap_or_else(|| "—".to_string());

                    lines.push(format!("| {} | {} | {} |", file.name, size, age));
                }

                if !result.is_last_page {
                    lines.push(format!(
                        "\n*More files available (showing first {})*",
                        result.files.len()
                    ));
                }
            }
        }
        Err(e) => {
            lines.push(format!("*Files error: {}*", e));
        }
    }

    ToolResult::text(lines.join("\n"))
}

/// Format ISO timestamp as relative age string
fn format_age_from_iso(iso: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(iso)
        .or_else(|_| chrono::DateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .or_else(|_| chrono::DateTime::parse_from_str(iso, "%Y-%m-%d %H:%M:%S%:z"));

    match parsed {
        Ok(dt) => {
            let now = chrono::Utc::now();
            let diff = now.signed_duration_since(dt);
            let mins = diff.num_minutes();
            if mins < 1 {
                "just now".to_string()
            } else if mins < 60 {
                format!("{}m ago", mins)
            } else if mins < 24 * 60 {
                format!("{}h ago", mins / 60)
            } else {
                let days = mins / (24 * 60);
                if days > 1 {
                    format!("{}d ago ⚠", days)
                } else {
                    format!("{}d ago", days)
                }
            }
        }
        Err(_) => iso.to_string(),
    }
}

/// Parse SOD status counts from the saved JSON file
fn parse_sod_status_from_file(file_path: &str) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();

    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return counts,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return counts,
    };

    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(status) = item.get("status").and_then(|s| s.as_str()) {
                *counts.entry(status.to_string()).or_insert(0) += 1;
            }
        }
    }

    counts
}
