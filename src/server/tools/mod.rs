// MCP Tool Registry
// Defines and dispatches tools for Claude Code

pub mod work;
pub mod crm;
pub mod email;
pub mod generate;
pub mod intercom;
pub mod docgen;
pub mod val_sync;
pub mod feed;
pub mod discussions;
pub mod notifications;
pub mod blog;
pub mod whatsapp;
pub mod apollo;

use super::protocol::{Tool, ToolResult};
use serde_json::Value;

/// List all available tools
pub fn list_tools() -> Vec<Tool> {
    let mut tools = Vec::new();

    // Project module tools (projects, tasks, milestones, initiatives, labels, users)
    tools.extend(work::tools());

    // CRM module tools (companies, contacts, activities)
    tools.extend(crm::tools());

    // Email campaign tools
    tools.extend(email::tools());

    // Generation tools (Gamma, Nanobanana)
    tools.extend(generate::tools());

    // Intercom tools (Help Center publishing)
    tools.extend(intercom::tools());

    // Document generation tools (Order forms, Proposals)
    tools.extend(docgen::tools());

    // VAL Sync tools
    tools.extend(val_sync::tools());

    // Feed tools
    tools.extend(feed::tools());

    // Discussion tools
    tools.extend(discussions::tools());

    // Notification tools
    tools.extend(notifications::tools());

    // Blog tools
    tools.extend(blog::tools());

    // WhatsApp summary tools
    tools.extend(whatsapp::tools());

    // Apollo prospect search tools
    tools.extend(apollo::tools());

    tools
}

