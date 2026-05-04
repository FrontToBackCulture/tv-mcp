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
            name: "list-val-spaces".to_string(),
            description:
                "List every VAL space (UI: 'Project') in a domain. Returns id, name, description, \
                 and audit fields for each space. Use this to discover space IDs before calling \
                 zone or table tools — VAL space IDs are not stable string slugs but numeric/uuid \
                 identifiers."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name (e.g. 'lab', 'koi')" }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-space".to_string(),
            description:
                "Fetch a single VAL space's metadata row (project_name, project_desc, audit \
                 fields). Use this before `update-val-space` to see the current state."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "space_id": { "type": "string", "description": "Space (project) id" }
                }),
                vec!["domain".to_string(), "space_id".to_string()],
            ),
        },
        Tool {
            name: "list-val-space-zones".to_string(),
            description:
                "List every zone (UI: 'Phase') under a VAL space. Returns the phase rows from \
                 phase_tbl: phase_id, phase_name, phase_desc, audit fields. Use this to map a \
                 space's structure before placing tables or queries."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "space_id": { "type": "string", "description": "Parent space (project) id" }
                }),
                vec!["domain".to_string(), "space_id".to_string()],
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
            name: "list-val-zones".to_string(),
            description:
                "List zones (UI: 'Phase') across ALL spaces in a domain. Use this when you need \
                 to find a zone by name without knowing its parent space. Pass `filters` for \
                 query-string narrowing (server passes them through to the listZones helper). \
                 For zones in one specific space, use `list-val-space-zones` instead."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Optional flat key→string|number|bool map merged into the query string."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-zone".to_string(),
            description:
                "Fetch a single VAL zone's metadata row (phase_id, phase_name, phase_desc, \
                 phase_pr_id, audit fields). Use before `update-val-zone` to see current state."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Zone (phase) id" }
                }),
                vec!["domain".to_string(), "zone_id".to_string()],
            ),
        },
        Tool {
            name: "list-val-zone-tables".to_string(),
            description:
                "List every table and perspective inside a VAL zone. Returns the zone's content \
                 — actual repo tables and perspective definitions (transpose / union views). Use \
                 this to map a zone before placing or cloning tables."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Zone (phase) id" }
                }),
                vec!["domain".to_string(), "zone_id".to_string()],
            ),
        },
        Tool {
            name: "create-val-table".to_string(),
            description:
                "Create a new VAL table inside a zone. \
                 Returns the new table (custom_tbl_<zone>_<seq> identifier in the response). \
                 `repo_type` defaults to 'general'. Use `extras` to pass advanced fields like \
                 autocalculate, populated_dates, or metadata. **For tags on create:** the \
                 create endpoint does NOT accept top-level `tags` — pass them inside metadata, \
                 e.g. `extras: { \"metadata\": { \"tags\": [\"foo\", \"bar\"] } }`. To set tags \
                 after creation, use `update-val-table` (which DOES accept top-level `tags`)."
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
                        "description": "Optional extra fields merged into the request body (autocalculate, populated_dates, metadata, etc.). Tags must go inside metadata: { metadata: { tags: [...] } }."
                    }
                }),
                vec!["domain".to_string(), "zone_id".to_string(), "name".to_string()],
            ),
        },
        Tool {
            name: "list-val-tables".to_string(),
            description:
                "List every VAL table across all spaces and zones in a domain. Pass `filters` for \
                 query-string narrowing (server passes them through to listTables). Use this to \
                 search for a table by name or prefix without iterating zones."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Optional flat key→string|number|bool map merged into the query string."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-table".to_string(),
            description:
                "Fetch a single VAL table's full definition — table-level metadata plus its \
                 fields (columns) and link configuration. Use before `update-val-table`, \
                 `clone-val-table`, or `add-val-table-field` to see the current schema."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (custom_tbl_<zone>_<seq> or numeric value)"
                    }
                }),
                vec!["domain".to_string(), "table_id".to_string()],
            ),
        },
        Tool {
            name: "update-val-table".to_string(),
            description:
                "Update an existing VAL table's metadata (display name, prefix, repo_type, \
                 autocalculate, populated_dates, metadata, tags). Pass `updates` with the fields \
                 to change. **For tags:** pass `tags` as a top-level array of strings in \
                 `updates` (e.g. `{ \"tags\": [\"foo\", \"bar\"] }`). Stored under \
                 `metadata.tags` server-side. Pass `[]` to clear all tags. Does NOT modify \
                 columns/fields — use `update-val-field` and `add-val-table-field(s)` for \
                 column changes."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (custom_tbl_<zone>_<seq> or numeric value)"
                    },
                    "updates": {
                        "type": "object",
                        "description": "Fields to update. Common: name (display name), prefix, repo_type, autocalculate, populated_dates, metadata, tags (array of strings; pass [] to clear)."
                    }
                }),
                vec!["domain".to_string(), "table_id".to_string(), "updates".to_string()],
            ),
        },
        Tool {
            name: "list-val-table-dependencies".to_string(),
            description:
                "List everything that depends on a VAL table — other tables (via linked fields), \
                 queries that select from it, workflows that read/write it. Run this BEFORE \
                 restructuring or reassigning a table to surface what would break."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (custom_tbl_<zone>_<seq> or numeric value)"
                    }
                }),
                vec!["domain".to_string(), "table_id".to_string()],
            ),
        },
        Tool {
            name: "remove-val-table-field".to_string(),
            description:
                "Remove a column from a VAL table. Drops the data in that column on this table; \
                 the field definition itself survives in dft_nodefields and can stay assigned to \
                 other tables. Use `find-val-tables-with-field` first to confirm the field isn't \
                 still in use elsewhere unexpectedly. `field` must include enough to identify the \
                 column (e.g. `{ id, column_name }` or `{ dft_nodefields_id, column_name }`)."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "table_id": {
                        "type": "string",
                        "description": "Table identifier (custom_tbl_<zone>_<seq> or numeric value)"
                    },
                    "zone_id": {
                        "type": "string",
                        "description": "Parent zone (phase) id — used for the entity-permission check."
                    },
                    "field": {
                        "type": "object",
                        "description": "Field identifier payload — at minimum `{ id, column_name }` or `{ dft_nodefields_id, column_name }`."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "table_id".to_string(),
                    "zone_id".to_string(),
                    "field".to_string(),
                ],
            ),
        },
        Tool {
            name: "list-val-fields".to_string(),
            description:
                "List every field definition in a VAL domain (across all tables). The only path \
                 to inspect field defs via MCP without raw SQL against `dft_nodefields`. Pass \
                 `convert: true` for the legacy converted shape (server flag)."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "convert": {
                        "type": "boolean",
                        "description": "Optional. If true, returns the converted/legacy shape."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "find-val-tables-with-field".to_string(),
            description:
                "Reverse lookup — list every table that has the given physical column. Use before \
                 renaming or restructuring a column to surface every table affected. \
                 \
                 `filters` MUST contain `column_name` set to the *physical* column name (e.g. \
                 `usr_befebfbfcbbf0de`, `pgad7e98...`, `bco1234...`), NOT the display name. The \
                 helper queries Postgres `information_schema.columns` directly, so display \
                 names like 'Brand' will not match. To go from display name → physical name, \
                 first call `list-val-fields` and look up the field's `column_name` (or \
                 `dft_nodefields_name`). \
                 \
                 Returns an array of table records (id, name, tablename, spaces, zones, etc.) \
                 — same shape as `list-val-tables` entries. May be very large (172+ tables in \
                 lab for a common column)."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Required. Must include `column_name` set to the physical column name (e.g. usr_befebfbfcbbf0de). Display names (e.g. 'Brand') will NOT match — use list-val-fields to resolve display→physical first."
                    }
                }),
                vec!["domain".to_string(), "filters".to_string()],
            ),
        },
        Tool {
            name: "add-val-table-field".to_string(),
            description:
                "Add a single new field (column) to a VAL table. \
                 \
                 `data_type` is one of the 19 platform FIELD_TYPES (source: \
                 val-react/src/components/repository/SelectFieldType.tsx). \
                 \
                 PRIMITIVES: 'text', 'int' (Number), 'numeric' (Decimal), 'bool' (Checkbox), \
                 'date', 'timestamp' (Date With Time), 'url'. \
                 SELECTS: 'select' (editable), 'select_controlled' (locked options), 'chips' \
                 (multi editable), 'chips_controlled' (multi locked). \
                 PEOPLE: 'person' (single user), 'multiperson' (user array). \
                 ATTACHMENTS: 'attachment'. \
                 LINKED: 'linked_select' (single ref), 'linked_multiselect' (ref array). \
                 CALCULATED (NOT supported here — these live in admin_ui_settings, not \
                 dft_nodefields_tbl): 'formula', 'rollupv2', 'rules'. \
                 \
                 Decision tree for required `extras` / `link_options`: \
                 • If `data_type` is a SELECT family → set `extras.predefined_values` to an \
                   array of {label, value, color?}. \
                 • If `data_type` is `linked_select` or `linked_multiselect` → set \
                   `link_options` with: `linked_table` (target table id), `linked_field` \
                   (target column), `source_field_display`, `existing_field`, \
                   `linked_field_display`, `table_display_name`. \
                 • Otherwise `extras` is fully optional. Common keys: `desc`, `category`, \
                   `column_length` (for varchar caps), `colour` (UI badge hex), \
                   `applyOnTableLevel`, `allowedValuesOnly`. \
                 \
                 For full type semantics, dropdown-option storage, linkage cardinality, the \
                 usr_<hash> physical naming, and worked examples — fetch \
                 `concepts/fields` via `get-docs-page`."
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
                        "description": "Platform FIELD_TYPES value. Primitives: text, int, numeric, bool, date, timestamp, url. Selects: select, select_controlled, chips, chips_controlled. People: person, multiperson. Attachments: attachment. Linked: linked_select, linked_multiselect.",
                        "enum": [
                            "text", "int", "numeric", "bool", "date", "timestamp", "url",
                            "select", "select_controlled", "chips", "chips_controlled",
                            "person", "multiperson",
                            "attachment",
                            "linked_select", "linked_multiselect"
                        ]
                    },
                    "extras": {
                        "type": "object",
                        "description": "Optional metadata. For select/chips families, include `predefined_values: [{label, value, color?}]`. Other keys: desc, category, column_length, colour, applyOnTableLevel, allowedValuesOnly."
                    },
                    "link_options": {
                        "type": "object",
                        "description": "Required only for linked_select / linked_multiselect. Keys: linked_table, source_field_display, existing_field, linked_field, linked_field_display, table_display_name."
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
                 `updates` must include identifiers AND core attributes — at minimum `id` \
                 (or `dft_nodefields_id`), `column_name`, `value` (the table id), `data_type`, \
                 and `category`. Even on partial updates, val-services revalidates the full \
                 field record, so `data_type` cannot be omitted. Optional keys: `name`, `desc`, \
                 `column_length`, `colour`, `predefined_values`, `table_name`. Use \
                 `list-val-fields` first to fetch current `data_type` and `category`."
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
                "**Adds** the specified tables to the zone (additive — preserves every existing \
                 table and repo_type already in the zone). Pass either a flat array of table id \
                 strings (e.g. `['custom_tbl_5_42']`) or the canonical grouped shape \
                 (`[{ id: <repo_type_id>, value: [<table_id>, ...] }, ...]`). \
                 \
                 **For removal**, use `remove-val-table-from-zone` — calling assign with a \
                 different table set will NOT remove anything. \
                 \
                 **Background:** the underlying val-services endpoint replaces the zone's entire \
                 `repo_phase_data` column on every call, so this tool fetches the current zone \
                 state and merges your additions before sending. Calling the raw endpoint \
                 directly with only the new ids would silently delete every other table (and \
                 every other repo_type) from the zone."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Target zone (phase) id" },
                    "tables": {
                        "type": "array",
                        "description": "Table ids to ADD to the zone. Either flat strings ('custom_tbl_<rt>_<seq>') or grouped objects ({ id: <repo_type_id>, value: [<table_id>, ...] })."
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
            name: "remove-val-table-from-zone".to_string(),
            description:
                "**Removes** the specified tables from the zone (subtractive — preserves every \
                 OTHER table and repo_type in the zone). Pass either a flat array of table id \
                 strings or the grouped shape, same as `assign-val-table-to-zone`. \
                 \
                 Tables that aren't currently in the zone are silently ignored (no error). \
                 \
                 **Background:** same client-side fetch+rewrite pattern as assign — the \
                 val-services endpoint is replacement-based, so this tool fetches the current \
                 zone state, drops the specified table ids, and sends back the full preserved \
                 payload."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "zone_id": { "type": "string", "description": "Zone (phase) id to remove tables from" },
                    "tables": {
                        "type": "array",
                        "description": "Table ids to REMOVE from the zone. Either flat strings ('custom_tbl_<rt>_<seq>') or grouped objects ({ id: <repo_type_id>, value: [<table_id>, ...] })."
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
            name: "list-val-queries".to_string(),
            description:
                "List every VAL query (datasource) in a domain. Returns id, name, and summary \
                 metadata for each. Pass `filters` for query-string narrowing — passed through to \
                 listAllDSQueries. Use to discover query IDs (`dsid`) before `get-val-query`, \
                 `update-val-query`, `copy-val-query`, or `execute-val-query`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Optional flat key→string|number|bool map merged into the query string."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-query".to_string(),
            description:
                "Fetch a single VAL query's full datasource (basicInfo + queryInfo). The proper \
                 read path before `update-val-query` — saveDSQuery does a full INSERT, so partial \
                 updates require fetching the current `datasource` first and merging client-side."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "dsid": {
                        "type": "string",
                        "description": "Query id (the `id` from querybuilder_master / list-val-queries)"
                    }
                }),
                vec!["domain".to_string(), "dsid".to_string()],
            ),
        },
        Tool {
            name: "execute-val-query".to_string(),
            description:
                "Run a saved VAL query and return its result rows. Different from \
                 `execute-val-sql` — this respects the saved query definition, query-level \
                 permissions, VAL's cache layer, and pagination. Use this to fetch what a user \
                 would see in a dashboard widget."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "dsid": { "type": "string", "description": "Query id" },
                    "use_cache": {
                        "type": "boolean",
                        "description": "Optional. Use VAL's query cache. Default false."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional row limit."
                    },
                    "paginate": {
                        "type": "object",
                        "description": "Optional pagination params (offset, page, pageSize, sortBy, sortDir, etc.) merged into the query string."
                    }
                }),
                vec!["domain".to_string(), "dsid".to_string()],
            ),
        },
        Tool {
            name: "test-val-query".to_string(),
            description:
                "Validate and dry-run a VAL query payload before saving. Pass `payload` with the \
                 same shape as a `datasource` (basicInfo + queryInfo). Use this to verify an \
                 LLM-authored query compiles and returns the expected shape before calling \
                 `create-val-query` or `update-val-query`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "payload": {
                        "type": "object",
                        "description": "Query payload — same shape as `datasource` (basicInfo, queryInfo)."
                    }
                }),
                vec!["domain".to_string(), "payload".to_string()],
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
                "Update an existing VAL query. tv-mcp deletes the existing row then re-inserts \
                 (mirrors the UI flow — saveDSQuery is INSERT-only). `updates` MUST include the \
                 full `datasource` (basicInfo + queryInfo) and `name`; optional: `category`, \
                 `permission`, `tags`. Use `get-val-query` to fetch the current datasource and \
                 merge changes client-side first."
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
            name: "list-val-linkages".to_string(),
            description:
                "List every linkage (cross-table relationship) defined in a VAL domain. Each \
                 linkage joins a source field to a target field — analogous to a foreign key. \
                 Response includes phase + project context for each linkage. Use to discover \
                 linkage IDs before `update-val-linkage`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "create-val-linkage".to_string(),
            description:
                "Create a linkage between two existing fields across tables. Different from \
                 `add-val-table-field` with `linked_*` data_type — that creates a NEW linked \
                 field; this connects two EXISTING fields. \
                 \
                 `linkage` must include: `repo_source_name` (source table id, e.g., \
                 'custom_tbl_<zone>_<seq>'), `repo_source_col` (source field column_name), \
                 `repo_assoc_name` (target table id), `repo_assoc_col` (target field \
                 column_name), `source_zone_id`, `target_zone_id`. Optional: \
                 `repo_source_display_col`, `repo_assoc_display_col`, `source_space`, \
                 `target_space`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "linkage": {
                        "type": "object",
                        "description": "Linkage payload — see tool description for required keys."
                    }
                }),
                vec!["domain".to_string(), "linkage".to_string()],
            ),
        },
        Tool {
            name: "update-val-linkage".to_string(),
            description:
                "Update an existing linkage. Partial updates are safe: tv-mcp fetches the \
                 current linkage row and merges your changes on top before sending (val-services \
                 writes every column via raw SQL, so missing keys would otherwise corrupt the row \
                 with literal 'undefined'). `linkage` must include `id` (linkage row id) and \
                 `source_zone_id` / `target_zone_id` (for the permission check). Common editable \
                 keys: `repo_source_col`, `repo_assoc_col`, `repo_source_display_col`, \
                 `repo_assoc_display_col`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "linkage": {
                        "type": "object",
                        "description": "Linkage update payload — must include id, source_zone_id, target_zone_id."
                    }
                }),
                vec!["domain".to_string(), "linkage".to_string()],
            ),
        },
        Tool {
            name: "list-val-integrations".to_string(),
            description:
                "List the integration connector types available on a VAL domain (Shopify, Xero, \
                 etc.). Use to discover what connectors can be wired up before \
                 `save-val-integration`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "list-val-integration-tables".to_string(),
            description:
                "List every integration-backed table in a VAL domain — tables that get \
                 auto-populated from a connector. Use to find existing integration setups."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-integration".to_string(),
            description:
                "Fetch one integration's full config — connector type, table mapping, field \
                 mappings, filter rules. Use before `save-val-integration` to see current state."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "identifier": {
                        "type": "string",
                        "description": "Integration identifier (e.g. connector slug or instance id)."
                    }
                }),
                vec!["domain".to_string(), "identifier".to_string()],
            ),
        },
        Tool {
            name: "get-val-integration-fields".to_string(),
            description:
                "List the fields a given integration connector can extract for a specific \
                 option (e.g. 'transactions', 'menus'). Use during setup to plan field \
                 mappings before `save-val-integration`. \
                 \
                 `filters` MUST contain both `identifier` (connector slug, e.g. 'dineconnect') \
                 and `optionId` (a connector-specific option id). The val-services gateway \
                 substitutes both into the URL path: \
                 `${integrationHost}/${identifier}/options/${optionId}/fields`. Without \
                 either, the URL contains `undefined` and the integration service responds \
                 with an opaque HTTP 500. \
                 \
                 The connector must also be `authenticated: true` (check via \
                 `list-val-integrations`) — unauthenticated connectors return upstream errors. \
                 Optional extras: `shortcode`, `values` (JSON-stringified prior selections), \
                 `id`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Required. Must include `identifier` (connector slug) and `optionId` (a connector option id). Optional extras: shortcode, values, id."
                    }
                }),
                vec!["domain".to_string(), "filters".to_string()],
            ),
        },
        Tool {
            name: "save-val-integration".to_string(),
            description:
                "Save an integration table config — wires a connector to a target VAL table with \
                 field mappings. `settings` is the full integration payload (server reads \
                 `settings.info.id` as the target table id for permission checks). Common shape: \
                 `{ info: { id, ... }, mappings: [...], filters: [...], schedule: {...} }`. \
                 Fetch via `get-val-integration` first when updating an existing one."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "settings": {
                        "type": "object",
                        "description": "Full integration config. Must include `info.id` (target table id)."
                    }
                }),
                vec!["domain".to_string(), "settings".to_string()],
            ),
        },
        Tool {
            name: "test-val-integration".to_string(),
            description:
                "Pre-flight test of a connector configuration — verifies auth + that the \
                 mappings produce a sensible row sample without persisting. Run before \
                 `extract-val-integration` to catch misconfiguration. Body shape mirrors \
                 `save-val-integration` payload."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "body": {
                        "type": "object",
                        "description": "Test payload — typically the same shape as save-val-integration's settings."
                    }
                }),
                vec!["domain".to_string(), "body".to_string()],
            ),
        },
        Tool {
            name: "extract-val-integration".to_string(),
            description:
                "Trigger a data extraction from a configured integration — pulls rows from the \
                 connector into the target VAL table. `body` identifies which integration to \
                 run; typical shape `{ identifier, optionId, table, ... }`. For scheduled \
                 connectors, this is a manual one-off run on top of the schedule."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "body": {
                        "type": "object",
                        "description": "Extraction payload identifying the integration."
                    }
                }),
                vec!["domain".to_string(), "body".to_string()],
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

        "list-val-spaces" => {
            let domain = require_str!(args, "domain");
            match val_admin::list_spaces(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-spaces failed: {}", e)),
            }
        }

        "get-val-space" => {
            let domain = require_str!(args, "domain");
            let space_id = require_str!(args, "space_id");
            match val_admin::get_space(&domain, &space_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-space failed: {}", e)),
            }
        }

        "list-val-space-zones" => {
            let domain = require_str!(args, "domain");
            let space_id = require_str!(args, "space_id");
            match val_admin::list_space_zones(&domain, &space_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-space-zones failed: {}", e)),
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

        "list-val-zones" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::list_zones(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-zones failed: {}", e)),
            }
        }

        "get-val-zone" => {
            let domain = require_str!(args, "domain");
            let zone_id = require_str!(args, "zone_id");
            match val_admin::get_zone(&domain, &zone_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-zone failed: {}", e)),
            }
        }

        "list-val-zone-tables" => {
            let domain = require_str!(args, "domain");
            let zone_id = require_str!(args, "zone_id");
            match val_admin::list_zone_tables(&domain, &zone_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-zone-tables failed: {}", e)),
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

        "list-val-tables" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::list_tables(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-tables failed: {}", e)),
            }
        }

        "get-val-table" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            match val_admin::get_table(&domain, &table_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-table failed: {}", e)),
            }
        }

        "update-val-table" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' must be an object".to_string()),
            };
            match val_admin::update_table(&domain, &table_id, updates).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-table failed: {}", e)),
            }
        }

        "list-val-table-dependencies" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            match val_admin::list_table_dependencies(&domain, &table_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-table-dependencies failed: {}", e)),
            }
        }

        "remove-val-table-field" => {
            let domain = require_str!(args, "domain");
            let table_id = require_str!(args, "table_id");
            let zone_id = require_str!(args, "zone_id");
            let field = match args.get("field") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'field' must be an object".to_string()),
            };
            match val_admin::remove_table_field(&domain, &table_id, &zone_id, field).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("remove-val-table-field failed: {}", e)),
            }
        }

        "list-val-fields" => {
            let domain = require_str!(args, "domain");
            let convert = args.get("convert").and_then(|v| v.as_bool());
            match val_admin::list_fields(&domain, convert).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-fields failed: {}", e)),
            }
        }

        "find-val-tables-with-field" => {
            let domain = require_str!(args, "domain");
            let filters = match args.get("filters") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'filters' must be an object".to_string()),
            };
            match val_admin::find_tables_with_field(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("find-val-tables-with-field failed: {}", e)),
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

        "remove-val-table-from-zone" => {
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
            match val_admin::remove_tables_from_zone(&domain, &zone_id, tables).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("remove-val-table-from-zone failed: {}", e)),
            }
        }

        "list-val-queries" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::list_queries(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-queries failed: {}", e)),
            }
        }

        "get-val-query" => {
            let domain = require_str!(args, "domain");
            let dsid = require_str!(args, "dsid");
            match val_admin::get_query(&domain, &dsid).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-query failed: {}", e)),
            }
        }

        "execute-val-query" => {
            let domain = require_str!(args, "domain");
            let dsid = require_str!(args, "dsid");
            let use_cache = args.get("use_cache").and_then(|v| v.as_bool());
            let limit = args.get("limit").and_then(|v| v.as_u64());
            let paginate = args.get("paginate").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::execute_query(&domain, &dsid, use_cache, limit, paginate).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("execute-val-query failed: {}", e)),
            }
        }

        "test-val-query" => {
            let domain = require_str!(args, "domain");
            let payload = match args.get("payload") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'payload' must be an object".to_string()),
            };
            match val_admin::test_query(&domain, payload).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("test-val-query failed: {}", e)),
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

        "list-val-linkages" => {
            let domain = require_str!(args, "domain");
            match val_admin::list_linkages(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-linkages failed: {}", e)),
            }
        }

        "create-val-linkage" => {
            let domain = require_str!(args, "domain");
            let linkage = match args.get("linkage") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'linkage' must be an object".to_string()),
            };
            match val_admin::create_linkage(&domain, linkage).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-linkage failed: {}", e)),
            }
        }

        "update-val-linkage" => {
            let domain = require_str!(args, "domain");
            let linkage = match args.get("linkage") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'linkage' must be an object".to_string()),
            };
            match val_admin::update_linkage(&domain, linkage).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-linkage failed: {}", e)),
            }
        }

        "list-val-integrations" => {
            let domain = require_str!(args, "domain");
            match val_admin::list_integrations(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-integrations failed: {}", e)),
            }
        }

        "list-val-integration-tables" => {
            let domain = require_str!(args, "domain");
            match val_admin::list_integration_tables(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-integration-tables failed: {}", e)),
            }
        }

        "get-val-integration" => {
            let domain = require_str!(args, "domain");
            let identifier = require_str!(args, "identifier");
            match val_admin::get_integration(&domain, &identifier).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-integration failed: {}", e)),
            }
        }

        "get-val-integration-fields" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match val_admin::get_integration_fields(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-integration-fields failed: {}", e)),
            }
        }

        "save-val-integration" => {
            let domain = require_str!(args, "domain");
            let settings = match args.get("settings") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'settings' must be an object".to_string()),
            };
            match val_admin::save_integration(&domain, settings).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("save-val-integration failed: {}", e)),
            }
        }

        "test-val-integration" => {
            let domain = require_str!(args, "domain");
            let body = match args.get("body") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'body' must be an object".to_string()),
            };
            match val_admin::test_integration(&domain, body).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("test-val-integration failed: {}", e)),
            }
        }

        "extract-val-integration" => {
            let domain = require_str!(args, "domain");
            let body = match args.get("body") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'body' must be an object".to_string()),
            };
            match val_admin::extract_integration(&domain, body).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("extract-val-integration failed: {}", e)),
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
