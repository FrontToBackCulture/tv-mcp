// Apollo MCP Tools
// Search Apollo.io for prospects — exposed to Claude Code via MCP

use crate::modules::apollo::api::ApolloClient;
use crate::modules::apollo::types::ApolloSearchFilters;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "apollo-search-people".to_string(),
            description: "Search Apollo.io for people matching filters. Free — does not consume credits. Returns names, titles, companies, LinkedIn URLs but NOT emails/phones (need enrichment for those).".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "person_titles": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by job titles (e.g., [\"CEO\", \"CFO\", \"COO\", \"Head of Finance\", \"Finance Director\"])"
                    },
                    "person_locations": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by person locations (e.g., [\"Singapore\"])"
                    },
                    "person_seniorities": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by seniority: c_suite, vp, director, manager, senior, entry"
                    },
                    "person_departments": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by department: finance, operations, engineering, sales, marketing, human_resources, master_finance"
                    },
                    "organization_locations": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by company location (e.g., [\"Singapore\"])"
                    },
                    "organization_num_employees_ranges": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Employee count ranges: \"1,10\", \"11,50\", \"51,200\", \"201,500\", \"501,1000\", \"1001,5000\", \"5001,10000\""
                    },
                    "organization_industry_tag_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Apollo industry tag IDs to filter by"
                    },
                    "q_organization_name": {
                        "type": "string",
                        "description": "Search by company name (e.g., \"Paradise Group\")"
                    },
                    "q_keywords": {
                        "type": "string",
                        "description": "Free text keyword search"
                    },
                    "contact_email_status": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Email status filter: verified, guessed, unavailable"
                    },
                    "page": {
                        "type": "integer",
                        "description": "Page number (default: 1)"
                    },
                    "per_page": {
                        "type": "integer",
                        "description": "Results per page (default: 25, max: 100)"
                    }
                }),
                vec![],
            ),
        },
    ]
}

pub async fn call(name: &str, arguments: Value) -> ToolResult {
    match name {
        "apollo-search-people" => {
            let filters = ApolloSearchFilters {
                person_titles: extract_string_array(&arguments, "person_titles"),
                person_locations: extract_string_array(&arguments, "person_locations"),
                person_seniorities: extract_string_array(&arguments, "person_seniorities"),
                person_departments: extract_string_array(&arguments, "person_departments"),
                organization_locations: extract_string_array(&arguments, "organization_locations"),
                organization_num_employees_ranges: extract_string_array(&arguments, "organization_num_employees_ranges"),
                organization_industry_tag_ids: extract_string_array(&arguments, "organization_industry_tag_ids"),
                organization_ids: None,
                q_organization_name: arguments.get("q_organization_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                q_keywords: arguments.get("q_keywords")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                contact_email_status: extract_string_array(&arguments, "contact_email_status"),
                page: arguments.get("page")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                per_page: arguments.get("per_page")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
            };

            let client = match ApolloClient::new() {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("Failed to init Apollo client: {}", e)),
            };

            match client.search_people(&filters).await {
                Ok(response) => {
                    // Build a cleaner summary for Claude
                    let summary = build_search_summary(&response);
                    ToolResult::text(summary)
                }
                Err(e) => ToolResult::error(format!("Apollo search failed: {}", e)),
            }
        }
        _ => ToolResult::error(format!("Unknown Apollo tool: {}", name)),
    }
}

/// Extract an optional Vec<String> from a JSON value
fn extract_string_array(args: &Value, key: &str) -> Option<Vec<String>> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
}

/// Build a human-readable summary of Apollo search results
fn build_search_summary(response: &crate::modules::apollo::types::ApolloSearchResponse) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "Found {} total results.\n", response.total_entries);

    for (i, person) in response.people.iter().enumerate() {
        let _ = writeln!(out, "### {}. {}", i + 1,
            person.name.as_deref().unwrap_or("Unknown"));

        if let Some(ref title) = person.title {
            let _ = writeln!(out, "- **Title:** {}", title);
        }
        if let Some(ref seniority) = person.seniority {
            let _ = writeln!(out, "- **Seniority:** {}", seniority);
        }
        if let Some(ref linkedin) = person.linkedin_url {
            let _ = writeln!(out, "- **LinkedIn:** {}", linkedin);
        }
        if let Some(ref city) = person.city {
            let _ = write!(out, "- **Location:** {}", city);
            if let Some(ref country) = person.country {
                let _ = write!(out, ", {}", country);
            }
            let _ = writeln!(out);
        }

        // Organization info
        if let Some(ref org) = person.organization {
            if let Some(ref name) = org.name {
                let _ = writeln!(out, "- **Company:** {}", name);
            }
            if let Some(ref industry) = org.industry {
                let _ = writeln!(out, "- **Industry:** {}", industry);
            }
            if let Some(emp) = org.estimated_num_employees {
                let _ = writeln!(out, "- **Employees:** {}", emp);
            }
            if let Some(ref website) = org.website_url {
                let _ = writeln!(out, "- **Website:** {}", website);
            }
        }

        let _ = writeln!(out, "- **Apollo ID:** {}", person.id);
        let _ = writeln!(out);
    }

    out
}
