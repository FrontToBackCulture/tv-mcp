// VAL Workflow MCP Tools
// 11 tools (read, run, write, plugin discovery).
// No delete-val-workflow by design — see bot-builder/CLAUDE.md.

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
            name: "create-val-workflow".to_string(),
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
            name: "update-val-workflow".to_string(),
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
            name: "execute-val-workflow".to_string(),
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
            name: "list-val-workflow-plugins".to_string(),
            description:
                "List the plugin classes available on a VAL domain (e.g. SQLWorkflowV2Plugin, ReportGeneratorPlugin). \
                 Use before `create-val-workflow` to discover what plugins exist."
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
            name: "list-val-workflows".to_string(),
            description:
                "List every VAL workflow (job_master row) in a domain. Returns id, name, cron \
                 expression, status, and summary metadata. Pass `filters` for query-string \
                 narrowing — passed through to the workflow-service. Use to discover workflow IDs \
                 before `get-val-workflow`, `update-val-workflow`, or `execute-val-workflow`."
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
            name: "get-val-workflow".to_string(),
            description:
                "Fetch one VAL workflow's full IJobDefinition (data.workflow.plugins, queue, \
                 cron, repeat config, audit fields). The proper read path before \
                 `update-val-workflow` mode='full' — needed to merge changes into the complete \
                 definition."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": {
                        "type": "string",
                        "description": "Workflow job_master id (e.g., '33563')"
                    }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "pause-val-workflow".to_string(),
            description:
                "Pause a recurring VAL workflow — stops it firing on its cron schedule without \
                 changing the schedule. Use to disable a misbehaving workflow during \
                 investigation. Pair with `resume-val-workflow` to re-enable. For permanent \
                 disablement, set `cron_expression: null` via `update-val-workflow` mode='meta'."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": { "type": "string", "description": "Workflow job_master id" }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "resume-val-workflow".to_string(),
            description:
                "Resume a paused VAL workflow — re-enables firing on its existing cron schedule. \
                 No-op if the workflow wasn't paused."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": { "type": "string", "description": "Workflow job_master id" }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "list-val-workflow-executions".to_string(),
            description:
                "List recent workflow execution records (live, not the offline \
                 `sync-all-domain-monitoring` snapshot). Pass `filters` to narrow by job_master \
                 id, status, time range, etc. — passed through to the workflow-service."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "filters": {
                        "type": "object",
                        "description": "Optional. Common keys: jobMasterId, status, from, to, limit."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "get-val-workflow-execution".to_string(),
            description:
                "Fetch one workflow execution's full record — status, start/end times, plugin \
                 step results, errors, and output. Use when triaging a specific run reported by \
                 `list-val-workflow-executions`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": { "type": "string", "description": "Execution id" }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "get-val-workflow-plugin-schema".to_string(),
            description:
                "Fetch the JSON schema for a plugin's `params`. \
                 Use this to construct a valid `data.workflow.plugins[].params` block before calling `create-val-workflow`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name"
                    },
                    "plugin_name": {
                        "type": "string",
                        "description": "Plugin class name from `list-val-workflow-plugins` (e.g. 'SQLWorkflowV2Plugin')"
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
        "create-val-workflow" => {
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
                Err(e) => ToolResult::error(format!("create-val-workflow failed: {}", e)),
            }
        }

        "update-val-workflow" => {
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
                Err(e) => ToolResult::error(format!("update-val-workflow failed: {}", e)),
            }
        }

        "execute-val-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            let overrides = args.get("overrides").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });

            match workflows::execute_workflow(&domain, &id, overrides).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("execute-val-workflow failed: {}", e)),
            }
        }

        "list-val-workflow-plugins" => {
            let domain = require_str!(args, "domain");
            match workflows::list_plugins(&domain).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-workflow-plugins failed: {}", e)),
            }
        }

        "list-val-workflows" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match workflows::list_workflows(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-workflows failed: {}", e)),
            }
        }

        "get-val-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            match workflows::get_workflow(&domain, &id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-workflow failed: {}", e)),
            }
        }

        "pause-val-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            match workflows::pause_workflow(&domain, &id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("pause-val-workflow failed: {}", e)),
            }
        }

        "resume-val-workflow" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            match workflows::resume_workflow(&domain, &id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("resume-val-workflow failed: {}", e)),
            }
        }

        "list-val-workflow-executions" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match workflows::list_workflow_executions(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-workflow-executions failed: {}", e)),
            }
        }

        "get-val-workflow-execution" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            match workflows::get_workflow_execution(&domain, &id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-workflow-execution failed: {}", e)),
            }
        }

        "get-val-workflow-plugin-schema" => {
            let domain = require_str!(args, "domain");
            let plugin_name = require_str!(args, "plugin_name");
            match workflows::get_plugin_schema(&domain, &plugin_name).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-workflow-plugin-schema failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown workflow tool: {}", name)),
    }
}
