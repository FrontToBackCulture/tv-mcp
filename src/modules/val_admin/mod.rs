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

/// DESTRUCTIVE — delete a field DEFINITION globally.
/// GET /db/admin-fields/deleteField?column_name=<physical_column_name>
///
/// This is NOT `remove_table_field` (which only detaches a column from ONE
/// table and leaves the definition intact). The val-services handler
/// (`AdminHelper.deleteField`) internally calls `returnTablesWithField(column_name)`,
/// removes the column from EVERY table that has it, then drops the row from
/// `dft_nodefields_tbl`. Irreversible.
///
/// `column_name` MUST be the physical column name (e.g. `usr_e0babecdbd0fa_1_1`),
/// NOT the display name — the underlying SQL matches `dft_nodefields_name` /
/// `information_schema.columns.column_name` exactly. Use `list_fields` to
/// resolve display → physical first.
///
/// The handler also destructures a `tables_affected` query param but never
/// uses it (it always recomputes via `returnTablesWithField`), so we send
/// only `column_name`. Returns `{ "message": "success" }` on success.
pub async fn delete_field(domain: &str, column_name: &str) -> CmdResult<Value> {
    let column_name = column_name.trim();
    if column_name.is_empty() {
        return Err(CommandError::Config(
            "'column_name' cannot be empty — pass the PHYSICAL column name (e.g. \
             'usr_e0babecdbd0fa_1_1'), not the display name. Use list-val-fields \
             to resolve display → physical first."
                .to_string(),
        ));
    }
    get_json_with_query(
        domain,
        "delete-val-field",
        "/db/admin-fields/deleteField",
        vec![("column_name".to_string(), column_name.to_string())],
    )
    .await
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

// ============================================================================
// Integration filters — filter descriptors + resolved filter values
// ============================================================================
//
// The val-services middleware proxies these to the integration service:
//   /db/integration/getIntegrationFilters            → /v1/integrations/{identifier}/options/{optionId}/filters
//   /db/integration/getIntegrationFilters/:filterId  → /v1/integrations/{identifier}/options/{optionId}/filters/{filterId}
//
// FB/IG-stack connectors (facebook, instagram, linkedin, google ads, ...) gate
// `test-val-integration` / `save-val-integration` on a Page Access Token that
// only the filter loaders can produce server-side. The descriptors expose the
// dependency graph (e.g. `ig_user_id` depends_on `page_id`); the value endpoint
// runs each loader and returns the materialised list — items for `page_id`
// include a `token` (PAT) and `selectedValue` that the downstream payload needs.

pub async fn list_integration_filters(
    domain: &str,
    identifier: &str,
    option_id: &str,
) -> CmdResult<Value> {
    if identifier.trim().is_empty() {
        return Err(CommandError::Config(
            "'identifier' (connector slug, e.g. 'instagram') cannot be empty".to_string(),
        ));
    }
    if option_id.trim().is_empty() {
        return Err(CommandError::Config(
            "'optionId' (connector option, e.g. 'media') cannot be empty".to_string(),
        ));
    }
    let query: Vec<(String, String)> = vec![
        ("identifier".to_string(), identifier.to_string()),
        ("optionId".to_string(), option_id.to_string()),
    ];
    get_json_with_query(
        domain,
        "list-val-integration-filters",
        "/db/integration/getIntegrationFilters",
        query,
    )
    .await
}

pub async fn get_integration_filter_values(
    domain: &str,
    identifier: &str,
    option_id: &str,
    filter_id: &str,
    query_params: Option<Value>,
) -> CmdResult<Value> {
    if identifier.trim().is_empty() {
        return Err(CommandError::Config(
            "'identifier' (connector slug, e.g. 'instagram') cannot be empty".to_string(),
        ));
    }
    if option_id.trim().is_empty() {
        return Err(CommandError::Config(
            "'optionId' (connector option, e.g. 'media') cannot be empty".to_string(),
        ));
    }
    if filter_id.trim().is_empty() {
        return Err(CommandError::Config(
            "'filterId' (e.g. 'page_id', 'ig_user_id') cannot be empty".to_string(),
        ));
    }
    let mut query: Vec<(String, String)> = vec![
        ("identifier".to_string(), identifier.to_string()),
        ("optionId".to_string(), option_id.to_string()),
    ];
    // val-services reads `?queryParams=<json>` and runs it through
    // `integrationHelper.constructQuery` server-side. For multi-dep filters
    // (e.g. `ig_media_id` needs both `page_id` and `ig_user_id`) the helper
    // ONLY handles the array form `[{column_name, value, data_type, valueType}]`
    // — the object-keyed form silently drops all but the first dep. We accept
    // the simpler `{ filterId: priorRow }` shape from the caller and expand to
    // the array form here.
    if let Some(qp) = query_params {
        let arr = build_query_params_array(qp)?;
        if !arr.is_empty() {
            let s = serde_json::to_string(&Value::Array(arr)).map_err(|e| {
                CommandError::Config(format!("'queryParams' failed to JSON-encode: {}", e))
            })?;
            query.push(("queryParams".to_string(), s));
        }
    }
    let path = format!("/db/integration/getIntegrationFilters/{}", filter_id);
    get_json_with_query(
        domain,
        "get-val-integration-filter-values",
        &path,
        query,
    )
    .await
}

/// Reshape caller input into the array form `[{column_name, value, ...}, ...]`
/// that val-services' `constructQuery` accepts for multi-dep filters.
///
/// Accepted top-level shapes:
///   1. Object keyed by filter id (recommended): `{page_id: <prior row>, ig_user_id: <prior row>}`.
///      Each row may be a full object (we extract value/selectedValue/id +
///      data_type/valueType) or a bare scalar.
///   2. Pre-built canonical array: `[{column_name, value, data_type?, valueType?}, ...]`.
///   3. Single canonical entry: `{column_name, value, ...}` — the pre-multi-dep
///      shape that worked single-dep against val-services directly. Kept for
///      back-compat so prior calls don't break.
///   4. A JSON string wrapping any of the above. Some MCP clients stringify
///      object-valued args; we unwrap one layer before dispatching.
fn build_query_params_array(qp: Value) -> CmdResult<Vec<Value>> {
    // 4: stringified shape — unwrap once and recurse.
    if let Value::String(s) = &qp {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let parsed: Value = serde_json::from_str(trimmed).map_err(|e| {
            CommandError::Config(format!(
                "'queryParams' was a string but couldn't be parsed as JSON: {}. \
                 Pass the value as a JSON object instead, e.g. {{ \"page_id\": <prior page row> }}.",
                e
            ))
        })?;
        return build_query_params_array(parsed);
    }

    match qp {
        Value::Null => Ok(Vec::new()),

        // 2: pre-built canonical array.
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                if !item.is_object() {
                    return Err(CommandError::Config(format!(
                        "'queryParams' array entry [{}] must be an object — \
                         expected `{{column_name, value, data_type?, valueType?}}`.",
                        i
                    )));
                }
            }
            Ok(items)
        }

        Value::Object(map) => {
            if map.is_empty() {
                return Ok(Vec::new());
            }
            // 3: single canonical entry — disambiguate from form 1 by detecting
            // the canonical `column_name`+`value` keys at the top level. A real
            // form-1 input is keyed by filter ids (e.g. `page_id`, `ig_user_id`),
            // so collision is vanishingly unlikely.
            if map.contains_key("column_name") && map.contains_key("value") {
                return Ok(vec![Value::Object(map)]);
            }
            // 1: object keyed by filter id.
            let mut arr = Vec::with_capacity(map.len());
            for (filter_id, row) in map.into_iter() {
                arr.push(prior_row_to_entry(&filter_id, row));
            }
            Ok(arr)
        }

        _ => Err(CommandError::Config(
            "'queryParams' must be one of:\n  \
              - object keyed by filter id (form 1, recommended): \
                `{page_id: <prior page row>, ig_user_id: <prior ig_user row>}`\n  \
              - array of canonical entries (form 2): \
                `[{column_name, value, data_type?, valueType?}, ...]`\n  \
              - single canonical entry (form 3, single-dep only): \
                `{column_name, value, ...}`\n\
             A JSON string wrapping any of the above is also accepted."
                .to_string(),
        )),
    }
}

