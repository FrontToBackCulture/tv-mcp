// Email Campaign MCP Tools
// Campaign and group management for the email module

use crate::modules::crm::{self as crm_commands, CreateActivity};
use crate::modules::email::campaigns::{self, CreateCampaign, UpdateCampaign};
use crate::modules::email::send::send_transactional_email;
use crate::core::supabase::get_client;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define email module tools
pub fn tools() -> Vec<Tool> {
    vec![
        // Entity email links (linked correspondence/campaigns on projects, tasks, companies, contacts)
        Tool {
            name: "list-entity-emails".to_string(),
            description: "List emails linked to a project, task, company, or contact. Returns email metadata (subject, from, date) from the email cache. Use this to see what correspondence is associated with an entity.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "entity_type": { "type": "string", "enum": ["project", "task", "company", "contact"], "description": "Type of entity to list emails for" },
                    "entity_id": { "type": "string", "description": "UUID of the entity" },
                    "email_type": { "type": "string", "enum": ["correspondence", "campaign"], "description": "Filter by email type (default: all)" },
                    "limit": { "type": "integer", "description": "Max results (default: 50)" }
                }),
                vec!["entity_type".to_string(), "entity_id".to_string()],
            ),
        },
        // Campaigns
        Tool {
            name: "list-email-campaigns".to_string(),
            description: "List email campaigns with optional filters (status, group, search).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "status": { "type": "string", "enum": ["draft", "scheduled", "sending", "sent", "partial", "failed"], "description": "Filter by campaign status" },
                    "group_id": { "type": "string", "description": "Filter by email group ID" },
                    "search": { "type": "string", "description": "Search by campaign name or subject" },
                    "limit": { "type": "integer", "description": "Max results (default: 50)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "create-email-campaign".to_string(),
            description: "Create a new email campaign. Use list-email-groups first to get valid group IDs. Set content_path to the relative path of the campaign HTML file in tv-knowledge (e.g., '6_Marketing/external/campaigns/email-campaigns/2026-03-my-campaign.html').".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Campaign name (required)" },
                    "subject": { "type": "string", "description": "Email subject line (required)" },
                    "from_name": { "type": "string", "description": "Sender display name (required)" },
                    "from_email": { "type": "string", "description": "Sender email address (required, default: hello@thinkval.com)" },
                    "group_id": { "type": "string", "description": "Target email group ID" },
                    "content_path": { "type": "string", "description": "Relative path to campaign HTML file in tv-knowledge" },
                    "html_body": { "type": "string", "description": "Inline HTML body (use content_path instead for file-based campaigns)" },
                    "bcc_email": { "type": "string", "description": "BCC recipient email" },
                    "category": { "type": "string", "description": "Campaign category for organization" },
                    "status": { "type": "string", "enum": ["draft", "scheduled"], "description": "Initial status (default: draft)" },
                    "tokens": { "type": "object", "description": "Custom template token key-value pairs (e.g. {\"report_url\": \"https://...\", \"chat_url\": \"https://...\"})" }
                }),
                vec!["name".to_string(), "subject".to_string(), "from_name".to_string()],
            ),
        },
        Tool {
            name: "update-email-campaign".to_string(),
            description: "Update an existing email campaign's metadata (name, subject, group, etc.).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "campaign_id": { "type": "string", "description": "The campaign UUID (required)" },
                    "name": { "type": "string" },
                    "subject": { "type": "string" },
                    "from_name": { "type": "string" },
                    "from_email": { "type": "string" },
                    "group_id": { "type": "string" },
                    "content_path": { "type": "string" },
                    "bcc_email": { "type": "string" },
                    "category": { "type": "string" },
                    "status": { "type": "string", "enum": ["draft", "scheduled"] },
                    "tokens": { "type": "object", "description": "Custom template token key-value pairs" }
                }),
                vec!["campaign_id".to_string()],
            ),
        },
        Tool {
            name: "delete-email-campaign".to_string(),
            description: "Delete an email campaign by ID. Only works for draft campaigns.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "campaign_id": { "type": "string", "description": "The campaign UUID to delete" }
                }),
                vec!["campaign_id".to_string()],
            ),
        },
        // Groups
        Tool {
            name: "list-email-groups".to_string(),
            description: "List all email groups (contact lists). Use to find group IDs when creating campaigns.".to_string(),
            input_schema: InputSchema::with_properties(json!({}), vec![]),
        },
        Tool {
            name: "create-email-group".to_string(),
            description: "Create a new email group (contact list).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Group name (required)" },
                    "description": { "type": "string", "description": "Group description" }
                }),
                vec!["name".to_string()],
            ),
        },
        // Transactional send (direct, no UI review)
        Tool {
            name: "send-email".to_string(),
            description: "Send a single 1-to-1 email immediately via AWS SES. Skips UI review. Use create-email-draft instead if the user should review before sending.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "to": { "type": "string", "description": "Recipient email address" },
                    "subject": { "type": "string", "description": "Email subject line" },
                    "html_body": { "type": "string", "description": "HTML email body" },
                    "from_name": { "type": "string", "description": "Sender display name (default: ThinkVAL)" },
                    "from_email": { "type": "string", "description": "Sender email address (default: hello@thinkval.com)" },
                    "log_crm_activity": { "type": "boolean", "description": "Log this send as a CRM email activity (default: false)" },
                    "crm_company_id": { "type": "string", "description": "Company UUID for CRM activity log (required if log_crm_activity is true)" },
                    "crm_contact_id": { "type": "string", "description": "Contact UUID for CRM activity log" },
                    "crm_project_id": { "type": "string", "description": "Deal/project UUID for CRM activity log" }
                }),
                vec!["to".to_string(), "subject".to_string(), "html_body".to_string()],
            ),
        },
        // Email draft (for UI review before sending)
        Tool {
            name: "create-email-draft".to_string(),
            description: "Create an email draft for a contact. The draft appears in the contact's detail panel in tv-client where the user can preview and send it. Use this for prospecting emails that need human review before sending.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "contact_id": { "type": "string", "description": "CRM contact UUID (required)" },
                    "company_id": { "type": "string", "description": "CRM company UUID" },
                    "to_email": { "type": "string", "description": "Recipient email address (required)" },
                    "subject": { "type": "string", "description": "Email subject line (required)" },
                    "html_body": { "type": "string", "description": "HTML email body (required)" },
                    "from_name": { "type": "string", "description": "Sender display name (default: ThinkVAL)" },
                    "from_email": { "type": "string", "description": "Sender email address (default: hello@thinkval.com)" },
                    "draft_type": { "type": "string", "description": "Draft type: 'manual' (default) or 'outreach' for automation-generated outreach emails" },
                    "context": { "type": "object", "description": "AI research notes / reasoning stored as JSONB — shown to reviewer so they understand why the email was drafted this way" },
                    "automation_run_id": { "type": "string", "description": "Links this draft to the automation run that created it" }
                }),
                vec!["contact_id".to_string(), "to_email".to_string(), "subject".to_string(), "html_body".to_string()],
            ),
        },
    ]
}

