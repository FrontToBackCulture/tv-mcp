// Apollo.io API client
// Wraps the Apollo REST API for search and enrichment

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{load_settings, KEY_APOLLO_API};

const APOLLO_BASE: &str = "https://api.apollo.io/api/v1";

pub struct ApolloClient {
    client: reqwest::Client,
    api_key: String,
}

impl ApolloClient {
    pub fn new() -> CmdResult<Self> {
        let settings = load_settings()?;
        let api_key = settings
            .keys
            .get(KEY_APOLLO_API)
            .cloned()
            .ok_or_else(|| CommandError::Config("Apollo API key not configured".into()))?;

        Ok(Self {
            client: crate::HTTP_CLIENT.clone(),
            api_key,
        })
    }

    /// Search people — free, no credits consumed.
    /// Returns names, titles, companies but NOT emails/phones (need enrichment for that).
    pub async fn search_people(
        &self,
        filters: &ApolloSearchFilters,
    ) -> CmdResult<ApolloSearchResponse> {
        let url = format!("{}/mixed_people/api_search", APOLLO_BASE);

        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(filters)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CommandError::Http { status, body });
        }

        let text = response.text().await.unwrap_or_default();
        let data: ApolloSearchResponse = serde_json::from_str(&text)
            .map_err(|e| {
                // Log first 2000 chars of response for debugging
                let preview = if text.len() > 2000 { &text[..2000] } else { &text };
                CommandError::Parse(format!("Failed to parse Apollo search response: {} — Response preview: {}", e, preview))
            })?;

        Ok(data)
    }

    /// Enrich a person — costs credits.
    /// Returns full contact details including email, phone, etc.
    pub async fn enrich_person(
        &self,
        person_id: &str,
    ) -> CmdResult<ApolloEnrichResponse> {
        let url = format!("{}/people/match", APOLLO_BASE);

        let body = serde_json::json!({
            "id": person_id,
        });

        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CommandError::Http { status, body });
        }

        let data: ApolloEnrichResponse = response
            .json()
            .await
            .map_err(|e| CommandError::Parse(format!("Failed to parse Apollo enrich response: {}", e)))?;

        Ok(data)
    }

    /// Request phone number reveal — costs 1 mobile credit.
    /// Phone is delivered asynchronously via webhook.
    pub async fn reveal_phone(
        &self,
        person_id: &str,
        webhook_url: &str,
    ) -> CmdResult<()> {
        let url = format!("{}/people/match", APOLLO_BASE);

        let body = serde_json::json!({
            "id": person_id,
            "reveal_phone_number": true,
            "webhook_url": webhook_url,
        });

        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CommandError::Http { status, body });
        }

        // Phone will be delivered asynchronously via webhook
        Ok(())
    }
}