fn prior_row_to_entry(filter_id: &str, row: Value) -> Value {
    // Caller hands us either the full prior filter row (object) or a bare value
    // (string/number). For an object, prefer the explicit `value` field, then
    // `selectedValue` (returned by the FB-stack page loader), then `id`. Carry
    // `data_type` and `valueType` through if present so constructQuery's
    // dynamic-value path (date formulas) still works.
    if let Value::Object(map) = &row {
        // FB-stack `page_id` is its own contract: val-services' integration
        // preProcessors call `JSON.parse(page_id)` and expect
        // `{ selectedValue, token }`. Without this auto-pack the caller has
        // to JSON.stringify by hand every time and `row.value` (typically the
        // page's display name) silently produces wrong downstream behaviour.
        if let Some(packed) = pack_fb_page_value(filter_id, map) {
            let mut entry = json!({
                "column_name": filter_id,
                "value": packed,
            });
            if let Some(dt) = map.get("data_type") {
                entry["data_type"] = dt.clone();
            }
            if let Some(vt) = map.get("valueType") {
                entry["valueType"] = vt.clone();
            }
            return entry;
        }

        let value = map
            .get("value")
            .or_else(|| map.get("selectedValue"))
            .or_else(|| map.get("id"))
            .cloned()
            .unwrap_or(Value::Null);
        let mut entry = json!({
            "column_name": filter_id,
            "value": value,
        });
        if let Some(dt) = map.get("data_type") {
            entry["data_type"] = dt.clone();
        }
        if let Some(vt) = map.get("valueType") {
            entry["valueType"] = vt.clone();
        }
        entry
    } else {
        json!({
            "column_name": filter_id,
            "value": row,
        })
    }
}

