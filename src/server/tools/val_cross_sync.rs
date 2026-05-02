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
