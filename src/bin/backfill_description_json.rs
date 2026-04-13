// One-shot backfill: populate `description_json` for tasks that have
// `description` (markdown) set but `description_json` null. Converts markdown
// via the existing `markdown_to_tiptap_json` helper and writes back.
//
// Run: cargo run --release --bin backfill_description_json

use tv_mcp::modules::work::markdown::markdown_to_tiptap_json;
use tv_mcp::core::supabase::get_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client().await?;

    let rows: Vec<serde_json::Value> = client
        .select(
            "tasks",
            "select=id,description&description=not.is.null&description_json=is.null",
        )
        .await?;

    println!("Found {} tasks needing backfill", rows.len());

    for row in rows {
        let id = row["id"].as_str().unwrap_or("").to_string();
        let md = row["description"].as_str().unwrap_or("");
        if id.is_empty() || md.is_empty() {
            continue;
        }
        let json = markdown_to_tiptap_json(md);
        let update = serde_json::json!({ "description_json": json });
        let _: serde_json::Value = client
            .update("tasks", &format!("id=eq.{}", id), &update)
            .await?;
        println!("  ✓ {}  ({} chars md)", id, md.len());
    }

    println!("Done.");
    Ok(())
}
