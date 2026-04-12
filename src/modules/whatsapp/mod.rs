// WhatsApp Summaries Module
// Daily AI-generated summaries of WhatsApp chats, linked to client initiatives

use crate::core::error::CmdResult;
use crate::core::supabase::get_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsappSummary {
    pub id: String,
    pub initiative_id: String,
    pub client_folder: String,
    pub date: String,
    pub summary: String,
    pub key_topics: Option<serde_json::Value>,
    pub action_items: Option<serde_json::Value>,
    pub participants: Option<serde_json::Value>,
    pub message_count: Option<i32>,
    pub media_notes: Option<String>,
    pub source_file: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWhatsappSummary {
    pub initiative_id: String,
    pub client_folder: String,
    pub date: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_topics: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_items: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participants: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
}

/// List WhatsApp summaries for an initiative, optionally filtered by date range

pub async fn whatsapp_list_summaries(
    initiative_id: String,
    after_date: Option<String>,
    before_date: Option<String>,
    limit: Option<u32>,
) -> CmdResult<Vec<WhatsappSummary>> {
    let client = get_client().await?;

    let limit_val = limit.unwrap_or(100);
    let mut query = format!(
        "select=*&initiative_id=eq.{}&order=date.desc&limit={}",
        initiative_id, limit_val
    );

    if let Some(after) = after_date {
        query.push_str(&format!("&date=gt.{}", after));
    }
    if let Some(before) = before_date {
        query.push_str(&format!("&date=lt.{}", before));
    }

    let summaries: Vec<WhatsappSummary> = client.select("whatsapp_summaries", &query).await?;
    Ok(summaries)
}

/// Get the latest summary date for an initiative (for delta detection)

pub async fn whatsapp_latest_date(
    initiative_id: String,
) -> CmdResult<Option<String>> {
    let client = get_client().await?;

    let query = format!(
        "select=date&initiative_id=eq.{}&order=date.desc&limit=1",
        initiative_id
    );

    let rows: Vec<serde_json::Value> = client.select("whatsapp_summaries", &query).await?;
    Ok(rows.first().and_then(|r| r["date"].as_str().map(|s| s.to_string())))
}

/// Upsert a WhatsApp summary (insert or update by initiative_id + date)

pub async fn whatsapp_upsert_summary(
    data: UpsertWhatsappSummary,
) -> CmdResult<WhatsappSummary> {
    let client = get_client().await?;

    // Check if exists
    let check_query = format!(
        "select=id&initiative_id=eq.{}&date=eq.{}",
        data.initiative_id, data.date
    );
    let existing: Vec<serde_json::Value> = client.select("whatsapp_summaries", &check_query).await?;

    if let Some(row) = existing.first() {
        // Update existing
        let id = row["id"].as_str().unwrap_or("");
        let update_query = format!("id=eq.{}", id);
        let summary: WhatsappSummary = client.update("whatsapp_summaries", &update_query, &data).await?;
        Ok(summary)
    } else {
        // Insert new
        let summary: WhatsappSummary = client.insert("whatsapp_summaries", &data).await?;
        Ok(summary)
    }
}

/// Delete a WhatsApp summary

pub async fn whatsapp_delete_summary(id: String) -> CmdResult<()> {
    let client = get_client().await?;
    let query = format!("id=eq.{}", id);
    client.delete("whatsapp_summaries", &query).await?;
    Ok(())
}
