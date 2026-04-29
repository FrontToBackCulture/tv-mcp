// Workflow Authoring MCP Tools
// 5 tools: create-workflow, update-workflow, execute-workflow,
//          list-workflow-plugins, get-workflow-plugin-schema
//
// No delete-workflow by design — see bot-builder/CLAUDE.md.

use crate::modules::workflows;
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
            name: "create-workflow".to_string(),
            description:
                "Create a new VAL workflow (job_master row) on a domain. \
                 `data.workflow.plugins[]` must contain at least one plugin step (e.g. SQLWorkflowV2Plugin). \
                 If `cron_expression` is set, the workflow starts firing immediately — omit or set null for one-off workflows. \
                 The server fills in `data.queue` and assigns `pluginId` via nanoid; do not pre-populate them. \
                 Returns the created job (with new `id`)."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'lab', 'ssg', 'koi')"
                    },
                    "name": {
                        "type": "string",
                        "description": "Workflow display name"
                    },
                    "data": {
                        "type": "object",
                        "description": "Full workflow definition payload. Must include `workflow.plugins[]`. May include `tags[]`, `description`, `repeat.runAsUser`."
                    },
                    "priority": {
                        "type": "number",
                        "description": "Job priority (1-5). Defaults to 3."
                    },
                    "cron_expression": {
                        "type": "string",
                        "description": "Optional cron expression. Omit/null for non-recurring workflows."
                    }
                }),
                vec!["domain".to_string(), "name".to_string(), "data".to_string()],
            ),
        },
        Tool {
            name: "update-workflow".to_string(),
            description:
                "Update an existing workflow. \
                 `mode='meta'` (default, PATCH) only supports `name`, `description`, `cron_expression`, `tags` — safe for routine edits. \
                 `mode='full'` (PUT) replaces the entire job; `updates.data` must be a complete IJobDefinition including `data.queue`. \
                 Use `mode='full'` when changing `plugins[]`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name"
                    },
                    "id": {
                        "type": "string",
                        "description": "Workflow job_master id (e.g., '33563')"
                    },
                    "updates": {
                        "type": "object",
                        "description": "Update body. For `meta`: { name?, description?, cron_expression?, tags? }. For `full`: complete job object including `data`."
                    },
                    "mode": {
                        "type": "string",
                        "description": "'meta' (PATCH, default) or 'full' (PUT)",
                        "enum": ["meta", "full"]
                    }
                }),
                vec!["domain".to_string(), "id".to_string(), "updates".to_string()],
            ),
        },
        Tool {
            name: "execute-workflow".to_string(),
            description:
                "Execute a workflow now. \
                 Without `overrides` → reruns the saved definition (POST /:id/rerun). \
                 With `overrides` → runs with in-memory edits without persisting (POST /:id/rerun_unsaved). \
                 `overrides` must follow the same shape as a create body ({ name, data, priority, cron_expression })."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name"
                    },
                    "id": {
                        "type": "string",
                        "description": "Workflow job_master id (e.g., '33563')"
                    },
                    "overrides": {
                        "type": "object",
                        "description": "Optional. If provided, runs as rerun_unsaved with these values. Shape: { name, data, priority, cron_expression }."
                    }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "list-workflow-plugins".to_string(),
            description:
                "List the plugin classes available on a VAL domain (e.g. SQLWorkflowV2Plugin, ReportGeneratorPlugin). \
                 Use before `create-workflow` to discover what plugins exist."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name"
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-workflow-plugin-schema".to_string(),
            description:
                "Fetch the JSON schema for a plugin's `params`. \
                 Use this to construct a valid `data.workflow.plugins[].params` block before calling `create-workflow`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name"
                    },
                    "plugin_name": {
                        "type": "string",
                        "description": "Plugin class name from `list-workflow-plugins` (e.g. 'SQLWorkflowV2Plugin')"
                    }
                }),
                vec!["domain".to_string(), "plugin_name".to_string()],
            ),
        },
    ]
}

// ============================================================================
// Tool Dispatch
// ============================================================================

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "create-workflow" => {
            let domain = require_str!(args, "domain");
            let wf_name = require_str!(args, "name");
            let data = match args.get("data") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'data' parameter is required and must be an object".to_string()),
            };
            let priority = args.get("priority").and_then(|v| v.as_i64()).unwrap_or(3);
            let cron = args.get("cron_expression").cloned().unwrap_or(Value::Null);

            let body = json!({
                "name": wf_name,
                "domain": domain,
                "data": data,
                "priority": priority,
                "cron_expression": cron,
            });

            match workflows::create_workflow(&domain, body).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-workflow failed: {}", e)),
            }
        }

        "update-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            let updates = match args.get("updates") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'updates' parameter is required and must be an object".to_string()),
            };
            let mode = args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("meta")
                .to_string();

            match workflows::update_workflow(&domain, &id, updates, &mode).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-workflow failed: {}", e)),
            }
        }

        "execute-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            let overrides = args.get("overrides").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });

            match workflows::execute_workflow(&domain, &id, overrides).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("execute-workflow failed: {}", e)),
            }
        }

        "list-workflow-plugins" => {
            let domain = require_str!(args, "domain");
            match workflows::list_plugins(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-workflow-plugins failed: {}", e)),
            }
        }

        "get-workflow-plugin-schema" => {
            let domain = require_str!(args, "domain");
            let plugin_name = require_str!(args, "plugin_name");
            match workflows::get_plugin_schema(&domain, &plugin_name).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-workflow-plugin-schema failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown workflow tool: {}", name)),
    }
}
