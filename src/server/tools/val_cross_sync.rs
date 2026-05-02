// VAL Cross-Domain Sync MCP Tools — promote tables/workflows/dashboards
// from one VAL domain to another (e.g., lab → koi).

use crate::modules::val_cross_sync;
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
            name: "sync-val-domain".to_string(),
            description:
                "Promote VAL artifacts from one domain to another (e.g., lab → koi). Wraps \
                 val-services /api/v1/sync via the workspace `sync-domain` edge function so \
                 sync jobs are tracked in the `solution_sync_jobs` Supabase table for audit. \
                 Returns the sync UUID — pass it to `get-val-sync-status` to poll progress. \
                 \
                 `resource_type` is one of `tables` (auto-fills spaces/zones/columns/linkages/\
                 tableforms/fieldcats — pass `space_ids`/`zone_ids` to scope), `workflows`, or \
                 `dashboards` (use `include_queries: true` to also sync the queries each \
                 dashboard references). \
                 \
                 Source and target must be different domains. The credentials for the source \
                 domain must be present in `val_domain_credentials`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "source": {
                        "type": "string",
                        "description": "Source VAL domain — typically 'lab', but any domain works."
                    },
                    "target": {
                        "type": "string",
                        "description": "Target VAL domain (e.g., 'koi', 'jfh')."
                    },
                    "resource_type": {
                        "type": "string",
                        "description": "What to sync.",
                        "enum": ["tables", "workflows", "dashboards"]
                    },
                    "resource_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "IDs of the resources to sync (table ids, workflow ids, or dashboard ids)."
                    },
                    "space_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Optional. For `tables`: scope which parent spaces to include."
                    },
                    "zone_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Optional. For `tables`: scope which parent zones to include."
                    },
                    "include_queries": {
                        "type": "boolean",
                        "description": "Optional. For `dashboards`: also sync the queries each dashboard references. Default false."
                    },
                    "override_creator": {
                        "type": "integer",
                        "description": "Optional. User id to attribute as creator on the target. Defaults to 1."
                    },
                    "instance_id": {
                        "type": "string",
                        "description": "Optional. Solution instance id this sync belongs to (for grouping in solution_sync_jobs)."
                    },
                    "system_id": {
                        "type": "string",
                        "description": "Optional. System id within the solution (used by solution-management UI)."
                    },
                    "system_type": {
                        "type": "string",
                        "description": "Optional. System type label."
                    }
                }),
                vec![
                    "source".to_string(),
                    "target".to_string(),
                    "resource_type".to_string(),
                    "resource_ids".to_string(),
                ],
            ),
        },
        Tool {
            name: "promote-val-resources".to_string(),
            description:
                "Bundled cross-domain promotion. Pushes tables / workflows / dashboards from \
                 `source` to `target` in dependency order (tables → workflows → dashboards), \
                 polling each step until done. Pass arrays of resource IDs for the types you \
                 want to promote (omit a type to skip it). For dashboards, set \
                 `include_queries: true` to also pull the queries each one references — strongly \
                 recommended. \
                 \
                 **For tables:** ALWAYS pass `space_ids` AND `zone_ids` for the parent space \
                 and zones containing the tables. Without them the tables are inserted on the \
                 target but orphaned (no parent zone), and the dashboard UI won't show them. \
                 Discover them via `list-val-tables({ domain: source })` → each table row has \
                 `spaces` and `zones` arrays. \
                 \
                 By default the tool waits for completion (poll interval 2s, timeout 120s); set \
                 `wait_for_completion: false` to fire-and-forget."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "source": { "type": "string", "description": "Source VAL domain (typically 'lab')." },
                    "target": { "type": "string", "description": "Target VAL domain (e.g., 'koi', 'studio')." },
                    "tables": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Table identifiers to promote (e.g. 'custom_tbl_1_5'). Auto-includes parent spaces/zones, columns, linkages, tableforms, fieldcats."
                    },
                    "workflows": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Workflow job IDs to promote."
                    },
                    "dashboards": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Dashboard IDs to promote."
                    },
                    "include_queries": {
                        "type": "boolean",
                        "description": "When promoting dashboards, also pull the queries each one references. Default false; set true to avoid empty dashboards on the target."
                    },
                    "space_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Optional. Scope tables sync by parent space."
                    },
                    "zone_ids": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Optional. Scope tables sync by parent zone."
                    },
                    "instance_id": { "type": "string", "description": "Optional. solution_instances.id for grouping." },
                    "system_id": { "type": "string", "description": "Optional. System id within solution." },
                    "system_type": { "type": "string", "description": "Optional. System type label." },
                    "override_creator": { "type": "integer", "description": "Optional. User id to attribute as creator on target. Defaults to 1." },
                    "wait_for_completion": { "type": "boolean", "description": "Default true. Set false to return immediately after each kickoff (no polling)." },
                    "poll_interval_secs": { "type": "integer", "description": "Default 2. Interval between get-val-sync-status polls." },
                    "poll_timeout_secs": { "type": "integer", "description": "Default 120. Per-step polling deadline before giving up." }
                }),
                vec!["source".to_string(), "target".to_string()],
            ),
        },
        Tool {
            name: "get-val-sync-status".to_string(),
            description:
                "Poll a cross-domain sync job's progress. Pass the `source` domain (where the \
                 sync was kicked off — same domain you passed to `sync-val-domain`) and the \
                 `sync_id` returned by that call. Returns status, completed steps, errors."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "source": {
                        "type": "string",
                        "description": "Source VAL domain — same `source` passed to `sync-val-domain`."
                    },
                    "sync_id": {
                        "type": "string",
                        "description": "Sync UUID returned by `sync-val-domain`."
                    }
                }),
                vec!["source".to_string(), "sync_id".to_string()],
            ),
        },
    ]
}

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "sync-val-domain" => {
            let source = require_str!(args, "source");
            let target = require_str!(args, "target");
            let resource_type = require_str!(args, "resource_type");

            let resource_ids: Vec<String> = match args.get("resource_ids") {
                Some(Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                _ => {
                    return ToolResult::error(
                        "'resource_ids' must be an array of strings".to_string(),
                    )
                }
            };
            let space_ids: Option<Vec<i64>> = args.get("space_ids").and_then(|v| {
                v.as_array().map(|arr| arr.iter().filter_map(|x| x.as_i64()).collect())
            });
            let zone_ids: Option<Vec<i64>> = args.get("zone_ids").and_then(|v| {
                v.as_array().map(|arr| arr.iter().filter_map(|x| x.as_i64()).collect())
            });
            let instance_id = args.get("instance_id").and_then(|v| v.as_str());
            let system_id = args.get("system_id").and_then(|v| v.as_str());
            let system_type = args.get("system_type").and_then(|v| v.as_str());
            let override_creator = args.get("override_creator").and_then(|v| v.as_i64());
            let include_queries = args.get("include_queries").and_then(|v| v.as_bool());

            match val_cross_sync::sync_val_domain(
                &source,
                &target,
                &resource_type,
                resource_ids,
                space_ids,
                zone_ids,
                instance_id,
                system_id,
                system_type,
                override_creator,
                include_queries,
            )
            .await
            {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("sync-val-domain failed: {}", e)),
            }
        }

        "promote-val-resources" => {
            let source = require_str!(args, "source");
            let target = require_str!(args, "target");

            fn str_array(v: Option<&Value>) -> Option<Vec<String>> {
                v.and_then(|val| val.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }
            fn int_array(v: Option<&Value>) -> Option<Vec<i64>> {
                v.and_then(|val| val.as_array()).map(|arr| {
                    arr.iter().filter_map(|x| x.as_i64()).collect()
                })
            }

            let req = val_cross_sync::PromoteRequest {
                tables: str_array(args.get("tables")),
                workflows: str_array(args.get("workflows")),
                dashboards: str_array(args.get("dashboards")),
                include_queries: args.get("include_queries").and_then(|v| v.as_bool()),
                space_ids: int_array(args.get("space_ids")),
                zone_ids: int_array(args.get("zone_ids")),
                instance_id: args.get("instance_id").and_then(|v| v.as_str()).map(String::from),
                system_id: args.get("system_id").and_then(|v| v.as_str()).map(String::from),
                system_type: args.get("system_type").and_then(|v| v.as_str()).map(String::from),
                override_creator: args.get("override_creator").and_then(|v| v.as_i64()),
                poll_interval_secs: args.get("poll_interval_secs").and_then(|v| v.as_u64()),
                poll_timeout_secs: args.get("poll_timeout_secs").and_then(|v| v.as_u64()),
                wait_for_completion: args
                    .get("wait_for_completion")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            };

            match val_cross_sync::promote_resources(&source, &target, req).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("promote-val-resources failed: {}", e)),
            }
        }

        "get-val-sync-status" => {
            let source = require_str!(args, "source");
            let sync_id = require_str!(args, "sync_id");
            match val_cross_sync::get_val_sync_status(&source, &sync_id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-sync-status failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown val_cross_sync tool: {}", name)),
    }
}
