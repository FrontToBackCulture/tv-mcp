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

async fn get_json(domain: &str, op: &str, path: &str) -> CmdResult<Value> {
    let path = path.to_string();
    with_auth_retry(domain, op, |base_url, token| {
        let path = path.clone();
        async move {
            val_api_request(&base_url, &token, "GET", &path, &[], None).await
        }
    })
    .await
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

pub async fn list_spaces(domain: &str) -> CmdResult<Value> {
    get_json(domain, "list-val-spaces", "/db/admin-projects/getProjects").await
}

pub async fn get_space(domain: &str, space_id: &str) -> CmdResult<Value> {
    if space_id.trim().is_empty() {
        return Err(CommandError::Config("'space_id' cannot be empty".to_string()));
    }
    // The /api/v1/spaces/:id/info backend route is not gateway-proxied.
    // Compose by listing all spaces and filtering client-side.
    let needle: i64 = space_id.parse().map_err(|_| {
        CommandError::Config(format!("'space_id' must be numeric (got '{}')", space_id))
    })?;
    let spaces = list_spaces(domain).await?;
    if let Value::Array(arr) = spaces {
        for sp in arr {
            if sp.get("project_id").and_then(|v| v.as_i64()) == Some(needle) {
                return Ok(sp);
            }
        }
    }
    Err(CommandError::Network(format!(
        "Space {} not found in domain '{}'",
        space_id, domain
    )))
}

pub async fn list_space_zones(domain: &str, space_id: &str) -> CmdResult<Value> {
    if space_id.trim().is_empty() {
        return Err(CommandError::Config("'space_id' cannot be empty".to_string()));
    }
    let path = format!("/db/admin-projects/getProjectDetails/{}", space_id);
    get_json(domain, "list-val-space-zones", &path).await
}

pub async fn list_zones(domain: &str, _filters: Option<Value>) -> CmdResult<Value> {
    // The /api/v1/admin/zones backend route is not gateway-proxied.
    // Compose by listing all spaces and concatenating zones from each.
    // (The `filters` arg is currently a no-op — call sites should filter the
    // returned array client-side until a server-side endpoint is exposed.)
    let spaces = list_spaces(domain).await?;
    let mut all_zones: Vec<Value> = Vec::new();
    if let Value::Array(arr) = spaces {
        for sp in arr {
            if let Some(id) = sp.get("project_id").and_then(|v| v.as_i64()) {
                let space_id = id.to_string();
                if let Ok(zones) = list_space_zones(domain, &space_id).await {
                    if let Value::Array(zone_arr) = zones {
                        for z in zone_arr {
                            all_zones.push(z);
                        }
                    }
                }
            }
        }
    }
    Ok(Value::Array(all_zones))
}

pub async fn get_zone(domain: &str, zone_id: &str) -> CmdResult<Value> {
    if zone_id.trim().is_empty() {
        return Err(CommandError::Config("'zone_id' cannot be empty".to_string()));
    }
    let path = format!("/db/zones/{}/info", zone_id);
    get_json(domain, "get-val-zone", &path).await
}

pub async fn list_zone_tables(domain: &str, zone_id: &str) -> CmdResult<Value> {
    if zone_id.trim().is_empty() {
        return Err(CommandError::Config("'zone_id' cannot be empty".to_string()));
    }
    let path = format!("/db/admin-phase/getPhaseDetails/{}", zone_id);
    get_json(domain, "list-val-zone-tables", &path).await
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
        // val-services Phase.updateZone fires movePerspectiveToSpace whenever
        // `phase_pr_id !== value` — and that path attempts to read JSON
        // columns that are null on freshly-created zones, throwing
        // "invalid input syntax for type json". Default `value` to the same
        // space id so the move is skipped unless the caller explicitly
        // overrides it (i.e. they actually want to move the zone).
        let pr_id = obj.get("phase_pr_id").cloned();
        if let Some(pr) = pr_id {
            obj.entry("value".to_string()).or_insert(pr);
        }
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

pub async fn update_table(domain: &str, table_id: &str, updates: Value) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    // val-services updateRepoTable runs a literal SQL UPDATE that writes EVERY
    // column on `repoTable` — any key the caller omits gets persisted as the
    // literal string "undefined" (same backend pattern as update_linkage,
    // update_zone, update_space). Fetch the current row and overlay the
    // caller's updates so partial payloads are safe.
    let current = get_table(domain, table_id).await?;
    let mut repo_table = current;
    if let (Some(merged_obj), Some(updates_obj)) =
        (repo_table.as_object_mut(), updates.as_object())
    {
        for (k, v) in updates_obj {
            merged_obj.insert(k.clone(), v.clone());
        }
    }
    if let Some(obj) = repo_table.as_object_mut() {
        // val-services /api/v1/tables/update needs three identifying fields:
        //   - `table_name` and `value` — the physical custom_tbl_<repo>_<id>
        //     used by the permission check + the SQL update target.
        //   - `id` — the trailing numeric table id (last segment), used by
        //     updateRepositoryTable for the row update; without it the
        //     handler throws "ID cannot be blank."
        obj.insert("value".to_string(), Value::String(table_id.to_string()));
        obj.entry("table_name".to_string())
            .or_insert_with(|| Value::String(table_id.to_string()));
        if !obj.contains_key("id") {
            if let Some(numeric) = table_id.rsplit('_').next() {
                if let Ok(n) = numeric.parse::<i64>() {
                    obj.insert("id".to_string(), Value::Number(n.into()));
                }
            }
        }
    }
    let body = json!({ "repoTable": repo_table });
    post_json(domain, "update-val-table", "/db/admin-repoTable/updateRepoTable", body).await
}

pub async fn get_table(domain: &str, table_id: &str) -> CmdResult<Value> {
    if table_id.trim().is_empty() {
        return Err(CommandError::Config("'table_id' cannot be empty".to_string()));
    }
    // /db/admin-repoType/getRepoTableDetails returns null in practice.
    // Compose by scanning all tables across all spaces+zones, matching by
    // tablename. Slow on large domains — call sites that already know the
    // zone should prefer list-val-zone-tables and filter directly.
    let needle = table_id.to_string();
    let tables = list_tables(domain, None).await?;
    if let Value::Array(arr) = tables {
        for t in arr {
            let tablename = t
                .get("tablename")
                .or_else(|| t.get("table_name"))
                .or_else(|| t.get("table"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if tablename == needle {
                return Ok(t);
            }
        }
    }
    Err(CommandError::Network(format!(
        "Table '{}' not found in domain '{}'",
        table_id, domain
    )))
}

pub async fn list_tables(domain: &str, _filters: Option<Value>) -> CmdResult<Value> {
    // The /api/v1/admin/tables backend route is not gateway-proxied.
    // Compose by walking spaces → zones → zone-tables. Dedupe by tablename
    // (a table can surface under multiple zones via phase_repo_tbl).
    // (The `filters` arg is currently a no-op — call sites should filter
    // the returned array client-side.)
    let zones = list_zones(domain, None).await?;
    let mut all_tables: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Value::Array(zone_arr) = zones {
        for z in zone_arr {
            if let Some(zid) = z.get("phase_id").and_then(|v| v.as_i64()) {
                let zone_id = zid.to_string();
                if let Ok(zts) = list_zone_tables(domain, &zone_id).await {
                    if let Value::Array(table_arr) = zts {
                        for t in table_arr {
                            let key = t
                                .get("tablename")
                                .or_else(|| t.get("table_name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !key.is_empty() && seen.insert(key) {
                                all_tables.push(t);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(Value::Array(all_tables))
}

pub async fn list_table_dependencies(domain: &str, table_id: &str) -> CmdResult<Value> {
    if table_id.trim().is_empty() {
        return Err(CommandError::Config("'table_id' cannot be empty".to_string()));
    }
    let path = format!("/db/dependencies/{}", table_id);
    get_json(domain, "list-val-table-dependencies", &path).await
}

pub async fn remove_table_field(
    domain: &str,
    table_id: &str,
    zone_id: &str,
    field: Value,
) -> CmdResult<Value> {
    if table_id.trim().is_empty() {
        return Err(CommandError::Config("'table_id' cannot be empty".to_string()));
    }
    if !field.is_object() {
        return Err(CommandError::Config(
            "'field' must be an object identifying the field (e.g. { id, column_name })".to_string(),
        ));
    }
    let mut field_payload = field;
    if let Some(obj) = field_payload.as_object_mut() {
        // val-services keys the table on `field.value`
        obj.insert("value".to_string(), Value::String(table_id.to_string()));
    }
    let body = json!({ "field": field_payload, "zone": zone_id });
    post_json(
        domain,
        "remove-val-table-field",
        "/db/admin-repoTable/deleteTableFields",
        body,
    )
    .await
}

pub async fn list_fields(domain: &str, convert: Option<bool>) -> CmdResult<Value> {
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(c) = convert {
        query.push(("convert".to_string(), c.to_string()));
    }
    get_json_with_query(domain, "list-val-fields", "/db/admin-fields/getAllFields/", query).await
}

pub async fn find_tables_with_field(domain: &str, filters: Value) -> CmdResult<Value> {
    let mut query: Vec<(String, String)> = Vec::new();
    if let Value::Object(map) = filters {
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
    // The val-services helper specifically reads `column_name` from the
    // query string (queries information_schema.columns directly). Other
    // keys are silently ignored upstream and result in an opaque HTTP 500
    // "Err" — fail fast here with a clearer message.
    if !query.iter().any(|(k, v)| k == "column_name" && !v.is_empty()) {
        return Err(CommandError::Config(
            "'filters' must contain `column_name` (the physical column name, e.g. \
             'usr_befebfbfcbbf0de'). Display names like 'Brand' will not match — use \
             list-val-fields first to resolve display name → physical name."
                .to_string(),
        ));
    }
    get_json_with_query(
        domain,
        "find-val-tables-with-field",
        "/db/admin-fields/returnTablesWithField",
        query,
    )
    .await
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
    // val-services /admin-fields/updateField reads `request.body.field` and
    // checks `field.table_name || field.value` for the permission check.
    // Wrap the updates payload accordingly.
    let body = json!({ "field": updates });
    post_json(domain, "update-val-field", "/db/admin-fields/updateField", body).await
}

/// Add tables to a zone — additive, preserves all existing zone members.
/// POST /db/admin-phase/updateTableAssignment
///
/// **Why this is non-trivial:** the val-services endpoint is a SET operation,
/// not an add. Its handler:
///   1. takes `details.tables` (a flat array of table id strings),
///   2. groups them by repo_type via `id.split('_')[2]`,
///   3. for each repo_type in the input, builds a fresh repo_type entry with
///      `value: <those grouped table ids>` (overwriting the existing entry),
///   4. then `updateZoneTableRelations` does an unconditional
///      `UPDATE phase_repo_tbl SET repo_phase_data = <new_json>` — which
///      replaces the entire column.
///
/// So calling the raw endpoint with `[new_table_id]` would:
///   - wipe every other table of the same repo_type, AND
///   - wipe every other repo_type from the zone entirely (perspectives included).
///
/// To make `assign` actually mean "add", we fetch the current zone state,
/// flatten its existing table ids (from every non-perspective repo_type),
/// dedupe-union with the new ids, and send the full union as the flat
/// `tables` payload. Result: val-services replays the SET with the union, so
/// no rows are lost.
///
/// **Perspective limitation:** the val-services handler chokes on non-table-id
/// values when grouping (`split('_')[2]` is undefined for perspective ids), so
/// we strip any pre-existing perspective entries from the payload. If a zone
/// has perspectives, `repo_phase_data`'s `pers` entry is rebuilt by val-services
/// from the perspective tables themselves, not from this assignment call.
pub async fn assign_table_to_zone(
    domain: &str,
    zone_id: &str,
    tables: Vec<Value>,
) -> CmdResult<Value> {
    if tables.is_empty() {
        return Err(CommandError::Config(
            "'tables' must contain at least one entry".to_string(),
        ));
    }

    let phase_resp = get_zone(domain, zone_id).await?;
    let existing_ids = collect_existing_table_ids(&phase_resp)?;
    let new_ids = flatten_input_ids(&tables)?;

    let mut union: Vec<String> = existing_ids;
    for id in new_ids {
        if !union.contains(&id) {
            union.push(id);
        }
    }

    let body = json!({
        "details": {
            "phaseId": zone_id,
            "tables": union,
        }
    });
    post_json(
        domain,
        "assign-val-table-to-zone",
        "/db/admin-phase/updateTableAssignment",
        body,
    )
    .await
}

/// Remove tables from a zone — preserves every OTHER table.
/// POST /db/admin-phase/updateTableAssignment (same endpoint as assign).
///
/// Same client-side fetch+rewrite pattern: fetch current state, subtract the
/// specified table ids, send back the remaining ids as the flat payload.
/// Tables not currently in the zone are silently ignored.
///
/// **Note on emptying a zone:** val-services has a special branch for
/// `details.tables.length === 0` that re-emits each existing repo_type with
/// `value: []`. So sending an empty array clears all tables but PRESERVES the
/// repo_type structure. Our remove routes through the same code path when the
/// caller removes every table.
pub async fn remove_tables_from_zone(
    domain: &str,
    zone_id: &str,
    tables: Vec<Value>,
) -> CmdResult<Value> {
    if tables.is_empty() {
        return Err(CommandError::Config(
            "'tables' must contain at least one entry".to_string(),
        ));
    }

    let phase_resp = get_zone(domain, zone_id).await?;
    let existing_ids = collect_existing_table_ids(&phase_resp)?;
    let remove_set = flatten_input_ids(&tables)?;

    let remaining: Vec<String> = existing_ids
        .into_iter()
        .filter(|id| !remove_set.contains(id))
        .collect();

    let body = json!({
        "details": {
            "phaseId": zone_id,
            "tables": remaining,
        }
    });
    post_json(
        domain,
        "remove-val-table-from-zone",
        "/db/admin-phase/updateTableAssignment",
        body,
    )
    .await
}

/// Collect every existing table id across the zone's repo_phase_data, as a
/// flat list of strings in the `custom_tbl_<rt>_<seq>` format. Skips the
/// `pers` (perspective) repo_type — its `value` entries aren't table ids and
/// would be misgrouped by val-services' `split('_')[2]` logic.
fn collect_existing_table_ids(phase_resp: &Value) -> CmdResult<Vec<String>> {
    let repo_data = extract_repo_phase_data(phase_resp)?;
    let mut ids: Vec<String> = Vec::new();
    for repo in &repo_data {
        if matches!(repo_id_str(repo).as_deref(), Some("pers")) {
            continue;
        }
        if let Some(arr) = repo.get("value").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    if is_custom_table_id(s) {
                        ids.push(s.to_string());
                    }
                }
            }
        }
    }
    Ok(ids)
}

fn is_custom_table_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('_').collect();
    parts.len() >= 4 && parts[0] == "custom" && parts[1] == "tbl"
}

/// Flatten the user's `tables` input into a deduplicated list of table id
/// strings. Accepts the flat form (`["custom_tbl_5_42", ...]`) and the
/// canonical grouped form (`[{ id: <rt>, value: [<table_id>, ...] }, ...]`).
fn flatten_input_ids(tables: &[Value]) -> CmdResult<Vec<String>> {
    let mut ids: Vec<String> = Vec::new();
    let mut push_unique = |s: String| {
        if !ids.contains(&s) {
            ids.push(s);
        }
    };
    for t in tables {
        if let Some(s) = t.as_str() {
            if !is_custom_table_id(s) {
                return Err(CommandError::Config(format!(
                    "Table id '{}' is not in the expected 'custom_tbl_<repo_type>_<seq>' format.",
                    s
                )));
            }
            push_unique(s.to_string());
        } else if let Some(obj) = t.as_object() {
            let values = obj.get("value").and_then(|v| v.as_array()).ok_or_else(|| {
                CommandError::Config(
                    "Each grouped entry must include 'value' as an array of table id strings"
                        .to_string(),
                )
            })?;
            for v in values {
                let s = v.as_str().ok_or_else(|| {
                    CommandError::Config(
                        "Grouped 'value' entries must be table id strings".to_string(),
                    )
                })?;
                if !is_custom_table_id(s) {
                    return Err(CommandError::Config(format!(
                        "Table id '{}' is not in the expected 'custom_tbl_<repo_type>_<seq>' format.",
                        s
                    )));
                }
                push_unique(s.to_string());
            }
        } else {
            return Err(CommandError::Config(
                "Each 'tables' entry must be a table id string or { id, value: [...] } object"
                    .to_string(),
            ));
        }
    }
    Ok(ids)
}

/// `get-val-zone` (`/db/zones/:id/info`) returns the phase_tbl row directly
/// (single object). `repo_phase_data` may be null on a zone that has never
/// had tables assigned — treat that as empty.
fn extract_repo_phase_data(phase_resp: &Value) -> CmdResult<Vec<Value>> {
    if !phase_resp.is_object() {
        return Err(CommandError::Config(
            "Unexpected zone response shape — expected an object".to_string(),
        ));
    }
    Ok(phase_resp
        .get("repo_phase_data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}

fn repo_id_str(repo: &Value) -> Option<String> {
    repo.get("id").and_then(|v| {
        v.as_str()
            .map(|s| s.to_string())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })
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
pub async fn list_queries(domain: &str, filters: Option<Value>) -> CmdResult<Value> {
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
    get_json_with_query(domain, "list-val-queries", "/db/data/v1/listAllDSQueries", query).await
}

pub async fn get_query(domain: &str, dsid: &str) -> CmdResult<Value> {
    if dsid.trim().is_empty() {
        return Err(CommandError::Config("'dsid' cannot be empty".to_string()));
    }
    let path = format!("/db/data/v1/getDSQuery/{}", dsid);
    get_json(domain, "get-val-query", &path).await
}

pub async fn execute_query(
    domain: &str,
    dsid: &str,
    use_cache: Option<bool>,
    limit: Option<u64>,
    paginate: Option<Value>,
) -> CmdResult<Value> {
    if dsid.trim().is_empty() {
        return Err(CommandError::Config("'dsid' cannot be empty".to_string()));
    }
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(c) = use_cache {
        query.push(("useCache".to_string(), c.to_string()));
    }
    if let Some(l) = limit {
        query.push(("limit".to_string(), l.to_string()));
    }
    if let Some(Value::Object(map)) = paginate {
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
    let path = format!("/db/data/v1/executeDSQuery/{}", dsid);
    get_json_with_query(domain, "execute-val-query", &path, query).await
}

pub async fn test_query(domain: &str, payload: Value) -> CmdResult<Value> {
    if !payload.is_object() {
        return Err(CommandError::Config(
            "'payload' must be an object — same shape as a `datasource` (basicInfo + queryInfo).".to_string(),
        ));
    }
    post_json(domain, "test-val-query", "/db/data/v1/testDSQuery", payload).await
}

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
///
/// val-services `saveDSQuery` does a literal INSERT — sending the same id twice
/// trips the `querybuilder_master_pkey` unique constraint. The UI works around
/// this by deleting the existing row first, then re-inserting. We mirror that
/// here so callers can treat this as a normal update.
pub async fn update_query(
    domain: &str,
    dsid: &str,
    updates: Value,
) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config("'updates' must be an object".to_string()));
    }
    if dsid.trim().is_empty() {
        return Err(CommandError::Config("'dsid' is required".to_string()));
    }
    // 1) Delete existing row (idempotent — endpoint tolerates absent ids).
    let delete_path = format!("/db/data/v1/deleteDSQuery/{}", dsid);
    let _ = get_json(domain, "update-val-query", &delete_path).await;

    // 2) Re-insert with the desired payload, forcing the dsid to match.
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
    // the original definition. `selected_table` is read by
    // handleCloneIntegrationSettings (it splits the string to find the source
    // repo + table id) — without it the clone fails with "Invalid table
    // selected".
    let body = json!({
        "repoTable": {
            "name": new_name,
            "prefix": new_prefix.unwrap_or(""),
            "value": zone_id,
            "parentId": source_table_id,
            "source_table": source_table_id,
            "selected_table": source_table_id,
            "metadata": {},
        }
    });
    post_json(domain, "clone-val-table", "/db/admin-repoTable/cloneTable", body).await
}

// ============================================================================
// Linkages — connect two existing fields across tables
// ============================================================================

pub async fn list_linkages(domain: &str) -> CmdResult<Value> {
    get_json(domain, "list-val-linkages", "/db/admin-repoType/getLinkages/").await
}

pub async fn create_linkage(domain: &str, mut linkage: Value) -> CmdResult<Value> {
    if !linkage.is_object() {
        return Err(CommandError::Config(
            "'linkage' must be an object".to_string(),
        ));
    }
    let required = [
        "repo_source_name",
        "repo_source_col",
        "repo_assoc_name",
        "repo_assoc_col",
        "source_zone_id",
        "target_zone_id",
    ];
    if let Some(obj) = linkage.as_object() {
        for k in required {
            if !obj.contains_key(k) {
                return Err(CommandError::Config(format!(
                    "'linkage.{}' is required",
                    k
                )));
            }
        }
    }
    // Route-level permission check reads `repo_source_tablename` /
    // `repo_assoc_tablename`; internals read `repo_source_name` / `repo_assoc_name`.
    // They're functionally identical (table id like custom_tbl_<zone>_<seq>) — mirror.
    if let Some(obj) = linkage.as_object_mut() {
        if !obj.contains_key("repo_source_tablename") {
            if let Some(v) = obj.get("repo_source_name").cloned() {
                obj.insert("repo_source_tablename".to_string(), v);
            }
        }
        if !obj.contains_key("repo_assoc_tablename") {
            if let Some(v) = obj.get("repo_assoc_name").cloned() {
                obj.insert("repo_assoc_tablename".to_string(), v);
            }
        }
    }
    let body = json!({ "linkage": linkage });
    post_json(domain, "create-val-linkage", "/db/admin-repoType/addLinkage", body).await
}

pub async fn update_linkage(domain: &str, updates: Value) -> CmdResult<Value> {
    if !updates.is_object() {
        return Err(CommandError::Config(
            "'linkage' must be an object".to_string(),
        ));
    }
    // updateLinkage runs a literal SQL UPDATE that writes EVERY column —
    // any missing key becomes the literal string "undefined" in SQL. To make
    // partial updates safe, fetch the current linkage row and merge user-
    // supplied changes on top.
    let id = updates
        .get("id")
        .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
        .ok_or_else(|| CommandError::Config("'linkage.id' is required".to_string()))?;

    let existing_list = list_linkages(domain).await?;
    let arr = existing_list
        .as_array()
        .ok_or_else(|| CommandError::Internal("list-val-linkages did not return an array".to_string()))?;
    let current = arr
        .iter()
        .find(|row| {
            row.get("id")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
                .as_deref() == Some(id.as_str())
        })
        .ok_or_else(|| CommandError::NotFound(format!("linkage id={} not found", id)))?
        .clone();

    // Start from current, overlay updates.
    let mut merged = current;
    if let (Some(merged_obj), Some(updates_obj)) = (merged.as_object_mut(), updates.as_object()) {
        for (k, v) in updates_obj {
            merged_obj.insert(k.clone(), v.clone());
        }
        // Mirror _name → _tablename for the route's permission check.
        if !merged_obj.contains_key("repo_source_tablename") {
            if let Some(v) = merged_obj.get("repo_source_name").cloned() {
                merged_obj.insert("repo_source_tablename".to_string(), v);
            }
        }
        if !merged_obj.contains_key("repo_assoc_tablename") {
            if let Some(v) = merged_obj.get("repo_assoc_name").cloned() {
                merged_obj.insert("repo_assoc_tablename".to_string(), v);
            }
        }
    }

    let body = json!({ "linkage": merged });
    post_json(domain, "update-val-linkage", "/db/admin-repoType/updateLinkage", body).await
}

// ============================================================================
// Integrations — connector configurations and integration-backed tables
// ============================================================================

pub async fn list_integrations(domain: &str) -> CmdResult<Value> {
    get_json(domain, "list-val-integrations", "/db/integration/listAllIntegrations").await
}

pub async fn list_integration_tables(domain: &str) -> CmdResult<Value> {
    get_json(
        domain,
        "list-val-integration-tables",
        "/db/integration/listAllIntegrationsTable",
    )
    .await
}

pub async fn get_integration(domain: &str, identifier: &str) -> CmdResult<Value> {
    if identifier.trim().is_empty() {
        return Err(CommandError::Config("'identifier' cannot be empty".to_string()));
    }
    let path = format!("/db/integration/{}", identifier);
    get_json(domain, "get-val-integration", &path).await
}

pub async fn get_integration_fields(
    domain: &str,
    filters: Option<Value>,
) -> CmdResult<Value> {
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
    // The val-services gateway substitutes both `identifier` and `optionId`
    // into the URL path: `${integrationHost}/${identifier}/options/${optionId}/fields`.
    // Without either, the URL contains `undefined` and the integration
    // service responds with an opaque HTTP 500 — fail fast here.
    let has = |name: &str| query.iter().any(|(k, v)| k == name && !v.is_empty());
    if !has("identifier") || !has("optionId") {
        return Err(CommandError::Config(
            "'filters' must contain both `identifier` (connector slug, e.g. 'dineconnect') \
             and `optionId` (a connector-specific option id like 'transactions' or 'menus'). \
             Option ids come from the connector's manifest; if you don't know them, inspect \
             the connector's options endpoint via the val-services UI or call \
             /db/integration/getIntegrationOptions/<identifier> directly. The connector must \
             also be `authenticated: true` (check via list-val-integrations) — unauthenticated \
             connectors will fail upstream."
                .to_string(),
        ));
    }
    get_json_with_query(
        domain,
        "get-val-integration-fields",
        "/db/integration/getIntegrationFields",
        query,
    )
    .await
}

pub async fn save_integration(domain: &str, settings: Value) -> CmdResult<Value> {
    if !settings.is_object() {
        return Err(CommandError::Config(
            "'settings' must be an object".to_string(),
        ));
    }
    // val-services reads body.settings.info.id for the table-permission check,
    // so the payload must be `{ settings: { info: { id, ... }, ... } }`.
    if settings
        .get("info")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
        return Err(CommandError::Config(
            "'settings.info.id' (target table id) is required".to_string(),
        ));
    }
    let body = json!({ "settings": settings });
    post_json(
        domain,
        "save-val-integration",
        "/db/integration/saveIntegration",
        body,
    )
    .await
}

pub async fn test_integration(domain: &str, body: Value) -> CmdResult<Value> {
    if !body.is_object() {
        return Err(CommandError::Config(
            "'body' must be an object — typically { settings, payload } depending on connector".to_string(),
        ));
    }
    post_json(domain, "test-val-integration", "/db/integration/test", body).await
}

pub async fn extract_integration(domain: &str, body: Value) -> CmdResult<Value> {
    if !body.is_object() {
        return Err(CommandError::Config(
            "'body' must be an object identifying the integration to extract".to_string(),
        ));
    }
    post_json(
        domain,
        "extract-val-integration",
        "/db/integration/extractIntegration",
        body,
    )
    .await
}
