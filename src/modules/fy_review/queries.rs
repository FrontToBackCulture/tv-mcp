// FY Review read queries — snapshots, monthly grid, recognition, reconciliations, checklist.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client_for_workspace;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn fmt_filter(key: &str, op: &str, val: &str) -> String {
    format!("{}={}.{}", key, op, urlencoding::encode(val))
}

// ─── Snapshots ───────────────────────────────────────────────────────────

pub async fn fy_list_snapshots(
    fy_code: Option<String>,
    source: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut parts: Vec<String> = Vec::new();
    if let Some(fy) = fy_code {
        parts.push(fmt_filter("fy_code", "eq", &fy));
    }
    if let Some(s) = source {
        parts.push(fmt_filter("source", "eq", &s));
    }
    parts.push("order=captured_at.desc".to_string());
    parts.push(format!("limit={}", limit.unwrap_or(50)));
    client.select("fy_snapshots", &parts.join("&")).await
}

pub async fn fy_diff_snapshots(
    snapshot_id_a: String,
    snapshot_id_b: String,
) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q_a = format!(
        "select=account_qbo_id,account_name,fs_line,balance,movement&snapshot_id=eq.{}",
        snapshot_id_a
    );
    let q_b = format!(
        "select=account_qbo_id,account_name,fs_line,balance,movement&snapshot_id=eq.{}",
        snapshot_id_b
    );
    let a: Vec<Value> = client.select("fy_snapshot_lines", &q_a).await?;
    let b: Vec<Value> = client.select("fy_snapshot_lines", &q_b).await?;

    // Index each by account_qbo_id
    let index = |rows: &[Value]| -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        for r in rows {
            let id = r.get("account_qbo_id").and_then(|v| v.as_str()).unwrap_or("");
            if !id.is_empty() {
                m.insert(id.to_string(), r.clone());
            }
        }
        m
    };
    let map_a = index(&a);
    let map_b = index(&b);

    let mut changed: Vec<Value> = Vec::new();
    for (id, row_a) in &map_a {
        if let Some(row_b) = map_b.get(id) {
            let bal_a = row_a.get("balance").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let bal_b = row_b.get("balance").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mov_a = row_a.get("movement").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mov_b = row_b.get("movement").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if (bal_a - bal_b).abs() > 0.001 || (mov_a - mov_b).abs() > 0.001 {
                changed.push(json!({
                    "account_qbo_id": id,
                    "account_name": row_a.get("account_name"),
                    "fs_line": row_a.get("fs_line"),
                    "balance_a": bal_a, "balance_b": bal_b,
                    "balance_delta": bal_b - bal_a,
                    "movement_a": mov_a, "movement_b": mov_b,
                    "movement_delta": mov_b - mov_a,
                }));
            }
        }
    }
    let added: Vec<Value> = map_b.iter()
        .filter(|(k, _)| !map_a.contains_key(*k))
        .map(|(_, v)| v.clone()).collect();
    let removed: Vec<Value> = map_a.iter()
        .filter(|(k, _)| !map_b.contains_key(*k))
        .map(|(_, v)| v.clone()).collect();

    Ok(json!({
        "changed": changed,
        "added": added,
        "removed": removed,
    }))
}

// ─── Monthly grid ────────────────────────────────────────────────────────

