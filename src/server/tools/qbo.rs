// QBO MCP Tools — read-only (+ sync triggers) for the mgmt workspace.

use crate::core::error::CmdResult;
use crate::modules::qbo::{connection, entities, reports, sync, transactions};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde::Serialize;
use serde_json::{json, Value};

fn result_to_tool<T: Serialize>(r: CmdResult<T>) -> ToolResult {
    match r {
        Ok(v) => ToolResult::json(&v),
        Err(e) => ToolResult::error(e.to_string()),
    }
}

pub fn tools() -> Vec<Tool> {
    vec![
        // ───── Connection / sync ─────
        Tool {
            name: "qbo-connection-status".to_string(),
            description: "Get the current QuickBooks connection (company, realm ID, token expiry, environment). Returns null if not connected.".to_string(),
            input_schema: InputSchema::with_properties(json!({}), vec![]),
        },
        Tool {
            name: "qbo-list-sync-runs".to_string(),
            description: "List recent QBO sync runs with status, records processed, duration.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "limit": { "type": "integer", "description": "Max runs to return (default 20)" } }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-trigger-sync".to_string(),
            description: "Trigger a QBO entity sync. Entities: accounts, customers, vendors, items, classes, invoices, bills, estimates, payments, bill_payments, expenses, journal_entries, or 'all'.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "entity": { "type": "string", "description": "Entity to sync (default 'all')" } }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-trigger-reports-sync".to_string(),
            description: "Refresh cached QBO reports (P&L, Balance Sheet, Cash Flow, Aged AR/AP) for all standard periods (mtd, ytd, last_month, last_quarter, prior_year — FY-aware).".to_string(),
            input_schema: InputSchema::with_properties(json!({}), vec![]),
        },
        Tool {
            name: "qbo-fetch-report".to_string(),
            description: "Fetch a single QBO report for a specific date range (e.g. FY2024 = Aug 2023–Jul 2024) and cache it. Subsequent `qbo-get-*` calls with the same label will return this snapshot. Use this for fiscal years or custom periods not in the default 5.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "report_type": { "type": "string", "enum": ["ProfitAndLoss", "BalanceSheet", "CashFlow", "AgedReceivables", "AgedPayables"] },
                    "label": { "type": "string", "description": "Cache key label, e.g. 'fy_2024' or 'custom_q1_2025'" },
                    "start_date": { "type": "string", "description": "YYYY-MM-DD. Omit for point-in-time reports (BS, Aged)" },
                    "end_date": { "type": "string", "description": "YYYY-MM-DD (required)" }
                }),
                vec!["report_type".to_string(), "label".to_string(), "end_date".to_string()],
            ),
        },

        // ───── Master data ─────
        Tool {
            name: "qbo-list-accounts".to_string(),
            description: "List QBO chart of accounts with optional filters.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "search": { "type": "string", "description": "Search account name" },
                    "account_type": { "type": "string", "description": "Filter by type (Bank, Expense, Income, etc.)" },
                    "active_only": { "type": "boolean", "description": "Include only active accounts (default true)" },
                    "limit": { "type": "integer", "description": "Max results (default 200)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-customers".to_string(),
            description: "List QBO customers with optional search across name, company, email.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "search": { "type": "string" },
                    "active_only": { "type": "boolean", "description": "Default true" },
                    "limit": { "type": "integer", "description": "Default 200" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-vendors".to_string(),
            description: "List QBO vendors with optional search.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "search": { "type": "string" },
                    "active_only": { "type": "boolean", "description": "Default true" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-items".to_string(),
            description: "List QBO products/services (items).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "search": { "type": "string" },
                    "active_only": { "type": "boolean", "description": "Default true" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },

        // ───── Transactions ─────
        Tool {
            name: "qbo-list-invoices".to_string(),
            description: "List QBO invoices with filters.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "customer_qbo_id": { "type": "string" },
                    "from_date": { "type": "string", "description": "YYYY-MM-DD" },
                    "to_date": { "type": "string", "description": "YYYY-MM-DD" },
                    "unpaid_only": { "type": "boolean", "description": "Only invoices with balance > 0" },
                    "limit": { "type": "integer", "description": "Default 200" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-bills".to_string(),
            description: "List QBO bills with filters.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "vendor_qbo_id": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "unpaid_only": { "type": "boolean" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-estimates".to_string(),
            description: "List QBO estimates (quotes).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "customer_qbo_id": { "type": "string" },
                    "status": { "type": "string", "description": "Pending | Accepted | Closed | Rejected" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-payments".to_string(),
            description: "List customer payments received.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "customer_qbo_id": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-bill-payments".to_string(),
            description: "List bill payments made to vendors.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "vendor_qbo_id": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-expenses".to_string(),
            description: "List expense transactions (QBO Purchase entities).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "account_qbo_id": { "type": "string", "description": "Payment account ID" },
                    "payee_qbo_id": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "qbo-list-journal-entries".to_string(),
            description: "List manual journal entries.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "limit": { "type": "integer" }
                }),
                vec![],
            ),
        },

        // ───── Reports ─────
        Tool {
            name: "qbo-get-pl".to_string(),
            description: "Get cached Profit & Loss report. Built-in periods: mtd, ytd (FY-to-date), last_month, last_quarter (FY quarter), prior_year (prior FY). Also accepts fy_YYYY or custom_* labels that have been fetched via qbo-fetch-report. ThinkVAL FY = Aug→Jul.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "period": { "type": "string", "description": "mtd | ytd | last_month | last_quarter | prior_year | fy_YYYY | custom_<label>" } }),
                vec!["period".to_string()],
            ),
        },
        Tool {
            name: "qbo-get-balance-sheet".to_string(),
            description: "Get cached Balance Sheet (point-in-time). Built-in periods: mtd, last_month, prior_year (end of prior FY = Jul 31). Also accepts fy_YYYY or custom_asof_* labels.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "period": { "type": "string", "description": "mtd | last_month | prior_year | fy_YYYY | custom_asof_<date>" } }),
                vec!["period".to_string()],
            ),
        },
        Tool {
            name: "qbo-get-cash-flow".to_string(),
            description: "Get cached Statement of Cash Flows for a period. Same period labels as qbo-get-pl.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "period": { "type": "string", "description": "mtd | ytd | last_month | last_quarter | prior_year | fy_YYYY | custom_*" } }),
                vec!["period".to_string()],
            ),
        },
        Tool {
            name: "qbo-get-aged-ar".to_string(),
            description: "Get cached Aged Receivables report (point-in-time). Same period labels as qbo-get-balance-sheet.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "period": { "type": "string", "description": "mtd | last_month | prior_year | fy_YYYY | custom_asof_<date>" } }),
                vec!["period".to_string()],
            ),
        },
        Tool {
            name: "qbo-get-aged-ap".to_string(),
            description: "Get cached Aged Payables report (point-in-time). Same period labels as qbo-get-balance-sheet.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({ "period": { "type": "string", "description": "mtd | last_month | prior_year | fy_YYYY | custom_asof_<date>" } }),
                vec!["period".to_string()],
            ),
        },
    ]
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}
fn opt_bool(v: &Value, key: &str) -> Option<bool> {
    v.get(key).and_then(|x| x.as_bool())
}
fn opt_i32(v: &Value, key: &str) -> Option<i32> {
    v.get(key).and_then(|x| x.as_i64()).map(|n| n as i32)
}
fn req_str(v: &Value, key: &str) -> Result<String, String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
        .ok_or_else(|| format!("Missing required argument: {}", key))
}

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "qbo-connection-status" => result_to_tool(connection::qbo_connection_status().await),
        "qbo-list-sync-runs" => result_to_tool(connection::qbo_list_sync_runs(opt_i32(&args, "limit")).await),
        "qbo-trigger-sync" => result_to_tool(sync::qbo_trigger_sync(opt_str(&args, "entity")).await),
        "qbo-trigger-reports-sync" => result_to_tool(sync::qbo_trigger_reports_sync().await),
        "qbo-fetch-report" => {
            let report_type = match req_str(&args, "report_type") {
                Ok(v) => v,
                Err(e) => return ToolResult::error(e),
            };
            let label = match req_str(&args, "label") {
                Ok(v) => v,
                Err(e) => return ToolResult::error(e),
            };
            let end_date = match req_str(&args, "end_date") {
                Ok(v) => v,
                Err(e) => return ToolResult::error(e),
            };
            let start_date = opt_str(&args, "start_date");
            result_to_tool(sync::qbo_fetch_report(&report_type, &label, start_date, end_date).await)
        }

        "qbo-list-accounts" => result_to_tool(
            entities::qbo_list_accounts(
                opt_str(&args, "search"),
                opt_str(&args, "account_type"),
                opt_bool(&args, "active_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-customers" => result_to_tool(
            entities::qbo_list_customers(
                opt_str(&args, "search"),
                opt_bool(&args, "active_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-vendors" => result_to_tool(
            entities::qbo_list_vendors(
                opt_str(&args, "search"),
                opt_bool(&args, "active_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-items" => result_to_tool(
            entities::qbo_list_items(
                opt_str(&args, "search"),
                opt_bool(&args, "active_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),

        "qbo-list-invoices" => result_to_tool(
            transactions::qbo_list_invoices(
                opt_str(&args, "customer_qbo_id"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_bool(&args, "unpaid_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-bills" => result_to_tool(
            transactions::qbo_list_bills(
                opt_str(&args, "vendor_qbo_id"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_bool(&args, "unpaid_only"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-estimates" => result_to_tool(
            transactions::qbo_list_estimates(
                opt_str(&args, "customer_qbo_id"),
                opt_str(&args, "status"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-payments" => result_to_tool(
            transactions::qbo_list_payments(
                opt_str(&args, "customer_qbo_id"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-bill-payments" => result_to_tool(
            transactions::qbo_list_bill_payments(
                opt_str(&args, "vendor_qbo_id"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-expenses" => result_to_tool(
            transactions::qbo_list_expenses(
                opt_str(&args, "account_qbo_id"),
                opt_str(&args, "payee_qbo_id"),
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),
        "qbo-list-journal-entries" => result_to_tool(
            transactions::qbo_list_journal_entries(
                opt_str(&args, "from_date"),
                opt_str(&args, "to_date"),
                opt_i32(&args, "limit"),
            )
            .await,
        ),

        "qbo-get-pl" => {
            let period = match req_str(&args, "period") {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            result_to_tool(reports::qbo_get_report("ProfitAndLoss", &period).await)
        }
        "qbo-get-balance-sheet" => {
            let period = match req_str(&args, "period") {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            result_to_tool(reports::qbo_get_report("BalanceSheet", &period).await)
        }
        "qbo-get-cash-flow" => {
            let period = match req_str(&args, "period") {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            result_to_tool(reports::qbo_get_report("CashFlow", &period).await)
        }
        "qbo-get-aged-ar" => {
            let period = match req_str(&args, "period") {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            result_to_tool(reports::qbo_get_report("AgedReceivables", &period).await)
        }
        "qbo-get-aged-ap" => {
            let period = match req_str(&args, "period") {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            result_to_tool(reports::qbo_get_report("AgedPayables", &period).await)
        }

        _ => ToolResult::error(format!("Unknown QBO tool: {}", name)),
    }
}
