// Discussions Module - Universal comment/discussion system
// Attaches comments to any entity: files, projects, companies, tasks, campaigns

use crate::core::error::CmdResult;
use crate::core::supabase::get_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub parent_id: Option<String>,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscussion {
    pub entity_type: String,
    pub entity_id: String,
    pub author: String,
    pub body: String,
    pub parent_id: Option<String>,
}

/// List discussions for an entity

pub async fn discussions_list(
    entity_type: String,
    entity_id: String,
    limit: Option<u32>,
) -> CmdResult<Vec<Discussion>> {
    let client = get_client().await?;

    let limit_val = limit.unwrap_or(100);
    let query = format!(
        "select=*&entity_type=eq.{}&entity_id=eq.{}&order=created_at.asc&limit={}",
        entity_type, entity_id, limit_val
    );

    let discussions: Vec<Discussion> = client.select("discussions", &query).await?;
    Ok(discussions)
}

/// Create a new discussion

pub async fn discussions_create(
    entity_type: String,
    entity_id: String,
    author: String,
    body: String,
    parent_id: Option<String>,
) -> CmdResult<Discussion> {
    let client = get_client().await?;

    let data = CreateDiscussion {
        entity_type,
        entity_id,
        author,
        body,
        parent_id,
    };

    let discussion: Discussion = client.insert("discussions", &data).await?;
    Ok(discussion)
}

/// Update a discussion's body

pub async fn discussions_update(id: String, body: String) -> CmdResult<Discussion> {
    let client = get_client().await?;

    let update = serde_json::json!({
        "body": body,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    let query = format!("id=eq.{}", id);
    let discussion: Discussion = client.update("discussions", &query, &update).await?;
    Ok(discussion)
}

/// Delete a discussion

pub async fn discussions_delete(id: String) -> CmdResult<()> {
    let client = get_client().await?;
    let query = format!("id=eq.{}", id);
    client.delete("discussions", &query).await?;
    Ok(())
}

/// Count discussions for an entity (lightweight, for badges)

pub async fn discussions_count(
    entity_type: String,
    entity_id: String,
) -> CmdResult<u32> {
    let client = get_client().await?;

    let query = format!(
        "select=id&entity_type=eq.{}&entity_id=eq.{}",
        entity_type, entity_id
    );

    let discussions: Vec<serde_json::Value> = client.select("discussions", &query).await?;
    Ok(discussions.len() as u32)
}
