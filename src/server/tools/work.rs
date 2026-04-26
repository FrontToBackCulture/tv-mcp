// Project Module MCP Tools
// Project, task, milestone, initiative, and label management tools

use crate::modules::work::{
    self, CreateInitiative, CreateLabel, CreateMilestone, CreateProject, CreateProjectUpdate,
    CreateProjectSession, CreateProjectArtifact, UpsertProjectContext, UpdateProjectSession,
    CreateTask, UpdateInitiative, UpdateMilestone, UpdateProject, UpdateTask,
    RegisterSkill,
};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define Project module tools
pub fn tools() -> Vec<Tool> {
    vec![
        // Projects
        Tool {
            name: "list-projects".to_string(),
            description: "List projects. Use project_type filter to show only work or deal projects.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "include_statuses": {
                        "type": "boolean",
                        "description": "Include task statuses for each project"
                    },
                    "project_type": {
                        "type": "string",
                        "enum": ["work", "deal"],
                        "description": "Filter by project type (default: all types)"
                    }
                }),
                vec![],
            ),
        },
        Tool {
            name: "get-project".to_string(),
            description: "Get details for a specific project by ID".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": {
                        "type": "string",
                        "description": "The project UUID"
                    }
                }),
                vec!["project_id".to_string()],
            ),
        },
        Tool {
            name: "create-project".to_string(),
            description: "Create a new project with default statuses. Set project_type to 'deal' for deal projects with pipeline stages.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Project name (required)" },
                    "description": { "type": "string", "description": "Project description" },
                    "slug": { "type": "string", "description": "URL-friendly identifier" },
                    "icon": { "type": "string", "description": "Icon identifier" },
                    "color": { "type": "string", "description": "Hex color" },
                    "identifier_prefix": { "type": "string", "description": "Task ID prefix (e.g., 'PRD')" },
                    "project_type": { "type": "string", "enum": ["work", "deal"], "description": "Project type (default: work)" },
                    "company_id": { "type": "string", "description": "Company UUID (for deal type)" },
                    "deal_stage": { "type": "string", "enum": ["target", "prospect", "lead", "qualified", "pilot", "proposal", "negotiation", "won", "lost"], "description": "Deal stage (for deal type)" },
                    "deal_solution": { "type": "string", "description": "Solution category (for deal type)" },
                    "deal_value": { "type": "number", "description": "Deal value (for deal type)" },
                    "deal_currency": { "type": "string", "description": "Currency code (default: SGD)" },
                    "deal_expected_close": { "type": "string", "description": "Expected close date YYYY-MM-DD (for deal type)" },
                    "deal_notes": { "type": "string", "description": "Deal notes (for deal type)" }
                }),
                vec!["name".to_string()],
            ),
        },
        Tool {
            name: "update-project".to_string(),
            description: "Update an existing project (works for all project types: work, deal)".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "The project UUID (required)" },
                    "name": { "type": "string", "description": "New project name" },
                    "description": { "type": "string", "description": "New description" },
                    "health": { "type": "string", "enum": ["on_track", "at_risk", "off_track"] },
                    "priority": { "type": "integer", "enum": [0, 1, 2, 3, 4] },
                    "status": { "type": "string", "enum": ["planned", "active", "completed", "paused"] },
                    "target_date": { "type": "string", "description": "Target date (YYYY-MM-DD)" },
                    "deal_stage": { "type": "string", "enum": ["target", "prospect", "lead", "qualified", "pilot", "proposal", "negotiation", "won", "lost"] },
                    "deal_value": { "type": "number" },
                    "deal_solution": { "type": "string" },
                    "deal_expected_close": { "type": "string" },
                    "deal_actual_close": { "type": "string" },
                    "deal_lost_reason": { "type": "string" },
                    "deal_won_notes": { "type": "string" },
                    "deal_proposal_path": { "type": "string" },
                    "deal_order_form_path": { "type": "string" },
                    "deal_notes": { "type": "string" },
                    "deal_stage_changed_at": { "type": "string", "description": "Manually set stage_changed_at timestamp" },
                    "preserve_stage_date": { "type": "boolean", "description": "If true, don't update deal_stage_changed_at when stage changes" }
                }),
                vec!["project_id".to_string()],
            ),
        },
        Tool {
            name: "delete-project".to_string(),
            description: "Delete a project (soft delete via archived_at).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "The project UUID (required)" }
                }),
                vec!["project_id".to_string()],
            ),
        },
        // Tasks
        Tool {
            name: "list-tasks".to_string(),
            description: "List tasks with optional filters".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Filter by project UUID" },
                    "status_id": { "type": "string", "description": "Filter by status UUID" },
                    "status_type": { "type": "string", "enum": ["backlog", "unstarted", "started", "review", "completed", "canceled"] },
                    "milestone_id": { "type": "string", "description": "Filter by milestone UUID" },
                    "company_id": { "type": "string", "description": "Filter by CRM company UUID" },
                    "task_type": { "type": "string", "enum": ["general", "target", "prospect", "follow_up"], "description": "Filter by task type" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "get-task".to_string(),
            description: "Get details for a specific task by ID".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "task_id": { "type": "string", "description": "The task UUID" }
                }),
                vec!["task_id".to_string()],
            ),
        },
        Tool {
            name: "create-task".to_string(),
            description: "Create a new task in a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Project UUID (required)" },
                    "status_id": { "type": "string", "description": "Status UUID (required)" },
                    "title": { "type": "string", "description": "Task title (required)" },
                    "description": { "type": "string", "description": "Task description" },
                    "priority": { "type": "integer", "description": "Priority: 0=None, 1=Urgent, 2=High, 3=Medium, 4=Low" },
                    "due_date": { "type": "string", "description": "Due date (YYYY-MM-DD)" },
                    "assignee_ids": { "type": "array", "items": { "type": "string" }, "description": "Array of user UUIDs to assign" },
                    "milestone_id": { "type": "string", "description": "Milestone UUID" },
                    "depends_on": { "type": "array", "items": { "type": "string" }, "description": "Task IDs this depends on" },
                    "session_ref": { "type": "string", "description": "Session folder path" },
                    "requires_review": { "type": "boolean", "description": "Requires human review" },
                    "company_id": { "type": "string", "description": "CRM company UUID" },
                    "contact_id": { "type": "string", "description": "CRM contact UUID" },
                    "task_type": { "type": "string", "enum": ["general", "target", "prospect", "follow_up"], "description": "Task type" }
                }),
                vec!["project_id".to_string(), "status_id".to_string(), "title".to_string()],
            ),
        },
        Tool {
            name: "update-task".to_string(),
            description: "Update an existing task".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "task_id": { "type": "string", "description": "The task UUID (required)" },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "status_id": { "type": "string" },
                    "priority": { "type": "integer" },
                    "due_date": { "type": "string" },
                    "assignee_ids": { "type": "array", "items": { "type": "string" }, "description": "Replaces all current assignees" },
                    "milestone_id": { "type": "string" },
                    "depends_on": { "type": "array", "items": { "type": "string" } },
                    "session_ref": { "type": "string" },
                    "requires_review": { "type": "boolean" },
                    "company_id": { "type": "string", "description": "CRM company UUID" },
                    "contact_id": { "type": "string", "description": "CRM contact UUID" },
                    "task_type": { "type": "string", "enum": ["general", "target", "prospect", "follow_up"], "description": "Task type" },
                    "triage_score": { "type": "integer", "description": "AI triage priority score 0-100 (higher = more urgent)" },
                    "triage_action": { "type": "string", "enum": ["do_now", "do_this_week", "defer", "delegate", "kill"], "description": "Recommended triage action" },
                    "triage_reason": { "type": "string", "description": "Justification for the triage score and action" },
                    "last_triaged_at": { "type": "string", "description": "ISO timestamp of when task was last triaged" }
                }),
                vec!["task_id".to_string()],
            ),
        },
        // Milestones
        Tool {
            name: "list-milestones".to_string(),
            description: "List milestones for a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "The project UUID" }
                }),
                vec!["project_id".to_string()],
            ),
        },
        Tool {
            name: "create-milestone".to_string(),
            description: "Create a new milestone for a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Project UUID (required)" },
                    "name": { "type": "string", "description": "Milestone name (required)" },
                    "description": { "type": "string", "description": "Milestone description" },
                    "target_date": { "type": "string", "description": "Target date (YYYY-MM-DD)" }
                }),
                vec!["project_id".to_string(), "name".to_string()],
            ),
        },
        Tool {
            name: "update-milestone".to_string(),
            description: "Update an existing milestone".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "milestone_id": { "type": "string", "description": "The milestone UUID (required)" },
                    "name": { "type": "string" },
                    "description": { "type": "string" },
                    "target_date": { "type": "string" }
                }),
                vec!["milestone_id".to_string()],
            ),
        },
        // Initiatives
        Tool {
            name: "list-initiatives".to_string(),
            description: "List all initiatives".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "include": { "type": "string", "enum": ["progress", "projects"], "description": "Include additional data" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "create-initiative".to_string(),
            description: "Create a new initiative (strategic layer above projects)".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Initiative name (required)" },
                    "description": { "type": "string" },
                    "slug": { "type": "string" },
                    "icon": { "type": "string" },
                    "color": { "type": "string" },
                    "owner": { "type": "string" },
                    "status": { "type": "string", "enum": ["planned", "active", "completed", "paused"] },
                    "health": { "type": "string", "enum": ["on_track", "at_risk", "off_track"] },
                    "target_date": { "type": "string" }
                }),
                vec!["name".to_string()],
            ),
        },
        Tool {
            name: "update-initiative".to_string(),
            description: "Update an existing initiative".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": { "type": "string", "description": "The initiative UUID (required)" },
                    "name": { "type": "string" },
                    "description": { "type": "string" },
                    "owner": { "type": "string" },
                    "status": { "type": "string", "enum": ["planned", "active", "completed", "paused"] },
                    "health": { "type": "string", "enum": ["on_track", "at_risk", "off_track"] },
                    "target_date": { "type": "string" }
                }),
                vec!["initiative_id".to_string()],
            ),
        },
        Tool {
            name: "delete-initiative".to_string(),
            description: "Delete an initiative (soft delete via archived_at).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": { "type": "string", "description": "The initiative UUID (required)" }
                }),
                vec!["initiative_id".to_string()],
            ),
        },
        // Initiative-Project linking
        Tool {
            name: "add-project-to-initiative".to_string(),
            description: "Link a project to an initiative. A project can only belong to one initiative.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": { "type": "string", "description": "The initiative UUID (required)" },
                    "project_id": { "type": "string", "description": "The project UUID to add (required)" }
                }),
                vec!["initiative_id".to_string(), "project_id".to_string()],
            ),
        },
        Tool {
            name: "remove-project-from-initiative".to_string(),
            description: "Remove a project from an initiative.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": { "type": "string", "description": "The initiative UUID (required)" },
                    "project_id": { "type": "string", "description": "The project UUID to remove (required)" }
                }),
                vec!["initiative_id".to_string(), "project_id".to_string()],
            ),
        },
        Tool {
            name: "list-initiative-projects".to_string(),
            description: "List all projects within an initiative.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "initiative_id": { "type": "string", "description": "The initiative UUID (required)" }
                }),
                vec!["initiative_id".to_string()],
            ),
        },
        // Labels
        Tool {
            name: "list-labels".to_string(),
            description: "List all labels".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "create-label".to_string(),
            description: "Create a new label".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Label name (required)" },
                    "color": { "type": "string", "description": "Hex color" },
                    "description": { "type": "string" }
                }),
                vec!["name".to_string()],
            ),
        },
        // Skills
        Tool {
            name: "register-skill".to_string(),
            description: "Register or update a skill in the Supabase skills registry. Upserts on slug.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "slug": { "type": "string", "description": "Skill slug matching the _skills/{slug}/ folder name (required)" },
                    "name": { "type": "string", "description": "Display name (defaults to slug)" },
                    "description": { "type": "string", "description": "What the skill does" },
                    "category": { "type": "string", "description": "Category (e.g. Delivery, Bot)" },
                    "subcategory": { "type": "string", "description": "Subcategory" },
                    "target": { "type": "string", "description": "Target audience (e.g. platform)" },
                    "status": { "type": "string", "enum": ["active", "deprecated", "draft"], "description": "Skill status (default: active)" },
                    "skill_type": { "type": "string", "enum": ["chat", "diagnostic", "report", "other"], "description": "Skill type" },
                    "domain": { "type": "string", "description": "Domain if domain-specific" },
                    "verified": { "type": "boolean", "description": "Whether skill is verified" },
                    "owner": { "type": "string", "description": "Skill owner" }
                }),
                vec!["slug".to_string()],
            ),
        },
        Tool {
            name: "list-skills".to_string(),
            description: "List all skills in the registry".to_string(),
            input_schema: InputSchema::empty(),
        },
        // Users
        Tool {
            name: "list-users".to_string(),
            description: "List all users (humans and bots)".to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "list-bots".to_string(),
            description: "List all registered bots".to_string(),
            input_schema: InputSchema::empty(),
        },
        // Pipeline
        Tool {
            name: "get-pipeline".to_string(),
            description: "Get deal pipeline statistics: total deals, value by stage, counts.".to_string(),
            input_schema: InputSchema::empty(),
        },
        // Project updates
        Tool {
            name: "list-project-updates".to_string(),
            description: "List status updates for a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "The project UUID" }
                }),
                vec!["project_id".to_string()],
            ),
        },
        Tool {
            name: "create-project-update".to_string(),
            description: "Create a status update for a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "The project UUID (required)" },
                    "content": { "type": "string", "description": "Update content (required)" },
                    "health": { "type": "string", "enum": ["on_track", "at_risk", "off_track"] },
                    "created_by": { "type": "string", "description": "User UUID of creator" }
                }),
                vec!["project_id".to_string(), "content".to_string()],
            ),
        },
        // Project sessions
        Tool {
            name: "add-project-session".to_string(),
            description: "Add a session entry to a project. If conversation_id matches an existing session, updates it instead.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Project UUID (required)" },
                    "summary": { "type": "string", "description": "What was accomplished" },
                    "decisions": { "type": "array", "items": { "type": "string" }, "description": "Decisions made" },
                    "next_steps": { "type": "array", "items": { "type": "string" }, "description": "Next steps" },
                    "open_questions": { "type": "array", "items": { "type": "string" }, "description": "Open questions" },
                    "notes": { "type": "string", "description": "Full session notes or transcript" },
                    "conversation_id": { "type": "string", "description": "Claude Code session UUID (for upsert dedup)" }
                }),
                vec!["project_id".to_string()],
            ),
        },
        Tool {
            name: "update-project-session".to_string(),
            description: "Update an existing project session entry".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "session_id": { "type": "string", "description": "Session UUID (required)" },
                    "summary": { "type": "string" },
                    "decisions": { "type": "array", "items": { "type": "string" } },
                    "next_steps": { "type": "array", "items": { "type": "string" } },
                    "open_questions": { "type": "array", "items": { "type": "string" } },
                    "notes": { "type": "string" },
                    "conversation_id": { "type": "string" }
                }),
                vec!["session_id".to_string()],
            ),
        },
        // Project artifacts
        Tool {
            name: "add-project-artifact".to_string(),
            description: "Add an artifact (file, skill, entity) to a project for tracking. Only reference files within tv-knowledge — never code repos (tv-client, tv-api, etc.). Never use type 'task'.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Project UUID (required)" },
                    "type": { "type": "string", "description": "Artifact type: skill, doc, crm_company, crm_deal, proposal, other (required). Never use 'task'." },
                    "reference": { "type": "string", "description": "Artifact identifier — UUID for DB entities, path relative to tv-knowledge for files (required). Must NOT reference code repos (tv-client/, tv-api/, etc.)." },
                    "label": { "type": "string", "description": "Human-readable label (required)" },
                    "session_id": { "type": "string", "description": "Session UUID or conversation_id to link artifact to" },
                    "preview_content": { "type": "string", "description": "Preview text for the artifact" }
                }),
                vec!["project_id".to_string(), "type".to_string(), "reference".to_string(), "label".to_string()],
            ),
        },
        Tool {
            name: "remove-project-artifact".to_string(),
            description: "Remove an artifact from a project".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "artifact_id": { "type": "string", "description": "Artifact UUID (required)" }
                }),
                vec!["artifact_id".to_string()],
            ),
        },
        // Project context
        Tool {
            name: "update-project-context".to_string(),
            description: "Upsert the rolling context for a project (context_summary, current_state, key_decisions). Used for cold-start context loading.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "project_id": { "type": "string", "description": "Project UUID (required)" },
                    "context_summary": { "type": "string", "description": "High-level summary of the project state" },
                    "current_state": { "type": "string", "description": "What's happening right now" },
                    "key_decisions": { "type": "array", "items": { "type": "string" }, "description": "Key decisions made" }
                }),
                vec!["project_id".to_string()],
            ),
        },
    ]
}

