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

// ============================================================================
// Bundled orchestrator — promote multiple resource types in one call.
// ============================================================================

#[derive(Default, Clone)]
pub struct PromoteRequest {
    pub tables: Option<Vec<String>>,
    pub workflows: Option<Vec<String>>,
    pub dashboards: Option<Vec<String>>,
    pub include_queries: Option<bool>,
    pub space_ids: Option<Vec<i64>>,
    pub zone_ids: Option<Vec<i64>>,
    pub instance_id: Option<String>,
    pub system_id: Option<String>,
    pub system_type: Option<String>,
    pub override_creator: Option<i64>,
    pub poll_interval_secs: Option<u64>,
    pub poll_timeout_secs: Option<u64>,
    pub wait_for_completion: bool,
}

/// Promote multiple resource types from `source` to `target` in one call.
/// Order: tables → workflows → dashboards (matches val-services dependency order).
/// Polls each step until done by default; pass `wait_for_completion: false` to fire-and-forget.
pub async fn promote_resources(
    source: &str,
    target: &str,
    req: PromoteRequest,
) -> CmdResult<Value> {
    if source.trim().is_empty() || target.trim().is_empty() {
        return Err(CommandError::Config(
            "'source' and 'target' cannot be empty".to_string(),
        ));
    }
    if source == target {
        return Err(CommandError::Config(
            "'source' and 'target' must be different domains".to_string(),
        ));
    }

    let total_resources: usize = [
        req.tables.as_ref().map(|v| v.len()).unwrap_or(0),
        req.workflows.as_ref().map(|v| v.len()).unwrap_or(0),
        req.dashboards.as_ref().map(|v| v.len()).unwrap_or(0),
    ]
    .iter()
    .sum();
    if total_resources == 0 {
        return Err(CommandError::Config(
            "supply at least one of 'tables', 'workflows', 'dashboards'".to_string(),
        ));
    }

    let poll_interval =
        std::time::Duration::from_secs(req.poll_interval_secs.unwrap_or(2));
    let poll_timeout =
        std::time::Duration::from_secs(req.poll_timeout_secs.unwrap_or(120));

    let mut steps: Vec<Value> = Vec::new();

    // Run each non-empty resource type in order.
    for (resource_type, resource_ids_opt) in [
        ("tables", req.tables.as_ref()),
        ("workflows", req.workflows.as_ref()),
        ("dashboards", req.dashboards.as_ref()),
    ] {
        let Some(resource_ids) = resource_ids_opt else { continue };
        if resource_ids.is_empty() {
            continue;
        }
        let include_queries = if resource_type == "dashboards" {
            req.include_queries
        } else {
            None
        };

        let kickoff = sync_val_domain(
            source,
            target,
            resource_type,
            resource_ids.clone(),
            req.space_ids.clone(),
            req.zone_ids.clone(),
            req.instance_id.as_deref(),
            req.system_id.as_deref(),
            req.system_type.as_deref(),
            req.override_creator,
            include_queries,
        )
        .await;

        let mut step = json!({
            "resource_type": resource_type,
            "resource_count": resource_ids.len(),
        });
        let step_obj = step.as_object_mut().unwrap();

        match kickoff {
            Err(e) => {
                step_obj.insert("status".to_string(), Value::String("error".to_string()));
                step_obj.insert("error".to_string(), Value::String(e.to_string()));
                steps.push(step);
                // Stop the chain — later steps may depend on earlier ones.
                return Ok(json!({
                    "source": source,
                    "target": target,
                    "steps": steps,
                    "completed": false,
                    "error": format!("{} sync failed; later steps skipped", resource_type),
                }));
            }
            Ok(kickoff_resp) => {
                step_obj.insert("kickoff".to_string(), kickoff_resp.clone());

                let sync_uuid = kickoff_resp
                    .get("sync_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let initial_status = kickoff_resp
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("queued")
                    .to_string();

                if !req.wait_for_completion {
                    step_obj.insert("status".to_string(), Value::String(initial_status));
                    step_obj.insert("waited".to_string(), Value::Bool(false));
                    steps.push(step);
                    continue;
                }

                if initial_status == "done" || initial_status == "success" {
                    step_obj.insert("status".to_string(), Value::String(initial_status));
                    step_obj.insert("waited".to_string(), Value::Bool(false));
                    steps.push(step);
                    continue;
                }

                if sync_uuid.is_empty() {
                    step_obj.insert("status".to_string(), Value::String(initial_status));
                    step_obj.insert(
                        "warning".to_string(),
                        Value::String(
                            "no sync_uuid returned — skipping poll".to_string(),
                        ),
                    );
                    steps.push(step);
                    continue;
                }

                // Poll get-val-sync-status until done or timeout.
                let started = std::time::Instant::now();
                let mut final_status = initial_status.clone();
                let mut final_payload = Value::Null;
                let mut polled = 0usize;
                loop {
                    if started.elapsed() > poll_timeout {
                        step_obj.insert(
                            "status".to_string(),
                            Value::String("timeout".to_string()),
                        );
                        step_obj.insert(
                            "error".to_string(),
                            Value::String(format!(
                                "exceeded {}s timeout while polling",
                                poll_timeout.as_secs()
                            )),
                        );
                        step_obj.insert("polls".to_string(), json!(polled));
                        steps.push(step);
                        return Ok(json!({
                            "source": source,
                            "target": target,
                            "steps": steps,
                            "completed": false,
                            "error": format!("{} sync timed out", resource_type),
                        }));
                    }
                    tokio::time::sleep(poll_interval).await;
                    polled += 1;
                    match get_val_sync_status(source, &sync_uuid).await {
                        Ok(payload) => {
                            // The status endpoint returns { results: [...] } where each
                            // entry has a `status` field. We're done when every entry
                            // is no longer 'pending'/'syncing'/'queued'.
                            let results = payload
                                .get("results")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let is_terminal = !results.is_empty()
                                && results.iter().all(|r| {
                                    let s = r
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    !matches!(s, "pending" | "syncing" | "queued" | "running")
                                });
                            if is_terminal {
                                let any_failed = results.iter().any(|r| {
                                    matches!(
                                        r.get("status").and_then(|v| v.as_str()).unwrap_or(""),
                                        "error" | "failed"
                                    )
                                });
                                final_status =
                                    if any_failed { "error".to_string() } else { "success".to_string() };
                                final_payload = payload;
                                break;
                            }
                            final_payload = payload;
                        }
                        Err(e) => {
                            // Transient errors — log but keep polling until timeout.
                            final_payload = json!({ "poll_error": e.to_string() });
                        }
                    }
                }
                step_obj.insert("status".to_string(), Value::String(final_status.clone()));
                step_obj.insert("polls".to_string(), json!(polled));
                step_obj.insert("results".to_string(), final_payload);
                steps.push(step);

                if final_status == "error" {
                    return Ok(json!({
                        "source": source,
                        "target": target,
                        "steps": steps,
                        "completed": false,
                        "error": format!("{} sync reported errors; later steps skipped", resource_type),
                    }));
                }
            }
        }
    }

    Ok(json!({
        "source": source,
        "target": target,
        "steps": steps,
        "completed": true,
    }))
}
