// VAL admin module
// Wraps val-services /db/admin-projects, /db/admin-phase, /db/admin-repoTable
// for space / zone / table create + update.
//
// All write endpoints proxy to the internal admin-service which does
// isActionAllowed checks per (entity, action). If a call returns 401/403, the
// bot user lacks grants on that entity — surface the raw error so the caller
// can see which scope is missing.

use crate::core::error::{CmdResult, CommandError};
use crate::modules::val_sync::api::val_api_request;
use crate::modules::val_sync::auth;
use crate::modules::val_sync::config::get_domain_config;
use serde_json::{json, Value};

// ============================================================================
// Internal helper — auth retry
// ============================================================================

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

// ============================================================================
// Spaces (UI: "Project")
// ============================================================================

pub async fn create_space(domain: &str, name: &str, description: Option<&str>) -> CmdResult<Value> {
    if name.trim().is_empty() {
        return Err(CommandError::Config("'name' cannot be empty".to_string()));
    }
    let body = json!({
        "project": {
            "project_name": name,
            "project_desc": description.unwrap_or(""),
        }
    });
    post_json(domain, "create-val-space", "/db/admin-projects/addProject", body).await
}

pub async fn update_space(domain: &str, space_id: &str, updates: Value) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    let mut project = updates;
    if let Some(obj) = project.as_object_mut() {
        obj.insert("project_id".to_string(), Value::String(space_id.to_string()));
    }
    let body = json!({ "project": project });
    post_json(domain, "update-val-space", "/db/admin-projects/updateProject", body).await
}

// ============================================================================
// Zones (UI: "Phase")
// ============================================================================

pub async fn create_zone(
    domain: &str,
    space_id: &str,
    name: &str,
    description: Option<&str>,
) -> CmdResult<Value> {
    if name.trim().is_empty() {
        return Err(CommandError::Config("'name' cannot be empty".to_string()));
    }
    // val-services reads `value` (cast to int) and writes it to `phase_pr_id`.
    // We send both for safety — `phase_pr_id` matches the entitlement check key,
    // `value` is what the SQL insert reads.
    let body = json!({
        "phase": {
            "phase_name": name,
            "phase_desc": description.unwrap_or(""),
            "value": space_id,
            "phase_pr_id": space_id,
            "repo_phase_data": [],
        }
    });
    post_json(domain, "create-val-zone", "/db/admin-phase/addPhase", body).await
}

pub async fn update_zone(
    domain: &str,
    zone_id: &str,
    space_id: &str,
    updates: Value,
) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    let mut phase = updates;
    if let Some(obj) = phase.as_object_mut() {
        obj.insert("phase_id".to_string(), Value::String(zone_id.to_string()));
        // The auth check needs phase_pr_id (the parent space id) on the body.
        obj.entry("phase_pr_id".to_string())
            .or_insert_with(|| Value::String(space_id.to_string()));
    }
    let body = json!({ "phase": phase });
    post_json(domain, "update-val-zone", "/db/admin-phase/updatePhase", body).await
}

// ============================================================================
// Tables (UI: "Repo Table")
// ============================================================================