/// Return a 12-month grid for a given FY and statement (pnl | bs).
/// Rows are grouped by fs_line, columns are the 12 months of the FY.
pub async fn fy_get_monthly_grid(
    fy_code: String,
    statement: String, // "pnl" | "bs"
) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;

    // Pull latest qbo snapshot per month in this FY
    let snap_q = format!(
        "select=id,period_start,period_label&fy_code=eq.{}&source=eq.qbo&granularity=eq.month&order=period_start.asc,captured_at.desc",
        fy_code
    );
    let snapshots: Vec<Value> = client.select("fy_snapshots", &snap_q).await?;

    // Keep only the latest snapshot per period_start
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for s in snapshots {
        let ps = s.get("period_start").and_then(|v| v.as_str()).unwrap_or("").to_string();
        latest.entry(ps).or_insert(s);
    }

    let account_type_filter = if statement == "pnl" { "pnl" } else { "bs" };
    let amount_field = if statement == "pnl" { "movement" } else { "balance" };

    // For each snapshot, get its lines
    // fs_line → month_label → value
    let mut grid: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut months: Vec<String> = Vec::new();
    let mut fs_line_meta: BTreeMap<String, (String, String, i32)> = BTreeMap::new(); // fs_line → (label, section, order)

    for (period_start, snap) in &latest {
        let label = snap.get("period_label").and_then(|v| v.as_str()).unwrap_or(period_start).to_string();
        months.push(label.clone());
        let snap_id = snap.get("id").and_then(|v| v.as_str()).unwrap_or("");

        let q = format!(
            "select=account_qbo_id,account_name,fs_line,{}&snapshot_id=eq.{}&account_type=eq.{}",
            amount_field, snap_id, account_type_filter
        );
        let lines: Vec<Value> = client.select("fy_snapshot_lines", &q).await?;

        for line in lines {
            let fs_line = line.get("fs_line").and_then(|v| v.as_str()).unwrap_or("unmapped").to_string();
            let amount = line.get(amount_field).and_then(|v| v.as_f64()).unwrap_or(0.0);
            *grid.entry(fs_line.clone()).or_default().entry(label.clone()).or_insert(0.0) += amount;
        }
    }

    // Attach fs_line metadata from fy_fs_mapping
    let map_rows: Vec<Value> = client
        .select("fy_fs_mapping", "select=fs_line,fs_section,display_order")
        .await
        .unwrap_or_default();
    for r in map_rows {
        let fs_line = r.get("fs_line").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let section = r.get("fs_section").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let order = r.get("display_order").and_then(|v| v.as_i64()).unwrap_or(999) as i32;
        fs_line_meta.entry(fs_line.clone()).or_insert((fs_line, section, order));
    }

    let mut rows: Vec<Value> = grid.into_iter()
        .map(|(fs_line, values)| {
            let (label, section, order) = fs_line_meta
                .get(&fs_line)
                .cloned()
                .unwrap_or_else(|| (fs_line.clone(), "unmapped".to_string(), 999));
            let month_values: Vec<f64> = months.iter()
                .map(|m| values.get(m).copied().unwrap_or(0.0))
                .collect();
            let total: f64 = month_values.iter().sum();
            json!({
                "fs_line": fs_line,
                "label": label,
                "section": section,
                "display_order": order,
                "values": month_values,
                "fy_total": total,
            })
        })
        .collect();
    rows.sort_by_key(|r| (
        r.get("section").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        r.get("display_order").and_then(|v| v.as_i64()).unwrap_or(999),
    ));

    Ok(json!({
        "fy_code": fy_code,
        "statement": statement,
        "months": months,
        "rows": rows,
    }))
}

// ─── Recognition schedule ────────────────────────────────────────────────

pub async fn fy_get_recognition_schedule(
    fy_code: Option<String>,
    customer_qbo_id: Option<String>,
    orderform_code: Option<String>,
    status: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut parts: Vec<String> = Vec::new();
    if let Some(fy) = fy_code {
        parts.push(fmt_filter("fy_code", "eq", &fy));
    }
    if let Some(c) = customer_qbo_id {
        parts.push(fmt_filter("customer_qbo_id", "eq", &c));
    }
    if let Some(o) = orderform_code {
        parts.push(fmt_filter("orderform_code", "eq", &o));
    }
    if let Some(s) = status {
        parts.push(fmt_filter("status", "eq", &s));
    }
    parts.push("order=orderform_code.asc,leg.asc,period_index.asc".to_string());
    parts.push(format!("limit={}", limit.unwrap_or(500)));
    client.select("recognition_schedule", &parts.join("&")).await
}

pub async fn fy_get_recognition_summary(fy_code: String) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = format!(
        "select=customer_qbo_id,customer_name,orderform_code,status&fy_code=eq.{}&limit=10000",
        fy_code
    );
    let rows: Vec<Value> = client.select("recognition_schedule", &q).await?;

    // Group by customer
    let mut by_customer: BTreeMap<String, (String, std::collections::HashSet<String>, BTreeMap<String, i32>)> = BTreeMap::new();
    for r in &rows {
        let cid = r.get("customer_qbo_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cname = r.get("customer_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let of = r.get("orderform_code").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let st = r.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let entry = by_customer.entry(cid).or_insert_with(|| (cname.clone(), Default::default(), Default::default()));
        entry.1.insert(of);
        *entry.2.entry(st).or_insert(0) += 1;
    }

    let mut customers: Vec<Value> = by_customer.into_iter()
        .map(|(cid, (cname, ofs, status_counts))| {
            let posted = *status_counts.get("posted").unwrap_or(&0);
            let missing = *status_counts.get("missing").unwrap_or(&0);
            let mismatched = *status_counts.get("mismatched").unwrap_or(&0);
            let expected = *status_counts.get("expected").unwrap_or(&0);
            let total = posted + missing + mismatched + expected;
            let issues = missing + mismatched;
            json!({
                "customer_qbo_id": cid,
                "customer_name": cname,
                "orderforms": ofs.len(),
                "total_rows": total,
                "posted": posted,
                "missing": missing,
                "mismatched": mismatched,
                "expected": expected,
                "issues": issues,
            })
        })
        .collect();
    customers.sort_by_key(|v| -(v.get("issues").and_then(|x| x.as_i64()).unwrap_or(0)));

    Ok(json!({
        "fy_code": fy_code,
        "customers": customers,
        "totals": {
            "total_rows": rows.len(),
        }
    }))
}

