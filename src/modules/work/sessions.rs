// Project Module - Session, Artifact, and Context Commands

use super::types::*;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client;

/// Build a current_state string from session data for auto-updating project context.
fn build_current_state(summary: Option<&str>, next_steps: Option<&[String]>) -> String {
    let mut parts = Vec::new();
    if let Some(s) = summary {
        parts.push(s.to_string());
    }
    if let Some(steps) = next_steps {
        if !steps.is_empty() {
            parts.push(format!("Next: {}", steps.join("; ")));
        }
    }
    parts.join(" ")
}

/// Auto-update project context current_state after session add/update.
/// Best-effort — does not fail the session operation if context update fails.
async fn sync_context_current_state(
    client: &crate::core::supabase::SupabaseClient,
    project_id: &str,
    summary: Option<&str>,
    next_steps: Option<&[String]>,
    decisions: Option<&serde_json::Value>,
) {
    let current_state = build_current_state(summary, next_steps);
    if current_state.is_empty() {
        return;
    }

    let mut context_data = serde_json::json!({
        "project_id": project_id,
        "current_state": current_state,
    });

    // If session has decisions, merge them into context too
    if let Some(d) = decisions {
        if d.is_array() && !d.as_array().unwrap().is_empty() {
            context_data["key_decisions"] = d.clone();
        }
    }

    let _: Result<ProjectContext, _> = client.upsert_on("project_context", &context_data, Some("project_id")).await;
}

// ============================================================================
// Sessions
// ============================================================================

/// Add a session entry to a project.
/// If conversation_id is provided and a session already exists for that conversation,
/// updates the existing session instead of creating a duplicate.
/// Also auto-updates the project context current_state from the session.
pub async fn project_add_session(data: CreateProjectSession) -> CmdResult<ProjectSession> {
    let client = get_client().await?;

    // Upsert by conversation_id: if a session with this conversation already exists, update it
    if let Some(ref conv_id) = data.conversation_id {
        let query = format!(
            "project_id=eq.{}&conversation_id=eq.{}",
            data.project_id, conv_id
        );
        let existing: Vec<ProjectSession> =
            client.select("project_sessions", &query).await?;

        if let Some(existing_session) = existing.into_iter().next() {
            let update = UpdateProjectSession {
                summary: data.summary.clone(),
                decisions: data.decisions.clone(),
                next_steps: data.next_steps.clone(),
                open_questions: data.open_questions,
                notes: data.notes,
                conversation_id: data.conversation_id,
            };
            let result: ProjectSession = client
                .update(
                    "project_sessions",
                    &format!("id=eq.{}", existing_session.id),
                    &update,
                )
                .await?;

            // Auto-sync context
            sync_context_current_state(
                &client,
                &data.project_id,
                update.summary.as_deref(),
                update.next_steps.as_deref(),
                update.decisions.as_ref(),
            )
            .await;

            return Ok(result);
        }
    }

    // Insert new session
    let insert_data = serde_json::to_value(&data).unwrap_or_default();
    let result: ProjectSession = client.insert("project_sessions", &insert_data).await?;

    // Auto-sync context
    sync_context_current_state(
        &client,
        &data.project_id,
        data.summary.as_deref(),
        data.next_steps.as_deref(),
        data.decisions.as_ref(),
    )
    .await;

    Ok(result)
}

/// Update a session entry. Also auto-updates project context current_state.
pub async fn project_update_session(
    id: String,
    data: UpdateProjectSession,
) -> CmdResult<ProjectSession> {
    let client = get_client().await?;
    let result: ProjectSession = client
        .update("project_sessions", &format!("id=eq.{}", id), &data)
        .await?;

    // Auto-sync context
    sync_context_current_state(
        &client,
        &result.project_id,
        data.summary.as_deref(),
        data.next_steps.as_deref(),
        data.decisions.as_ref(),
    )
    .await;

    Ok(result)
}

// ============================================================================
// Artifacts
// ============================================================================

/// Add an artifact to a project.
/// If session_id is provided and matches a conversation_id in project_sessions,
/// resolves it to the session's actual UUID PK.
pub async fn project_add_artifact(data: CreateProjectArtifact) -> CmdResult<ProjectArtifact> {
    let client = get_client().await?;

    let mut insert_data = serde_json::to_value(&data).unwrap_or_default();
    if let Some(obj) = insert_data.as_object_mut() {
        // Resolve session_id: caller may pass conversation_id instead of the session's PK.
        if let Some(sid) = obj.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            let conv_query = format!(
                "project_id=eq.{}&conversation_id=eq.{}&select=id",
                data.project_id, sid
            );
            let conv_matches: Vec<serde_json::Value> = client
                .select("project_sessions", &conv_query)
                .await
                .unwrap_or_default();

            if let Some(first) = conv_matches.first() {
                if let Some(real_id) = first.get("id").and_then(|v| v.as_str()) {
                    obj.insert("session_id".to_string(), serde_json::Value::String(real_id.to_string()));
                }
            }
        }
    }

    client.insert("project_artifacts", &insert_data).await
}

/// Remove an artifact from a project
pub async fn project_remove_artifact(artifact_id: String) -> CmdResult<()> {
    let client = get_client().await?;
    let query = format!("id=eq.{}", artifact_id);
    client.delete("project_artifacts", &query).await
}

// ============================================================================
// Context
// ============================================================================

/// Upsert the rolling context for a project.
pub async fn project_update_context(data: UpsertProjectContext) -> CmdResult<ProjectContext> {
    let client = get_client().await?;
    let upsert_data = serde_json::to_value(&data).unwrap_or_default();
    client.upsert_on("project_context", &upsert_data, Some("project_id")).await
}
