// Apollo API types
// Maps Apollo.io REST API response structures

use serde::{Deserialize, Serialize};

// ============================================================================
// Search Request
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_titles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_locations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_seniorities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_locations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_num_employees_ranges: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q_organization_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q_keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_email_status: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_departments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_industry_tag_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i32>,
}

// ============================================================================
// Search Response (Basic plan — fields are minimal/obfuscated)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloSearchResponse {
    #[serde(default)]
    pub people: Vec<ApolloPerson>,
    #[serde(default)]
    pub total_entries: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloPerson {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name_obfuscated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seniority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub departments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_numbers: Option<Vec<ApolloPhone>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<ApolloOrganization>,
    // Basic plan boolean indicators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_email: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_city: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_country: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_direct_phone: Option<serde_json::Value>, // Can be bool or string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refreshed_at: Option<String>,
    // Catch-all for unknown fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloPhone {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sanitized_number: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloOrganization {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_num_employees: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_revenue: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    // Basic plan boolean indicators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_industry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_phone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_city: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_country: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_zip_code: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_revenue: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_employee_count: Option<bool>,
    // Catch-all for unknown fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ============================================================================
// Enrichment Response
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloEnrichResponse {
    pub person: ApolloPerson,
}

// ============================================================================
// Existing Check Request
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloCheckPerson {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
}

/// Match result: Apollo person ID -> CRM company ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloExistingMatch {
    pub apollo_id: String,
    pub company_id: String,
    pub contact_name: String,
}

// ============================================================================
// Import Request (from frontend)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloImportRequest {
    pub people: Vec<ApolloPerson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApolloImportResult {
    pub companies_created: i32,
    pub companies_existing: i32,
    pub contacts_created: i32,
    pub contacts_existing: i32,
    pub enriched: i32,
    pub enrich_failed: i32,
    pub errors: Vec<String>,
}
