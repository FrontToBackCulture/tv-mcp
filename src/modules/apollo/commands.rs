// Apollo Tauri commands
// Search, enrich, and import prospects from Apollo.io

use super::api::ApolloClient;
use super::types::*;
use crate::modules::crm::types::{Company, Contact};
use crate::core::error::CmdResult;
use crate::core::supabase::get_client;

/// Search Apollo for people matching filters.
/// Free — does not consume credits.

pub async fn apollo_search_people(
    filters: ApolloSearchFilters,
) -> CmdResult<ApolloSearchResponse> {
    let client = ApolloClient::new()?;
    client.search_people(&filters).await
}

/// Enrich a person by Apollo ID — costs credits.
/// Returns full contact details (email, phone, etc).

pub async fn apollo_enrich_person(person_id: String) -> CmdResult<ApolloEnrichResponse> {
    let client = ApolloClient::new()?;
    client.enrich_person(&person_id).await
}

/// Request phone number reveal for a contact — costs 1 mobile credit.
/// Requires the contact to have a source_id (Apollo person ID).
/// Phone is delivered asynchronously via webhook to tv-api.

pub async fn apollo_reveal_phone(
    contact_id: String,
) -> CmdResult<String> {
    let apollo = ApolloClient::new()?;
    let db = get_client().await?;

    // Get the contact's Apollo source_id
    let contacts: Vec<Contact> = db
        .select("crm_contacts", &format!("id=eq.{}&limit=1", contact_id))
        .await?;

    let contact = contacts.first()
        .ok_or_else(|| crate::core::error::CommandError::NotFound("Contact not found".into()))?;

    let source_id = contact.source_id.as_ref()
        .ok_or_else(|| crate::core::error::CommandError::Config(
            "Contact has no Apollo ID — import from Apollo first".into()
        ))?;

    // Get webhook URL from settings (email_api_base_url + /webhooks/apollo/phone)
    let settings = crate::core::settings::load_settings()?;
    let api_base = settings.keys
        .get(crate::core::settings::KEY_EMAIL_API_BASE_URL)
        .ok_or_else(|| crate::core::error::CommandError::Config(
            "Email API base URL not configured — needed for Apollo webhook".into()
        ))?;

    let webhook_url = format!("{}/webhooks/apollo/phone", api_base.trim_end_matches('/'));

    apollo.reveal_phone(source_id, &webhook_url).await?;

    Ok(format!("Phone reveal requested for {}. It will appear shortly.", contact.name))
}

/// Check which Apollo people already exist in CRM contacts.
/// Matches by source_id OR by first_name + company name combo.
/// Returns matches with company IDs so the UI can link to the CRM record.

pub async fn apollo_check_existing(
    people: Vec<ApolloCheckPerson>,
) -> CmdResult<Vec<ApolloExistingMatch>> {
    let db = get_client().await?;
    let mut matches = Vec::new();

    // Load all contacts and companies once
    let all_contacts: Vec<Contact> = db
        .select("crm_contacts", "limit=5000")
        .await
        .unwrap_or_default();

    let all_companies: Vec<Company> = db
        .select("crm_companies", "limit=5000")
        .await
        .unwrap_or_default();

    // Map company_id -> (company_name_lower, company_id)
    let company_names: std::collections::HashMap<&str, String> = all_companies
        .iter()
        .map(|c| {
            let name = c.display_name.as_deref().unwrap_or(&c.name).to_lowercase();
            (c.id.as_str(), name)
        })
        .collect();

    // Build contact lookup: source_id -> (contact_name, company_id)
    let mut source_id_map: std::collections::HashMap<&str, (&str, &str)> =
        std::collections::HashMap::new();
    for c in &all_contacts {
        if let (Some(ref sid), Some(ref cid)) = (&c.source_id, &c.company_id) {
            source_id_map.insert(sid.as_str(), (&c.name, cid.as_str()));
        }
    }

    // Extract first name from CRM contact — handles "John Smith" and "john@company.com"
    fn extract_first_name(name: &str) -> Option<String> {
        let name = name.trim().to_lowercase();
        if name.contains('@') {
            // Email as name — take part before @
            name.split('@').next().map(|s| s.to_string())
        } else {
            // Normal name — take first word
            name.split_whitespace().next().map(|s| s.to_string())
        }
    }

    // Build list of (first_name_lower, company_name_lower, contact_name, company_id)
    let contact_entries: Vec<(String, String, String, String)> = all_contacts
        .iter()
        .filter_map(|c| {
            let first_name = extract_first_name(&c.name)?;
            let cid = c.company_id.as_deref()?;
            let company_name = company_names.get(cid)?;
            Some((first_name, company_name.clone(), c.name.clone(), cid.to_string()))
        })
        .collect();

    for person in &people {
        // 1. Match by source_id
        if let Some(&(contact_name, company_id)) = source_id_map.get(person.id.as_str()) {
            matches.push(ApolloExistingMatch {
                apollo_id: person.id.clone(),
                company_id: company_id.to_string(),
                contact_name: contact_name.to_string(),
            });
            continue;
        }

        // 2. Match by first_name + organization name (fuzzy)
        if let Some(ref first_name) = person.first_name {
            if let Some(ref org_name) = person.organization_name {
                let apollo_first = first_name.to_lowercase();
                let apollo_org = org_name.to_lowercase();
                let found = contact_entries.iter().find(|(crm_first, crm_org, _, _)| {
                    crm_first == &apollo_first
                        && (crm_org.contains(&apollo_org)
                            || apollo_org.contains(crm_org.as_str()))
                });
                if let Some((_, _, contact_name, company_id)) = found {
                    matches.push(ApolloExistingMatch {
                        apollo_id: person.id.clone(),
                        company_id: company_id.clone(),
                        contact_name: contact_name.clone(),
                    });
                    continue;
                }
            }
        }
    }

    Ok(matches)
}

