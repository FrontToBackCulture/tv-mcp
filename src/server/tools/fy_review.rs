// FY Review MCP Tools — read snapshots, recognition, reconciliation,
// checklist + trigger capture/build actions. Mgmt-workspace only.

use crate::core::error::CmdResult;
use crate::modules::fy_review::{actions, queries};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde::Serialize;
use serde_json::{json, Value};

fn result_to_tool<T: Serialize>(r: CmdResult<T>) -> ToolResult {
    match r {
        Ok(v) => ToolResult::json(&v),
        Err(e) => ToolResult::error(e.to_string()),
    }
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}
fn req_str(args: &Value, key: &str) -> Result<String, String> {
    opt_str(args, key).ok_or_else(|| format!("missing required field: {}", key))
}
fn opt_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "fy-list-snapshots".to_string(),
            description: "List FY snapshot headers (metadata only — not the underlying lines). Filter by fy_code and/or source ('qbo' | 'fs' | 'manual').".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string", "description": "e.g. FY2024" },
                    "source": { "type": "string", "description": "qbo | fs | manual" },
                    "limit": { "type": "integer", "description": "Default 50" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "fy-get-monthly-grid".to_string(),
            description: "Return 12-month P&L or BS grid for an FY. Rows grouped by fs_line (matches the statutory FS presentation); columns are the 12 monthly snapshots. Uses the latest captured snapshot per month.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string", "description": "e.g. FY2024" },
                    "statement": { "type": "string", "enum": ["pnl", "bs"], "description": "pnl (Profit & Loss movement) or bs (Balance Sheet month-end balance)" }
                }),
                vec!["fy_code".to_string(), "statement".to_string()],
            ),
        },
        Tool {
            name: "fy-diff-snapshots".to_string(),
            description: "Diff two snapshots by account. Returns changed / added / removed lines. Use to detect closed-period drift (pass the latest and prior snapshot ids for the same period).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "snapshot_id_a": { "type": "string" },
                    "snapshot_id_b": { "type": "string" }
                }),
                vec!["snapshot_id_a".to_string(), "snapshot_id_b".to_string()],
            ),
        },
        Tool {
            name: "fy-get-recognition-schedule".to_string(),
            description: "Return rows from recognition_schedule. Each row is an expected monthly recognition for one orderform × leg (SUB|SVC) × period. Status: posted | missing | mismatched | expected | orphan | waived. Filter by fy_code, customer, orderform, status.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string" },
                    "customer_qbo_id": { "type": "string" },
                    "orderform_code": { "type": "string" },
                    "status": { "type": "string" },
                    "limit": { "type": "integer", "description": "Default 500" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "fy-get-recognition-summary".to_string(),
            description: "Per-customer summary of recognition status for an FY: orderforms, total rows, posted / missing / mismatched / expected counts. Sorted by issue count desc.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "fy_code": { "type": "string" } }),
                vec!["fy_code".to_string()],
            ),
        },
        Tool {
            name: "fy-list-orderforms".to_string(),
            description: "List orderforms (contract master data derived from QBO JEs). Filter by customer_qbo_id or status (active | completed | cancelled).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "customer_qbo_id": { "type": "string" },
                    "status": { "type": "string" },
                    "limit": { "type": "integer", "description": "Default 200" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "fy-get-reconciliation".to_string(),
            description: "Return reconciliation rows for a closed FY: official FS values vs latest QBO snapshot, variance, status (open | investigating | resolved | accepted), resolution notes. Seeded from submitted FS.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "fy_code": { "type": "string" } }),
                vec!["fy_code".to_string()],
            ),
        },
        Tool {
            name: "fy-update-reconciliation".to_string(),
            description: "Update status and/or resolution note on a reconciliation row. Setting status to 'resolved' or 'accepted' stamps resolved_at.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string" },
                    "status": { "type": "string", "enum": ["open", "investigating", "resolved", "accepted"] },
                    "resolution_note": { "type": "string" },
                    "resolved_by": { "type": "string" }
                }),
                vec!["id".to_string()],
            ),
        },
        Tool {
            name: "fy-list-checklist".to_string(),
            description: "Return close checklist items for an FY, optionally filtered by period_start (month). Covers bank rec, GST filing, payroll booking, recognition posting, depreciation, etc.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string" },
                    "period_start": { "type": "string", "description": "YYYY-MM-DD month start" }
                }),
                vec!["fy_code".to_string()],
            ),
        },
        Tool {
            name: "fy-update-checklist".to_string(),
            description: "Update status / notes on a checklist item. Setting status to 'done' stamps completed_at.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string" },
                    "status": { "type": "string", "enum": ["pending", "in_progress", "done", "na"] },
                    "notes": { "type": "string" },
                    "completed_by": { "type": "string" }
                }),
                vec!["id".to_string()],
            ),
        },
        Tool {
            name: "fy-capture-snapshot".to_string(),
            description: "Trigger a fresh QBO capture for an FY (or a single month if period_start given). Inserts new snapshot rows — old ones retained for drift detection.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string", "description": "e.g. FY2024" },
                    "period_start": { "type": "string", "description": "Optional YYYY-MM-DD — captures only this month" }
                }),
                vec!["fy_code".to_string()],
            ),
        },
        Tool {
            name: "fy-list-drift-alerts".to_string(),
            description: "Return drift alerts — each row is an account line in a previously-captured period whose value changed on recapture. Filter by fy_code and/or status (open | acknowledged | investigated | resolved).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string" },
                    "status": { "type": "string" },
                    "limit": { "type": "integer", "description": "Default 200" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "fy-acknowledge-drift-alert".to_string(),
            description: "Acknowledge / update status on a drift alert. Sets acknowledged_at when status changes.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string" },
                    "status": { "type": "string", "enum": ["open", "acknowledged", "investigated", "resolved"] },
                    "note": { "type": "string" },
                    "acknowledged_by": { "type": "string" }
                }),
                vec!["id".to_string()],
            ),
        },
        Tool {
            name: "fy-watchdog-run".to_string(),
            description: "Run the drift watchdog: recapture current + prior FY snapshots, diff against prior captures, insert fy_drift_alerts rows for any account line whose balance/movement changed beyond threshold (default $1). Returns summary of alerts created.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string", "description": "Default: current + prior FY" },
                    "period_start": { "type": "string", "description": "YYYY-MM-DD to narrow to one month" },
                    "threshold": { "type": "number", "description": "Min $ delta to record; default 1.0" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "fy-build-recognition".to_string(),
            description: "Rebuild the recognition_schedule from posted QBO JEs (doc_number matching {orderform}-{SUB|SVC}-{N}). Optionally scope to a single fy_code or orderform. Preserves user-set notes.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "fy_code": { "type": "string" },
                    "orderform_code": { "type": "string" }
                }),
                vec![],
            ),
        },
    ]
}

