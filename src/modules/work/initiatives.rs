// Work Module - Initiative Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

/// List all initiatives

pub async fn work_list_initiatives(include_projects: Option<bool>) -> CmdResult<Vec<Initiative>> {
    let client = get_client().await?;

    let query = if include_projects.unwrap_or(false) {
        "select=*,projects:initiative_projects(project:projects(*))&archived_at=is.null&order=sort_order.asc"
    } else {
        "archived_at=is.null&order=sort_order.asc"
    };

    client.select("initiatives", query).await
}

/// Get a single initiative by ID

pub async fn work_get_initiative(initiative_id: String) -> CmdResult<Initiative> {
    let client = get_client().await?;

    let query = format!(
        "select=*,projects:initiative_projects(project:projects(*))&id=eq.{}",
        initiative_id
    );

    client
        .select_single("initiatives", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("Initiative not found: {}", initiative_id)))
}

/// Create a new initiative

pub async fn work_create_initiative(data: CreateInitiative) -> CmdResult<Initiative> {
    let client = get_client().await?;

    // Generate slug if not provided
    let mut insert_data = serde_json::to_value(&data)?;
    if let Some(obj) = insert_data.as_object_mut() {
        if obj.get("slug").map_or(true, |v| v.is_null()) {
            obj.insert("slug".to_string(), serde_json::Value::String(slugify(&data.name)));
        }
    }

    client.insert("initiatives", &insert_data).await
}

/// Update an initiative

pub async fn work_update_initiative(
    initiative_id: String,
    data: UpdateInitiative,
) -> CmdResult<Initiative> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", initiative_id);
    client.update("initiatives", &query, &data).await
}

/// Delete an initiative (soft delete or archive)

pub async fn work_delete_initiative(initiative_id: String, archive: Option<bool>) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", initiative_id);

    if archive.unwrap_or(false) {
        // Soft delete - set archived_at
        let now = chrono::Utc::now().to_rfc3339();
        let data = serde_json::json!({ "archived_at": now });
        let _: Initiative = client.update("initiatives", &query, &data).await?;
    } else {
        // Hard delete
        client.delete("initiatives", &query).await?;
    }

    Ok(())
}

/// Add a project to an initiative

pub async fn work_add_project_to_initiative(
    initiative_id: String,
    project_id: String,
) -> CmdResult<InitiativeProject> {
    let client = get_client().await?;

    let data = serde_json::json!({
        "initiative_id": initiative_id,
        "project_id": project_id
    });

    client.insert("initiative_projects", &data).await
}

/// Remove a project from an initiative

pub async fn work_remove_project_from_initiative(
    initiative_id: String,
    project_id: String,
) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!(
        "initiative_id=eq.{}&project_id=eq.{}",
        initiative_id, project_id
    );
    client.delete("initiative_projects", &query).await
}

/// List projects in an initiative

pub async fn work_list_initiative_projects(initiative_id: String) -> CmdResult<Vec<Project>> {
    let client = get_client().await?;

    // Query the junction table with project join
    let query = format!(
        "select=project:projects(*)&initiative_id=eq.{}&order=sort_order.asc",
        initiative_id
    );

    #[derive(serde::Deserialize)]
    struct JunctionRow {
        project: Project,
    }

    let rows: Vec<JunctionRow> = client.select("initiative_projects", &query).await?;
    Ok(rows.into_iter().map(|r| r.project).collect())
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
