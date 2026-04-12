// CRM Module - Contact Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

/// List contacts with optional filters

pub async fn crm_list_contacts(
    company_id: Option<String>,
    search: Option<String>,
) -> CmdResult<Vec<Contact>> {
    let client = get_client().await?;

    let mut filters = vec!["order=is_primary.desc,name.asc".to_string()];

    if let Some(cid) = company_id {
        filters.push(format!("company_id=eq.{}", cid));
    }
    if let Some(s) = search {
        filters.push(format!("or=(name.ilike.*{}*,email.ilike.*{}*)", s, s));
    }

    let query = filters.join("&");
    client.select("crm_contacts", &query).await
}

/// Find contact by email

pub async fn crm_find_contact(email: String) -> CmdResult<Option<Contact>> {
    let client = get_client().await?;

    // Email is stored lowercase
    let query = format!("email=eq.{}", email.to_lowercase());
    client.select_single("crm_contacts", &query).await
}

/// Get a single contact by ID

pub async fn crm_get_contact(contact_id: String) -> CmdResult<Contact> {
    let client = get_client().await?;

    let query = format!("select=*,company:crm_companies(*)&id=eq.{}", contact_id);

    client
        .select_single("crm_contacts", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("Contact not found: {}", contact_id)))
}

/// Create a new contact

pub async fn crm_create_contact(data: CreateContact) -> CmdResult<Contact> {
    let client = get_client().await?;

    // Normalize email to lowercase
    let mut insert_data = serde_json::to_value(&data)?;
    if let Some(obj) = insert_data.as_object_mut() {
        if let Some(email) = obj.get("email").and_then(|e| e.as_str()) {
            obj.insert("email".to_string(), serde_json::Value::String(email.to_lowercase()));
        }
        // Set defaults
        if obj.get("is_primary").map_or(true, |v| v.is_null()) {
            obj.insert("is_primary".to_string(), serde_json::Value::Bool(false));
        }
        if obj.get("is_active").map_or(true, |v| v.is_null()) {
            obj.insert("is_active".to_string(), serde_json::Value::Bool(true));
        }
    }

    client.insert("crm_contacts", &insert_data).await
}

/// Update a contact

pub async fn crm_update_contact(contact_id: String, data: UpdateContact) -> CmdResult<Contact> {
    let client = get_client().await?;

    // Normalize email if present
    let mut update_data = serde_json::to_value(&data)?;
    if let Some(obj) = update_data.as_object_mut() {
        if let Some(email) = obj.get("email").and_then(|e| e.as_str()) {
            obj.insert("email".to_string(), serde_json::Value::String(email.to_lowercase()));
        }
    }

    let query = format!("id=eq.{}", contact_id);
    client.update("crm_contacts", &query, &update_data).await
}

/// Delete a contact

pub async fn crm_delete_contact(contact_id: String) -> CmdResult<()> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", contact_id);
    client.delete("crm_contacts", &query).await
}
