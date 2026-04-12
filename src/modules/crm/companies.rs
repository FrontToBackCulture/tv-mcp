// CRM Module - Company Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

/// Build PostgREST query string for listing companies
pub(crate) fn build_list_companies_query(
    search: &Option<String>,
    stage: &Option<String>,
    industry: &Option<String>,
    limit: Option<i32>,
) -> String {
    let mut filters = vec![];

    if let Some(s) = search {
        filters.push(format!("or=(name.ilike.*{}*,display_name.ilike.*{}*)", s, s));
    }
    if let Some(st) = stage {
        filters.push(format!("stage=eq.{}", st));
    }
    if let Some(ind) = industry {
        filters.push(format!("industry=eq.{}", ind));
    }

    let limit_val = limit.unwrap_or(50);
    filters.push(format!("limit={}", limit_val));
    filters.push("order=updated_at.desc".to_string());

    filters.join("&")
}

/// List companies with optional filters

pub async fn crm_list_companies(
    search: Option<String>,
    stage: Option<String>,
    industry: Option<String>,
    limit: Option<i32>,
) -> CmdResult<Vec<Company>> {
    let client = get_client().await?;
    let query = build_list_companies_query(&search, &stage, &industry, limit);
    client.select("crm_companies", &query).await
}

/// Build query for finding company by name
pub(crate) fn build_find_by_name_query(name: &str) -> String {
    format!("or=(name.ilike.*{}*,display_name.ilike.*{}*)&limit=1", name, name)
}

/// Build query for finding company by domain
pub(crate) fn build_find_by_domain_query(domain: &str) -> String {
    format!("website.ilike.*{}*&limit=1", domain)
}

/// Build query for getting company by ID with optional relations
pub(crate) fn build_get_company_query(company_id: &str, include_relations: bool) -> String {
    if include_relations {
        format!(
            "select=*,contacts:crm_contacts(*),activities:crm_activities(*)&id=eq.{}",
            company_id
        )
    } else {
        format!("id=eq.{}", company_id)
    }
}

/// Find company by name or domain (fuzzy search)

pub async fn crm_find_company(
    name: Option<String>,
    domain: Option<String>,
) -> CmdResult<Option<Company>> {
    let client = get_client().await?;

    if let Some(n) = name {
        let query = build_find_by_name_query(&n);
        return client.select_single("crm_companies", &query).await;
    }

    if let Some(d) = domain {
        let query = build_find_by_domain_query(&d);
        return client.select_single("crm_companies", &query).await;
    }

    Ok(None)
}

/// Get a single company by ID with optional relations

pub async fn crm_get_company(company_id: String, include_relations: Option<bool>) -> CmdResult<Company> {
    let client = get_client().await?;
    let query = build_get_company_query(&company_id, include_relations.unwrap_or(false));

    client
        .select_single("crm_companies", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("Company not found: {}", company_id)))
}

/// Create a new company

pub async fn crm_create_company(data: CreateCompany) -> CmdResult<Company> {
    let client = get_client().await?;

    // Set default stage if not provided
    let mut insert_data = serde_json::to_value(&data)?;
    if let Some(obj) = insert_data.as_object_mut() {
        if obj.get("stage").map_or(true, |v| v.is_null()) {
            obj.insert("stage".to_string(), serde_json::Value::String("prospect".to_string()));
        }
        if obj.get("source").map_or(true, |v| v.is_null()) {
            obj.insert("source".to_string(), serde_json::Value::String("manual".to_string()));
        }
    }

    client.insert("crm_companies", &insert_data).await
}

/// Update a company

pub async fn crm_update_company(company_id: String, data: UpdateCompany) -> CmdResult<Company> {
    let client = get_client().await?;

    // Check if stage is changing for activity logging
    if let Some(new_stage) = &data.stage {
        let current: Company = crm_get_company(company_id.clone(), None).await?;
        if let Some(old_stage) = &current.stage {
            if old_stage != new_stage {
                // Create stage_change activity
                let activity = serde_json::json!({
                    "company_id": company_id,
                    "type": "stage_change",
                    "old_value": old_stage,
                    "new_value": new_stage,
                    "activity_date": chrono::Utc::now().to_rfc3339()
                });
                let _: Activity = client.insert("crm_activities", &activity).await?;
            }
        }
    }

    let query = format!("id=eq.{}", company_id);
    client.update("crm_companies", &query, &data).await
}

/// Delete a company and all related records

