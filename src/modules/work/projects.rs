// Project Module - Project Commands
// Unified handler for work projects and deals

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

/// List all projects with optional type filter

pub async fn work_list_projects(
    include_statuses: Option<bool>,
    project_type: Option<String>,
) -> CmdResult<Vec<Project>> {
    let client = get_client().await?;

    // Statuses are global, not per-project — always select just project fields
    let select = "*";

    let mut query = format!("select={}&archived_at=is.null&order=sort_order.asc", select);

    if let Some(pt) = project_type {
        query.push_str(&format!("&project_type=eq.{}", pt));
    }

    client.select("projects", &query).await
}

/// Get a single project by ID — joins sessions/artifacts/context and deal data

pub async fn work_get_project(project_id: String) -> CmdResult<Project> {
    let client = get_client().await?;

    // Get the basic project
    let query = format!(
        "select=*&id=eq.{}",
        project_id
    );

    let mut project: Project = client
        .select_single("projects", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("Project not found: {}", project_id)))?;

    // Join sessions/artifacts/context for any project type
    let sessions_query = format!(
        "project_id=eq.{}&order=date.desc",
        project_id
    );
    let sessions: Vec<ProjectSession> = client
        .select("project_sessions", &sessions_query)
        .await
        .unwrap_or_default();

    let artifacts_query = format!(
        "project_id=eq.{}&order=created_at.desc",
        project_id
    );
    let artifacts: Vec<ProjectArtifact> = client
        .select("project_artifacts", &artifacts_query)
        .await
        .unwrap_or_default();

    let context_query = format!("project_id=eq.{}", project_id);
    let context: Option<ProjectContext> = client
        .select_single("project_context", &context_query)
        .await
        .unwrap_or(None);

    project.sessions = Some(sessions);
    project.artifacts = Some(artifacts);
    project.context = context;

    // If deal type, join company
    if project.project_type.as_deref() == Some("deal") {
        if let Some(ref cid) = project.company_id {
            let company_query = format!("id=eq.{}", cid);
            let company: Option<Company> = client
                .select_single("crm_companies", &company_query)
                .await
                .unwrap_or(None);
            project.company = company.map(Box::new);
        }
    }

    Ok(project)
}

/// Create a new project with default statuses

pub async fn work_create_project(data: CreateProject) -> CmdResult<Project> {
    let client = get_client().await?;

    // Build insert data
    let mut insert_data = serde_json::to_value(&data)?;
    if let Some(obj) = insert_data.as_object_mut() {
        // Generate slug if not provided
        if obj.get("slug").map_or(true, |v| v.is_null()) {
            obj.insert("slug".to_string(), serde_json::Value::String(slugify(&data.name)));
        }
        // Default project_type
        if obj.get("project_type").map_or(true, |v| v.is_null()) {
            obj.insert("project_type".to_string(), serde_json::Value::String("work".to_string()));
        }

        // Deal-specific defaults
        if data.project_type.as_deref() == Some("deal") {
            if obj.get("deal_stage").map_or(true, |v| v.is_null()) {
                obj.insert("deal_stage".to_string(), serde_json::Value::String("prospect".to_string()));
            }
            if obj.get("deal_currency").map_or(true, |v| v.is_null()) {
                obj.insert("deal_currency".to_string(), serde_json::Value::String("SGD".to_string()));
            }
            let now = chrono::Utc::now().to_rfc3339();
            obj.insert("deal_stage_changed_at".to_string(), serde_json::Value::String(now));

            if obj.get("identifier_prefix").map_or(true, |v| v.is_null()) {
                obj.insert("identifier_prefix".to_string(), serde_json::Value::String("DEAL".to_string()));
            }
            if obj.get("status").map_or(true, |v| v.is_null()) {
                obj.insert("status".to_string(), serde_json::Value::String("active".to_string()));
            }
        }

    }

    // Create project
    let project: Project = client.insert("projects", &insert_data).await?;

    // Statuses are global — no per-project statuses needed

    // Deal-specific: update company stage if prospect → opportunity
    if data.project_type.as_deref() == Some("deal") {
        if let Some(ref cid) = data.company_id {
            let company: Option<crate::modules::crm::types::Company> = client
                .select_single("crm_companies", &format!("id=eq.{}", cid))
                .await
                .unwrap_or(None);

            if let Some(c) = company {
                if c.stage.as_deref() == Some("prospect") {
                    let update_data = serde_json::json!({ "stage": "opportunity" });
                    let _: crate::modules::crm::types::Company = client
                        .update("crm_companies", &format!("id=eq.{}", cid), &update_data)
                        .await?;

                    // Log stage change activity
                    let activity = serde_json::json!({
                        "company_id": cid,
                        "type": "stage_change",
                        "old_value": "prospect",
                        "new_value": "opportunity",
                        "activity_date": chrono::Utc::now().to_rfc3339()
                    });
                    let _: crate::modules::crm::types::Activity = client.insert("crm_activities", &activity).await?;
                }
            }
        }
    }

    // Return project with statuses
    work_get_project(project.id).await
}

/// Update a project (handles deal stage change logic)

