// QBO master-data reads — accounts, customers, vendors, items.

use super::MGMT_WORKSPACE_ID;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client_for_workspace;
use serde_json::Value;

fn build_list_query(
    search: Option<&str>,
    search_cols: &[&str],
    active_only: bool,
    order_col: &str,
    limit: Option<i32>,
) -> String {
    let mut parts = vec![];
    if let Some(s) = search {
        let s = s.replace('*', "");
        let ors: Vec<String> = search_cols
            .iter()
            .map(|c| format!("{}.ilike.*{}*", c, s))
            .collect();
        parts.push(format!("or=({})", ors.join(",")));
    }
    if active_only {
        parts.push("active=eq.true".to_string());
    }
    parts.push(format!("order={}.asc", order_col));
    parts.push(format!("limit={}", limit.unwrap_or(200)));
    parts.join("&")
}

pub async fn qbo_list_accounts(
    search: Option<String>,
    account_type: Option<String>,
    active_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let mut q = build_list_query(
        search.as_deref(),
        &["name"],
        active_only.unwrap_or(true),
        "name",
        limit,
    );
    if let Some(t) = account_type {
        q.push_str(&format!("&account_type=eq.{}", t));
    }
    client.select("qbo_accounts", &q).await
}

pub async fn qbo_list_customers(
    search: Option<String>,
    active_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = build_list_query(
        search.as_deref(),
        &["display_name", "company_name", "email"],
        active_only.unwrap_or(true),
        "display_name",
        limit,
    );
    client.select("qbo_customers", &q).await
}

pub async fn qbo_list_vendors(
    search: Option<String>,
    active_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = build_list_query(
        search.as_deref(),
        &["display_name", "company_name", "email"],
        active_only.unwrap_or(true),
        "display_name",
        limit,
    );
    client.select("qbo_vendors", &q).await
}

pub async fn qbo_list_items(
    search: Option<String>,
    active_only: Option<bool>,
    limit: Option<i32>,
) -> CmdResult<Vec<Value>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let q = build_list_query(
        search.as_deref(),
        &["name"],
        active_only.unwrap_or(true),
        "name",
        limit,
    );
    client.select("qbo_items", &q).await
}