pub async fn crm_delete_company(company_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    // Delete in order: activities, email_links, contacts, company
    client.delete("crm_activities", &format!("company_id=eq.{}", company_id)).await?;
    client.delete("crm_email_company_links", &format!("company_id=eq.{}", company_id)).await?;
    client.delete("crm_contacts", &format!("company_id=eq.{}", company_id)).await?;
    client.delete("crm_companies", &format!("id=eq.{}", company_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------
    // build_list_companies_query
    // -------------------------------------------------------

    #[test]
    fn list_query_no_filters_has_default_limit_and_order() {
        let q = build_list_companies_query(&None, &None, &None, None);
        assert_eq!(q, "limit=50&order=updated_at.desc");
    }

    #[test]
    fn list_query_custom_limit() {
        let q = build_list_companies_query(&None, &None, &None, Some(10));
        assert!(q.contains("limit=10"));
        assert!(!q.contains("limit=50"));
    }

    #[test]
    fn list_query_with_search() {
        let q = build_list_companies_query(
            &Some("koi".to_string()), &None, &None, None,
        );
        assert!(q.contains("or=(name.ilike.*koi*,display_name.ilike.*koi*)"));
    }

    #[test]
    fn list_query_with_stage() {
        let q = build_list_companies_query(
            &None, &Some("client".to_string()), &None, None,
        );
        assert!(q.contains("stage=eq.client"));
    }

    #[test]
    fn list_query_with_industry() {
        let q = build_list_companies_query(
            &None, &None, &Some("F&B".to_string()), None,
        );
        assert!(q.contains("industry=eq.F&B"));
    }

    #[test]
    fn list_query_all_filters_combined() {
        let q = build_list_companies_query(
            &Some("test".to_string()),
            &Some("prospect".to_string()),
            &Some("Tech".to_string()),
            Some(5),
        );
        assert!(q.contains("or=(name.ilike.*test*,display_name.ilike.*test*)"));
        assert!(q.contains("stage=eq.prospect"));
        assert!(q.contains("industry=eq.Tech"));
        assert!(q.contains("limit=5"));
        assert!(q.contains("order=updated_at.desc"));
        // Verify order: filters come before limit and order
        let limit_pos = q.find("limit=5").unwrap();
        let order_pos = q.find("order=").unwrap();
        assert!(limit_pos < order_pos);
    }

    // -------------------------------------------------------
    // build_find_by_name_query
    // -------------------------------------------------------

    #[test]
    fn find_by_name_query() {
        let q = build_find_by_name_query("Acme");
        assert_eq!(q, "or=(name.ilike.*Acme*,display_name.ilike.*Acme*)&limit=1");
    }

    // -------------------------------------------------------
    // build_find_by_domain_query
    // -------------------------------------------------------

    #[test]
    fn find_by_domain_query() {
        let q = build_find_by_domain_query("acme.com");
        assert_eq!(q, "website.ilike.*acme.com*&limit=1");
    }

    // -------------------------------------------------------
    // build_get_company_query
    // -------------------------------------------------------

    #[test]
    fn get_company_without_relations() {
        let q = build_get_company_query("abc-123", false);
        assert_eq!(q, "id=eq.abc-123");
    }

    #[test]
    fn get_company_with_relations() {
        let q = build_get_company_query("abc-123", true);
        assert!(q.contains("select=*,contacts:crm_contacts(*)"));
        assert!(q.contains("activities:crm_activities(*)"));
        assert!(q.contains("id=eq.abc-123"));
    }

    // -------------------------------------------------------
    // Default values in create_company
    // -------------------------------------------------------

    #[test]
    fn create_company_defaults_stage_and_source() {
        let data = CreateCompany {
            name: "Test".to_string(),
            display_name: None,
            industry: None,
            website: None,
            stage: None,
            source: None,
            notes: None,
            tags: None,
        };
        let mut val = serde_json::to_value(&data).unwrap();
        if let Some(obj) = val.as_object_mut() {
            if obj.get("stage").map_or(true, |v| v.is_null()) {
                obj.insert("stage".to_string(), serde_json::Value::String("prospect".to_string()));
            }
            if obj.get("source").map_or(true, |v| v.is_null()) {
                obj.insert("source".to_string(), serde_json::Value::String("manual".to_string()));
            }
        }
        assert_eq!(val["stage"], "prospect");
        assert_eq!(val["source"], "manual");
    }

    #[test]
    fn create_company_preserves_explicit_stage() {
        let data = CreateCompany {
            name: "Test".to_string(),
            display_name: None,
            industry: None,
            website: None,
            stage: Some("client".to_string()),
            source: Some("apollo".to_string()),
            notes: None,
            tags: None,
        };
        let mut val = serde_json::to_value(&data).unwrap();
        if let Some(obj) = val.as_object_mut() {
            if obj.get("stage").map_or(true, |v| v.is_null()) {
                obj.insert("stage".to_string(), serde_json::Value::String("prospect".to_string()));
            }
        }
        assert_eq!(val["stage"], "client");
        assert_eq!(val["source"], "apollo");
    }
}
