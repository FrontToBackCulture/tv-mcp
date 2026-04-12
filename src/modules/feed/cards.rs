// Feed Module - Card Commands

use super::types::*;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client;

/// List feed cards with optional filters

pub async fn feed_list_cards(
    card_type: Option<String>,
    category: Option<String>,
    source_ref: Option<String>,
    include_archived: Option<bool>,
) -> CmdResult<Vec<FeedCard>> {
    let client = get_client().await?;

    let mut filters = vec!["select=*".to_string()];

    if include_archived != Some(true) {
        filters.push("archived=eq.false".to_string());
    }

    if let Some(ct) = card_type {
        filters.push(format!("card_type=eq.{}", ct));
    }
    if let Some(cat) = category {
        filters.push(format!("category=eq.{}", cat));
    }
    if let Some(sr) = source_ref {
        filters.push(format!("source_ref=eq.{}", sr));
    }

    filters.push("order=pinned.desc,created_at.desc".to_string());

    let query = filters.join("&");
    client.select("feed_cards", &query).await
}

/// Create a new feed card

pub async fn feed_create_card(data: CreateFeedCard) -> CmdResult<FeedCard> {
    let client = get_client().await?;

    let insert_data = serde_json::json!({
        "card_type": data.card_type,
        "category": data.category,
        "badge": data.badge,
        "title": data.title,
        "body": data.body,
        "source": data.source,
        "source_detail": data.source_detail,
        "triggers": data.triggers,
        "chips": data.chips,
        "stats": data.stats,
        "features": data.features,
        "author": data.author,
        "cta_label": data.cta_label,
        "cta_action": data.cta_action,
        "scheduled_date": data.scheduled_date,
        "pinned": data.pinned.unwrap_or(false),
        "created_by": data.created_by,
        "source_ref": data.source_ref,
        "visual": data.visual,
        "series_id": data.series_id,
        "series_order": data.series_order.unwrap_or(0),
    });

    client.insert("feed_cards", &insert_data).await
}

/// Update a feed card

pub async fn feed_update_card(card_id: String, data: UpdateFeedCard) -> CmdResult<FeedCard> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", card_id);
    client.update("feed_cards", &query, &data).await
}

/// Delete a feed card (soft delete — sets archived=true)

pub async fn feed_delete_card(card_id: String) -> CmdResult<FeedCard> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", card_id);
    let update = serde_json::json!({ "archived": true });
    client.update("feed_cards", &query, &update).await
}
