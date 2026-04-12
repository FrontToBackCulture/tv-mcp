// CRM Module MCP Tools
// Company, contact, and activity management tools

use crate::modules::crm::{self, CreateActivity, CreateCompany, CreateContact, UpdateActivity, UpdateCompany, UpdateContact};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define CRM module tools
pub fn tools() -> Vec<Tool> {
    vec![
        // Companies
        Tool {
            name: "list-crm-companies".to_string(),
            description: "List companies in the CRM with optional filters".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "search": { "type": "string", "description": "Search by company name" },
                    "stage": { "type": "string", "enum": ["prospect", "opportunity", "client", "churned", "partner"] },
                    "industry": { "type": "string", "description": "Filter by industry" },
                    "limit": { "type": "integer", "description": "Max results to return (default: 50)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "find-crm-company".to_string(),
            description: "Find a company by name or domain. Use this before creating to avoid duplicates.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Company name to search for" },
                    "domain": { "type": "string", "description": "Website domain to match (e.g., 'koi.com')" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "get-crm-company".to_string(),
            description: "Get full details for a company by ID, including contacts and recent activity.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "The company UUID" },
                    "include_relations": { "type": "boolean", "description": "Include contacts and activities" }
                }),
                vec!["company_id".to_string()],
            ),
        },
        Tool {
            name: "create-crm-company".to_string(),
            description: "Create a new company in the CRM. Use find-crm-company first to check if it exists.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "name": { "type": "string", "description": "Company name (required)" },
                    "display_name": { "type": "string", "description": "Display name if different from name" },
                    "industry": { "type": "string", "description": "Industry category" },
                    "website": { "type": "string", "description": "Company website URL" },
                    "stage": { "type": "string", "enum": ["prospect", "opportunity", "client", "churned", "partner"], "description": "Relationship stage (default: prospect)" },
                    "source": { "type": "string", "enum": ["apollo", "inbound", "referral", "manual", "existing"], "description": "Lead source" },
                    "notes": { "type": "string", "description": "Notes about the company" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for categorization" }
                }),
                vec!["name".to_string()],
            ),
        },
        Tool {
            name: "update-crm-company".to_string(),
            description: "Update an existing company's details.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "The company UUID (required)" },
                    "name": { "type": "string" },
                    "display_name": { "type": "string" },
                    "industry": { "type": "string" },
                    "website": { "type": "string" },
                    "stage": { "type": "string", "enum": ["prospect", "opportunity", "client", "churned", "partner"] },
                    "client_folder_path": { "type": "string", "description": "Path to client folder in knowledge base (e.g., 3_Clients/lag)" },
                    "deal_folder_path": { "type": "string", "description": "Path to deal folder in knowledge base (e.g., 4_Sales/deals/les-amis)" },
                    "research_folder_path": { "type": "string", "description": "Path to research profile folder in knowledge base (e.g., 4_Sales/research/companies/les-amis-group)" },
                    "domain_id": { "type": "string", "description": "VAL domain ID if client" },
                    "notes": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "outreach_status": { "type": "string", "enum": ["drafting", "contacted", "replied", "meeting_booked"], "description": "Outreach pipeline status" }
                }),
                vec!["company_id".to_string()],
            ),
        },
        Tool {
            name: "delete-crm-company".to_string(),
            description: "Delete a company and all related records (contacts, activities).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "The company UUID" }
                }),
                vec!["company_id".to_string()],
            ),
        },
        // Contacts
        Tool {
            name: "list-crm-contacts".to_string(),
            description: "List contacts, optionally filtered by company.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "Filter by company UUID" },
                    "search": { "type": "string", "description": "Search by name or email" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "find-crm-contact".to_string(),
            description: "Find a contact by email address.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "email": { "type": "string", "description": "Email address to search for" }
                }),
                vec!["email".to_string()],
            ),
        },
        Tool {
            name: "create-crm-contact".to_string(),
            description: "Create a new contact. Company is optional — omit for EDM-only contacts.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "Company UUID (optional — omit for contacts without a company)" },
                    "name": { "type": "string", "description": "Contact name (required)" },
                    "email": { "type": "string", "description": "Email address (required)" },
                    "phone": { "type": "string" },
                    "role": { "type": "string", "description": "Job title/role" },
                    "department": { "type": "string" },
                    "is_primary": { "type": "boolean", "description": "Set as primary contact for company" },
                    "notes": { "type": "string" },
                    "linkedin_url": { "type": "string" },
                    "prospect_stage": { "type": "string", "enum": ["new", "researched", "drafted", "sent", "opened", "replied"], "description": "Outbound prospect pipeline stage. Set to 'new' to add contact to outbound pipeline." },
                    "prospect_type": { "type": "array", "items": { "type": "string", "enum": ["prospect", "influencer", "peer", "customer", "door_opener"] }, "description": "Classification tags: prospect, influencer, peer, customer, door_opener. Can have multiple." },
                    "prospect_type_reason": { "type": "string", "description": "Justification for the prospect_type classification — why these tags were assigned" },
                    "linkedin_connect_msg": { "type": "string", "description": "Message to send with LinkedIn connection request" },
                    "linkedin_dm_msg": { "type": "string", "description": "Message to send if already connected on LinkedIn" },
                    "email_outreach_msg": { "type": "string", "description": "Draft email outreach message" },
                    "linkedin_connected": { "type": "boolean", "description": "Whether already connected on LinkedIn" },
                    "edm_status": { "type": "string", "enum": ["active", "unsubscribed", "bounced"], "description": "Email deliverability status for campaigns (default: active)" },
                    "source": { "type": "string", "enum": ["apollo", "web_search", "acra", "referral", "manual"], "description": "Where this contact was discovered" },
                    "email_status": { "type": "string", "enum": ["verified", "guessed", "unknown"], "description": "Email verification confidence: verified (confirmed from official source), guessed (pattern-matched), unknown (no email found)" }
                }),
                vec!["name".to_string(), "email".to_string()],
            ),
        },
        Tool {
            name: "update-crm-contact".to_string(),
            description: "Update an existing contact.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "contact_id": { "type": "string", "description": "Contact UUID (required)" },
                    "name": { "type": "string" },
                    "email": { "type": "string" },
                    "phone": { "type": "string" },
                    "role": { "type": "string" },
                    "department": { "type": "string" },
                    "is_primary": { "type": "boolean" },
                    "is_active": { "type": "boolean" },
                    "notes": { "type": "string" },
                    "linkedin_url": { "type": "string" },
                    "prospect_stage": { "type": "string", "enum": ["new", "researched", "drafted", "sent", "opened", "replied"], "description": "Outbound prospect pipeline stage. Set to 'new' to add contact to outbound pipeline." },
                    "prospect_type": { "type": "array", "items": { "type": "string", "enum": ["prospect", "influencer", "peer", "customer", "door_opener"] }, "description": "Classification tags: prospect, influencer, peer, customer, door_opener. Can have multiple." },
                    "prospect_type_reason": { "type": "string", "description": "Justification for the prospect_type classification — why these tags were assigned" },
                    "linkedin_connect_msg": { "type": "string", "description": "Message to send with LinkedIn connection request" },
                    "linkedin_dm_msg": { "type": "string", "description": "Message to send if already connected on LinkedIn" },
                    "email_outreach_msg": { "type": "string", "description": "Draft email outreach message" },
                    "linkedin_connected": { "type": "boolean", "description": "Whether already connected on LinkedIn" },
                    "edm_status": { "type": "string", "enum": ["active", "unsubscribed", "bounced"], "description": "Email deliverability status for campaigns" },
                    "source": { "type": "string", "enum": ["apollo", "web_search", "acra", "referral", "manual"], "description": "Where this contact was discovered" },
                    "email_status": { "type": "string", "enum": ["verified", "guessed", "unknown"], "description": "Email verification confidence" }
                }),
                vec!["contact_id".to_string()],
            ),
        },
        // Activities (general — works for companies, projects, tasks, or standalone)
        Tool {
            name: "log-activity".to_string(),
            description: "Log an activity (note, call, meeting) for a company, project, or task.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "Company UUID (optional)" },
                    "type": { "type": "string", "enum": ["note", "call", "meeting", "email", "task"], "description": "Activity type (required)" },
                    "subject": { "type": "string", "description": "Activity subject/title" },
                    "content": { "type": "string", "description": "Activity content/notes" },
                    "contact_id": { "type": "string", "description": "Link to a contact (optional)" },
                    "project_id": { "type": "string", "description": "Link to a project (optional)" },
                    "task_id": { "type": "string", "description": "Link to a task (optional)" },
                    "activity_date": { "type": "string", "description": "When the activity occurred (ISO date, default: now)" }
                }),
                vec!["type".to_string()],
            ),
        },
        Tool {
            name: "list-activities".to_string(),
            description: "List activities for a company, project, or task.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "company_id": { "type": "string", "description": "Filter by company UUID" },
                    "project_id": { "type": "string", "description": "Filter by project UUID" },
                    "task_id": { "type": "string", "description": "Filter by task UUID" },
                    "type": { "type": "string", "enum": ["note", "call", "meeting", "email", "task", "stage_change"] },
                    "limit": { "type": "integer", "description": "Max results (default: 20)" }
                }),
                vec![],
            ),
        },
        Tool {
            name: "update-activity".to_string(),
            description: "Update an existing activity.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "activity_id": { "type": "string", "description": "The activity UUID (required)" },
                    "type": { "type": "string", "enum": ["note", "call", "meeting", "email", "task"], "description": "Activity type" },
                    "subject": { "type": "string", "description": "Activity subject/title" },
                    "content": { "type": "string", "description": "Activity content/notes" },
                    "activity_date": { "type": "string", "description": "When the activity occurred (ISO date)" },
                    "company_id": { "type": "string", "description": "Company UUID" },
                    "contact_id": { "type": "string", "description": "Contact UUID" },
                    "project_id": { "type": "string", "description": "Project UUID" },
                    "task_id": { "type": "string", "description": "Task UUID" }
                }),
                vec!["activity_id".to_string()],
            ),
        },
        Tool {
            name: "delete-activity".to_string(),
            description: "Delete an activity.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "activity_id": { "type": "string", "description": "The activity UUID (required)" }
                }),
                vec!["activity_id".to_string()],
            ),
        },
    ]
}

