// Work Module - Task Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;
use chrono::{Datelike, Timelike};

/// List tasks with optional filters

pub async fn work_list_tasks(
    project_id: Option<String>,
    status_id: Option<String>,
    status_type: Option<String>,
    milestone_id: Option<String>,
    company_id: Option<String>,
    task_type: Option<String>,
) -> CmdResult<Vec<Task>> {
    let client = get_client().await?;

    let mut filters = vec!["select=*,project:projects(*),status:task_statuses(*),assignees:task_assignees(user:users(*))".to_string()];

    if let Some(pid) = project_id {
        filters.push(format!("project_id=eq.{}", pid));
    }
    if let Some(sid) = status_id {
        filters.push(format!("status_id=eq.{}", sid));
    }
    if let Some(st) = status_type {
        filters.push(format!("status.type=eq.{}", st));
    }
    if let Some(mid) = milestone_id {
        filters.push(format!("milestone_id=eq.{}", mid));
    }
    if let Some(cid) = company_id {
        filters.push(format!("company_id=eq.{}", cid));
    }
    if let Some(tt) = task_type {
        filters.push(format!("task_type=eq.{}", tt));
    }

    filters.push("order=sort_order.asc,created_at.desc".to_string());

    let query = filters.join("&");
    client.select("tasks", &query).await
}

/// Get a single task by ID

pub async fn work_get_task(task_id: String) -> CmdResult<Task> {
    let client = get_client().await?;

    let query = format!(
        "select=*,project:projects(*),status:task_statuses(*),assignees:task_assignees(user:users(*))&id=eq.{}",
        task_id
    );

    client
        .select_single("tasks", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("Task not found: {}", task_id)))
}

/// Create a new task

pub async fn work_create_task(data: CreateTask) -> CmdResult<Task> {
    let client = get_client().await?;

    // Get next task number for the project
    let project: Project = client
        .select_single(
            "projects",
            &format!("id=eq.{}", data.project_id),
        )
        .await?
        .ok_or_else(|| CommandError::NotFound("Project not found".into()))?;

    let next_number = project.next_task_number.unwrap_or(1);

    // Leave description_json null — tv-client's TaskDetailPanel falls back to
    // ReactMarkdown on the plain description column when description_json is null,
    // which handles markdown (bold, links, lists, etc.) correctly. The previous
    // pulldown_cmark-based conversion often emitted raw markdown as literal text
    // nodes, so relying on the markdown fallback is simpler and more reliable.
    let insert_data = serde_json::json!({
        "project_id": data.project_id,
        "status_id": data.status_id,
        "title": data.title,
        "description": data.description,
        "description_json": serde_json::Value::Null,
        "priority": data.priority.unwrap_or(0),
        "due_date": data.due_date,
        "milestone_id": data.milestone_id,
        "depends_on": data.depends_on,
        "session_ref": data.session_ref,
        "requires_review": data.requires_review,
        "company_id": data.company_id,
        "contact_id": data.contact_id,
        "task_type": data.task_type,
        "task_type_changed_at": if data.task_type.is_some() { Some(chrono::Utc::now().to_rfc3339()) } else { None },
        "task_number": next_number
    });

    // Create task
    let task: Task = client.insert("tasks", &insert_data).await?;

    // Increment project's next_task_number
    let update_data = serde_json::json!({ "next_task_number": next_number + 1 });
    let _: serde_json::Value = client
        .update("projects", &format!("id=eq.{}", data.project_id), &update_data)
        .await?;

    // Insert assignees into junction table
    if let Some(assignee_ids) = &data.assignee_ids {
        for user_id in assignee_ids {
            let row = serde_json::json!({ "task_id": task.id, "user_id": user_id });
            let result: Result<serde_json::Value, _> = client.insert("task_assignees", &row).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("duplicate") && !msg.contains("23505") {
                    return Err(e);
                }
            }
        }
    }

    // Return task with joins
    work_get_task(task.id).await
}

/// Update a task

