// Blog Module MCP Tools
// Blog article management for tv-website

use crate::modules::blog::{self, CreateBlogArticle, UpdateBlogArticle};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define Blog module tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-blog-articles".to_string(),
            description: "List blog articles from tv-website. Filter by status, category, or featured flag.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "status": { "type": "string", "enum": ["draft", "published"], "description": "Filter by status" },
                    "category": { "type": "string", "description": "Filter by category (e.g. 'Product', 'Company')" },
                    "featured": { "type": "boolean", "description": "Filter by featured flag" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "get-blog-article".to_string(),
            description: "Get a single blog article by ID or slug.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "id": { "type": "string", "description": "Article UUID" },
                    "slug": { "type": "string", "description": "Article slug (e.g. 'ai-scanning-launch')" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "create-blog-article".to_string(),
            description: "Create a new blog article. Defaults to draft status. Set status to 'published' to go live.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "slug": { "type": "string", "description": "URL slug (required, must be unique)" },
                    "title": { "type": "string", "description": "Article title (required)" },
                    "description": { "type": "string", "description": "Short description for cards/SEO" },
                    "content": { "type": "string", "description": "Full article body in Markdown" },
                    "category": { "type": "string", "description": "Category (e.g. 'Product', 'Company')" },
                    "author": { "type": "string", "description": "Author name (default: 'ThinkVAL Team')" },
                    "read_time": { "type": "string", "description": "Read time (e.g. '5 min read')" },
                    "color": { "type": "string", "description": "Background color for card (default: '#EEF8F9')" },
                    "illustration": { "type": "string", "description": "Path to illustration image" },
                    "featured": { "type": "boolean", "description": "Show in featured section (default: false)" },
                    "status": { "type": "string", "enum": ["draft", "published"], "description": "Article status (default: 'draft')" }
                }),
                vec!["slug".to_string(), "title".to_string()],
            ),
        },
        Tool {
            name: "update-blog-article".to_string(),
            description: "Update a blog article. Pass only the fields you want to change.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "article_id": { "type": "string", "description": "Article UUID (required)" },
                    "slug": { "type": "string" },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "content": { "type": "string" },
                    "category": { "type": "string" },
                    "author": { "type": "string" },
                    "read_time": { "type": "string" },
                    "color": { "type": "string" },
                    "illustration": { "type": "string" },
                    "featured": { "type": "boolean" },
                    "status": { "type": "string", "enum": ["draft", "published"] }
                }),
                vec!["article_id".to_string()],
            ),
        },
        Tool {
            name: "delete-blog-article".to_string(),
            description: "Delete a blog article permanently.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "article_id": { "type": "string", "description": "Article UUID (required)" }
                }),
                vec!["article_id".to_string()],
            ),
        },
    ]
}

/// Call a Blog module tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "list-blog-articles" => {
            let status = args.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());
            let category = args.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());
            let featured = args.get("featured").and_then(|v| v.as_bool());
            match blog::blog_list_articles(status, category, featured).await {
                Ok(articles) => ToolResult::json(&articles),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "get-blog-article" => {
            let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let slug = args.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string());
            match blog::blog_get_article(id, slug).await {
                Ok(article) => ToolResult::json(&article),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-blog-article" => {
            let data: CreateBlogArticle = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match blog::blog_create_article(data).await {
                Ok(article) => ToolResult::json(&article),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-blog-article" => {
            let article_id = match args.get("article_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("article_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("article_id");
            }
            let data: UpdateBlogArticle = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match blog::blog_update_article(article_id, data).await {
                Ok(article) => ToolResult::json(&article),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-blog-article" => {
            let article_id = match args.get("article_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("article_id is required".to_string()),
            };
            match blog::blog_delete_article(article_id).await {
                Ok(()) => ToolResult::json(&serde_json::json!({"deleted": true})),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        _ => ToolResult::error(format!("Unknown blog tool: {}", name)),
    }
}
