// Supabase REST API Client
// Generic client for making authenticated requests to Supabase

use crate::core::error::{CmdResult, CommandError};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Serialize};

/// Supabase client for making REST API requests
pub struct SupabaseClient {
    base_url: String,
    anon_key: String,
    /// Optional JWT token for authenticated access (overrides anon_key in Authorization header)
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl SupabaseClient {
    /// Create a new Supabase client
    pub fn new(url: &str, anon_key: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            anon_key: anon_key.to_string(),
            auth_token: None,
            client: crate::HTTP_CLIENT.clone(),
        }
    }

    /// Create a new Supabase client with a JWT auth token
    pub fn new_with_token(url: &str, anon_key: &str, auth_token: &str) -> Self {
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            anon_key: anon_key.to_string(),
            auth_token: Some(auth_token.to_string()),
            client: crate::HTTP_CLIENT.clone(),
        }
    }

    /// Get the base URL for this client
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the underlying HTTP client for direct requests
    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Get authentication headers for external calls (edge functions, etc.)
    pub fn auth_headers(&self) -> HeaderMap {
        self.headers()
    }

    /// Build headers for Supabase requests
    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        // apikey header always uses anon_key (required by Supabase)
        if let Ok(val) = HeaderValue::from_str(&self.anon_key) {
            headers.insert("apikey", val);
        }
        // Authorization header uses JWT if available, otherwise anon_key
        let bearer = match &self.auth_token {
            Some(token) => format!("Bearer {}", token),
            None => format!("Bearer {}", self.anon_key),
        };
        if let Ok(val) = HeaderValue::from_str(&bearer) {
            headers.insert(AUTHORIZATION, val);
        }
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("Prefer", HeaderValue::from_static("return=representation"));
        headers
    }

    /// Check response status and return typed error if not success
    async fn check_response(&self, response: reqwest::Response) -> CmdResult<reqwest::Response> {
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CommandError::Http { status, body });
        }
        Ok(response)
    }

    /// GET request - select from table
    pub async fn select<T: DeserializeOwned>(
        &self,
        table: &str,
        query: &str,
    ) -> CmdResult<Vec<T>> {
        let url = if query.is_empty() {
            format!("{}/rest/v1/{}", self.base_url, table)
        } else {
            format!("{}/rest/v1/{}?{}", self.base_url, table, query)
        };

        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let response = self.check_response(response).await?;
        Ok(response.json().await?)
    }

    /// GET single row
    pub async fn select_single<T: DeserializeOwned>(
        &self,
        table: &str,
        query: &str,
    ) -> CmdResult<Option<T>> {
        let results: Vec<T> = self.select(table, query).await?;
        Ok(results.into_iter().next())
    }

    /// POST request - insert into table
    pub async fn insert<T: Serialize, R: DeserializeOwned>(
        &self,
        table: &str,
        data: &T,
    ) -> CmdResult<R> {
        let url = format!("{}/rest/v1/{}", self.base_url, table);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(data)
            .send()
            .await?;

        let response = self.check_response(response).await?;

        let results: Vec<R> = response.json().await?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| CommandError::Internal("No data returned from insert".into()))
    }

    /// PATCH request - update rows
    pub async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        table: &str,
        query: &str,
        data: &T,
    ) -> CmdResult<R> {
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, query);

        let response = self
            .client
            .patch(&url)
            .headers(self.headers())
            .json(data)
            .send()
            .await?;

        let response = self.check_response(response).await?;

        let results: Vec<R> = response.json().await?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| CommandError::Internal("No data returned from update".into()))
    }

    /// POST request with upsert - insert or update on conflict
    #[allow(dead_code)]
    pub async fn upsert<T: Serialize, R: DeserializeOwned>(
        &self,
        table: &str,
        data: &T,
    ) -> CmdResult<R> {
        self.upsert_on(table, data, None).await
    }

    /// POST request with upsert on a specific conflict column
    pub async fn upsert_on<T: Serialize, R: DeserializeOwned>(
        &self,
        table: &str,
        data: &T,
        on_conflict: Option<&str>,
    ) -> CmdResult<R> {
        let mut url = format!("{}/rest/v1/{}", self.base_url, table);
        if let Some(col) = on_conflict {
            url.push_str(&format!("?on_conflict={}", col));
        }

        let mut headers = self.headers();
        // Override Prefer header for upsert
        headers.insert("Prefer", reqwest::header::HeaderValue::from_static("return=representation,resolution=merge-duplicates"));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(data)
            .send()
            .await?;

        let response = self.check_response(response).await?;

        let results: Vec<R> = response.json().await?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| CommandError::Internal("No data returned from upsert".into()))
    }

    /// DELETE request - delete rows
    pub async fn delete(&self, table: &str, query: &str) -> CmdResult<()> {
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, query);

        let response = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?;

        self.check_response(response).await?;
        Ok(())
    }

    /// RPC call - call a database function
    #[allow(dead_code)]
    pub async fn rpc<T: Serialize, R: DeserializeOwned>(
        &self,
        function: &str,
        params: &T,
    ) -> CmdResult<R> {
        let url = format!("{}/rest/v1/rpc/{}", self.base_url, function);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(params)
            .send()
            .await?;

        let response = self.check_response(response).await?;
        let body_text = response.text().await.map_err(|e| {
            eprintln!("[supabase:rpc] Failed to read response body for '{}': {}", function, e);
            CommandError::Internal(format!("Failed to read RPC response: {}", e))
        })?;
        serde_json::from_str(&body_text).map_err(|e| {
            eprintln!("[supabase:rpc] Failed to decode response for '{}': {} — body: {}", function, e, &body_text[..body_text.len().min(500)]);
            CommandError::Internal(format!("error decoding response body: {}", e))
        })
    }
}

