// JE write ops — create, update (amount / doc_number / txn_date), delete.
// All operations target the mgmt Supabase edge functions, which in turn call
// QBO's /v3/company/{realm}/journalentry endpoint and mirror the result into
// qbo_journal_entries.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{get_workspace_setting, KEY_SUPABASE_ANON_KEY, KEY_SUPABASE_URL};
use serde::{Deserialize, Serialize};
use serde_json::Value;

async fn invoke(name: &str, body: Value) -> CmdResult<Value> {
    let url = get_workspace_setting(MGMT_WORKSPACE_ID, KEY_SUPABASE_URL).ok_or_else(|| {
        CommandError::Config("mgmt workspace Supabase URL not configured".into())
    })?;
    let anon_key = get_workspace_setting(MGMT_WORKSPACE_ID, KEY_SUPABASE_ANON_KEY).ok_or_else(
        || CommandError::Config("mgmt workspace Supabase anon key not configured".into()),
    )?;

    let client = crate::HTTP_CLIENT.clone();
    let res = client
        .post(format!("{}/functions/v1/{}", url, name))
        .header("Authorization", format!("Bearer {}", anon_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    Ok(res.json().await?)
}

/// One proposed JE. Balanced: same `amount` is posted Dr to one account and
/// Cr to another. `customer_qbo_id` is required by the edge function today;
/// for vendor/employee-linked postings pass the vendor's qbo_id — QBO will
/// resolve the correct Entity type by ID.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProposedEntry {
    pub doc_number: String,
    pub txn_date: String,
    pub description: String,
    pub amount: f64,
    pub dr_account_qbo_id: String,
    pub cr_account_qbo_id: String,
    pub customer_qbo_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

pub async fn qbo_create_journal_entry(entries: Vec<ProposedEntry>) -> CmdResult<Value> {
    invoke(
        "qbo-create-journal-entry",
        serde_json::json!({ "entries": entries, "triggered_by": "mcp" }),
    )
    .await
}

pub async fn qbo_update_je_amount(qbo_id: &str, amount: f64) -> CmdResult<Value> {
    invoke(
        "qbo-update-je-amount",
        serde_json::json!({ "qbo_id": qbo_id, "amount": amount }),
    )
    .await
}

pub async fn qbo_update_je_docnumber(qbo_id: &str, doc_number: &str) -> CmdResult<Value> {
    invoke(
        "qbo-update-je-docnumber",
        serde_json::json!({ "qbo_id": qbo_id, "doc_number": doc_number }),
    )
    .await
}

pub async fn qbo_update_je_txndate(qbo_id: &str, txn_date: &str) -> CmdResult<Value> {
    invoke(
        "qbo-update-je-txndate",
        serde_json::json!({ "qbo_id": qbo_id, "txn_date": txn_date }),
    )
    .await
}

pub async fn qbo_delete_journal_entry(qbo_id: &str) -> CmdResult<Value> {
    invoke("qbo-delete-journal-entry", serde_json::json!({ "qbo_id": qbo_id })).await
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccrualInput {
    pub description: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub expense_account_qbo_id: String,
    pub liability_account_qbo_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_qbo_id: Option<String>,
    /// "Vendor" | "Customer" | "Employee"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    pub accrual_date: String,   // YYYY-MM-DD (last day of prior month)
    pub reversal_date: String,  // YYYY-MM-DD (first day of clicked month)
    pub doc_prefix: String,     // up to ~10 chars
}

/// Post a matched accrual + reversal JE pair. Convenience wrapper around
/// `qbo-post-accrual` — same shift semantics the Expense Review UI uses.
pub async fn qbo_post_accrual(input: AccrualInput) -> CmdResult<Value> {
    let mut body = serde_json::to_value(input).map_err(|e| CommandError::Config(e.to_string()))?;
    if let Value::Object(ref mut map) = body {
        map.insert("triggered_by".to_string(), Value::String("mcp".to_string()));
    }
    invoke("qbo-post-accrual", body).await
}