pub async fn create_table(
    domain: &str,
    zone_id: &str,
    name: &str,
    description: Option<&str>,
    prefix: Option<&str>,
    repo_type: Option<&str>,
    extras: Option<Value>,
) -> CmdResult<Value> {
    if name.trim().is_empty() {
        return Err(CommandError::Config("'name' cannot be empty".to_string()));
    }
    let mut repo_table = json!({
        "name": name,
        "description": description.unwrap_or(""),
        "prefix": prefix.unwrap_or(""),
        "value": zone_id,
        "repo_type": repo_type.unwrap_or("general"),
        "autocalculate": false,
        "populated_dates": false,
        "metadata": {},
    });
    if let Some(extra) = extras {
        if let (Some(target), Some(extra_obj)) = (repo_table.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }
    let body = json!({ "repoTable": repo_table });
    post_json(domain, "create-val-table", "/db/admin-repoTable/addRepoTable", body).await
}

// ============================================================================
// Fields — single + bulk add, update, assign-to-zone
// ============================================================================

/// Add a single new field (column) to a table.
/// POST /db/admin-repoTable/updateTableFields
///
/// `data_type` accepts the canonical VAL types: `text`, `number`, `decimal`,
/// `date`, `boolean`/`checkbox`, `select`, `chips`, `linked_text`,
/// `linked_select`, `linked_multiselect`, `person`, `multiperson`,
/// `attachment`, `url`.
///
/// For linked types, pass `link_options` with `linked_table`, `linked_field`,
/// `linked_field_display`, etc. — see val-services
/// `RepoType.updateRepositoryTableFields` for the full set.
pub async fn add_table_field(
    domain: &str,
    table_id: &str,
    name: &str,
    data_type: &str,
    extras: Option<Value>,
    link_options: Option<Value>,
) -> CmdResult<Value> {
    if name.trim().is_empty() {
        return Err(CommandError::Config("'name' cannot be empty".to_string()));
    }
    if data_type.trim().is_empty() {
        return Err(CommandError::Config("'data_type' cannot be empty".to_string()));
    }
    let mut repo_table = json!({
        "value": table_id,
        "name": name,
        "data_type": data_type,
        "category": "General",
    });
    if let Some(extra) = extras {
        if let (Some(target), Some(extra_obj)) = (repo_table.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }
    if let Some(link) = link_options {
        if let (Some(target), Some(link_obj)) = (repo_table.as_object_mut(), link.as_object()) {
            for (k, v) in link_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }
    let body = json!({ "repoTable": repo_table });
    post_json(domain, "add-val-table-field", "/db/admin-repoTable/updateTableFields", body).await
}

/// Add multiple new fields to a table in a single transaction.
/// POST /api/1/tables/:tableId/fields/add
///
/// Each entry in `fields` should follow the same shape as `add_table_field`'s
/// repoTable body (minus `value`, which is supplied by the path param):
/// `{ name, data_type, category?, desc?, predefined_values?, ... }`.
pub async fn add_table_fields_bulk(
    domain: &str,
    table_id: &str,
    fields: Vec<Value>,
) -> CmdResult<Value> {
    if fields.is_empty() {
        return Err(CommandError::Config("'fields' must contain at least one field".to_string()));
    }
    let path = format!("/api/1/tables/{}/fields/add", table_id);
    let body = json!({ "fields": fields });
    post_json(domain, "add-val-table-fields", &path, body).await
}

/// Update an existing field's metadata (name, description, predefined values, etc.).
/// POST /db/admin-fields/updateField
///
/// `updates` must include enough identification — at minimum `id` (or
/// `dft_nodefields_id`), `column_name`, and `value` (= table id) — plus the
/// fields to change. Other recognized keys: `name`, `data_type`, `desc`,
/// `category`, `column_length`, `colour`, `predefined_values`, `table_name`.
pub async fn update_field(domain: &str, updates: Value) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    post_json(domain, "update-val-field", "/db/admin-fields/updateField", updates).await
}

/// Move a set of tables to a different zone.
/// POST /db/admin-phase/updateTableAssignment
///
/// `tables` is an array of table ids/objects — val-services groups them by
/// repo type internally before reassigning, so passing the full repo-type
/// objects is safest. Most callers pass `[{ id: <repo_type_id>, value: [<table_id>, ...] }, ...]`.
pub async fn assign_table_to_zone(
    domain: &str,
    zone_id: &str,
    tables: Vec<Value>,
) -> CmdResult<Value> {
    if tables.is_empty() {
        return Err(CommandError::Config("'tables' must contain at least one entry".to_string()));
    }
    let body = json!({
        "details": {
            "phaseId": zone_id,
            "tables": tables,
        }
    });
    post_json(domain, "assign-val-table-to-zone", "/db/admin-phase/updateTableAssignment", body).await
}

// ============================================================================
// Queries (DSQuery / querybuilder_master)
// ============================================================================

/// Create a new VAL query (datasource).
/// POST /db/data/v1/createDSQuery
///
/// `datasource` is the nested query config (`{ basicInfo, queryInfo: { tableInfo, fields, filters, joins, ... } }`).
/// Cloning the shape of a synced query in `tv-knowledge/.../queries/<id>/definition.json`
/// is the easiest starting point.
pub async fn create_query(
    domain: &str,
    name: &str,
    datasource: Value,
    extras: Option<Value>,
) -> CmdResult<Value> {
    if name.trim().is_empty() {
        return Err(CommandError::Config("'name' cannot be empty".to_string()));
    }
    if !datasource.is_object() {
        return Err(CommandError::Config(
            "'datasource' must be an object".to_string(),
        ));
    }
    let mut body = json!({
        "name": name,
        "category": "private",
        "datasource": datasource,
        "permission": {},
    });
    if let Some(extra) = extras {
        if let (Some(target), Some(extra_obj)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }
    post_json(domain, "create-val-query", "/db/data/v1/createDSQuery", body).await
}

/// Update an existing VAL query.
/// POST /db/data/v1/saveDSQuery
///
/// The query id MUST be passed as `dsid` (val-services uses that name, not `id`).
/// Pass the full `datasource` to replace it; partial updates aren't supported by
/// this endpoint — it does an INSERT with the same id (versioning by row history).
pub async fn update_query(
    domain: &str,
    dsid: &str,
    updates: Value,
) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    let mut body = updates;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("dsid".to_string(), Value::String(dsid.to_string()));
    }
    post_json(domain, "update-val-query", "/db/data/v1/saveDSQuery", body).await
}

/// Copy an existing query into a new one with a different name.
/// POST /db/data/v1/copyDSQuery
///
/// Pass the full `datasource` from the source query — val-services renames it
/// internally and writes a new row with a fresh id. Returns `{ dsid: <new_id>, ... }`.
pub async fn copy_query(
    domain: &str,
    new_name: &str,
    source_datasource: Value,
    extras: Option<Value>,
) -> CmdResult<Value> {
    if new_name.trim().is_empty() {
        return Err(CommandError::Config("'new_name' cannot be empty".to_string()));
    }
    if !source_datasource.is_object() {
        return Err(CommandError::Config(
            "'source_datasource' must be an object".to_string(),
        ));
    }
    let mut body = json!({
        "name": new_name,
        "category": "private",
        "datasource": source_datasource,
    });
    if let Some(extra) = extras {
        if let (Some(target), Some(extra_obj)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }
    post_json(domain, "copy-val-query", "/db/data/v1/copyDSQuery", body).await
}

pub async fn clone_table(
    domain: &str,
    source_table_id: &str,
    zone_id: &str,
    new_name: &str,
    new_prefix: Option<&str>,
) -> CmdResult<Value> {
    if new_name.trim().is_empty() {
        return Err(CommandError::Config("'new_name' cannot be empty".to_string()));
    }
    // Clone uses the same body wrapper as create. `value` = target zone (auth
    // check entity); `parentId`/source table id passed for the helper to find
    // the original definition.
    let body = json!({
        "repoTable": {
            "name": new_name,
            "prefix": new_prefix.unwrap_or(""),
            "value": zone_id,
            "parentId": source_table_id,
            "source_table": source_table_id,
            "metadata": {},
        }
    });
    post_json(domain, "clone-val-table", "/db/admin-repoTable/cloneTable", body).await
}