// ─── Reconciliation ──────────────────────────────────────────────────────

pub async fn fy_get_reconciliation(fy_code: String) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = format!(
        "fy_code=eq.{}&order=fs_line.asc&limit=500",
        fy_code
    );
    client.select("fy_reconciliations", &q).await
}

pub async fn fy_update_reconciliation(
    id: String,
    status: Option<String>,
    resolution_note: Option<String>,
    resolved_by: Option<String>,
) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut body = json!({});
    if let Some(s) = status {
        body["status"] = Value::String(s.clone());
        if s == "resolved" || s == "accepted" {
            body["resolved_at"] = Value::String(chrono::Utc::now().to_rfc3339());
        }
    }
    if let Some(n) = resolution_note {
        body["resolution_note"] = Value::String(n);
    }
    if let Some(b) = resolved_by {
        body["resolved_by"] = Value::String(b);
    }
    let q = format!("id=eq.{}", id);
    let updated: Value = client.update("fy_reconciliations", &q, &body).await?;
    Ok(updated)
}

// ─── Checklist ───────────────────────────────────────────────────────────

pub async fn fy_list_checklist(
    fy_code: String,
    period_start: Option<String>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut parts: Vec<String> = vec![fmt_filter("fy_code", "eq", &fy_code)];
    if let Some(p) = period_start {
        parts.push(fmt_filter("period_start", "eq", &p));
    }
    parts.push("order=period_start.asc,category.asc,item_key.asc".to_string());
    parts.push("limit=500".to_string());
    client.select("fy_close_checklist", &parts.join("&")).await
}

pub async fn fy_update_checklist(
    id: String,
    status: Option<String>,
    notes: Option<String>,
    completed_by: Option<String>,
) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut body = json!({});
    if let Some(s) = status {
        body["status"] = Value::String(s.clone());
        if s == "done" {
            body["completed_at"] = Value::String(chrono::Utc::now().to_rfc3339());
        }
    }
    if let Some(n) = notes {
        body["notes"] = Value::String(n);
    }
    if let Some(b) = completed_by {
        body["completed_by"] = Value::String(b);
    }
    let q = format!("id=eq.{}", id);
    let updated: Value = client.update("fy_close_checklist", &q, &body).await?;
    Ok(updated)
}

// ─── Drift alerts ────────────────────────────────────────────────────────

pub async fn fy_list_drift_alerts(
    fy_code: Option<String>,
    status: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut parts: Vec<String> = Vec::new();
    if let Some(fy) = fy_code {
        parts.push(fmt_filter("fy_code", "eq", &fy));
    }
    if let Some(s) = status {
        parts.push(fmt_filter("status", "eq", &s));
    }
    parts.push("order=detected_at.desc".to_string());
    parts.push(format!("limit={}", limit.unwrap_or(200)));
    client.select("fy_drift_alerts", &parts.join("&")).await
}

pub async fn fy_acknowledge_drift_alert(
    id: String,
    status: Option<String>,
    note: Option<String>,
    acknowledged_by: Option<String>,
) -> CmdResult<Value> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut body = json!({});
    if let Some(s) = status {
        body["status"] = Value::String(s);
        body["acknowledged_at"] = Value::String(chrono::Utc::now().to_rfc3339());
    }
    if let Some(n) = note {
        body["note"] = Value::String(n);
    }
    if let Some(b) = acknowledged_by {
        body["acknowledged_by"] = Value::String(b);
    }
    let q = format!("id=eq.{}", id);
    let updated: Value = client.update("fy_drift_alerts", &q, &body).await?;
    Ok(updated)
}

// ─── Orderforms ──────────────────────────────────────────────────────────

pub async fn fy_list_orderforms(
    customer_qbo_id: Option<String>,
    status: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = customer_qbo_id {
        parts.push(fmt_filter("customer_qbo_id", "eq", &c));
    }
    if let Some(s) = status {
        parts.push(fmt_filter("status", "eq", &s));
    }
    parts.push("order=start_date.desc".to_string());
    parts.push(format!("limit={}", limit.unwrap_or(200)));
    client.select("orderforms", &parts.join("&")).await
}
