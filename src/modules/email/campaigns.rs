// Email Campaign CRUD Commands

use crate::core::error::CmdResult;
use crate::core::supabase::get_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailCampaign {
    pub id: Option<String>,
    pub name: Option<String>,
    pub subject: Option<String>,
    pub from_name: Option<String>,
    pub from_email: Option<String>,
    pub group_id: Option<String>,
    pub html_body: Option<String>,
    pub content_path: Option<String>,
    pub bcc_email: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub scheduled_at: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: Option<String>,
    pub tokens: Option<serde_json::Value>,
    pub send_channel: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCampaign {
    pub name: String,
    pub subject: String,
    pub from_name: String,
    pub from_email: String,
    pub group_id: Option<String>,
    pub html_body: Option<String>,
    pub content_path: Option<String>,
    pub bcc_email: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_channel: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCampaign {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_channel: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailGroup {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// List email campaigns with optional filters
pub async fn list_campaigns(
    status: Option<String>,
    group_id: Option<String>,
    search: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<EmailCampaign>> {
    let client = get_client().await?;

    let mut filters = vec![];

    if let Some(s) = search {
        filters.push(format!("or=(name.ilike.*{}*,subject.ilike.*{}*)", s, s));
    }
    if let Some(st) = status {
        filters.push(format!("status=eq.{}", st));
    }
    if let Some(gid) = group_id {
        filters.push(format!("group_id=eq.{}", gid));
    }

    let limit_val = limit.unwrap_or(50);
    filters.push(format!("limit={}", limit_val));
    filters.push("order=updated_at.desc".to_string());

    let query = filters.join("&");
    client.select("email_campaigns", &query).await
}

/// Create a new email campaign
pub async fn create_campaign(data: CreateCampaign) -> CmdResult<EmailCampaign> {
    let client = get_client().await?;
    client.insert("email_campaigns", &data).await
}

/// Update an existing email campaign
pub async fn update_campaign(campaign_id: &str, data: UpdateCampaign) -> CmdResult<EmailCampaign> {
    let client = get_client().await?;
    let query = format!("id=eq.{}", campaign_id);
    client.update("email_campaigns", &query, &data).await
}

/// Delete an email campaign
pub async fn delete_campaign(campaign_id: &str) -> CmdResult<()> {
    let client = get_client().await?;
    let query = format!("id=eq.{}", campaign_id);
    client.delete("email_campaigns", &query).await
}

/// List email groups
pub async fn list_groups() -> CmdResult<Vec<EmailGroup>> {
    let client = get_client().await?;
    client.select("email_groups", "order=name.asc").await
}

/// Create a new email group
pub async fn create_group(name: &str, description: Option<&str>) -> CmdResult<EmailGroup> {
    let client = get_client().await?;
    let mut data = serde_json::json!({ "name": name });
    if let Some(desc) = description {
        data["description"] = serde_json::json!(desc);
    }
    client.insert("email_groups", &data).await
}
