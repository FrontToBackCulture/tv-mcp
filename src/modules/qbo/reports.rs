// QBO reports — cached P&L, Balance Sheet, Cash Flow, Aged AR/AP

use super::MGMT_WORKSPACE_ID;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client_for_workspace;
use serde_json::Value;

pub const VALID_REPORT_TYPES: &[&str] = &[
    "ProfitAndLoss",
    "BalanceSheet",
    "CashFlow",
    "AgedReceivables",
    "AgedPayables",
];

/// Built-in period labels (FY-aware — ThinkVAL's FY runs Aug→Jul).
///   mtd           = calendar month-to-date
///   ytd           = FY-to-date (from Aug 1)
///   last_month    = prior calendar month
///   last_quarter  = prior FY-aligned quarter
///   prior_year    = full prior FY (Aug 1 YYYY-1 → Jul 31 YYYY)
/// Other accepted labels: fy_YYYY, custom_<start>_<end>, custom_asof_<date>.
pub const BUILT_IN_PERIODS: &[&str] = &[
    "mtd",
    "ytd",
    "last_month",
    "last_quarter",
    "prior_year",
];

pub async fn qbo_get_report(report_type: &str, period: &str) -> CmdResult<Option<Value>> {
    if !VALID_REPORT_TYPES.contains(&report_type) {
        return Err(CommandError::Internal(format!(
            "Invalid report_type '{}'. Must be one of: {}",
            report_type,
            VALID_REPORT_TYPES.join(", ")
        )));
    }

    // Point-in-time reports only snapshot at mtd / last_month / prior_year in
    // the default cache set — remap other built-in periods to the nearest
    // point-in-time. Custom fy_YYYY / custom_* labels pass through as-is.
    let point_in_time = matches!(
        report_type,
        "BalanceSheet" | "AgedReceivables" | "AgedPayables"
    );
    let effective_period = if point_in_time && period == "ytd" {
        "mtd"
    } else if point_in_time && period == "last_quarter" {
        "last_month"
    } else {
        period
    };

    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let query = format!(
        "report_type=eq.{}&params->>label=eq.{}&order=generated_at.desc&limit=1",
        report_type, effective_period
    );
    let rows: Vec<Value> = client.select("qbo_reports_cache", &query).await?;
    Ok(rows.into_iter().next())
}