/// Call a CRM module tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        // Companies
        "list-crm-companies" => {
            let search = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());
            let stage = args.get("stage").and_then(|v| v.as_str()).map(|s| s.to_string());
            let industry = args.get("industry").and_then(|v| v.as_str()).map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);
            match crm::crm_list_companies(search, stage, industry, limit).await {
                Ok(companies) => ToolResult::json(&companies),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "find-crm-company" => {
            let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
            let domain = args.get("domain").and_then(|v| v.as_str()).map(|s| s.to_string());
            match crm::crm_find_company(name, domain).await {
                Ok(company) => ToolResult::json(&company),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "get-crm-company" => {
            let company_id = match args.get("company_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("company_id is required".to_string()),
            };
            let include_relations = args.get("include_relations").and_then(|v| v.as_bool());
            match crm::crm_get_company(company_id, include_relations).await {
                Ok(company) => ToolResult::json(&company),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-crm-company" => {
            let data: CreateCompany = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_create_company(data).await {
                Ok(company) => ToolResult::json(&company),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-crm-company" => {
            let company_id = match args.get("company_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("company_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("company_id");
            }
            let data: UpdateCompany = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_update_company(company_id, data).await {
                Ok(company) => ToolResult::json(&company),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-crm-company" => {
            let company_id = match args.get("company_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("company_id is required".to_string()),
            };
            match crm::crm_delete_company(company_id).await {
                Ok(()) => ToolResult::text("Company deleted successfully".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Contacts
        "list-crm-contacts" => {
            let company_id = args.get("company_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let search = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());
            match crm::crm_list_contacts(company_id, search).await {
                Ok(contacts) => ToolResult::json(&contacts),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "find-crm-contact" => {
            let email = match args.get("email").and_then(|v| v.as_str()) {
                Some(e) => e.to_string(),
                None => return ToolResult::error("email is required".to_string()),
            };
            match crm::crm_find_contact(email).await {
                Ok(contact) => ToolResult::json(&contact),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "create-crm-contact" => {
            let mut create_args = args.clone();
            if let Some(obj) = create_args.as_object_mut() {
                // Fix prospect_type if passed as JSON string instead of array
                if let Some(val) = obj.get("prospect_type").cloned() {
                    if let Some(s) = val.as_str() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(s) {
                            obj.insert("prospect_type".to_string(), serde_json::json!(parsed));
                        }
                    }
                }
            }
            let data: CreateContact = match serde_json::from_value(create_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_create_contact(data).await {
                Ok(contact) => ToolResult::json(&contact),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-crm-contact" => {
            let contact_id = match args.get("contact_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("contact_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("contact_id");
                // Fix prospect_type if passed as JSON string instead of array
                if let Some(val) = obj.get("prospect_type").cloned() {
                    if let Some(s) = val.as_str() {
                        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(s) {
                            obj.insert("prospect_type".to_string(), serde_json::json!(parsed));
                        }
                    }
                }
            }
            let data: UpdateContact = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_update_contact(contact_id, data).await {
                Ok(contact) => ToolResult::json(&contact),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        // Activities
        "log-activity" => {
            let data: CreateActivity = match serde_json::from_value(args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_log_activity(data).await {
                Ok(activity) => ToolResult::json(&activity),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "list-activities" => {
            let company_id = args.get("company_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let project_id = args.get("project_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let task_id = args.get("task_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let activity_type = args.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);
            match crm::crm_list_activities(company_id, None, project_id, task_id, activity_type, limit).await {
                Ok(activities) => ToolResult::json(&activities),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "update-activity" => {
            let activity_id = match args.get("activity_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("activity_id is required".to_string()),
            };
            let mut data_args = args.clone();
            if let Some(obj) = data_args.as_object_mut() {
                obj.remove("activity_id");
            }
            let data: UpdateActivity = match serde_json::from_value(data_args) {
                Ok(d) => d,
                Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
            };
            match crm::crm_update_activity(activity_id, data).await {
                Ok(activity) => ToolResult::json(&activity),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }
        "delete-activity" => {
            let activity_id = match args.get("activity_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => return ToolResult::error("activity_id is required".to_string()),
            };
            match crm::crm_delete_activity(activity_id).await {
                Ok(()) => ToolResult::text("Activity deleted successfully".to_string()),
                Err(e) => ToolResult::error(e.to_string()),
            }
        }

        _ => ToolResult::error(format!("Unknown CRM tool: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -------------------------------------------------------
    // Tool definitions
    // -------------------------------------------------------

    #[test]
    fn crm_tools_count() {
        let t = tools();
        // Companies (6) + Contacts (4) + Activities (4) = 14
        assert_eq!(t.len(), 14);
    }

    #[test]
    fn create_company_requires_name() {
        let t = tools();
        let tool = t.iter().find(|t| t.name == "create-crm-company").unwrap();
        assert_eq!(tool.input_schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn get_company_requires_company_id() {
        let t = tools();
        let tool = t.iter().find(|t| t.name == "get-crm-company").unwrap();
        assert_eq!(
            tool.input_schema.required,
            Some(vec!["company_id".to_string()])
        );
    }

    #[test]
    fn create_contact_requires_name_and_email() {
        let t = tools();
        let tool = t.iter().find(|t| t.name == "create-crm-contact").unwrap();
        let req = tool.input_schema.required.as_ref().unwrap();
        assert!(req.contains(&"name".to_string()));
        assert!(req.contains(&"email".to_string()));
        assert!(!req.contains(&"company_id".to_string()));
    }

    #[test]
    fn log_activity_requires_type() {
        let t = tools();
        let tool = t.iter().find(|t| t.name == "log-activity").unwrap();
        assert_eq!(
            tool.input_schema.required,
            Some(vec!["type".to_string()])
        );
    }

    #[test]
    fn list_companies_has_no_required_fields() {
        let t = tools();
        let tool = t.iter().find(|t| t.name == "list-crm-companies").unwrap();
        assert!(tool.input_schema.required.is_none());
    }

    // -------------------------------------------------------
    // Argument parsing — CreateCompany deserialization
    // -------------------------------------------------------

    #[test]
    fn create_company_parses_minimal_args() {
        let args = json!({"name": "Acme Corp"});
        let data: CreateCompany = serde_json::from_value(args).unwrap();
        assert_eq!(data.name, "Acme Corp");
        assert!(data.stage.is_none());
        assert!(data.tags.is_none());
    }

    #[test]
    fn create_company_parses_full_args() {
        let args = json!({
            "name": "Acme Corp",
            "display_name": "ACME",
            "industry": "F&B",
            "website": "https://acme.com",
            "stage": "prospect",
            "source": "apollo",
            "notes": "Good lead",
            "tags": ["enterprise", "f&b"]
        });
        let data: CreateCompany = serde_json::from_value(args).unwrap();
        assert_eq!(data.display_name, Some("ACME".to_string()));
        assert_eq!(data.tags, Some(vec!["enterprise".to_string(), "f&b".to_string()]));
    }

    #[test]
    fn create_company_fails_without_name() {
        let args = json!({"industry": "Tech"});
        let result: Result<CreateCompany, _> = serde_json::from_value(args);
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // Argument parsing — UpdateCompany deserialization
    // -------------------------------------------------------

    #[test]
    fn update_company_parses_partial_fields() {
        let args = json!({"stage": "client", "domain_id": "abc-123"});
        let data: UpdateCompany = serde_json::from_value(args).unwrap();
        assert_eq!(data.stage, Some("client".to_string()));
        assert_eq!(data.domain_id, Some("abc-123".to_string()));
        assert!(data.name.is_none());
    }

    #[test]
    fn update_company_parses_empty_object() {
        let args = json!({});
        let data: UpdateCompany = serde_json::from_value(args).unwrap();
        assert!(data.name.is_none());
        assert!(data.stage.is_none());
    }

    // -------------------------------------------------------
    // Argument parsing — CreateContact deserialization
    // -------------------------------------------------------

    #[test]
    fn create_contact_parses_with_company() {
        let args = json!({
            "company_id": "comp-1",
            "name": "John Doe",
            "email": "john@example.com"
        });
        let data: CreateContact = serde_json::from_value(args).unwrap();
        assert_eq!(data.company_id, Some("comp-1".to_string()));
        assert_eq!(data.email, "john@example.com");
        assert!(data.phone.is_none());
    }

    #[test]
    fn create_contact_parses_without_company() {
        let args = json!({
            "name": "John Doe",
            "email": "john@example.com"
        });
        let data: CreateContact = serde_json::from_value(args).unwrap();
        assert!(data.company_id.is_none());
        assert_eq!(data.email, "john@example.com");
    }

    #[test]
    fn create_contact_fails_without_email() {
        let args = json!({"name": "John"});
        let result: Result<CreateContact, _> = serde_json::from_value(args);
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // Argument parsing — CreateActivity deserialization
    // -------------------------------------------------------

    #[test]
    fn create_activity_parses_with_type_field() {
        // 'type' is a reserved word in Rust — verify serde rename works
        let args = json!({
            "company_id": "comp-1",
            "type": "meeting",
            "subject": "Q1 Review",
            "content": "Discussed roadmap"
        });
        let data: CreateActivity = serde_json::from_value(args).unwrap();
        assert_eq!(data.activity_type, "meeting");
        assert_eq!(data.subject, Some("Q1 Review".to_string()));
    }

    #[test]
    fn create_activity_fails_without_type() {
        let args = json!({"company_id": "comp-1", "subject": "Hello"});
        let result: Result<CreateActivity, _> = serde_json::from_value(args);
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // Argument extraction patterns (from call function)
    // -------------------------------------------------------

    #[test]
    fn extract_optional_string_from_args() {
        let args = json!({"search": "koi", "limit": 10});
        let search = args.get("search").and_then(|v| v.as_str()).map(|s| s.to_string());
        let missing = args.get("stage").and_then(|v| v.as_str()).map(|s| s.to_string());
        assert_eq!(search, Some("koi".to_string()));
        assert_eq!(missing, None);
    }

    #[test]
    fn extract_limit_as_i32() {
        let args = json!({"limit": 25});
        let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);
        assert_eq!(limit, Some(25));
    }

    #[test]
    fn extract_limit_from_string_returns_none() {
        // If Claude sends limit as string "25" instead of number, as_i64 returns None
        let args = json!({"limit": "25"});
        let limit = args.get("limit").and_then(|v| v.as_i64()).map(|n| n as i32);
        assert_eq!(limit, None); // This is a known edge case
    }

    #[test]
    fn extract_bool_from_args() {
        let args = json!({"include_relations": true});
        let val = args.get("include_relations").and_then(|v| v.as_bool());
        assert_eq!(val, Some(true));
    }

    #[test]
    fn required_field_extraction_pattern() {
        // Pattern used in get-crm-company, update-crm-company, etc.
        let args = json!({"company_id": "abc-123"});
        let id = args.get("company_id").and_then(|v| v.as_str());
        assert_eq!(id, Some("abc-123"));

        let missing_args = json!({});
        let id = missing_args.get("company_id").and_then(|v| v.as_str());
        assert!(id.is_none());
    }

    #[test]
    fn update_company_strips_company_id_before_deser() {
        // Simulates the pattern in call() for update-crm-company
        let mut args = json!({
            "company_id": "abc",
            "name": "New Name",
            "stage": "client"
        });
        if let Some(obj) = args.as_object_mut() {
            obj.remove("company_id");
        }
        let data: UpdateCompany = serde_json::from_value(args).unwrap();
        assert_eq!(data.name, Some("New Name".to_string()));
        assert_eq!(data.stage, Some("client".to_string()));
    }
}
