// VAL Dashboards module — wraps val-services /db/dashboard/v1/... endpoints.
// No delete tool by design (policy).

use crate::core::error::{CmdResult, CommandError};
use crate::modules::val_sync::api::val_api_request;
use crate::modules::val_sync::auth;
use crate::modules::val_sync::config::get_domain_config;
use serde_json::{json, Value};

async fn with_auth_retry<F, Fut>(domain: &str, op: &str, f: F) -> CmdResult<Value>
where
    F: Fn(String, String) -> Fut,
    Fut: std::future::Future<
        Output = Result<Value, crate::modules::val_sync::api::ValApiError>,
    >,
{
    let domain_config = get_domain_config(domain)?;
    let base_url = format!("https://{}.thinkval.io", domain_config.api_domain());

    let (token, _) = auth::ensure_auth(domain).await?;
    match f(base_url.clone(), token).await {
        Ok(v) => Ok(v),
        Err(e) if e.is_auth_error() => {
            let (new_token, _) = auth::reauth(domain).await?;
            f(base_url, new_token).await.map_err(|e| {
                CommandError::Network(format!("{} failed after reauth: {}", op, e))
            })
        }
        Err(e) => Err(CommandError::Network(format!("{} failed: {}", op, e))),
    }
}

async fn get_json_with_query(
    domain: &str,
    op: &str,
    path: &str,
    query: Vec<(String, String)>,
) -> CmdResult<Value> {
    let path = path.to_string();
    with_auth_retry(domain, op, |base_url, token| {
        let path = path.clone();
        let query = query.clone();
        async move {
            let q: Vec<(&str, &str)> = query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            val_api_request(&base_url, &token, "GET", &path, &q, None).await
        }
    })
    .await
}

async fn post_json(domain: &str, op: &str, path: &str, body: Value) -> CmdResult<Value> {
    let path = path.to_string();
    with_auth_retry(domain, op, |base_url, token| {
        let body = body.clone();
        let path = path.clone();
        async move {
            val_api_request(&base_url, &token, "POST", &path, &[], Some(body)).await
        }
    })
    .await
}

pub async fn list_dashboards(domain: &str, filters: Option<Value>) -> CmdResult<Value> {
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(Value::Object(map)) = filters {
        for (k, v) in map.into_iter() {
            let s = match v {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            query.push((k, s));
        }
    }
    get_json_with_query(domain, "list-val-dashboards", "/db/dashboard/v1/listAllDashboards", query).await
}

pub async fn get_dashboard(domain: &str, id: &str) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    let path = format!("/db/dashboard/v1/getDashboard/{}", id);
    get_json_with_query(domain, "get-val-dashboard", &path, vec![]).await
}

/// `dashboard` is the full saveDashboard payload — for create, omit `id` (or set it null).
pub async fn create_dashboard(domain: &str, dashboard: Value) -> CmdResult<Value> {
    if !dashboard.is_object() {
        return Err(CommandError::Config(
            "'dashboard' must be an object — full saveDashboard payload (basicInfo, dashboardInfo, etc.).".to_string(),
        ));
    }
    let body = json!({ "dashboard": dashboard });
    post_json(domain, "create-val-dashboard", "/db/dashboard/v1/saveDashboard", body).await
}

/// Update existing dashboard. saveDashboard does a full INSERT/UPDATE keyed by id —
/// fetch via `get-val-dashboard` first and merge client-side.
pub async fn update_dashboard(domain: &str, id: &str, dashboard: Value) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    if !dashboard.is_object() {
        return Err(CommandError::Config(
            "'dashboard' must be an object".to_string(),
        ));
    }
    let mut dashboard = dashboard;
    if let Some(obj) = dashboard.as_object_mut() {
        obj.insert("id".to_string(), Value::String(id.to_string()));
    }
    let body = json!({ "dashboard": dashboard });
    post_json(domain, "update-val-dashboard", "/db/dashboard/v1/saveDashboard", body).await
}

pub async fn duplicate_dashboard(
    domain: &str,
    source_id: &str,
    new_name: Option<&str>,
) -> CmdResult<Value> {
    if source_id.trim().is_empty() {
        return Err(CommandError::Config("'source_id' cannot be empty".to_string()));
    }
    let mut body_obj = serde_json::Map::new();
    body_obj.insert("id".to_string(), Value::String(source_id.to_string()));
    if let Some(n) = new_name {
        body_obj.insert("name".to_string(), Value::String(n.to_string()));
    }
    let body = json!({ "dashboard": Value::Object(body_obj) });
    post_json(domain, "duplicate-val-dashboard", "/db/dashboard/v1/duplicateDashboard", body).await
}
