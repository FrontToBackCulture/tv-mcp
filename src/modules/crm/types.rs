// CRM Module Types
// Data structures for companies, contacts, and activities

use serde::{Deserialize, Serialize};

// ============================================================================
// Companies
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>, // prospect | opportunity | client | churned | partner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>, // apollo | inbound | referral | manual | existing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referred_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub employee_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_revenue: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    // Nested data (from joins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<Contact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<Activity>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompany {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCompany {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research_folder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outreach_status: Option<String>,
}

// ============================================================================
// Contacts
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seniority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connect_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_dm_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_outreach_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edm_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    // Nested data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<Box<Company>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connect_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_dm_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_outreach_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edm_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prospect_type_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connect_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_dm_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_outreach_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edm_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_status: Option<String>,
}

// ============================================================================
// Activities
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub company_id: Option<String>,
    #[serde(rename = "type")]
    pub activity_type: String, // email | note | meeting | call | task | stage_change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActivity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(rename = "type")]
    pub activity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateActivity {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

// ============================================================================
// Email Links
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailCompanyLink {
    pub id: String,
    pub email_id: String,
    pub company_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    pub match_type: String, // contact_email | domain | manual
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    // Nested data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<Box<Company>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEmailRequest {
    pub email_id: String,
    pub company_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    pub match_type: String,
}

// Pipeline Stats live in work/types.rs (deals are projects now)
