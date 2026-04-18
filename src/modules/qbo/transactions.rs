// QBO transaction reads — invoices, bills, estimates, payments, expenses, JEs.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client_for_workspace;
use serde_json::Value;

fn transaction_query(
    from_date: Option<&str>,
    to_date: Option<&str>,
    unpaid_only: Option<bool>,
    extra: &[String],
    order_col: &str,
    limit: Option<i32>,
) -> String {
    let mut parts: Vec<String> = extra.to_vec();
    if let Some(f) = from_date {
        parts.push(format!("txn_date=gte.{}", f));
    }
    if let Some(t) = to_date {
        parts.push(format!("txn_date=lte.{}", t));
    }
    if unpaid_only.unwrap_or(false) {
        parts.push("balance=gt.0".to_string());
    }
    parts.push(format!("order={}.desc", order_col));
    parts.push(format!("limit={}", limit.unwrap_or(200)));
    parts.join("&")
}

pub async fn qbo_list_invoices(
    customer_qbo_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    unpaid_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(c) = customer_qbo_id {
        extra.push(format!("customer_qbo_id=eq.{}", c));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        unpaid_only,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_invoices", &q).await
}

pub async fn qbo_list_bills(
    vendor_qbo_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    unpaid_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(v) = vendor_qbo_id {
        extra.push(format!("vendor_qbo_id=eq.{}", v));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        unpaid_only,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_bills", &q).await
}

pub async fn qbo_list_estimates(
    customer_qbo_id: Option<String>,
    status: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(c) = customer_qbo_id {
        extra.push(format!("customer_qbo_id=eq.{}", c));
    }
    if let Some(s) = status {
        extra.push(format!("status=eq.{}", s));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        None,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_estimates", &q).await
}

pub async fn qbo_list_payments(
    customer_qbo_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(c) = customer_qbo_id {
        extra.push(format!("customer_qbo_id=eq.{}", c));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        None,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_payments", &q).await
}

pub async fn qbo_list_bill_payments(
    vendor_qbo_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(v) = vendor_qbo_id {
        extra.push(format!("vendor_qbo_id=eq.{}", v));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        None,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_bill_payments", &q).await
}

pub async fn qbo_list_expenses(
    account_qbo_id: Option<String>,
    payee_qbo_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut extra = vec![];
    if let Some(a) = account_qbo_id {
        extra.push(format!("account_qbo_id=eq.{}", a));
    }
    if let Some(p) = payee_qbo_id {
        extra.push(format!("payee_qbo_id=eq.{}", p));
    }
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        None,
        &extra,
        "txn_date",
        limit,
    );
    client.select("qbo_expenses", &q).await
}

pub async fn qbo_list_journal_entries(
    from_date: Option<String>,
    to_date: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = transaction_query(
        from_date.as_deref(),
        to_date.as_deref(),
        None,
        &[],
        "txn_date",
        limit,
    );
    client.select("qbo_journal_entries", &q).await
}