pub async fn work_update_task(task_id: String, data: UpdateTask) -> CmdResult<Task> {
    let client = get_client().await?;

    // Handle assignee replacement first (independent of other fields)
    if let Some(assignee_ids) = &data.assignee_ids {
        client.delete("task_assignees", &format!("task_id=eq.{}", task_id)).await?;
        for user_id in assignee_ids {
            let row = serde_json::json!({ "task_id": task_id, "user_id": user_id });
            let result: Result<serde_json::Value, _> = client.insert("task_assignees", &row).await;
            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("duplicate") && !msg.contains("23505") {
                    return Err(e);
                }
            }
        }
    }

    let mut update_data = serde_json::to_value(&data)?;

    // If description is being updated, clear description_json so the UI falls
    // back to ReactMarkdown on the fresh markdown. See work_create_task for why.
    if data.description.is_some() {
        if let Some(obj) = update_data.as_object_mut() {
            obj.insert("description_json".to_string(), serde_json::Value::Null);
        }
    }

    // If task_type is changing, update task_type_changed_at
    if data.task_type.is_some() {
        if let Some(obj) = update_data.as_object_mut() {
            obj.insert(
                "task_type_changed_at".to_string(),
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            );
        }
    }

    // Check if status is changing to completed
    if let Some(status_id) = &data.status_id {
        let status: Option<TaskStatus> = client
            .select_single("task_statuses", &format!("id=eq.{}", status_id))
            .await?;

        if let Some(s) = status {
            if s.status_type == "completed" {
                if let Some(obj) = update_data.as_object_mut() {
                    obj.insert(
                        "completed_at".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                }
            }
        }
    }

    let _: serde_json::Value = client
        .update("tasks", &format!("id=eq.{}", task_id), &update_data)
        .await?;

    work_get_task(task_id).await
}

/// Delete a task

pub async fn work_delete_task(task_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", task_id);
    client.delete("tasks", &query).await
}

/// Add labels to a task