#[allow(dead_code)]
/// Create a SupabaseClient from explicit URL and key (for testing and direct usage)
pub fn client_from(url: &str, anon_key: &str) -> SupabaseClient {
    SupabaseClient::new(url, anon_key)
}

// ── Bot JWT Authentication ──────────────────────────────────────────────────
// When TV_BOT_API_KEY is set, tv-mcp authenticates with the gateway on startup
// and uses a scoped JWT for all Supabase queries instead of the anon key.

use std::sync::OnceLock;

/// Cached bot JWT token (minted on first use, refreshed when expired)
static BOT_JWT: OnceLock<std::sync::Mutex<Option<BotToken>>> = OnceLock::new();

#[derive(Clone)]
struct BotToken {
    token: String,
    expires_at: u64,
}

/// Gateway URL for bot authentication
const GATEWAY_URL: &str = "https://tccyronrnsimacqfhxzd.supabase.co";

/// Authenticate with the gateway using the bot API key and get a workspace JWT.
async fn mint_bot_jwt(api_key: &str) -> CmdResult<BotToken> {
    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post(format!("{}/functions/v1/bot-token", GATEWAY_URL))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "api_key": api_key }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status, body });
    }

    let data: serde_json::Value = response.json().await?;
    let token = data["token"].as_str()
        .ok_or_else(|| CommandError::Internal("No token in bot-token response".into()))?
        .to_string();
    let expires_at = data["expires_at"].as_u64()
        .ok_or_else(|| CommandError::Internal("No expires_at in bot-token response".into()))?;

    let bot_name = data["bot"].get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
    log::info!("Bot JWT minted for {} (expires in ~1h)", bot_name);

    Ok(BotToken { token, expires_at })
}

