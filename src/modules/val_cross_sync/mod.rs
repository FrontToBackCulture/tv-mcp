// VAL Cross-Domain Sync — promote tables/workflows/dashboards between domains.
// Wraps the `sync-domain` Supabase edge function (which calls val-services
// /api/v1/sync) so we get solution_sync_jobs audit trail and the
// resource_type shorthand for free. Status queries call val-services directly.

use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;
use crate::modules::val_sync::api::val_api_request;
use crate::modules::val_sync::auth;
use crate::modules::val_sync::config::get_domain_config;
use serde_json::{json, Value};

/// Invoke the workspace `sync-domain` edge function.
async fn invoke_sync_domain(body: Value) -> CmdResult<Value> {
    let client = get_client().await?;
    let url = format!("{}/functions/v1/sync-domain", client.base_url());

    let res = client
        .http_client()
        .post(&url)
        .headers(client.auth_headers())
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = res.status();
    let bytes = res.bytes().await?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes).to_string();
        return Err(CommandError::Http {
            status: status.as_u16(),
            body,
        });
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|e| CommandError::Network(format!("parse: {}", e)))
}

#[allow(clippy::too_many_arguments)]
pub async fn sync_val_domain(
    source: &str,
    target: &str,
    resource_type: &str,
    resource_ids: Vec<String>,
    space_ids: Option<Vec<i64>>,
    zone_ids: Option<Vec<i64>>,
    instance_id: Option<&str>,
    system_id: Option<&str>,
    system_type: Option<&str>,
    override_creator: Option<i64>,
    include_queries: Option<bool>,
) -> CmdResult<Value> {
    if source.trim().is_empty() {
        return Err(CommandError::Config("'source' cannot be empty".to_string()));
    }
    if target.trim().is_empty() {
        return Err(CommandError::Config("'target' cannot be empty".to_string()));
    }
    if source == target {
        return Err(CommandError::Config(
            "'source' and 'target' must be different domains".to_string(),
        ));
    }
    match resource_type {
        "tables" | "workflows" | "dashboards" => {}
        other => {
            return Err(CommandError::Config(format!(
                "'resource_type' must be 'tables', 'workflows', or 'dashboards' (got '{}')",
                other
            )));
        }
    }
    if resource_ids.is_empty() {
        return Err(CommandError::Config(
            "'resource_ids' must contain at least one id".to_string(),
        ));
    }

    let mut body = json!({
        "source": source,
        "target": target,
        "resource_type": resource_type,
        "resource_ids": resource_ids,
    });
    if let Some(obj) = body.as_object_mut() {
        if let Some(v) = space_ids {
            obj.insert("space_ids".to_string(), json!(v));
        }
        if let Some(v) = zone_ids {
            obj.insert("zone_ids".to_string(), json!(v));
        }
        if let Some(v) = instance_id {
            obj.insert("instance_id".to_string(), Value::String(v.to_string()));
        }
        if let Some(v) = system_id {
            obj.insert("system_id".to_string(), Value::String(v.to_string()));
        }
        if let Some(v) = system_type {
            obj.insert("system_type".to_string(), Value::String(v.to_string()));
        }
        if let Some(v) = override_creator {
            obj.insert("override_creator".to_string(), json!(v));
        }
        if let Some(v) = include_queries {
            obj.insert("include_queries".to_string(), json!(v));
        }
    }

    invoke_sync_domain(body).await
}

/// Poll val-services for a sync job's progress. Status is tracked on the
/// SOURCE domain side (where the sync was kicked off).
pub async fn get_val_sync_status(source: &str, sync_id: &str) -> CmdResult<Value> {
    if source.trim().is_empty() {
        return Err(CommandError::Config("'source' cannot be empty".to_string()));
    }
    if sync_id.trim().is_empty() {
        return Err(CommandError::Config("'sync_id' cannot be empty".to_string()));
    }

    let domain_config = get_domain_config(source)?;
    let base_url = format!("https://{}.thinkval.io", domain_config.api_domain());

    let send = |token: String| {
        let base_url = base_url.clone();
        let sync_id = sync_id.to_string();
        async move {
            val_api_request(
                &base_url,
                &token,
                "GET",
                "/api/v1/sync/status",
                &[("id", sync_id.as_str())],
                None,
            )
            .await
        }
    };

    let (token, _) = auth::ensure_auth(source).await?;
    match send(token).await {
        Ok(v) => Ok(v),
        Err(e) if e.is_auth_error() => {
            let (new_token, _) = auth::reauth(source).await?;
            send(new_token).await.map_err(|e| {
                CommandError::Network(format!("get-val-sync-status failed after reauth: {}", e))
            })
        }
        Err(e) => Err(CommandError::Network(format!("get-val-sync-status failed: {}", e))),
    }
}
