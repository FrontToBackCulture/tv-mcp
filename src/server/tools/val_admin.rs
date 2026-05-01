// VAL Admin MCP Tools
// Phase 1: 6 tools — create/update space, create/update zone, create/clone table.
// No delete tools by design.

use crate::modules::val_admin;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

macro_rules! require_str {
    ($args:expr, $key:expr) => {
        match $args.get($key).and_then(|v| v.as_str()) {
            Some(v) => v.to_string(),
            None => {
                return ToolResult::error(format!("'{}' parameter is required", $key));
            }
        }
    };
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "create-val-space".to_string(),
            description:
                "Create a new VAL space (UI label: 'Project'). \
                 Returns the new space with its `id`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name (e.g., 'lab', 'koi')" },
                    "name": { "type": "string", "description": "Space name (must be non-empty)" },
                    "description": { "type": "string", "description": "Optional description" }
                }),
                vec!["domain".to_string(), "name".to_string()],
            ),
        },
        Tool {
            name: "update-val-space".to_string(),
            description:
                "Update an existing VAL space. Pass `updates` with the fields to change \
                 (e.g. { project_name, project_desc })."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "space_id": { "type": "string", "description": "Space (project) id" },
                    "updates": {
                        "type": "object",
                        "description": "Fields to update. Common: project_name, project_desc."
                    }
                }),
                vec!["domain".to_string(), "space_id".to_string(), "updates".to_string()],
            ),
        },
        Tool {
            name: "create-val-zone".to_string(),
            description:
                "Create a new VAL zone (UI label: 'Phase') under a space. \
                 Returns the new zone with its `id`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "space_id": { "type": "string", "description": "Parent space id" },
                    "name": { "type": "string", "description": "Zone name (must be unique within the domain)" },
                    "description": { "type": "string", "description": "Optional description" }
                }),
                vec!["domain".to_string(), "space_id".to_string(), "name".to_string()],
            ),
        },
        Tool {
            name: "update-val-zone".to_string(),
            description:
                "Update an existing VAL zone. Pass `updates` with the fields to change \
                 (e.g. { phase_name, phase_desc })."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Zone (phase) id" },
                    "space_id": { "type": "string", "description": "Parent space id (required for the entity-permission check)" },
                    "updates": {
                        "type": "object",
                        "description": "Fields to update. Common: phase_name, phase_desc."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "zone_id".to_string(),
                    "space_id".to_string(),
                    "updates".to_string(),
                ],
            ),
        },
        Tool {
            name: "create-val-table".to_string(),
            description:
                "Create a new VAL table inside a zone. \
                 Returns the new table (custom_tbl_<zone>_<seq> identifier in the response). \
                 `repo_type` defaults to 'general'. Use `extras` to pass advanced fields like \
                 autocalculate, populated_dates, or metadata."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Parent zone (phase) id" },
                    "name": { "type": "string", "description": "Table display name" },
                    "description": { "type": "string", "description": "Optional description" },
                    "prefix": { "type": "string", "description": "Optional record prefix (used in record IDs)" },
                    "repo_type": { "type": "string", "description": "Repo type. Defaults to 'general'." },
                    "extras": {
                        "type": "object",
                        "description": "Optional extra fields merged into the request body (autocalculate, populated_dates, metadata, etc.)"
                    }
                }),
                vec!["domain".to_string(), "zone_id".to_string(), "name".to_string()],
            ),
        },
        Tool {
            name: "add-val-table-field".to_string(),
            description:
                "Add a single new field (column) to a VAL table. \
                 `data_type` accepts: 'text', 'number', 'decimal', 'date', 'boolean', 'checkbox', \
                 'select', 'chips', 'person', 'multiperson', 'attachment', 'url', \
                 'linked_text', 'linked_select', 'linked_multiselect'. \
                 For linked types, pass `link_options` with `linked_table`, `linked_field`, etc. \
                 Use `extras` for optional metadata: `desc`, `category`, `column_length`, `colour`, \
                 `predefined_values`, `applyOnTableLevel`, `allowedValuesOnly`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (e.g. 'custom_tbl_34_1552')"
                    },
                    "name": { "type": "string", "description": "Field display name" },
                    "data_type": {
                        "type": "string",
                        "description": "VAL data type — see tool description for full list"
                    },
                    "extras": {
                        "type": "object",
                        "description": "Optional metadata merged into the request body"
                    },
                    "link_options": {
                        "type": "object",
                        "description": "Required only for linked_* data_types. Keys: linked_table, source_field_display, existing_field, linked_field, linked_field_display, table_display_name."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "table_id".to_string(),
                    "name".to_string(),
                    "data_type".to_string(),
                ],
            ),
        },
        Tool {
            name: "add-val-table-fields".to_string(),
            description:
                "Add multiple new fields to a VAL table in a single transaction. \
                 If any field fails validation, the whole batch is rolled back. \
                 Each entry in `fields` follows the same shape as `add-val-table-field`'s body \
                 (without `table_id` — that comes from the path): \
                 `{ name, data_type, category?, desc?, predefined_values?, linked_table?, ... }`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (e.g. 'custom_tbl_34_1552')"
                    },
                    "fields": {
                        "type": "array",
                        "description": "Array of field definitions. Must contain at least one entry.",
                        "items": { "type": "object" }
                    }
                }),
                vec![
                    "domain".to_string(),
                    "table_id".to_string(),
                    "fields".to_string(),
                ],
            ),
        },
        Tool {
            name: "update-val-field".to_string(),
            description:
                "Update an existing field's metadata. \
                 `updates` must include identifiers — at minimum `id` (or `dft_nodefields_id`), \
                 `column_name`, and `value` (the table id) — plus the fields to change. \
                 Recognized keys: `name`, `data_type`, `desc`, `category`, `column_length`, \
                 `colour`, `predefined_values`, `table_name`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "updates": {
                        "type": "object",
                        "description": "Field update payload — must identify the field and include changes"
                    }
                }),
                vec!["domain".to_string(), "updates".to_string()],
            ),
        },
        Tool {
            name: "assign-val-table-to-zone".to_string(),
            description:
                "Move one or more tables to a target zone. \
                 `tables` is grouped by repo type internally — pass either an array of table ids \
                 or the canonical shape: `[{ id: <repo_type_id>, value: [<table_id>, ...] }, ...]`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Target zone (phase) id" },
                    "tables": {
                        "type": "array",
                        "description": "Table ids or repo-type-grouped objects to assign to the target zone"
                    }
                }),
                vec![
                    "domain".to_string(),
                    "zone_id".to_string(),
                    "tables".to_string(),
                ],
            ),
        },
        Tool {
            name: "create-val-query".to_string(),
            description:
                "Create a new VAL query (datasource). \
                 `datasource` is the nested query config: \
                 `{ basicInfo: { name, ... }, queryInfo: { tableInfo, fields, filters, joins, ... } }`. \
                 The synced JSON at `tv-knowledge/0_Platform/domains/<domain>/queries/<id>/definition.json` \
                 is the canonical starting template. \
                 Use `extras` to set `category` ('private' default), `permission`, `tags`, `ai_metadata`. \
                 Returns the new query with its `id`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "name": { "type": "string", "description": "Query display name" },
                    "datasource": {
                        "type": "object",
                        "description": "Full query config — see tool description for shape"
                    },
                    "extras": {
                        "type": "object",
                        "description": "Optional fields: category, permission, tags, ai_metadata"
                    }
                }),
                vec![
                    "domain".to_string(),
                    "name".to_string(),
                    "datasource".to_string(),
                ],
            ),
        },
        Tool {
            name: "update-val-query".to_string(),
            description:
                "Update an existing VAL query. \
                 `updates` should include `name`, `datasource`, and optionally `category`, \
                 `permission`, `tags`. Note: the endpoint does a full INSERT with the same id, so \
                 partial updates require fetching the current `datasource` first via `sync-val-queries` \
                 or `execute-val-sql` and merging client-side."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "dsid": {
                        "type": "string",
                        "description": "Query id (the `id` from querybuilder_master / sync-val-queries output)"
                    },
                    "updates": {
                        "type": "object",
                        "description": "Update payload — should include the full `datasource`"
                    }
                }),
                vec![
                    "domain".to_string(),
                    "dsid".to_string(),
                    "updates".to_string(),
                ],
            ),
        },
        Tool {
            name: "copy-val-query".to_string(),
            description:
                "Copy an existing query into a new one with a different name. \
                 Pass `source_datasource` (the source query's full `datasource`) — val-services renames \
                 it and writes a new row. Returns `{ dsid: <new_id>, ... }`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "new_name": { "type": "string", "description": "Display name for the new query" },
                    "source_datasource": {
                        "type": "object",
                        "description": "The source query's full `datasource` config"
                    },
                    "extras": {
                        "type": "object",
                        "description": "Optional: tags, ai_metadata"
                    }
                }),
                vec![
                    "domain".to_string(),
                    "new_name".to_string(),
                    "source_datasource".to_string(),
                ],
            ),
        },
        Tool {
            name: "clone-val-table".to_string(),
            description:
                "Clone an existing VAL table into a target zone with a new name. \
                 The clone copies the table structure (columns) but not the data."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "source_table_id": {
                        "type": "string",
                        "description": "Source table identifier (e.g. 'custom_tbl_34_1552' or its numeric id)"
                    },
                    "zone_id": { "type": "string", "description": "Target zone (phase) id" },
                    "new_name": { "type": "string", "description": "Display name for the clone" },
                    "new_prefix": { "type": "string", "description": "Optional new record prefix" }
                }),
                vec![
                    "domain".to_string(),
                    "source_table_id".to_string(),
                    "zone_id".to_string(),
                    "new_name".to_string(),
                ],
            ),
        },
    ]
}