/// Get the bot JWT, minting or refreshing if needed.
async fn get_bot_jwt(api_key: &str) -> CmdResult<String> {
    let mutex = BOT_JWT.get_or_init(|| std::sync::Mutex::new(None));
    let existing = {
        let guard = mutex.lock().unwrap();
        guard.clone()
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Refresh if expired or expiring within 5 minutes
    if let Some(ref bt) = existing {
        if bt.expires_at > now + 300 {
            return Ok(bt.token.clone());
        }
    }

    // Mint new JWT
    let new_token = mint_bot_jwt(api_key).await?;
    let token = new_token.token.clone();
    {
        let mut guard = mutex.lock().unwrap();
        *guard = Some(new_token);
    }
    Ok(token)
}

tokio::task_local! {
    /// Task-local override that pins `get_client()` to a specific workspace's
    /// credentials for the scope of one async task. Set via
    /// `WORKSPACE_OVERRIDE.scope(Some(ws_id), async { ... })` by the
    /// background sync loops so each per-workspace iteration reads the
    /// correct Supabase project without refactoring every internal
    /// `get_client()` call in the sync code.
    pub static WORKSPACE_OVERRIDE: Option<String>;
}

/// Helper to get Supabase client from settings.
///
/// Resolution order:
///   1. If `WORKSPACE_OVERRIDE` task-local is set, use workspace-scoped keys
///      (`ws:{id}:supabase_url` + `ws:{id}:supabase_anon_key`). This is how
///      bg sync loops route each per-workspace iteration to the correct
///      Supabase project.
///   2. Otherwise, read the global (unscoped) keys — legacy behavior.
///
/// If TV_BOT_API_KEY is set, uses an authenticated JWT instead of anon key.
pub async fn get_client() -> CmdResult<SupabaseClient> {
    use crate::core::settings::{
        get_workspace_setting, settings_get_key, KEY_SUPABASE_ANON_KEY, KEY_SUPABASE_URL,
    };

    // Check task-local workspace override first
    let ws_override = WORKSPACE_OVERRIDE
        .try_with(|w| w.clone())
        .ok()
        .flatten();

    let (url, anon_key) = if let Some(workspace_id) = ws_override {
        let url = get_workspace_setting(&workspace_id, KEY_SUPABASE_URL).ok_or_else(|| {
            CommandError::Config(format!(
                "Supabase URL not configured for workspace {}",
                workspace_id
            ))
        })?;
        let anon_key = get_workspace_setting(&workspace_id, KEY_SUPABASE_ANON_KEY).ok_or_else(
            || {
                CommandError::Config(format!(
                    "Supabase anon key not configured for workspace {}",
                    workspace_id
                ))
            },
        )?;
        (url, anon_key)
    } else {
        let url = settings_get_key(KEY_SUPABASE_URL.to_string())?.ok_or_else(|| {
            CommandError::Config("Supabase URL not configured. Go to Settings to add it.".into())
        })?;
        let anon_key = settings_get_key(KEY_SUPABASE_ANON_KEY.to_string())?.ok_or_else(|| {
            CommandError::Config(
                "Supabase anon key not configured. Go to Settings to add it.".into(),
            )
        })?;
        (url, anon_key)
    };

    // If bot API key is set, authenticate and use JWT
    if let Ok(api_key) = std::env::var("TV_BOT_API_KEY") {
        if !api_key.is_empty() {
            let jwt = get_bot_jwt(&api_key).await?;
            return Ok(SupabaseClient::new_with_token(&url, &anon_key, &jwt));
        }
    }

    Ok(SupabaseClient::new(&url, &anon_key))
}

/// Explicit workspace-scoped client fetcher — same logic as `get_client()`
/// but takes the workspace ID directly rather than reading from the
/// task-local override. Useful for code paths that know their workspace at
/// call time and don't want to wrap in `WORKSPACE_OVERRIDE.scope(...)`.
#[allow(dead_code)]
pub async fn get_client_for_workspace(workspace_id: &str) -> CmdResult<SupabaseClient> {
    use crate::core::settings::{get_workspace_setting, KEY_SUPABASE_ANON_KEY, KEY_SUPABASE_URL};

    let url = get_workspace_setting(workspace_id, KEY_SUPABASE_URL).ok_or_else(|| {
        CommandError::Config(format!(
            "Supabase URL not configured for workspace {}",
            workspace_id
        ))
    })?;
    let anon_key = get_workspace_setting(workspace_id, KEY_SUPABASE_ANON_KEY).ok_or_else(|| {
        CommandError::Config(format!(
            "Supabase anon key not configured for workspace {}",
            workspace_id
        ))
    })?;

    if let Ok(api_key) = std::env::var("TV_BOT_API_KEY") {
        if !api_key.is_empty() {
            let jwt = get_bot_jwt(&api_key).await?;
            return Ok(SupabaseClient::new_with_token(&url, &anon_key, &jwt));
        }
    }

    Ok(SupabaseClient::new(&url, &anon_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param, header};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestRow {
        id: String,
        name: String,
    }

    async fn setup() -> (MockServer, SupabaseClient) {
        let server = MockServer::start().await;
        let client = SupabaseClient::new(&server.uri(), "test-key");
        (server, client)
    }

    // -------------------------------------------------------
    // Headers
    // -------------------------------------------------------

    #[tokio::test]
    async fn select_sends_correct_auth_headers() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .and(path("/rest/v1/items"))
            .and(header("apikey", "test-key"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .expect(1)
            .mount(&server)
            .await;

        let result: Vec<TestRow> = client.select("items", "").await.unwrap();
        assert!(result.is_empty());
    }

    // -------------------------------------------------------
    // SELECT
    // -------------------------------------------------------

    #[tokio::test]
    async fn select_returns_parsed_rows() {
        let (server, client) = setup().await;

        let body = json!([
            {"id": "1", "name": "Alice"},
            {"id": "2", "name": "Bob"}
        ]);

        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let rows: Vec<TestRow> = client.select("users", "").await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Alice");
        assert_eq!(rows[1].name, "Bob");
    }

    #[tokio::test]
    async fn select_with_query_appends_params() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .and(query_param("stage", "eq.client"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .expect(1)
            .mount(&server)
            .await;

        let _: Vec<TestRow> = client.select("users", "stage=eq.client").await.unwrap();
    }

    #[tokio::test]
    async fn select_empty_query_has_no_question_mark() {
        let (server, client) = setup().await;

        // Path must match exactly (no trailing ?)
        Mock::given(method("GET"))
            .and(path("/rest/v1/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let _: Vec<TestRow> = client.select("items", "").await.unwrap();
    }

    #[tokio::test]
    async fn select_single_returns_first_row() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([{"id": "1", "name": "Only"}])),
            )
            .mount(&server)
            .await;

        let row: Option<TestRow> = client.select_single("users", "id=eq.1").await.unwrap();
        assert_eq!(row.unwrap().name, "Only");
    }

    #[tokio::test]
    async fn select_single_returns_none_for_empty() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let row: Option<TestRow> = client.select_single("users", "id=eq.999").await.unwrap();
        assert!(row.is_none());
    }

    // -------------------------------------------------------
    // INSERT
    // -------------------------------------------------------

    #[tokio::test]
    async fn insert_sends_post_and_returns_result() {
        let (server, client) = setup().await;

        Mock::given(method("POST"))
            .and(path("/rest/v1/users"))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!([{"id": "new-1", "name": "Charlie"}])),
            )
            .expect(1)
            .mount(&server)
            .await;

        let data = json!({"name": "Charlie"});
        let result: TestRow = client.insert("users", &data).await.unwrap();
        assert_eq!(result.id, "new-1");
        assert_eq!(result.name, "Charlie");
    }

    #[tokio::test]
    async fn insert_errors_on_empty_response() {
        let (server, client) = setup().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!([])))
            .mount(&server)
            .await;

        let data = json!({"name": "Ghost"});
        let result: Result<TestRow, _> = client.insert("users", &data).await;
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // UPDATE
    // -------------------------------------------------------

    #[tokio::test]
    async fn update_sends_patch() {
        let (server, client) = setup().await;

        Mock::given(method("PATCH"))
            .and(path("/rest/v1/users"))
            .and(query_param("id", "eq.1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([{"id": "1", "name": "Updated"}])),
            )
            .expect(1)
            .mount(&server)
            .await;

        let data = json!({"name": "Updated"});
        let result: TestRow = client.update("users", "id=eq.1", &data).await.unwrap();
        assert_eq!(result.name, "Updated");
    }

    // -------------------------------------------------------
    // DELETE
    // -------------------------------------------------------

    #[tokio::test]
    async fn delete_sends_delete_request() {
        let (server, client) = setup().await;

        Mock::given(method("DELETE"))
            .and(path("/rest/v1/users"))
            .and(query_param("id", "eq.1"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        client.delete("users", "id=eq.1").await.unwrap();
    }

    // -------------------------------------------------------
    // UPSERT
    // -------------------------------------------------------

    #[tokio::test]
    async fn upsert_on_includes_conflict_param() {
        let (server, client) = setup().await;

        Mock::given(method("POST"))
            .and(path("/rest/v1/users"))
            .and(query_param("on_conflict", "email"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([{"id": "1", "name": "Upserted"}])),
            )
            .expect(1)
            .mount(&server)
            .await;

        let data = json!({"name": "Upserted", "email": "a@b.com"});
        let result: TestRow = client.upsert_on("users", &data, Some("email")).await.unwrap();
        assert_eq!(result.name, "Upserted");
    }

    // -------------------------------------------------------
    // RPC
    // -------------------------------------------------------

    #[tokio::test]
    async fn rpc_calls_function_endpoint() {
        let (server, client) = setup().await;

        Mock::given(method("POST"))
            .and(path("/rest/v1/rpc/my_function"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": 42})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let result: serde_json::Value = client.rpc("my_function", &json!({"x": 1})).await.unwrap();
        assert_eq!(result["result"], 42);
    }

    // -------------------------------------------------------
    // Error handling
    // -------------------------------------------------------

    #[tokio::test]
    async fn http_404_returns_command_error() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let result: Result<Vec<TestRow>, _> = client.select("missing", "").await;
        match result {
            Err(CommandError::Http { status, body }) => {
                assert_eq!(status, 404);
                assert_eq!(body, "not found");
            }
            other => panic!("Expected Http error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn http_500_returns_command_error() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&server)
            .await;

        let result: Result<Vec<TestRow>, _> = client.select("broken", "").await;
        assert!(matches!(result, Err(CommandError::Http { status: 500, .. })));
    }

    #[tokio::test]
    async fn malformed_json_returns_error() {
        let (server, client) = setup().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let result: Result<Vec<TestRow>, _> = client.select("bad", "").await;
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // client_from helper
    // -------------------------------------------------------

    #[test]
    fn client_from_creates_client() {
        let client = client_from("https://example.supabase.co", "key123");
        assert_eq!(client.base_url, "https://example.supabase.co");
        assert_eq!(client.anon_key, "key123");
    }

    #[test]
    fn client_strips_trailing_slash() {
        let client = SupabaseClient::new("https://example.supabase.co/", "key");
        assert_eq!(client.base_url, "https://example.supabase.co");
    }
}