pub async fn work_update_project(project_id: String, data: UpdateProject) -> CmdResult<Project> {
    let client = get_client().await?;

    // Get current project for stage change detection
    let current: Project = work_get_project(project_id.clone()).await?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut update_data = serde_json::to_value(&data)?;

    // Deal stage change logic
    if current.project_type.as_deref() == Some("deal") {
        if let Some(new_stage) = &data.deal_stage {
            if let Some(old_stage) = &current.deal_stage {
                if old_stage != new_stage {
                    if let Some(obj) = update_data.as_object_mut() {
                        if !data.preserve_stage_date.unwrap_or(false) {
                            obj.insert("deal_stage_changed_at".to_string(), serde_json::Value::String(now.clone()));
                        }
                        obj.insert("deal_stale_snoozed_until".to_string(), serde_json::Value::Null);
                    }

                    // Log stage change activity
                    if let Some(ref cid) = current.company_id {
                        let activity = serde_json::json!({
                            "company_id": cid,
                            "project_id": project_id,
                            "type": "stage_change",
                            "old_value": old_stage,
                            "new_value": new_stage,
                            "activity_date": now
                        });
                        let _: crate::modules::crm::types::Activity = client.insert("crm_activities", &activity).await?;

                        // If deal is won, update company stage to client
                        if new_stage == "won" {
                            let company_update = serde_json::json!({ "stage": "client" });
                            let _: crate::modules::crm::types::Company = client
                                .update("crm_companies", &format!("id=eq.{}", cid), &company_update)
                                .await?;

                            let company_activity = serde_json::json!({
                                "company_id": cid,
                                "type": "stage_change",
                                "old_value": "opportunity",
                                "new_value": "client",
                                "activity_date": chrono::Utc::now().to_rfc3339()
                            });
                            let _: crate::modules::crm::types::Activity = client.insert("crm_activities", &company_activity).await?;
                        }
                    }
                }
            }
        }
    }

    let query = format!("id=eq.{}", project_id);
    let _: Project = client.update("projects", &query, &update_data).await?;

    work_get_project(project_id).await
}

/// Delete a project (soft delete by setting archived_at)

pub async fn work_delete_project(project_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    // Get project to check type for cleanup
    let project: Project = work_get_project(project_id.clone()).await?;

    // Deal-specific cleanup: delete related activities
    if project.project_type.as_deref() == Some("deal") {
        let _ = client.delete("crm_activities", &format!("project_id=eq.{}", project_id)).await;
    }

    let query = format!("id=eq.{}", project_id);
    let now = chrono::Utc::now().to_rfc3339();
    let data = serde_json::json!({ "archived_at": now });

    let _: Project = client.update("projects", &query, &data).await?;
    Ok(())
}

/// Get pipeline statistics from projects table

pub async fn work_get_pipeline() -> CmdResult<PipelineStats> {
    let client = get_client().await?;

    let deals: Vec<Project> = client
        .select(
            "projects",
            "project_type=eq.deal&deal_stage=in.(lead,qualified,pilot,proposal,negotiation)&archived_at=is.null&select=deal_stage,deal_value",
        )
        .await?;

    let stages = ["lead", "qualified", "pilot", "proposal", "negotiation"];
    let mut by_stage = Vec::new();
    let mut total_value = 0.0;
    let mut total_deals = 0;

    for stage in stages {
        let stage_deals: Vec<&Project> = deals.iter().filter(|d| d.deal_stage.as_deref() == Some(stage)).collect();
        let count = stage_deals.len() as i32;
        let value: f64 = stage_deals.iter().filter_map(|d| d.deal_value).sum();

        by_stage.push(PipelineStage {
            stage: stage.to_string(),
            count,
            value,
        });

        total_deals += count;
        total_value += value;
    }

    Ok(PipelineStats {
        by_stage,
        total_value,
        total_deals,
    })
}

/// List task statuses for a project

pub async fn work_list_project_statuses(project_id: String) -> CmdResult<Vec<TaskStatus>> {
    let client = get_client().await?;

    // Statuses are global — return all, ignore project_id
    let query = "order=sort_order.asc".to_string();
    client.select("task_statuses", &query).await
}

/// List project updates (status updates)

pub async fn work_list_project_updates(project_id: String) -> CmdResult<Vec<ProjectUpdate>> {
    let client = get_client().await?;

    let query = format!(
        "project_id=eq.{}&order=created_at.desc",
        project_id
    );
    client.select("project_updates", &query).await
}

/// Create a project update

pub async fn work_create_project_update(
    project_id: String,
    data: CreateProjectUpdate,
) -> CmdResult<ProjectUpdate> {
    let client = get_client().await?;

    let insert_data = serde_json::json!({
        "project_id": project_id,
        "content": data.content,
        "health": data.health,
        "created_by": data.created_by
    });

    // Create update
    let update: ProjectUpdate = client.insert("project_updates", &insert_data).await?;

    // Also update project health if provided
    if let Some(health) = &data.health {
        let query = format!("id=eq.{}", project_id);
        let health_data = serde_json::json!({ "health": health });
        let _: Project = client.update("projects", &query, &health_data).await?;
    }

    Ok(update)
}

/// Delete a project update

pub async fn work_delete_project_update(update_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", update_id);
    client.delete("project_updates", &query).await
}

// Helper function to create URL-friendly slug
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
