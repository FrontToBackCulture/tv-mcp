// MCP Tool Registry
// Defines and dispatches tools for Claude Code

pub mod work;
pub mod crm;
pub mod email;
pub mod generate;
pub mod intercom;
pub mod docgen;
pub mod dashboards;
pub mod val_admin;
pub mod val_cross_sync;
pub mod val_drive;
pub mod val_sync;
pub mod workflows;
pub mod discussions;
pub mod notifications;
pub mod blog;
pub mod docs;
pub mod whatsapp;
pub mod apollo;
pub mod diagnostics;
pub mod fy_review;
pub mod mcp_tools;
pub mod qbo;

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

    // Workflow authoring tools (create / update / execute / plugin discovery)
    tools.extend(workflows::tools());

    // VAL admin authoring tools (spaces / zones / tables)
    tools.extend(val_admin::tools());

    // VAL Drive — file/folder operations
    tools.extend(val_drive::tools());

    // VAL Dashboards
    tools.extend(dashboards::tools());

    // VAL Cross-Domain Sync (lab → other)
    tools.extend(val_cross_sync::tools());

    // Discussion tools
    tools.extend(discussions::tools());

    // Notification tools
    tools.extend(notifications::tools());

    // Blog tools
    tools.extend(blog::tools());

    // Docs portal tools (gated /docs section on tv-website)
    tools.extend(docs::tools());

    // WhatsApp summary tools
    tools.extend(whatsapp::tools());

    // Apollo prospect search tools
    tools.extend(apollo::tools());

    // Diagnostics tools
    tools.extend(diagnostics::tools());

    // QBO (mgmt workspace) tools
    tools.extend(qbo::tools());

    // FY Review (mgmt workspace) tools
    tools.extend(fy_review::tools());

    // MCP tools registry (self-discovery + metadata)
    tools.extend(mcp_tools::tools());

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

    // CRM module tools (companies, contacts) + activities (general).
    if name == "list-companies" || name == "find-company" || name == "get-company" ||
       name == "create-company" || name == "update-company" || name == "delete-company" ||
       name == "list-contacts" || name == "find-contact" || name == "create-contact" ||
       name == "update-contact" ||
       name == "add-activity" || name == "list-activities" ||
       name == "update-activity" || name == "delete-activity" {
        return crm::call(name, arguments).await;
    }

    // Email campaign + transactional + linked email tools
    if name.starts_with("list-email-") || name.starts_with("create-email-") ||
       name.starts_with("update-email-") || name.starts_with("delete-email-") ||
       name == "send-email" || name == "list-linked-emails" {
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

    // VAL Sync tools — excludes `sync-val-domain` which belongs to the cross-domain
    // dispatcher below (it's a write op, not a sync-from-source-of-truth pull).
    if (name.starts_with("sync-val-") && name != "sync-val-domain")
        || name.starts_with("sync-all-domain-")
        || name == "execute-val-sql"
        || name == "execute-supabase-sql"
    {
        return val_sync::call(name, arguments).await;
    }

    // VAL Drive tools
    if name == "list-val-drive-folders" || name == "list-val-drive-files"
        || name == "check-val-drive-files-all-domains" || name == "check-val-drive-file-exists"
        || name == "create-val-drive-folder"
        || name == "rename-val-drive-file" || name == "move-val-drive-file" {
        return val_drive::call(name, arguments).await;
    }

    // VAL Dashboards
    if name == "list-val-dashboards" || name == "get-val-dashboard"
        || name == "create-val-dashboard" || name == "update-val-dashboard"
        || name == "duplicate-val-dashboard"
        || name == "add-val-dashboard-widget" || name == "update-val-dashboard-widget" {
        return dashboards::call(name, arguments).await;
    }

    // VAL Cross-Domain Sync
    if name == "sync-val-domain" || name == "get-val-sync-status" || name == "promote-val-resources" {
        return val_cross_sync::call(name, arguments).await;
    }

    // Workflow authoring tools
    if name == "create-val-workflow" || name == "update-val-workflow" || name == "execute-val-workflow"
        || name == "list-val-workflow-plugins" || name == "get-val-workflow-plugin-schema"
        || name == "list-val-workflows" || name == "get-val-workflow"
        || name == "pause-val-workflow" || name == "resume-val-workflow"
        || name == "list-val-workflow-executions" || name == "get-val-workflow-execution" {
        return workflows::call(name, arguments).await;
    }

    // VAL admin authoring tools (spaces, zones, tables, fields, queries)
    if name == "create-val-space" || name == "update-val-space"
        || name == "list-val-spaces" || name == "get-val-space"
        || name == "list-val-space-zones"
        || name == "create-val-zone" || name == "update-val-zone"
        || name == "list-val-zones" || name == "get-val-zone"
        || name == "list-val-zone-tables"
        || name == "create-val-table" || name == "clone-val-table"
        || name == "update-val-table" || name == "get-val-table"
        || name == "list-val-tables" || name == "list-val-table-dependencies"
        || name == "add-val-table-field" || name == "add-val-table-fields"
        || name == "remove-val-table-field"
        || name == "update-val-field" || name == "assign-val-table-to-zone"
        || name == "list-val-fields" || name == "find-val-tables-with-field"
        || name == "create-val-query" || name == "update-val-query"
        || name == "copy-val-query"
        || name == "list-val-queries" || name == "get-val-query"
        || name == "execute-val-query" || name == "test-val-query"
        || name == "list-val-linkages" || name == "create-val-linkage"
        || name == "update-val-linkage"
        || name == "list-val-integrations" || name == "list-val-integration-tables"
        || name == "get-val-integration" || name == "get-val-integration-fields"
        || name == "save-val-integration" || name == "test-val-integration"
        || name == "extract-val-integration" {
        return val_admin::call(name, arguments).await;
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

    // Docs portal tools
    if name.ends_with("-docs-page") || name.ends_with("-docs-pages") {
        return docs::call(name, arguments).await;
    }

    // WhatsApp summary tools
    if name.ends_with("-whatsapp-summary") || name.ends_with("-whatsapp-summaries") ||
       name == "get-latest-whatsapp-summary-date" {
        return whatsapp::call(name, arguments).await;
    }

    // Apollo prospect search tools
    if name.starts_with("apollo-") {
        return apollo::call(name, arguments).await;
    }

    // Diagnostics
    if name == "diagnostics" {
        return diagnostics::call(name, arguments).await;
    }

    // QBO tools (mgmt workspace)
    if name.starts_with("qbo-") {
        return qbo::call(name, arguments).await;
    }

    // FY Review tools (mgmt workspace)
    if name.starts_with("fy-") {
        return fy_review::call(name, arguments).await;
    }

    // MCP tools registry
    if name == "sync-mcp-tools" || name == "list-mcp-tools" {
        return mcp_tools::call(name, arguments).await;
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
            "list-companies", "find-company", "get-company",
            "create-company", "update-company", "delete-company",
            "list-contacts", "find-contact", "create-contact",
            "update-contact", "add-activity", "list-activities",
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
