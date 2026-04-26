// Docs Module - Page Commands
//
// docs_pages live in the workspace `public` schema and feed the gated
// /docs portal on tv-website. Sections (`docs_sections`) are stable; pages
// are the editable unit. Upsert is on (section_id, slug).

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;
use serde_json::json;

/// Resolve a section slug to its UUID
async fn resolve_section_id(section_slug: &str) -> CmdResult<String> {
    let client = get_client().await?;
    let query = format!("slug=eq.{}&select=id", section_slug);
    let row: Option<serde_json::Value> = client.select_single("docs_sections", &query).await?;
    let row = row.ok_or_else(|| {
        CommandError::from(format!(
            "Unknown docs section '{}'. Run `list-docs-pages` to see available sections.",
            section_slug
        ))
    })?;
    row.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CommandError::Internal("docs_sections.id missing in response".into()))
}

/// List docs pages, optionally filtered by section
///
/// Returns metadata only (no body_md) so listings stay light. Use
/// `get-docs-page` to fetch a page's full body.
pub async fn list_docs_pages(
    section: Option<String>,
    visible_only: Option<bool>,
) -> CmdResult<Vec<serde_json::Value>> {
    let client = get_client().await?;

    let mut filters = vec![
        "select=id,section_id,slug,title,summary,tags,sort_order,visible,updated_at"
            .to_string(),
    ];

    if let Some(s) = section {
        let section_id = resolve_section_id(&s).await?;
        filters.push(format!("section_id=eq.{}", section_id));
    }

    if visible_only.unwrap_or(false) {
        filters.push("visible=eq.true".to_string());
    }

    filters.push("order=sort_order.asc,title.asc".to_string());

    let query = filters.join("&");
    client.select("docs_pages", &query).await
}

/// Get a single docs page (full body) by section + slug, or by id
pub async fn get_docs_page(
    id: Option<String>,
    section: Option<String>,
    slug: Option<String>,
) -> CmdResult<DocsPage> {
    let client = get_client().await?;

    let query = if let Some(id) = id {
        format!("id=eq.{}", id)
    } else if let (Some(section), Some(slug)) = (section, slug) {
        let section_id = resolve_section_id(&section).await?;
        format!("section_id=eq.{}&slug=eq.{}", section_id, slug)
    } else {
        return Err(CommandError::from(
            "Provide either `id`, or both `section` and `slug`.",
        ));
    };

    match client.select_single("docs_pages", &query).await? {
        Some(page) => Ok(page),
        None => Err(CommandError::from("Docs page not found")),
    }
}

/// Upsert a docs page (create or replace by section + slug)
///
/// Use this for both new pages and edits. The unique key is
/// (section_id, slug); same slug + section overwrites.
pub async fn upsert_docs_page(data: UpsertDocsPage) -> CmdResult<DocsPage> {
    let client = get_client().await?;
    let section_id = resolve_section_id(&data.section).await?;

    let payload = json!({
        "section_id": section_id,
        "slug": data.slug,
        "title": data.title,
        "summary": data.summary,
        "body_md": data.body_md.unwrap_or_default(),
        "tags": data.tags.unwrap_or_default(),
        "sort_order": data.sort_order.unwrap_or(10),
        "visible": data.visible.unwrap_or(true),
    });

    client
        .upsert_on("docs_pages", &payload, Some("section_id,slug"))
        .await
}

/// Delete a docs page by section + slug, or by id
pub async fn delete_docs_page(
    id: Option<String>,
    section: Option<String>,
    slug: Option<String>,
) -> CmdResult<()> {
    let client = get_client().await?;

    let query = if let Some(id) = id {
        format!("id=eq.{}", id)
    } else if let (Some(section), Some(slug)) = (section, slug) {
        let section_id = resolve_section_id(&section).await?;
        format!("section_id=eq.{}&slug=eq.{}", section_id, slug)
    } else {
        return Err(CommandError::from(
            "Provide either `id`, or both `section` and `slug`.",
        ));
    };

    client.delete("docs_pages", &query).await
}
