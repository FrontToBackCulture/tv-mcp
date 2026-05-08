// VAL AI Package — bundled assign + generate + push tool.
//
// One MCP call per domain runs the full pipeline that the tv-client
// "Domains → AI" tab exposes as three buttons:
//   1. Assign Skills  — write ai_config.json
//   2. Generate Package — copy skills from _skills/ into the domain's ai/
//   3. Push to S3 — sync ai/ to s3://production.thinkval.static/solutions/{domain}/
//
// The orchestrator skill `/sync-domain-skills` loops domains and calls this
// once per target.

use crate::modules::val_ai;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

pub fn tools() -> Vec<Tool> {
    vec![Tool {
        name: "sync-domain-ai-package".to_string(),
        description:
            "Sync a domain's AI skill package end-to-end: assign skills → generate the local \
             ai/ folder → push to S3. Mirrors the three buttons in tv-client's Domains → AI tab \
             but in one call. Idempotent on assignment: re-running with the same `add` list is \
             a no-op for assignments but still regenerates the package and re-pushes (so use \
             this whenever a skill's source files change). \
             \n\n**Inputs:** \
             \n- `domain` (required): VAL domain slug, e.g. 'lag'. \
             \n- `add`: skill slugs to add to the assignment list. \
             \n- `remove`: skill slugs to remove. \
             \n- `replace`: full replacement list (mutually exclusive with add/remove). \
             \n- `skip_push`: if true, runs assign + generate locally but skips the S3 upload \
               (use for dry-runs before publishing). \
             \n\n**Returns** the before/after assignment list, package stats (skills_copied, \
             instructions_generated), and S3 stats (files_uploaded, bytes, duration_ms). \
             \n\n**Requires:** `knowledge_path` set in tv-mcp settings (points to the \
             tv-knowledge root; skills are read from `{knowledge_path}/_skills/`), and AWS \
             credentials (`aws_access_key_id`, `aws_secret_access_key`) in settings for the S3 \
             push. Domain `global_path` is read from `~/.tv-desktop/val-sync-config.json`."
                .to_string(),
        input_schema: InputSchema::with_properties(
            json!({
                "domain": {
                    "type": "string",
                    "description": "VAL domain slug (e.g. 'lag', 'koi')."
                },
                "add": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Skill slugs to assign. Idempotent — already-assigned slugs are no-ops."
                },
                "remove": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Skill slugs to unassign. Idempotent — slugs not currently assigned are no-ops."
                },
                "replace": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Full replacement skill list. Mutually exclusive with add/remove."
                },
                "skip_push": {
                    "type": "boolean",
                    "description": "If true, runs assign + generate locally but skips the S3 push. Use for dry-runs."
                }
            }),
            vec!["domain".to_string()],
        ),
    }]
}

pub async fn call(name: &str, arguments: Value) -> ToolResult {
    if name != "sync-domain-ai-package" {
        return ToolResult::error(format!("Unknown val_ai tool: {}", name));
    }

    let domain = match arguments.get("domain").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return ToolResult::error("'domain' parameter is required".to_string()),
    };
    let add = string_array(&arguments, "add");
    let remove = string_array(&arguments, "remove");
    let replace = arguments
        .get("replace")
        .filter(|v| !v.is_null())
        .map(|v| string_array_from_value(v));
    let skip_push = arguments
        .get("skip_push")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match val_ai::sync_domain_ai_package(domain, add, remove, replace, skip_push).await {
        Ok(result) => ToolResult::json(&result),
        Err(e) => ToolResult::error(format!("sync-domain-ai-package failed: {}", e)),
    }
}

fn string_array(args: &Value, key: &str) -> Vec<String> {
    match args.get(key) {
        Some(v) => string_array_from_value(v),
        None => Vec::new(),
    }
}

fn string_array_from_value(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