/// Call a Project module tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        // Projects
        "list-projects" => {
            let include_statuses = args.get("include_statuses").and_then(|v| v.as_bool());
            let project_type = args.get("project_type").and_then(|v| v.as_str()).map(|s| s.to_string());
            match work::work_list_projects(include_statuses, project_type).await {
                Ok(projects) => ToolResult::json(&projects),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "get-project" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_get_project(project_id).await {
                Ok(project) => ToolResult::json(&project),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-project" => {
            let data: CreateProject = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_project(data).await {
                Ok(project) => ToolResult::json(&project),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-project" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("project_id");
            }
            let data: UpdateProject = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_update_project(project_id, data).await {
                Ok(project) => ToolResult::json(&project),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-project" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_delete_project(project_id).await {
                Ok(()) => ToolResult::text("Project deleted successfully.".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Tasks
        "list-tasks" => {
            let project_id = args.get("project_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let status_id = args.get("status_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let status_type = args.get("status_type").and_then(|v| v.as_str()).map(|s| s.to_string());
            let milestone_id = args.get("milestone_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let company_id = args.get("company_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let task_type = args.get("task_type").and_then(|v| v.as_str()).map(|s| s.to_string());
            match work::work_list_tasks(project_id, status_id, status_type, milestone_id, company_id, task_type).await {
                Ok(tasks) => ToolResult::json(&tasks),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "get-task" => {
            let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("task_id is required".to_string()),
            };
            match work::work_get_task(task_id).await {
                Ok(task) => ToolResult::json(&task),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-task" => {
            let data: CreateTask = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_task(data).await {
                Ok(task) => ToolResult::json(&task),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-task" => {
            let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("task_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("task_id");
            }
            let data: UpdateTask = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_update_task(task_id, data).await {
                Ok(task) => ToolResult::json(&task),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Milestones
        "list-milestones" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_list_milestones(project_id).await {
                Ok(milestones) => ToolResult::json(&milestones),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-milestone" => {
            let data: CreateMilestone = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_milestone(data).await {
                Ok(milestone) => ToolResult::json(&milestone),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-milestone" => {
            let milestone_id = match args.get("milestone_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("milestone_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("milestone_id");
            }
            let data: UpdateMilestone = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_update_milestone(milestone_id, data).await {
                Ok(milestone) => ToolResult::json(&milestone),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Initiatives
        "list-initiatives" => {
            let include_projects = args.get("include").and_then(|v| v.as_str()) == Some("projects");
            match work::work_list_initiatives(Some(include_projects)).await {
                Ok(initiatives) => ToolResult::json(&initiatives),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-initiative" => {
            let data: CreateInitiative = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_initiative(data).await {
                Ok(initiative) => ToolResult::json(&initiative),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-initiative" => {
            let initiative_id = match args.get("initiative_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("initiative_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("initiative_id");
            }
            let data: UpdateInitiative = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_update_initiative(initiative_id, data).await {
                Ok(initiative) => ToolResult::json(&initiative),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "delete-initiative" => {
            let initiative_id = match args.get("initiative_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("initiative_id is required".to_string()),
            };
            match work::work_delete_initiative(initiative_id, Some(true)).await {
                Ok(()) => ToolResult::text("Initiative deleted successfully.".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Initiative-Project linking
        "add-project-to-initiative" => {
            let initiative_id = match args.get("initiative_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("initiative_id is required".to_string()),
            };
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_add_project_to_initiative(initiative_id, project_id).await {
                Ok(link) => ToolResult::json(&link),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "remove-project-from-initiative" => {
            let initiative_id = match args.get("initiative_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("initiative_id is required".to_string()),
            };
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_remove_project_from_initiative(initiative_id, project_id).await {
                Ok(()) => ToolResult::text("Project removed from initiative successfully.".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "list-initiative-projects" => {
            let initiative_id = match args.get("initiative_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("initiative_id is required".to_string()),
            };
            match work::work_list_initiative_projects(initiative_id).await {
                Ok(projects) => ToolResult::json(&projects),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Labels
        "list-labels" => {
            match work::work_list_labels().await {
                Ok(labels) => ToolResult::json(&labels),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-label" => {
            let data: CreateLabel = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_label(data).await {
                Ok(label) => ToolResult::json(&label),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Skills
        "register-skill" => {
            let data: RegisterSkill = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_register_skill(data).await {
                Ok(skill) => ToolResult::json(&skill),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "list-skills" => {
            match work::work_list_skills().await {
                Ok(skills) => ToolResult::json(&skills),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Users
        "list-users" => {
            match work::work_list_users().await {
                Ok(users) => ToolResult::json(&users),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "list-bots" => {
            match work::work_list_bots().await {
                Ok(bots) => ToolResult::json(&bots),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Pipeline
        "get-pipeline" => {
            match work::work_get_pipeline().await {
                Ok(stats) => ToolResult::json(&stats),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Project updates
        "list-project-updates" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            match work::work_list_project_updates(project_id).await {
                Ok(updates) => ToolResult::json(&updates),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-project-update" => {
            let project_id = match args.get("project_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("project_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("project_id");
            }
            let data: CreateProjectUpdate = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::work_create_project_update(project_id, data).await {
                Ok(update) => ToolResult::json(&update),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Project sessions
        "add-project-session" => {
            let data: CreateProjectSession = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::project_add_session(data).await {
                Ok(session) => ToolResult::json(&session),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-project-session" => {
            let session_id = match args.get("session_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("session_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("session_id");
            }
            let data: UpdateProjectSession = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::project_update_session(session_id, data).await {
                Ok(session) => ToolResult::json(&session),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Project artifacts
        "add-project-artifact" => {
            let data: CreateProjectArtifact = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            if data.artifact_type == "task" {
                return ToolResult::error("Cannot add artifacts with type 'task'. Tasks belong in the tasks list (create-task / update-task), not as project artifacts.".to_string());
            }
            // Reject references to code repos — artifacts should only point to tv-knowledge files or UUIDs
            let ref_lower = data.reference.to_lowercase();
            let code_repo_prefixes = ["tv-client/", "tv-api/", "tv-portal/", "tv-website/", "tv-support/", "val-", "valrpa/"];
            if code_repo_prefixes.iter().any(|p| ref_lower.starts_with(p)) {
                return ToolResult::error(format!(
                    "Cannot add artifact referencing code repo '{}'. Artifacts should only reference files within the knowledge base (tv-knowledge), not source code. Use session notes to record code references instead.",
                    data.reference.split('/').next().unwrap_or(&data.reference)
                ));
            }
            match work::project_add_artifact(data).await {
                Ok(artifact) => ToolResult::json(&artifact),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "remove-project-artifact" => {
            let artifact_id = match args.get("artifact_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("artifact_id is required".to_string()),
            };
            match work::project_remove_artifact(artifact_id).await {
                Ok(()) => ToolResult::text("Artifact removed successfully.".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Project context
        "update-project-context" => {
            let data: UpsertProjectContext = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match work::project_update_context(data).await {
                Ok(context) => ToolResult::json(&context),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        _ => ToolResult::error(format!("Unknown project tool: {}", name)),
    }
}