/// Call a tool by name
pub async fn call_tool(name: &str, arguments: Value) -> ToolResult {
    // Project module tools
    if name.starts_with("list-project") || name.starts_with("get-project") ||
       name.starts_with("create-project") || name.starts_with("update-project") ||
       name.starts_with("delete-project") ||
       name.starts_with("list-task") || name.starts_with("get-task") ||
       name.starts_with("create-task") || name.starts_with("update-task") ||
       name.starts_with("list-milestone") || name.starts_with("create-milestone") ||
       name.starts_with("update-milestone") ||
       name.starts_with("list-initiative") || name.starts_with("create-initiative") ||
       name.starts_with("update-initiative") || name.starts_with("delete-initiative") ||
       name == "add-project-to-initiative" || name == "remove-project-from-initiative" ||
       name == "list-initiative-projects" ||
       name.starts_with("list-label") || name.starts_with("create-label") ||
       name == "register-skill" || name == "list-skills" ||
       name == "list-users" || name == "list-bots" ||
       name == "get-pipeline" ||
       name.starts_with("add-project-") || name.starts_with("remove-project-") {
        return work::call(name, arguments).await;
    }

    // CRM module tools (companies, contacts) + activities (general)
    if name.starts_with("list-crm-") || name.starts_with("find-crm-") ||
       name.starts_with("get-crm-") || name.starts_with("create-crm-") ||
       name.starts_with("update-crm-") || name.starts_with("delete-crm-") ||
       name == "log-activity" || name == "list-activities" ||
       name == "update-activity" || name == "delete-activity" {
        return crm::call(name, arguments).await;
    }

    // Email campaign + transactional + entity email tools
    if name.starts_with("list-email-") || name.starts_with("create-email-") ||
       name.starts_with("update-email-") || name.starts_with("delete-email-") ||
       name == "send-email" || name == "list-entity-emails" {
        return email::call(name, arguments).await;
    }

    // Generation tools
    if name.starts_with("gamma-") || name.starts_with("nanobanana-") {
        return generate::call(name, arguments).await;
    }

    // Intercom tools
    if name.starts_with("list-intercom-") || name.starts_with("publish-to-intercom") {
        return intercom::call(name, arguments).await;
    }

    // Document generation tools
    if name.starts_with("generate-order-form") || name.starts_with("generate-proposal") || name == "check-document-type" {
        return docgen::call(name, arguments).await;
    }

    // VAL Sync tools
    if name.starts_with("sync-val-") || name.starts_with("sync-all-domain-") || name == "execute-val-sql" || name == "execute-supabase-sql" || name == "list-drive-files" || name == "check-all-domain-drive-files" {
        return val_sync::call(name, arguments).await;
    }

    // Feed tools
    if name.ends_with("-feed-card") || name.ends_with("-feed-cards") {
        return feed::call(name, arguments).await;
    }

    // Discussion tools
    if name.ends_with("-discussion") || name.ends_with("-discussions") {
        return discussions::call(name, arguments).await;
    }

    // Notification tools
    if name.ends_with("-notification") || name.ends_with("-notifications") || name.starts_with("mark-notification-") {
        return notifications::call(name, arguments).await;
    }

    // Blog tools
    if name.ends_with("-blog-article") || name.ends_with("-blog-articles") {
        return blog::call(name, arguments).await;
    }

    // WhatsApp summary tools
    if name.ends_with("-whatsapp-summary") || name.ends_with("-whatsapp-summaries") ||
       name == "whatsapp-latest-date" {
        return whatsapp::call(name, arguments).await;
    }

    // Apollo prospect search tools
    if name.starts_with("apollo-") {
        return apollo::call(name, arguments).await;
    }

    ToolResult::error(format!("Unknown tool: {}", name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // -------------------------------------------------------
    // Tool registry completeness
    // -------------------------------------------------------

    #[test]
    fn list_tools_returns_non_empty() {
        let tools = list_tools();
        assert!(!tools.is_empty(), "Tool registry should not be empty");
    }

    #[test]
    fn all_tool_names_are_unique() {
        let tools = list_tools();
        let mut seen = HashSet::new();
        for tool in &tools {
            assert!(
                seen.insert(&tool.name),
                "Duplicate tool name: {}",
                tool.name
            );
        }
    }

    #[test]
    fn all_tools_have_descriptions() {
        let tools = list_tools();
        for tool in &tools {
            assert!(
                !tool.description.is_empty(),
                "Tool '{}' has empty description",
                tool.name
            );
        }
    }

    #[test]
    fn all_tools_have_object_schema_type() {
        let tools = list_tools();
        for tool in &tools {
            assert_eq!(
                tool.input_schema.schema_type, "object",
                "Tool '{}' schema type should be 'object'",
                tool.name
            );
        }
    }

    #[test]
    fn required_fields_exist_in_properties() {
        let tools = list_tools();
        for tool in &tools {
            if let (Some(props), Some(required)) =
                (&tool.input_schema.properties, &tool.input_schema.required)
            {
                let props_obj = props.as_object().expect("properties should be an object");
                for req_field in required {
                    assert!(
                        props_obj.contains_key(req_field),
                        "Tool '{}': required field '{}' not found in properties",
                        tool.name,
                        req_field
                    );
                }
            }
        }
    }

    // -------------------------------------------------------
    // Expected tools exist in registry
    // -------------------------------------------------------

    fn tool_names() -> HashSet<String> {
        list_tools().into_iter().map(|t| t.name).collect()
    }

    #[test]
    fn crm_tools_registered() {
        let names = tool_names();
        let expected = [
            "list-crm-companies", "find-crm-company", "get-crm-company",
            "create-crm-company", "update-crm-company", "delete-crm-company",
            "list-crm-contacts", "find-crm-contact", "create-crm-contact",
            "update-crm-contact", "log-activity", "list-activities",
            "update-activity", "delete-activity",
        ];
        for name in expected {
            assert!(names.contains(name), "Missing CRM tool: {}", name);
        }
    }

    #[test]
    fn work_tools_registered() {
        let names = tool_names();
        let expected = [
            "list-projects", "get-project", "create-project", "update-project",
            "list-tasks", "get-task", "create-task", "update-task",
            "list-milestones", "create-milestone",
            "list-initiatives", "create-initiative",
            "list-labels", "create-label", "list-users",
        ];
        for name in expected {
            assert!(names.contains(name), "Missing work tool: {}", name);
        }
    }

    #[test]
    fn apollo_tools_registered() {
        let names = tool_names();
        let expected = ["apollo-search-people"];
        for name in expected {
            assert!(names.contains(name), "Missing Apollo tool: {}", name);
        }
    }

    #[test]
    fn email_tools_registered() {
        let names = tool_names();
        let expected = [
            "list-email-campaigns", "create-email-campaign",
            "update-email-campaign", "delete-email-campaign",
            "list-email-groups", "create-email-group",
        ];
        for name in expected {
            assert!(names.contains(name), "Missing email tool: {}", name);
        }
    }

    // -------------------------------------------------------
    // Dispatch routing — verify tool names route to correct module
    // -------------------------------------------------------
    // We can't call the actual handlers (they hit Supabase), but we can
    // verify that unknown tools produce the right error.

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let result = call_tool("nonexistent-tool", serde_json::json!({})).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].text.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn unknown_tool_includes_name_in_error() {
        let result = call_tool("foo-bar-baz", serde_json::json!({})).await;
        assert!(result.content[0].text.contains("foo-bar-baz"));
    }
}