/// Import selected Apollo prospects into CRM.
/// Enriches each person first (costs credits), then creates companies and contacts
/// with full data (names, emails, phones). Skips duplicates by source_id or email.

pub async fn apollo_import_prospects(
    request: ApolloImportRequest,
) -> CmdResult<ApolloImportResult> {
    let apollo = ApolloClient::new()?;
    let db = get_client().await?;
    let mut result = ApolloImportResult {
        companies_created: 0,
        companies_existing: 0,
        contacts_created: 0,
        contacts_existing: 0,
        enriched: 0,
        enrich_failed: 0,
        errors: vec![],
    };

    // Step 1: Enrich each person to get full data
    let mut enriched_people: Vec<ApolloPerson> = Vec::new();

    for person in &request.people {
        // Check if contact already exists by source_id before enriching
        let existing: Vec<Contact> = db
            .select("crm_contacts", &format!("source_id=eq.{}&limit=1", person.id))
            .await
            .unwrap_or_default();
        if !existing.is_empty() {
            result.contacts_existing += 1;
            continue;
        }

        match apollo.enrich_person(&person.id).await {
            Ok(enriched) => {
                result.enriched += 1;
                enriched_people.push(enriched.person);
            }
            Err(e) => {
                result.enrich_failed += 1;
                result.errors.push(format!(
                    "Failed to enrich {}: {}",
                    person.first_name.as_deref().unwrap_or(&person.id),
                    e
                ));
            }
        }
    }

    // Step 2: Group enriched people by organization
    let mut org_people: std::collections::HashMap<String, Vec<&ApolloPerson>> =
        std::collections::HashMap::new();

    for person in &enriched_people {
        let org_id = person
            .organization_id
            .clone()
            .or_else(|| person.organization.as_ref().and_then(|o| o.id.clone()))
            .unwrap_or_else(|| format!("unknown_{}", person.id));
        org_people.entry(org_id).or_default().push(person);
    }

    // Step 3: Create companies and contacts
    for (org_id, people) in &org_people {
        let org = people.first().and_then(|p| p.organization.as_ref());

        // Check if company already exists — try source_id, then name, then website domain
        let existing_company: Option<Company> = {
            // 1. Match by Apollo org ID
            let by_source: Vec<Company> = db
                .select("crm_companies", &format!("source_id=eq.{}&limit=1", org_id))
                .await
                .unwrap_or_default();

            if let Some(c) = by_source.into_iter().next() {
                Some(c)
            } else if let Some(name) = org.and_then(|o| o.name.as_deref()) {
                // 2. Match by company name (case-insensitive, contains either way)
                // Try exact first, then partial match
                let by_name: Vec<Company> = db
                    .select("crm_companies", &format!("or=(name.ilike.*{}*,display_name.ilike.*{}*)&limit=1", name, name))
                    .await
                    .unwrap_or_default();

                if let Some(c) = by_name.into_iter().next() {
                    Some(c)
                } else if let Some(domain) = org.and_then(|o| o.primary_domain.as_deref()).or_else(|| org.and_then(|o| o.website_url.as_deref())) {
                    // 3. Match by website domain
                    let by_website: Vec<Company> = db
                        .select("crm_companies", &format!("website=ilike.*{}*&limit=1", domain))
                        .await
                        .unwrap_or_default();
                    by_website.into_iter().next()
                } else {
                    None
                }
            } else {
                None
            }
        };

        let company_id = if let Some(existing) = existing_company {
            result.companies_existing += 1;
            // Update source_id if missing (so future imports match faster)
            if existing.source_id.is_none() {
                let _ = db.update::<_, Company>(
                    "crm_companies",
                    &format!("id=eq.{}", existing.id),
                    &serde_json::json!({ "source_id": org_id, "source": "apollo" }),
                ).await;
            }
            existing.id
        } else {
            let org_name = org
                .and_then(|o| o.name.clone())
                .unwrap_or_else(|| "Unknown Company".to_string());

            let mut create_data = serde_json::json!({
                "name": org_name,
                "stage": "prospect",
                "source": "apollo",
                "source_id": org_id,
            });

            if let Some(o) = org {
                if let Some(ref website) = o.website_url {
                    create_data["website"] = serde_json::Value::String(website.clone());
                }
                if let Some(ref industry) = o.industry {
                    create_data["industry"] = serde_json::Value::String(industry.clone());
                }
                if let Some(emp) = o.estimated_num_employees {
                    create_data["employee_count"] = serde_json::json!(emp);
                }
                if let Some(rev) = o.annual_revenue {
                    create_data["annual_revenue"] = serde_json::json!(rev);
                }
            }

            if let Some(ref tags) = request.tags {
                create_data["tags"] = serde_json::json!(tags);
            }

            match db.insert::<_, Company>("crm_companies", &create_data).await {
                Ok(company) => {
                    result.companies_created += 1;
                    company.id
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Failed to create company {}: {}",
                        org.and_then(|o| o.name.as_deref()).unwrap_or("unknown"),
                        e
                    ));
                    continue;
                }
            }
        };

        // Create or update contacts
        for (i, person) in people.iter().enumerate() {
            // Check by email if available — update with Apollo ID if found
            if let Some(ref email) = person.email {
                if !email.is_empty() {
                    let existing: Vec<Contact> = db
                        .select("crm_contacts", &format!("email=eq.{}&limit=1", email))
                        .await
                        .unwrap_or_default();
                    if let Some(existing_contact) = existing.first() {
                        // Backfill source_id and source on existing contact
                        if existing_contact.source_id.is_none() {
                            let _ = db.update::<_, Contact>(
                                "crm_contacts",
                                &format!("id=eq.{}", existing_contact.id),
                                &serde_json::json!({ "source_id": person.id, "source": "apollo" }),
                            ).await;
                        }
                        result.contacts_existing += 1;
                        continue;
                    }
                }
            }

            let name = person
                .name
                .clone()
                .or_else(|| {
                    match (&person.first_name, &person.last_name) {
                        (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
                        (Some(f), None) => Some(f.clone()),
                        (None, Some(l)) => Some(l.clone()),
                        _ => None,
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());

            let email = person
                .email
                .clone()
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| format!("unknown_{}@apollo.import", person.id));

            let mut create_data = serde_json::json!({
                "company_id": company_id,
                "name": name,
                "email": email,
                "source": "apollo",
                "source_id": person.id,
                "is_primary": i == 0,
                "prospect_stage": "new",
            });

            if let Some(ref title) = person.title {
                create_data["role"] = serde_json::Value::String(title.clone());
            }
            if let Some(ref email_status) = person.email_status {
                create_data["email_status"] = serde_json::Value::String(email_status.clone());
            }
            if let Some(ref linkedin) = person.linkedin_url {
                create_data["linkedin_url"] = serde_json::Value::String(linkedin.clone());
            }
            if let Some(ref seniority) = person.seniority {
                create_data["seniority"] = serde_json::Value::String(seniority.clone());
            }
            if let Some(ref departments) = person.departments {
                if let Some(dept) = departments.first() {
                    create_data["department"] = serde_json::Value::String(dept.clone());
                }
            }
            if let Some(ref phones) = person.phone_numbers {
                if let Some(phone) = phones.first() {
                    let number = phone
                        .sanitized_number
                        .as_ref()
                        .or(phone.raw_number.as_ref());
                    if let Some(num) = number {
                        create_data["phone"] = serde_json::Value::String(num.clone());
                    }
                }
            }

            match db.insert::<_, Contact>("crm_contacts", &create_data).await {
                Ok(_) => {
                    result.contacts_created += 1;
                }
                Err(e) => {
                    result.errors.push(format!("Failed to create contact {}: {}", name, e));
                }
            }
        }
    }

    Ok(result)
}
