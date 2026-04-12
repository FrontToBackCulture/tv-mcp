// Notifications Module — @mention notifications from discussions

use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub recipient: String,
    pub r#type: String,
    pub discussion_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub actor: String,
    pub body_preview: String,
    pub read: bool,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotification {
    pub recipient: String,
    pub r#type: String,
    pub discussion_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub actor: String,
    pub body_preview: String,
}

/// List notifications for a recipient

pub async fn notifications_list(
    recipient: String,
    unread_only: Option<bool>,
    limit: Option<u32>,
) -> CmdResult<Vec<Notification>> {
    let client = get_client().await?;

    let limit_val = limit.unwrap_or(50);
    let mut query = format!(
        "select=*&recipient=eq.{}&order=created_at.desc&limit={}",
        recipient, limit_val
    );

    if unread_only.unwrap_or(false) {
        query.push_str("&read=eq.false");
    }

    let notifications: Vec<Notification> = client.select("notifications", &query).await?;
    Ok(notifications)
}

/// Count unread notifications for a recipient

pub async fn notifications_unread_count(recipient: String) -> CmdResult<u32> {
    let client = get_client().await?;

    let query = format!(
        "select=id&recipient=eq.{}&read=eq.false",
        recipient
    );

    let rows: Vec<serde_json::Value> = client.select("notifications", &query).await?;
    Ok(rows.len() as u32)
}

/// Mark a single notification as read

pub async fn notifications_mark_read(id: String) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", id);
    let update = serde_json::json!({ "read": true });
    let _: serde_json::Value = client.update("notifications", &query, &update).await?;
    Ok(())
}

/// Mark all notifications as read for a recipient

pub async fn notifications_mark_all_read(recipient: String) -> CmdResult<()> {
    let _client = get_client().await?;

    let query = format!("recipient=eq.{}&read=eq.false", recipient);
    let update = serde_json::json!({ "read": true });

    // Use the raw PATCH — update may return empty if no rows matched, which is fine
    let url = format!(
        "{}/rest/v1/notifications?{}",
        {
            use crate::core::settings::{settings_get_key, KEY_SUPABASE_URL};
            settings_get_key(KEY_SUPABASE_URL.to_string())?
                .ok_or_else(|| CommandError::Config("Supabase URL not configured".into()))?
        },
        query
    );

    let anon_key = {
        use crate::core::settings::{settings_get_key, KEY_SUPABASE_ANON_KEY};
        settings_get_key(KEY_SUPABASE_ANON_KEY.to_string())?
            .ok_or_else(|| CommandError::Config("Supabase anon key not configured".into()))?
    };

    let response = crate::HTTP_CLIENT
        .patch(&url)
        .header("apikey", &anon_key)
        .header("Authorization", format!("Bearer {}", anon_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .json(&update)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    Ok(())
}