pub async fn call(name: &str, args: Value) -> ToolResult {
    let args = &args;
    match name {
        "fy-list-snapshots" => result_to_tool(
            queries::fy_list_snapshots(
                opt_str(args, "fy_code"),
                opt_str(args, "source"),
                opt_i32(args, "limit"),
            ).await,
        ),
        "fy-get-monthly-grid" => {
            let fy_code = match req_str(args, "fy_code") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            let statement = match req_str(args, "statement") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_get_monthly_grid(fy_code, statement).await)
        }
        "fy-diff-snapshots" => {
            let a = match req_str(args, "snapshot_id_a") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            let b = match req_str(args, "snapshot_id_b") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_diff_snapshots(a, b).await)
        }
        "fy-get-recognition-schedule" => result_to_tool(
            queries::fy_get_recognition_schedule(
                opt_str(args, "fy_code"),
                opt_str(args, "customer_qbo_id"),
                opt_str(args, "orderform_code"),
                opt_str(args, "status"),
                opt_i32(args, "limit"),
            ).await,
        ),
        "fy-get-recognition-summary" => {
            let fy = match req_str(args, "fy_code") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_get_recognition_summary(fy).await)
        }
        "fy-list-orderforms" => result_to_tool(
            queries::fy_list_orderforms(
                opt_str(args, "customer_qbo_id"),
                opt_str(args, "status"),
                opt_i32(args, "limit"),
            ).await,
        ),
        "fy-get-reconciliation" => {
            let fy = match req_str(args, "fy_code") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_get_reconciliation(fy).await)
        }
        "fy-update-reconciliation" => {
            let id = match req_str(args, "id") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_update_reconciliation(
                id,
                opt_str(args, "status"),
                opt_str(args, "resolution_note"),
                opt_str(args, "resolved_by"),
            ).await)
        }
        "fy-list-checklist" => {
            let fy = match req_str(args, "fy_code") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_list_checklist(fy, opt_str(args, "period_start")).await)
        }
        "fy-update-checklist" => {
            let id = match req_str(args, "id") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_update_checklist(
                id,
                opt_str(args, "status"),
                opt_str(args, "notes"),
                opt_str(args, "completed_by"),
            ).await)
        }
        "fy-capture-snapshot" => {
            let fy = match req_str(args, "fy_code") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(actions::fy_capture_snapshot(fy, opt_str(args, "period_start")).await)
        }
        "fy-list-drift-alerts" => result_to_tool(
            queries::fy_list_drift_alerts(
                opt_str(args, "fy_code"),
                opt_str(args, "status"),
                opt_i32(args, "limit"),
            ).await,
        ),
        "fy-acknowledge-drift-alert" => {
            let id = match req_str(args, "id") {
                Ok(v) => v, Err(e) => return ToolResult::error(e),
            };
            result_to_tool(queries::fy_acknowledge_drift_alert(
                id,
                opt_str(args, "status"),
                opt_str(args, "note"),
                opt_str(args, "acknowledged_by"),
            ).await)
        }
        "fy-watchdog-run" => {
            let threshold = args.get("threshold").and_then(|v| v.as_f64());
            result_to_tool(actions::fy_watchdog_run(
                opt_str(args, "fy_code"),
                opt_str(args, "period_start"),
                threshold,
            ).await)
        }
        "fy-build-recognition" => result_to_tool(
            actions::fy_build_recognition(
                opt_str(args, "fy_code"),
                opt_str(args, "orderform_code"),
            ).await,
        ),
        _ => ToolResult::error(format!("Unknown fy-review tool: {}", name)),
    }
}