// ============================================================================
// Tool Dispatch
// ============================================================================

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "create-val-space" => {
            let domain = require_str!(args, "domain");
            let space_name = require_str!(args, "name");
            let desc = args.get("description").and_then(|v| v.as_str());
            match val_admin::create_space(&domain, &space_name, desc).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-space failed: {}", e)),
            }
        }

        "update-val-space" => {
            let domain = require_str!(args, "domain");
            let space_id = require_str!(args, "space_id");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' must be an object".to_string()),
            };
            match val_admin::update_space(&domain, &space_id, updates).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-space failed: {}", e)),
            }
        }

        "create-val-zone" => {
            let domain = require_str!(args, "domain");
            let space_id = require_str!(args, "space_id");
            let zone_name = require_str!(args, "name");
            let desc = args.get("description").and_then(|v| v.as_str());
            match val_admin::create_zone(&domain, &space_id, &zone_name, desc).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-zone failed: {}", e)),
            }
        }

        "update-val-zone" => {
            let domain = require_str!(args, "domain");
            let zone_id = require_str!(args, "zone_id");
            let space_id = require_str!(args, "space_id");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' must be an object".to_string()),
            };
            match val_admin::update_zone(&domain, &zone_id, &space_id, updates).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-zone failed: {}", e)),
            }
        }

        "create-val-table" => {
            let domain = require_str!(args, "domain");
            let zone_id = require_str!(args, "zone_id");
            let table_name = require_str!(args, "name");
            let desc = args.get("description").and_then(|v| v.as_str());
            let prefix = args.get("prefix").and_then(|v| v.as_str());
            let repo_type = args.get("repo_type").and_then(|v| v.as_str());
            let extras = args.get("extras").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::create_table(
                &domain, &zone_id, &table_name, desc, prefix, repo_type, extras,
            )
            .await
            {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-table failed: {}", e)),
            }
        }

        "add-val-table-field" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            let field_name = require_str!(args, "name");
            let data_type = require_str!(args, "data_type");
            let extras = args.get("extras").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            let link_options = args.get("link_options").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::add_table_field(
                &domain,
                &table_id,
                &field_name,
                &data_type,
                extras,
                link_options,
            )
            .await
            {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("add-val-table-field failed: {}", e)),
            }
        }

        "add-val-table-fields" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            let fields = match args.get("fields").and_then(|v| v.as_array()) {
                Some(arr) if !arr.is_empty() => arr.clone(),
                _ => {
                    return ToolResult::error(
                        "'fields' must be a non-empty array".to_string(),
                    );
                }
            };
            match val_admin::add_table_fields_bulk(&domain, &table_id, fields).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("add-val-table-fields failed: {}", e)),
            }
        }

        "update-val-field" => {
            let domain = require_str!(args, "domain");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' must be an object".to_string()),
            };
            match val_admin::update_field(&domain, updates).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-field failed: {}", e)),
            }
        }

        "assign-val-table-to-zone" => {
            let domain = require_str!(args, "domain");
            let zone_id = require_str!(args, "zone_id");
            let tables = match args.get("tables").and_then(|v| v.as_array()) {
                Some(arr) if !arr.is_empty() => arr.clone(),
                _ => {
                    return ToolResult::error(
                        "'tables' must be a non-empty array".to_string(),
                    );
                }
            };
            match val_admin::assign_table_to_zone(&domain, &zone_id, tables).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("assign-val-table-to-zone failed: {}", e)),
            }
        }

        "create-val-query" => {
            let domain = require_str!(args, "domain");
            let q_name = require_str!(args, "name");
            let datasource = match args.get("datasource") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'datasource' must be an object".to_string()),
            };
            let extras = args.get("extras").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::create_query(&domain, &q_name, datasource, extras).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-query failed: {}", e)),
            }
        }

        "update-val-query" => {
            let domain = require_str!(args, "domain");
            let dsid = require_str!(args, "dsid");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' must be an object".to_string()),
            };
            match val_admin::update_query(&domain, &dsid, updates).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-query failed: {}", e)),
            }
        }

        "copy-val-query" => {
            let domain = require_str!(args, "domain");
            let new_name = require_str!(args, "new_name");
            let source_datasource = match args.get("source_datasource") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'source_datasource' must be an object".to_string()),
            };
            let extras = args.get("extras").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::copy_query(&domain, &new_name, source_datasource, extras).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("copy-val-query failed: {}", e)),
            }
        }

        "clone-val-table" => {
            let domain = require_str!(args, "domain");
            let source_table_id = require_str!(args, "source_table_id");
            let zone_id = require_str!(args, "zone_id");
            let new_name = require_str!(args, "new_name");
            let new_prefix = args.get("new_prefix").and_then(|v| v.as_str());
            match val_admin::clone_table(
                &domain, &source_table_id, &zone_id, &new_name, new_prefix,
            )
            .await
            {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("clone-val-table failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown val-admin tool: {}", name)),
    }
}
