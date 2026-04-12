// Blog Module - Article Commands

use super::types::*;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client;

/// List blog articles with optional filters

pub async fn blog_list_articles(
    status: Option<String>,
    category: Option<String>,
    featured: Option<bool>,
) -> CmdResult<Vec<BlogArticle>> {
    let client = get_client().await?;

    let mut filters = vec!["select=*".to_string()];

    if let Some(s) = status {
        filters.push(format!("status=eq.{}", s));
    }
    if let Some(cat) = category {
        filters.push(format!("category=eq.{}", cat));
    }
    if let Some(f) = featured {
        filters.push(format!("featured=eq.{}", f));
    }

    filters.push("order=published_at.desc.nullslast,created_at.desc".to_string());

    let query = filters.join("&");
    client.select("blog_articles", &query).await
}

/// Get a single blog article by ID or slug

pub async fn blog_get_article(
    id: Option<String>,
    slug: Option<String>,
) -> CmdResult<BlogArticle> {
    let client = get_client().await?;

    let query = if let Some(id) = id {
        format!("id=eq.{}", id)
    } else if let Some(slug) = slug {
        format!("slug=eq.{}", slug)
    } else {
        return Err(crate::core::error::CommandError::from("Either id or slug is required"));
    };

    match client.select_single("blog_articles", &query).await? {
        Some(article) => Ok(article),
        None => Err(crate::core::error::CommandError::from("Article not found")),
    }
}

/// Create a new blog article

pub async fn blog_create_article(data: CreateBlogArticle) -> CmdResult<BlogArticle> {
    let client = get_client().await?;

    let insert_data = serde_json::json!({
        "slug": data.slug,
        "title": data.title,
        "description": data.description,
        "content": data.content,
        "category": data.category,
        "author": data.author.unwrap_or_else(|| "ThinkVAL Team".to_string()),
        "read_time": data.read_time,
        "color": data.color.unwrap_or_else(|| "#EEF8F9".to_string()),
        "illustration": data.illustration,
        "featured": data.featured.unwrap_or(false),
        "status": data.status.unwrap_or_else(|| "draft".to_string()),
    });

    client.insert("blog_articles", &insert_data).await
}

/// Update a blog article

pub async fn blog_update_article(article_id: String, data: UpdateBlogArticle) -> CmdResult<BlogArticle> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", article_id);
    client.update("blog_articles", &query, &data).await
}

/// Delete a blog article

pub async fn blog_delete_article(article_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", article_id);
    client.delete("blog_articles", &query).await
}
