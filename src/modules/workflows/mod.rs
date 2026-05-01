// Workflow authoring module
// Wraps val-services /api/v1/workflow/... endpoints with create/update/execute.
// Reuses val_sync auth + HTTP client. Do NOT add a delete tool here — see CLAUDE.md.

use crate::core::error::{CmdResult, CommandError};
use crate::modules::val_sync::api::val_api_request;
use crate::modules::val_sync::auth;
use crate::modules::val_sync::config::get_domain_config;
use serde_json::Value;

// ============================================================================
// Internal helpers
// ============================================================================

/// Run a VAL API call with auth retry: invokes the closure with a token, and
/// if the call returns AuthExpired, refreshes the token and retries once.
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

// ============================================================================
// Plugin discovery
// ============================================================================

/// List available workflow plugin classes for a domain.
/// GET /api/v1/workflow/plugins
pub async fn list_plugins(domain: &str) -> CmdResult<Value> {
    with_auth_retry(domain, "list-val-workflow-plugins", |base_url, token| {
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "GET",
                "/api/v1/workflow/plugins",
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

/// Get the JSON schema for a plugin's `params`.
/// GET /api/v1/workflow/plugins/:name/schema
pub async fn get_plugin_schema(domain: &str, plugin_name: &str) -> CmdResult<Value> {
    let path = format!("/api/v1/workflow/plugins/{}/schema", plugin_name);
    with_auth_retry(domain, "get-val-workflow-plugin-schema", |base_url, token| {
        let path = path.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "GET",
                &path,
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

// ============================================================================
// Create / Update / Execute
// ============================================================================

/// Create a workflow.
/// POST /api/v1/workflow/?domain=<domain>
///
/// `body` must include at minimum: `name`, `data.workflow.plugins`. The
/// server fills in `data.queue` from the domain and assigns `pluginId` via
/// nanoid. If `cron_expression` is set, the workflow starts firing
/// immediately — pass null/omit for one-off workflows.
pub async fn create_workflow(domain: &str, body: Value) -> CmdResult<Value> {
    // Ensure the body carries a `domain` field — val-services prefers body.domain
    // over query param when both are present.
    let mut body = body;
    if body.get("domain").is_none() {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("domain".to_string(), Value::String(domain.to_string()));
        }
    }

    with_auth_retry(domain, "create-val-workflow", |base_url, token| {
        let body = body.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "POST",
                "/api/v1/workflow/",
                &[("domain", domain.as_str())],
                Some(body),
            )
            .await
        }
    })
    .await
}

/// Update a workflow.
///
/// - `mode = "meta"` → PATCH /api/v1/workflow/:id (only name/description/
///   cron_expression/tags are allowed by val-services `updateJobSchema`).
/// - `mode = "full"` → PUT /api/v1/workflow/:id (full replacement; `body.data`
///   must be a complete IJobDefinition including `data.queue`).
///
/// Default to "meta" — full replacements are easy to corrupt.
pub async fn update_workflow(
    domain: &str,
    id: &str,
    body: Value,
    mode: &str,
) -> CmdResult<Value> {
    let method = match mode {
        "full" => "PUT",
        "meta" | "" => "PATCH",
        other => {
            return Err(CommandError::Config(format!(
                "Invalid update mode '{}'. Use 'meta' (PATCH) or 'full' (PUT).",
                other
            )));
        }
    };
    let path = format!("/api/v1/workflow/{}", id);

    with_auth_retry(domain, "update-val-workflow", |base_url, token| {
        let body = body.clone();
        let path = path.clone();
        let method = method.to_string();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                &method,
                &path,
                &[("domain", domain.as_str())],
                Some(body),
            )
            .await
        }
    })
    .await
}

// ============================================================================
// Read-side discovery
// ============================================================================

/// List all workflows in a domain.
/// GET /api/v1/workflow/?domain=<domain>
pub async fn list_workflows(domain: &str, filters: Option<Value>) -> CmdResult<Value> {
    let mut query: Vec<(String, String)> = vec![("domain".to_string(), domain.to_string())];
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
    with_auth_retry(domain, "list-val-workflows", |base_url, token| {
        let query = query.clone();
        async move {
            let q: Vec<(&str, &str)> = query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            val_api_request(&base_url, &token, "GET", "/api/v1/workflow/", &q, None).await
        }
    })
    .await
}

/// Fetch one workflow's full job definition.
/// GET /api/v1/workflow/:id?domain=<domain>
pub async fn get_workflow(domain: &str, id: &str) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    let path = format!("/api/v1/workflow/{}", id);
    with_auth_retry(domain, "get-val-workflow", |base_url, token| {
        let path = path.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "GET",
                &path,
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

// ============================================================================
// Operational — pause / resume
// ============================================================================

pub async fn pause_workflow(domain: &str, id: &str) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    let path = format!("/api/v1/workflow/{}/pause", id);
    with_auth_retry(domain, "pause-val-workflow", |base_url, token| {
        let path = path.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "POST",
                &path,
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

pub async fn resume_workflow(domain: &str, id: &str) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    let path = format!("/api/v1/workflow/{}/resume", id);
    with_auth_retry(domain, "resume-val-workflow", |base_url, token| {
        let path = path.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "POST",
                &path,
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

// ============================================================================
// Execution history
// ============================================================================

pub async fn list_workflow_executions(
    domain: &str,
    filters: Option<Value>,
) -> CmdResult<Value> {
    let mut query: Vec<(String, String)> = vec![("domain".to_string(), domain.to_string())];
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
    with_auth_retry(domain, "list-val-workflow-executions", |base_url, token| {
        let query = query.clone();
        async move {
            let q: Vec<(&str, &str)> = query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            val_api_request(&base_url, &token, "GET", "/api/v1/workflow/executions", &q, None).await
        }
    })
    .await
}

pub async fn get_workflow_execution(domain: &str, id: &str) -> CmdResult<Value> {
    if id.trim().is_empty() {
        return Err(CommandError::Config("'id' cannot be empty".to_string()));
    }
    let path = format!("/api/v1/workflow/executions/{}", id);
    with_auth_retry(domain, "get-val-workflow-execution", |base_url, token| {
        let path = path.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "GET",
                &path,
                &[("domain", domain.as_str())],
                None,
            )
            .await
        }
    })
    .await
}

/// Execute (rerun) a workflow.
///
/// - No `overrides` → POST /api/v1/workflow/:id/rerun (uses saved definition).
/// - `overrides` set → POST /api/v1/workflow/:id/rerun_unsaved (run with
///   in-memory edits to data/cron without persisting).
pub async fn execute_workflow(
    domain: &str,
    id: &str,
    overrides: Option<Value>,
) -> CmdResult<Value> {
    let (path, body) = match overrides {
        Some(b) => (format!("/api/v1/workflow/{}/rerun_unsaved", id), Some(b)),
        None => (format!("/api/v1/workflow/{}/rerun", id), None),
    };

    with_auth_retry(domain, "execute-val-workflow", |base_url, token| {
        let path = path.clone();
        let body = body.clone();
        let domain = domain.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "POST",
                &path,
                &[("domain", domain.as_str())],
                body,
            )
            .await
        }
    })
    .await
}
