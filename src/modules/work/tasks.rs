// Work Module - Task Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

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

    // Inherit project's company_id if task didn't specify one
    let company_id = data.company_id.clone().or_else(|| project.company_id.clone());

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
        "company_id": company_id,
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