pub async fn work_add_task_labels(task_id: String, label_ids: Vec<String>) -> CmdResult<()> {
    let client = get_client().await?;

    for label_id in label_ids {
        let data = serde_json::json!({
            "task_id": task_id,
            "label_id": label_id
        });
        // Use upsert behavior by catching conflicts
        let result: Result<serde_json::Value, _> = client.insert("task_labels", &data).await;
        if let Err(e) = result {
            // Ignore duplicate key errors
            let msg = e.to_string();
            if !msg.contains("duplicate") && !msg.contains("23505") {
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Remove labels from a task

pub async fn work_remove_task_labels(task_id: String, label_ids: Vec<String>) -> CmdResult<()> {
    let client = get_client().await?;

    for label_id in label_ids {
        let query = format!("task_id=eq.{}&label_id=eq.{}", task_id, label_id);
        client.delete("task_labels", &query).await?;
    }

    Ok(())
}

/// Add assignees to a task

pub async fn work_add_task_assignees(task_id: String, user_ids: Vec<String>) -> CmdResult<()> {
    let client = get_client().await?;
    for user_id in user_ids {
        let data = serde_json::json!({ "task_id": task_id, "user_id": user_id });
        let result: Result<serde_json::Value, _> = client.insert("task_assignees", &data).await;
        if let Err(e) = result {
            let msg = e.to_string();
            if !msg.contains("duplicate") && !msg.contains("23505") {
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Remove assignees from a task

pub async fn work_remove_task_assignees(task_id: String, user_ids: Vec<String>) -> CmdResult<()> {
    let client = get_client().await?;
    for user_id in user_ids {
        let query = format!("task_id=eq.{}&user_id=eq.{}", task_id, user_id);
        client.delete("task_assignees", &query).await?;
    }
    Ok(())
}

// ============================================================================
// Task Triage (Claude-powered)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskTriageProgress {
    pub message: String,
    pub phase: String, // "starting", "running", "complete", "error"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriageContext {
    pub id: String,
    pub level: String,
    pub name: String,
    pub text: String,
    pub boost: i32,
    pub suppress: bool,
    pub match_team_id: Option<String>,
    pub match_user_id: Option<String>,
    pub match_project_id: Option<String>,
    pub match_company_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextMatch {
    pub context_id: String,
    pub context_name: String,
    pub level: String,
    pub boost: i32,       // weighted contribution (points added/subtracted)
    pub raw_boost: i32,   // original boost value from the context
    pub text: String,     // context text for display
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriageProposal {
    pub task_id: String,
    pub title: String,
    pub project: String,
    #[serde(default = "default_type")]
    #[serde(rename = "type")]
    pub item_type: String,          // "task" or "deal"
    pub triage_score: i32,
    pub triage_action: String,      // do_now, do_this_week, defer, delegate, kill
    pub triage_reason: String,      // actionable next step
    // Structured metadata
    pub due_date: Option<String>,           // original due date
    pub days_overdue: Option<i32>,          // positive = overdue, 0 = today, negative = days until
    pub suggested_due_date: Option<String>, // if recommending a reschedule
    // Deal-specific
    pub deal_stage: Option<String>,
    pub deal_value: Option<f64>,
    pub days_stale: Option<i32>,            // days since last CRM activity
    pub company: Option<String>,
    // Context influence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_matches: Option<Vec<ContextMatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_bonus: Option<i32>,
}

fn default_type() -> String {
    "task".to_string()
}


/// Apply a single triage proposal to a task

pub async fn work_apply_triage(
    task_id: String,
    triage_score: i32,
    triage_action: String,
    triage_reason: String,
    context_matches: Option<serde_json::Value>,
) -> CmdResult<Task> {
    let client = get_client().await?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut update_data = serde_json::json!({
        "triage_score": triage_score,
        "triage_action": triage_action,
        "triage_reason": triage_reason,
        "last_triaged_at": now
    });
    if let Some(cm) = context_matches {
        update_data["triage_context_matches"] = cm;
    } else {
        update_data["triage_context_matches"] = serde_json::Value::Null;
    }

    let _: serde_json::Value = client
        .update("tasks", &format!("id=eq.{}", task_id), &update_data)
        .await?;

    work_get_task(task_id).await
}

/// Generate a strategic triage summary using Claude — pulls full context from Supabase

pub async fn work_triage_summary(proposals_json: String) -> CmdResult<String> {
    let api_key = crate::core::settings::settings_get_key(
        crate::core::settings::KEY_ANTHROPIC_API.to_string(),
    )?
    .ok_or_else(|| CommandError::Config("Anthropic API key not configured".into()))?;

    let sb_client = get_client().await?;

    // Load prompt config
    let configs: Vec<serde_json::Value> = sb_client
        .select("triage_config", "id=eq.default")
        .await
        .unwrap_or_default();
    let config = configs.first();
    let system = config
        .and_then(|c| c["summary_system_prompt"].as_str())
        .unwrap_or("Write a strategic summary.")
        .to_string();
    let model = config
        .and_then(|c| c["summary_model"].as_str())
        .unwrap_or("claude-sonnet-4-6-20250514")
        .to_string();
    let max_tokens = config
        .and_then(|c| c["summary_max_tokens"].as_i64())
        .unwrap_or(1000) as u32;

    // ── Pull full context from Supabase ──

    // 1. Initiatives
    let initiatives: Vec<serde_json::Value> = sb_client
        .select("initiatives", "select=id,name,status,health,description&status=in.(active,planned)&order=name")
        .await.unwrap_or_default();

    // 2. Initiative-project links
    let init_links: Vec<serde_json::Value> = sb_client
        .select("initiative_project_links", "select=initiative_id,project_id")
        .await.unwrap_or_default();

    // 3. Active projects with company info
    let projects: Vec<serde_json::Value> = sb_client
        .select("projects", "select=id,name,project_type,status,description,color,company:crm_companies(name,display_name,stage)&status=eq.active&order=name")
        .await.unwrap_or_default();

    // 4. Active tasks with status, assignees, description
    let tasks: Vec<serde_json::Value> = sb_client
        .select("tasks", "select=id,title,description,priority,due_date,triage_score,triage_action,triage_reason,project_id,status:task_statuses(name,type),assignees:task_assignees(user:users(name)),company:crm_companies!tasks_company_id_fkey(name,display_name)&status.type=neq.complete&order=due_date.asc.nullslast")
        .await.unwrap_or_default();

    // 5. Active plans
    let plans: Vec<serde_json::Value> = sb_client
        .select("plans", "select=horizon,title&status=eq.active&order=horizon")
        .await.unwrap_or_default();

    // ── Build structured context ──
    let mut context = String::with_capacity(20000);

    // Plans
    if !plans.is_empty() {
        context.push_str("=== ACTIVE PLANS ===\n");
        for p in &plans {
            let horizon = p["horizon"].as_str().unwrap_or("?");
            let title = p["title"].as_str().unwrap_or("");
            context.push_str(&format!("[{}] {}\n", horizon, title));
        }
        context.push('\n');
    }

    // Build project-to-initiative map
    let mut proj_to_init: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for link in &init_links {
        let iid = link["initiative_id"].as_str().unwrap_or("");
        let pid = link["project_id"].as_str().unwrap_or("");
        if let Some(init) = initiatives.iter().find(|i| i["id"].as_str() == Some(iid)) {
            proj_to_init.insert(pid.to_string(), init["name"].as_str().unwrap_or("").to_string());
        }
    }

    // Build project-to-tasks map
    let mut proj_tasks: std::collections::HashMap<String, Vec<&serde_json::Value>> = std::collections::HashMap::new();
    for t in &tasks {
        let pid = t["project_id"].as_str().unwrap_or("none").to_string();
        proj_tasks.entry(pid).or_default().push(t);
    }

    // Group by initiative
    context.push_str("=== INITIATIVES & PROJECTS ===\n");
    let mut seen_projects = std::collections::HashSet::new();

    for init in &initiatives {
        let init_name = init["name"].as_str().unwrap_or("?");
        let init_status = init["status"].as_str().unwrap_or("?");
        let init_health = init["health"].as_str().unwrap_or("?");
        let init_desc = init["description"].as_str().unwrap_or("");
        context.push_str(&format!("\nINITIATIVE: {} [{}] [{}]\n", init_name, init_status, init_health));
        if !init_desc.is_empty() { context.push_str(&format!("  {}\n", &init_desc[..init_desc.len().min(200)])); }

        // Find projects in this initiative
        for link in &init_links {
            if link["initiative_id"].as_str() != init["id"].as_str() { continue; }
            let pid = link["project_id"].as_str().unwrap_or("");
            seen_projects.insert(pid.to_string());

            if let Some(proj) = projects.iter().find(|p| p["id"].as_str() == Some(pid)) {
                let pname = proj["name"].as_str().unwrap_or("?");
                let ptype = proj["project_type"].as_str().unwrap_or("work");
                let company = proj["company"]["display_name"].as_str()
                    .or(proj["company"]["name"].as_str())
                    .unwrap_or("");
                let stage = proj["company"]["stage"].as_str().unwrap_or("");
                let pdesc = proj["description"].as_str().unwrap_or("");

                context.push_str(&format!("  PROJECT: {} ({}{}{})\n",
                    pname, ptype,
                    if !company.is_empty() { format!(", {}", company) } else { String::new() },
                    if !stage.is_empty() { format!(", {}", stage) } else { String::new() },
                ));
                if !pdesc.is_empty() { context.push_str(&format!("    {}\n", &pdesc[..pdesc.len().min(150)])); }

                // Tasks in this project
                if let Some(ptasks) = proj_tasks.get(pid) {
                    for t in ptasks.iter().take(10) {
                        let title = t["title"].as_str().unwrap_or("?");
                        let status = t["status"]["name"].as_str().unwrap_or("?");
                        let due = t["due_date"].as_str().unwrap_or("no date");
                        let score = t["triage_score"].as_i64().map(|s| format!(" [{}]", s)).unwrap_or_default();
                        let action = t["triage_action"].as_str().unwrap_or("");
                        let assignees: Vec<&str> = t["assignees"].as_array()
                            .map(|a| a.iter().filter_map(|x| x["user"]["name"].as_str()).collect())
                            .unwrap_or_default();
                        let desc_preview = t["description"].as_str()
                            .map(|d| if d.len() > 100 { format!(" — {}...", &d[..100]) } else { format!(" — {}", d) })
                            .unwrap_or_default();

                        context.push_str(&format!("    - [{}] {} (due: {}, {}{}) {}{}\n",
                            status, title, due,
                            if !assignees.is_empty() { assignees.join(", ") } else { "unassigned".into() },
                            if !action.is_empty() { format!(", {}{}", action, score) } else { String::new() },
                            desc_preview,
                            "",
                        ));
                    }
                    if ptasks.len() > 10 {
                        context.push_str(&format!("    ... and {} more tasks\n", ptasks.len() - 10));
                    }
                }
            }
        }
    }

    // Unlinked projects
    let unlinked: Vec<&serde_json::Value> = projects.iter()
        .filter(|p| !seen_projects.contains(p["id"].as_str().unwrap_or("")))
        .collect();
    if !unlinked.is_empty() {
        context.push_str("\nUNLINKED PROJECTS (not in any initiative):\n");
        for proj in unlinked.iter().take(20) {
            let pid = proj["id"].as_str().unwrap_or("");
            let pname = proj["name"].as_str().unwrap_or("?");
            let ptype = proj["project_type"].as_str().unwrap_or("work");
            let task_count = proj_tasks.get(pid).map(|t| t.len()).unwrap_or(0);
            context.push_str(&format!("  {} ({}, {} tasks)\n", pname, ptype, task_count));
        }
    }

    // Active triage contexts (strategic priorities set by admin)
    let triage_ctx_rows: Vec<serde_json::Value> = sb_client
        .select("triage_contexts", "select=*&active=eq.true&order=level,name")
        .await.unwrap_or_default();

    if !triage_ctx_rows.is_empty() {
        // Fetch weights for display
        let tc_weights = config
            .and_then(|c| c.get("context_weights"))
            .cloned()
            .unwrap_or(serde_json::json!({"company": 10, "team": 15, "individual": 10, "product": 25, "customer": 40}));

        context.push_str("\n=== ACTIVE TRIAGE CONTEXTS (admin-set strategic priorities) ===\n");
        context.push_str("These contexts influence scoring. Use them to frame your recommendations.\n\n");
        for row in &triage_ctx_rows {
            let level = row["level"].as_str().unwrap_or("?");
            let name = row["name"].as_str().unwrap_or("?");
            let text = row["text"].as_str().unwrap_or("");
            let boost = row["boost"].as_i64().unwrap_or(10);
            let suppress = row["suppress"].as_bool().unwrap_or(false);
            let weight = tc_weights.get(level).and_then(|v| v.as_i64()).unwrap_or(0);
            let modifier = if suppress { format!("-{}", boost.abs()) } else { format!("+{}", boost) };
            context.push_str(&format!("[{}] {} (weight: {}%, boost: {}): {}\n",
                level.to_uppercase(), name, weight, modifier, text));
        }
        context.push('\n');
    }

    // Triage results (from frontend)
    context.push_str(&format!("\n=== TRIAGE SCORES ===\n{}\n", proposals_json));

    eprintln!("[triage_summary] Built context: {} chars, sending to {} ...", context.len(), model);

    // ── Call Claude ──
    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": 0.3,
            "system": system,
            "messages": [{ "role": "user", "content": context }]
        }))
        .send()
        .await
        .map_err(|e| CommandError::Network(format!("API request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    #[derive(serde::Deserialize)]
    struct ContentBlock { text: Option<String> }
    #[derive(serde::Deserialize)]
    struct ApiResponse { content: Vec<ContentBlock> }

    let api_response: ApiResponse = response.json().await?;
    let summary = api_response.content.first()
        .and_then(|c| c.text.clone())
        .unwrap_or_else(|| "Triage complete — no summary generated.".to_string());

    eprintln!("[triage_summary] Summary: {}", &summary[..summary.len().min(200)]);
    Ok(summary)
}


// ============================================================================
// Triage Context CRUD
// ============================================================================

/// List all triage contexts

pub async fn work_list_triage_contexts() -> CmdResult<Vec<serde_json::Value>> {
    let client = get_client().await?;
    let rows: Vec<serde_json::Value> = client
        .select("triage_contexts", "select=*&order=level,name")
        .await?;
    Ok(rows)
}

/// Create or update a triage context

pub async fn work_upsert_triage_context(data: serde_json::Value) -> CmdResult<serde_json::Value> {
    let client = get_client().await?;

    let has_id = data.get("id").and_then(|v| v.as_str()).map_or(false, |s| !s.is_empty());

    if has_id {
        let id = data["id"].as_str().unwrap();
        let mut update = data.clone();
        update["updated_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
        let result: serde_json::Value = client
            .update("triage_contexts", &format!("id=eq.{}", id), &update)
            .await?;
        Ok(result)
    } else {
        let result: serde_json::Value = client
            .insert("triage_contexts", &data)
            .await?;
        Ok(result)
    }
}

/// Delete a triage context

pub async fn work_delete_triage_context(id: String) -> CmdResult<()> {
    let client = get_client().await?;
    client.delete("triage_contexts", &format!("id=eq.{}", id)).await?;
    Ok(())
}

/// Get context weights from triage_config

pub async fn work_get_context_weights() -> CmdResult<serde_json::Value> {
    let client = get_client().await?;
    let rows: Vec<serde_json::Value> = client
        .select("triage_config", "id=eq.default")
        .await.unwrap_or_default();
    let weights = rows.first()
        .and_then(|c| c.get("context_weights"))
        .cloned()
        .unwrap_or(serde_json::json!({"company": 10, "team": 15, "individual": 10, "product": 25, "customer": 40}));
    Ok(weights)
}

/// Update context weights in triage_config

pub async fn work_set_context_weights(weights: serde_json::Value) -> CmdResult<serde_json::Value> {
    let client = get_client().await?;
    let update = serde_json::json!({ "context_weights": weights });
    let result: serde_json::Value = client
        .update("triage_config", "id=eq.default", &update)
        .await?;
    Ok(result)
}
