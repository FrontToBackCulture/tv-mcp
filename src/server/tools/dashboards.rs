// VAL Dashboard MCP Tools — 5 tools.
// No delete-* by design (policy).

use crate::modules::dashboards;
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
            name: "list-val-dashboards".to_string(),
            description:
                "List every VAL dashboard in a domain. Returns id, name, type, summary metadata. \
                 Pass `filters` for query-string narrowing — passed through to listAllDashboards. \
                 Discovery before get/update/duplicate."
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
            name: "get-val-dashboard".to_string(),
            description:
                "Fetch one VAL dashboard's full definition (basicInfo, dashboardInfo, layout, \
                 widgets). Use before `update-val-dashboard` — saveDashboard does a full \
                 INSERT/UPDATE, so partial updates require fetching first and merging \
                 client-side."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": { "type": "string", "description": "Dashboard id" }
                }),
                vec!["domain".to_string(), "id".to_string()],
            ),
        },
        Tool {
            name: "create-val-dashboard".to_string(),
            description:
                "Create a new VAL dashboard. tv-mcp uses the two-step create flow: hits \
                 `createDashboard?name=...&category=...` (server assigns id), then if you \
                 supplied `widgets`, `settings`, or `permission`, follows up with `saveDashboard` \
                 to populate the layout. Required: `dashboard.name`. Optional: `category` \
                 (defaults 'private'), `widgets`, `settings`, `permission`. Returns the new \
                 dashboard's id."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "dashboard": {
                        "type": "object",
                        "description": "Full dashboard payload (basicInfo + dashboardInfo + layout + widgets). No id field."
                    }
                }),
                vec!["domain".to_string(), "dashboard".to_string()],
            ),
        },
        Tool {
            name: "update-val-dashboard".to_string(),
            description:
                "Update an existing VAL dashboard. `dashboard` should be the full payload — \
                 saveDashboard replaces, so partial updates require fetching via \
                 `get-val-dashboard` first and merging client-side."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "id": { "type": "string", "description": "Dashboard id" },
                    "dashboard": {
                        "type": "object",
                        "description": "Full dashboard payload to save."
                    }
                }),
                vec!["domain".to_string(), "id".to_string(), "dashboard".to_string()],
            ),
        },
        Tool {
            name: "duplicate-val-dashboard".to_string(),
            description:
                "Duplicate an existing VAL dashboard into a new one. Provide the source id and \
                 optionally a new display name. Use this instead of `create-val-dashboard` when \
                 you want to clone an existing layout."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "source_id": { "type": "string", "description": "Source dashboard id" },
                    "new_name": {
                        "type": "string",
                        "description": "Optional display name for the new dashboard."
                    }
                }),
                vec!["domain".to_string(), "source_id".to_string()],
            ),
        },
    ]
}

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "list-val-dashboards" => {
            let domain = require_str!(args, "domain");
            let filters = args.get("filters").and_then(|v| {
                if v.is_object() { Some(v.clone()) } else { None }
            });
            match dashboards::list_dashboards(&domain, filters).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("list-val-dashboards failed: {}", e)),
            }
        }

        "get-val-dashboard" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            match dashboards::get_dashboard(&domain, &id).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("get-val-dashboard failed: {}", e)),
            }
        }

        "create-val-dashboard" => {
            let domain = require_str!(args, "domain");
            let dashboard = match args.get("dashboard") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'dashboard' must be an object".to_string()),
            };
            match dashboards::create_dashboard(&domain, dashboard).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-dashboard failed: {}", e)),
            }
        }

        "update-val-dashboard" => {
            let domain = require_str!(args, "domain");
            let id = require_str!(args, "id");
            let dashboard = match args.get("dashboard") {
                Some(v) if v.is_object() => v.clone(),
                _ => return ToolResult::error("'dashboard' must be an object".to_string()),
            };
            match dashboards::update_dashboard(&domain, &id, dashboard).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("update-val-dashboard failed: {}", e)),
            }
        }

        "duplicate-val-dashboard" => {
            let domain = require_str!(args, "domain");
            let source_id = require_str!(args, "source_id");
            let new_name = args.get("new_name").and_then(|v| v.as_str());
            match dashboards::duplicate_dashboard(&domain, &source_id, new_name).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("duplicate-val-dashboard failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown dashboard tool: {}", name)),
    }
}