/// Handle email tool calls
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "list-entity-emails" => {
            let entity_type = match args.get("entity_type").and_then(|v| v.as_str()) {
                Some(v) => v,
                None => return ToolResult::error("entity_type is required".to_string()),
            };
            let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
                Some(v) => v,
                None => return ToolResult::error("entity_id is required".to_string()),
            };
            let email_type = args.get("email_type").and_then(|v| v.as_str());
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);

            let client = match get_client().await {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("{}", e)),
            };

            // Step 1: Get linked email IDs from email_entity_links
            let mut link_query = format!(
                "entity_type=eq.{}&entity_id=eq.{}&order=created_at.desc&limit={}",
                entity_type, entity_id, limit
            );
            if let Some(et) = email_type {
                link_query.push_str(&format!("&email_type=eq.{}", et));
            }

            let links: Vec<serde_json::Value> = match client.select("email_entity_links", &link_query).await {
                Ok(v) => v,
                Err(e) => return ToolResult::error(format!("Failed to fetch links: {}", e)),
            };

            if links.is_empty() {
                return ToolResult::json(&json!([]));
            }

            // Step 2: Collect correspondence email IDs to look up in email_cache
            let email_ids: Vec<&str> = links.iter()
                .filter(|l| l.get("email_type").and_then(|v| v.as_str()) == Some("correspondence"))
                .filter_map(|l| l.get("email_id").and_then(|v| v.as_str()))
                .collect();

            // Step 3: Fetch metadata from email_cache
            let mut cache_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
            if !email_ids.is_empty() {
                // PostgREST in filter: id=in.(val1,val2,...)
                let ids_csv: String = email_ids.iter()
                    .map(|id| format!("\"{}\"", id))
                    .collect::<Vec<_>>()
                    .join(",");
                let cache_query = format!("id=in.({})", ids_csv);
                let cached: Vec<serde_json::Value> = match client.select("email_cache", &cache_query).await {
                    Ok(v) => v,
                    Err(_) => vec![], // Graceful fallback — cache may not have all entries
                };
                for entry in cached {
                    if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                        cache_map.insert(id.to_string(), entry);
                    }
                }
            }

            // Step 4: Merge link data with cached metadata
            let results: Vec<serde_json::Value> = links.iter().map(|link| {
                let email_id = link.get("email_id").and_then(|v| v.as_str()).unwrap_or("");
                let email_type_val = link.get("email_type").and_then(|v| v.as_str()).unwrap_or("");
                let match_method = link.get("match_method").and_then(|v| v.as_str());
                let relevance_score = link.get("relevance_score").and_then(|v| v.as_f64());
                let linked_at = link.get("created_at").and_then(|v| v.as_str());

                let mut result = json!({
                    "email_id": email_id,
                    "email_type": email_type_val,
                    "linked_at": linked_at,
                });

                if let Some(m) = match_method {
                    result["match_method"] = json!(m);
                }
                if let Some(s) = relevance_score {
                    result["relevance_score"] = json!(s);
                }

                // Merge cached metadata if available
                if let Some(cached) = cache_map.get(email_id) {
                    if let Some(s) = cached.get("subject").and_then(|v| v.as_str()) {
                        result["subject"] = json!(s);
                    }
                    if let Some(s) = cached.get("from_email").and_then(|v| v.as_str()) {
                        result["from_email"] = json!(s);
                    }
                    if let Some(s) = cached.get("from_name").and_then(|v| v.as_str()) {
                        result["from_name"] = json!(s);
                    }
                    if let Some(s) = cached.get("received_at").and_then(|v| v.as_str()) {
                        result["received_at"] = json!(s);
                    }
                    if let Some(s) = cached.get("body_preview").and_then(|v| v.as_str()) {
                        result["body_preview"] = json!(s);
                    }
                }

                result
            }).collect();

            ToolResult::json(&results)
        }

        "list-email-campaigns" => {
            let status = args.get("status").and_then(|v| v.as_str()).map(String::from);
            let group_id = args.get("group_id").and_then(|v| v.as_str()).map(String::from);
            let search = args.get("search").and_then(|v| v.as_str()).map(String::from);
            let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);

            match campaigns::list_campaigns(status, group_id, search, limit).await {
                Ok(list) => ToolResult::json(&list),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "create-email-campaign" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("name is required".to_string()),
            };
            let subject = match args.get("subject").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("subject is required".to_string()),
            };
            let from_name = match args.get("from_name").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("from_name is required".to_string()),
            };
            let from_email = args.get("from_email")
                .and_then(|v| v.as_str())
                .unwrap_or("hello@thinkval.com")
                .to_string();

            let data = CreateCampaign {
                name,
                subject,
                from_name,
                from_email,
                group_id: args.get("group_id").and_then(|v| v.as_str()).map(String::from),
                html_body: args.get("html_body").and_then(|v| v.as_str()).map(String::from),
                content_path: args.get("content_path").and_then(|v| v.as_str()).map(String::from),
                bcc_email: args.get("bcc_email").and_then(|v| v.as_str()).map(String::from),
                category: args.get("category").and_then(|v| v.as_str()).map(String::from),
                status: args.get("status").and_then(|v| v.as_str()).map(String::from),
                tokens: args.get("tokens").cloned(),
                send_channel: args.get("send_channel").and_then(|v| v.as_str()).map(String::from),
            };

            match campaigns::create_campaign(data).await {
                Ok(campaign) => ToolResult::json(&campaign),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "update-email-campaign" => {
            let campaign_id = match args.get("campaign_id").and_then(|v| v.as_str()) {
                Some(v) => v,
                None => return ToolResult::error("campaign_id is required".to_string()),
            };

            let data = UpdateCampaign {
                name: args.get("name").and_then(|v| v.as_str()).map(String::from),
                subject: args.get("subject").and_then(|v| v.as_str()).map(String::from),
                from_name: args.get("from_name").and_then(|v| v.as_str()).map(String::from),
                from_email: args.get("from_email").and_then(|v| v.as_str()).map(String::from),
                group_id: args.get("group_id").and_then(|v| v.as_str()).map(String::from),
                html_body: args.get("html_body").and_then(|v| v.as_str()).map(String::from),
                content_path: args.get("content_path").and_then(|v| v.as_str()).map(String::from),
                bcc_email: args.get("bcc_email").and_then(|v| v.as_str()).map(String::from),
                category: args.get("category").and_then(|v| v.as_str()).map(String::from),
                status: args.get("status").and_then(|v| v.as_str()).map(String::from),
                tokens: args.get("tokens").cloned(),
                send_channel: args.get("send_channel").and_then(|v| v.as_str()).map(String::from),
            };

            match campaigns::update_campaign(campaign_id, data).await {
                Ok(campaign) => ToolResult::json(&campaign),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "delete-email-campaign" => {
            let campaign_id = match args.get("campaign_id").and_then(|v| v.as_str()) {
                Some(v) => v,
                None => return ToolResult::error("campaign_id is required".to_string()),
            };

            match campaigns::delete_campaign(campaign_id).await {
                Ok(_) => ToolResult::text("Campaign deleted successfully".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "list-email-groups" => {
            match campaigns::list_groups().await {
                Ok(groups) => ToolResult::json(&groups),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "create-email-group" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(v) => v,
                None => return ToolResult::error("name is required".to_string()),
            };
            let description = args.get("description").and_then(|v| v.as_str());

            match campaigns::create_group(name, description).await {
                Ok(group) => ToolResult::json(&group),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        "send-email" => {
            let to = match args.get("to").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("to is required".to_string()),
            };
            let subject = match args.get("subject").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("subject is required".to_string()),
            };
            let html_body = match args.get("html_body").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("html_body is required".to_string()),
            };
            let from_name = args.get("from_name").and_then(|v| v.as_str()).map(String::from);
            let from_email = args.get("from_email").and_then(|v| v.as_str()).map(String::from);

            let result = match send_transactional_email(
                &to,
                &subject,
                &html_body,
                from_name.as_deref(),
                from_email.as_deref(),
            )
            .await
            {
                Ok(r) => r,
                Err(e) => return ToolResult::error(format!("{}", e)),
            };

            if !result.success {
                return ToolResult::error(
                    result.error.unwrap_or_else(|| "Send failed".to_string()),
                );
            }

            // Optional CRM activity logging
            let log_crm = args
                .get("log_crm_activity")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if log_crm {
                let company_id = args
                    .get("crm_company_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if company_id.is_none() {
                    return ToolResult::json(&json!({
                        "success": true,
                        "message_id": result.message_id,
                        "warning": "log_crm_activity was true but crm_company_id was not provided — activity not logged"
                    }));
                }
                let activity = CreateActivity {
                    company_id,
                    activity_type: "email".to_string(),
                    contact_id: args
                        .get("crm_contact_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    project_id: args
                        .get("crm_project_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    task_id: None,
                    email_id: None,
                    subject: Some(subject.clone()),
                    content: Some(format!("Sent to: {}", to)),
                    activity_date: None,
                };
                if let Err(e) = crm_commands::crm_log_activity(activity).await {
                    return ToolResult::json(&json!({
                        "success": true,
                        "message_id": result.message_id,
                        "warning": format!("Email sent but CRM activity log failed: {}", e)
                    }));
                }
            }

            ToolResult::json(&json!({
                "success": true,
                "message_id": result.message_id
            }))
        }

        "create-email-draft" => {
            let contact_id = match args.get("contact_id").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("contact_id is required".to_string()),
            };
            let to_email = match args.get("to_email").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("to_email is required".to_string()),
            };
            let subject = match args.get("subject").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("subject is required".to_string()),
            };
            let html_body = match args.get("html_body").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => return ToolResult::error("html_body is required".to_string()),
            };
            let from_name = args
                .get("from_name")
                .and_then(|v| v.as_str())
                .unwrap_or("ThinkVAL");
            let from_email = args
                .get("from_email")
                .and_then(|v| v.as_str())
                .unwrap_or("hello@thinkval.com");
            let company_id = args.get("company_id").and_then(|v| v.as_str());

            let client = match get_client().await {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("{}", e)),
            };

            let mut insert = json!({
                "contact_id": contact_id,
                "to_email": to_email,
                "subject": subject,
                "html_body": html_body,
                "from_name": from_name,
                "from_email": from_email,
                "status": "draft"
            });
            if let Some(cid) = company_id {
                insert["company_id"] = json!(cid);
            }
            if let Some(dt) = args.get("draft_type").and_then(|v| v.as_str()) {
                insert["draft_type"] = json!(dt);
            }
            if let Some(ctx) = args.get("context") {
                insert["context"] = ctx.clone();
            }
            if let Some(run_id) = args.get("automation_run_id").and_then(|v| v.as_str()) {
                insert["automation_run_id"] = json!(run_id);
            }

            match client.insert::<_, serde_json::Value>("email_drafts", &insert).await {
                Ok(row) => ToolResult::json(&row),
                Err(e) => ToolResult::error(format!("Failed to create draft: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown email tool: {}", name)),
    }
}