/// FB-stack `page_id` (facebook, instagram, linkedin, google ads, ...) is the
/// only filter whose downstream consumer (`JSON.parse(page_id)` in the
/// preProcessors) expects a JSON-stringified `{selectedValue, token}` instead
/// of a bare value. Detect that shape — row has both `token` and one of
/// `selectedValue`/`id` — and return the packed string. Returns `None` for
/// anything else, including a `page_id` row that already has `value` set to a
/// JSON string (caller has pre-packed; don't double-encode).
fn pack_fb_page_value(filter_id: &str, row: &serde_json::Map<String, Value>) -> Option<String> {
    if filter_id != "page_id" {
        return None;
    }
    // Caller already JSON-stringified into `value` themselves — leave alone.
    if let Some(Value::String(s)) = row.get("value") {
        if s.trim_start().starts_with('{') {
            return None;
        }
    }
    let token = row.get("token").and_then(|t| t.as_str())?;
    let selected = row
        .get("selectedValue")
        .or_else(|| row.get("id"))
        .cloned()?;
    serde_json::to_string(&json!({
        "selectedValue": selected,
        "token": token,
    }))
    .ok()
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
    let body = normalize_test_integration_body(body);
    post_json(domain, "test-val-integration", "/db/integration/test", body).await
}

/// val-services' `returnDataWithFieldsSelected` does
/// `_.find(fieldsSelected, { column: key })` and writes `accum[found.path] = ...`
/// — so `fields` MUST be `[{column, path}, ...]`. The natural shorthand
/// `["col1", "col2"]` silently produces empty rows because no `{column}` match
/// is found. Expand bare-string entries to `{column: c, path: c}` here so
/// callers don't have to remember the dual-key contract.
///
/// Also auto-packs `body.values[]` entries whose `column_name === "page_id"`
/// (or `column === "page_id"`) into the `JSON.stringify({selectedValue, token})`
/// shape that val-services' FB-stack preProcessors expect — same trap as the
/// queryParams path.
fn normalize_test_integration_body(mut body: Value) -> Value {
    if let Some(obj) = body.as_object_mut() {
        if let Some(fields) = obj.get_mut("fields") {
            if let Some(arr) = fields.as_array_mut() {
                for item in arr.iter_mut() {
                    match item {
                        Value::String(s) => {
                            let col = s.clone();
                            *item = json!({ "column": col, "path": col });
                        }
                        Value::Object(map) => {
                            // Common caller mistake: `{column_name, ...}` instead
                            // of `{column, ...}`. Mirror column_name → column when
                            // only the former is present. Path defaults to column
                            // unless explicitly set.
                            if !map.contains_key("column") {
                                if let Some(v) = map.get("column_name").cloned() {
                                    map.insert("column".to_string(), v);
                                }
                            }
                            if !map.contains_key("path") {
                                if let Some(v) = map.get("column").cloned() {
                                    map.insert("path".to_string(), v);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(values) = obj.get_mut("values") {
            if let Some(arr) = values.as_array_mut() {
                for item in arr.iter_mut() {
                    if let Some(map) = item.as_object_mut() {
                        let col = map
                            .get("column_name")
                            .or_else(|| map.get("column"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if col.as_deref() == Some("page_id") {
                            if let Some(packed) = pack_fb_page_value("page_id", map) {
                                map.insert("value".to_string(), Value::String(packed));
                            }
                        }
                    }
                }
            }
        }
    }
    body
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Each test name traces back to a row in the bug report's input table.

    #[test]
    fn form1_full_row_with_token() {
        let qp = json!({
            "page_id": {"column_name": "page_id", "value": "1057513914104696", "token": "EAA..."}
        });
        let arr = build_query_params_array(qp).expect("form 1 with full row should parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form1_minimal_row() {
        let qp = json!({
            "page_id": {"value": "1057513914104696", "token": "EAA..."}
        });
        let arr = build_query_params_array(qp).expect("form 1 minimal row should parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form1_scalar_value() {
        let qp = json!({"page_id": "1057513914104696"});
        let arr = build_query_params_array(qp).expect("form 1 scalar should parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form1_multi_dep() {
        let qp = json!({
            "page_id":    {"column_name": "page_id",    "value": "1057513914104696"},
            "ig_user_id": {"column_name": "ig_user_id", "value": "17841480160011990"}
        });
        let arr = build_query_params_array(qp).expect("multi-dep should parse");
        assert_eq!(arr.len(), 2);
        let cols: std::collections::HashSet<&str> = arr
            .iter()
            .filter_map(|e| e["column_name"].as_str())
            .collect();
        assert!(cols.contains("page_id"));
        assert!(cols.contains("ig_user_id"));
    }

    #[test]
    fn form2_canonical_array() {
        let qp = json!([
            {"column_name": "page_id", "value": "1057513914104696"}
        ]);
        let arr = build_query_params_array(qp).expect("form 2 array should pass through");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form3_flat_canonical_entry() {
        // Pre-multi-dep shape that worked single-dep against val-services.
        let qp = json!({"column_name": "page_id", "value": "1057513914104696"});
        let arr = build_query_params_array(qp).expect("form 3 flat entry should parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form4_json_stringified_object() {
        let qp = json!(r#"{"page_id":"1057513914104696"}"#);
        let arr = build_query_params_array(qp).expect("stringified form-1 should unwrap and parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn form4_json_stringified_array() {
        let qp = json!(r#"[{"column_name":"page_id","value":"1057513914104696"}]"#);
        let arr = build_query_params_array(qp).expect("stringified form-2 should unwrap and parse");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn null_yields_empty_array() {
        let arr = build_query_params_array(Value::Null).unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn empty_object_yields_empty_array() {
        let arr = build_query_params_array(json!({})).unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn scalar_input_rejected_with_clear_message() {
        let err = build_query_params_array(json!(42))
            .expect_err("bare scalar must be rejected");
        let msg = format!("{:?}", err);
        assert!(msg.contains("form 1"), "error should reference accepted forms; got: {}", msg);
    }

    #[test]
    fn stringified_garbage_rejected() {
        let err = build_query_params_array(json!("not valid json {"))
            .expect_err("garbage string must be rejected");
        let msg = format!("{:?}", err);
        assert!(msg.contains("couldn't be parsed as JSON"), "got: {}", msg);
    }

    #[test]
    fn array_entry_must_be_object() {
        let err = build_query_params_array(json!(["page_id", "1057"]))
            .expect_err("array of scalars must be rejected");
        let msg = format!("{:?}", err);
        assert!(msg.contains("entry [0]"), "should call out which index; got: {}", msg);
    }

    #[test]
    fn prior_row_carries_data_type_and_value_type() {
        let qp = json!({
            "since": {"value": "2026-01-01", "data_type": "date", "valueType": "dynamic"}
        });
        let arr = build_query_params_array(qp).unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["data_type"], "date");
        assert_eq!(arr[0]["valueType"], "dynamic");
    }

    // ---------- test-val-integration body.fields normalization ----------

    #[test]
    fn fields_strings_expand_to_column_path() {
        let body = normalize_test_integration_body(json!({
            "fields": ["likes", "comments"]
        }));
        let fields = body["fields"].as_array().unwrap();
        assert_eq!(fields[0]["column"], "likes");
        assert_eq!(fields[0]["path"], "likes");
        assert_eq!(fields[1]["column"], "comments");
        assert_eq!(fields[1]["path"], "comments");
    }

    #[test]
    fn fields_column_name_mirrored_to_column() {
        let body = normalize_test_integration_body(json!({
            "fields": [{"column_name": "likes"}]
        }));
        let f = &body["fields"][0];
        assert_eq!(f["column"], "likes");
        assert_eq!(f["path"], "likes");
    }

    #[test]
    fn fields_canonical_pass_through() {
        let body = normalize_test_integration_body(json!({
            "fields": [{"column": "likes", "path": "metrics.likes", "type": "concat"}]
        }));
        let f = &body["fields"][0];
        assert_eq!(f["column"], "likes");
        assert_eq!(f["path"], "metrics.likes");
        assert_eq!(f["type"], "concat");
    }

    // ---------- page_id auto-pack for FB-stack preProcessors ----------

    #[test]
    fn page_id_queryparams_autopacked_with_token() {
        let qp = json!({
            "page_id": {
                "name": "House of wellness",
                "selectedValue": "1057513914104696",
                "token": "EAA-fb-page-access-token"
            }
        });
        let arr = build_query_params_array(qp).unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["column_name"], "page_id");
        let packed = arr[0]["value"].as_str().expect("page_id value must be packed JSON string");
        let parsed: Value = serde_json::from_str(packed).unwrap();
        assert_eq!(parsed["selectedValue"], "1057513914104696");
        assert_eq!(parsed["token"], "EAA-fb-page-access-token");
    }

    #[test]
    fn page_id_queryparams_autopacked_from_id_alias() {
        let qp = json!({
            "page_id": {
                "id": "1057513914104696",
                "token": "EAA-fb-page-access-token"
            }
        });
        let arr = build_query_params_array(qp).unwrap();
        let parsed: Value = serde_json::from_str(arr[0]["value"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["selectedValue"], "1057513914104696");
        assert_eq!(parsed["token"], "EAA-fb-page-access-token");
    }

    #[test]
    fn page_id_queryparams_no_token_uses_normal_extraction() {
        // No token → caller isn't using FB-stack shape. Treat normally.
        let qp = json!({
            "page_id": {"selectedValue": "1057513914104696", "name": "House of wellness"}
        });
        let arr = build_query_params_array(qp).unwrap();
        assert_eq!(arr[0]["value"], "1057513914104696");
    }

    #[test]
    fn page_id_queryparams_pre_packed_value_not_double_encoded() {
        // Caller already JSON-stringified value themselves → don't re-pack.
        let pre = r#"{"selectedValue":"1057513914104696","token":"EAA"}"#;
        let qp = json!({
            "page_id": {"value": pre, "token": "EAA", "selectedValue": "1057513914104696"}
        });
        let arr = build_query_params_array(qp).unwrap();
        // The caller-provided value should win; not double-encoded.
        assert_eq!(arr[0]["value"], pre);
    }

    #[test]
    fn other_filter_ids_never_autopacked() {
        // ig_user_id has no JSON.parse contract — must not be packed even with token.
        let qp = json!({
            "ig_user_id": {"selectedValue": "178414...", "token": "EAA"}
        });
        let arr = build_query_params_array(qp).unwrap();
        assert_eq!(arr[0]["value"], "178414...");
    }

    #[test]
    fn test_integration_values_page_id_autopacked() {
        let body = normalize_test_integration_body(json!({
            "values": [
                {"column_name": "page_id", "selectedValue": "1057513914104696", "token": "EAA"},
                {"column_name": "since", "value": "2026-01-01"}
            ]
        }));
        let values = body["values"].as_array().unwrap();
        let packed = values[0]["value"].as_str().expect("page_id value must be packed JSON string");
        let parsed: Value = serde_json::from_str(packed).unwrap();
        assert_eq!(parsed["selectedValue"], "1057513914104696");
        assert_eq!(parsed["token"], "EAA");
        // Other entries left alone.
        assert_eq!(values[1]["value"], "2026-01-01");
    }

    #[test]
    fn test_integration_values_page_id_already_stringified_untouched() {
        let pre = r#"{"selectedValue":"1057513914104696","token":"EAA"}"#;
        let body = normalize_test_integration_body(json!({
            "values": [
                {"column_name": "page_id", "value": pre, "token": "EAA", "selectedValue": "1057513914104696"}
            ]
        }));
        assert_eq!(body["values"][0]["value"], pre);
    }
}
