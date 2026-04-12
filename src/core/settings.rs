// Settings module - Simple JSON file storage for API keys and app settings
// Stores settings in ~/.tv-desktop/settings.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::core::error::CmdResult;

// Known API key names
pub const KEY_GAMMA_API: &str = "gamma_api_key";
pub const KEY_GEMINI_API: &str = "gemini_api_key";
pub const KEY_GITHUB_CLIENT_ID: &str = "github_client_id";
pub const KEY_GITHUB_CLIENT_SECRET: &str = "github_client_secret";
pub const KEY_SUPABASE_URL: &str = "supabase_url";
pub const KEY_SUPABASE_ANON_KEY: &str = "supabase_anon_key";
pub const KEY_OPENAI_API: &str = "openai_api_key";
pub const KEY_INTERCOM_API: &str = "intercom_api_key";
pub const KEY_MS_GRAPH_CLIENT_ID: &str = "ms_graph_client_id";
pub const KEY_MS_GRAPH_TENANT_ID: &str = "ms_graph_tenant_id";
pub const KEY_MS_GRAPH_CLIENT_SECRET: &str = "ms_graph_client_secret";
pub const KEY_ANTHROPIC_API: &str = "anthropic_api_key";
pub const KEY_AWS_ACCESS_KEY_ID: &str = "aws_access_key_id";
pub const KEY_AWS_SECRET_ACCESS_KEY: &str = "aws_secret_access_key";
pub const KEY_GA4_CLIENT_ID: &str = "ga4_client_id";
pub const KEY_GA4_CLIENT_SECRET: &str = "ga4_client_secret";
pub const KEY_GA4_PROPERTY_ID: &str = "ga4_property_id";
pub const KEY_GA4_WEBSITE_PROPERTY_ID: &str = "ga4_website_property_id";
pub const KEY_EMAIL_API_BASE_URL: &str = "email_api_base_url";
pub const KEY_NOTION_API: &str = "notion_api_key";
pub const KEY_NOTION_DEFAULT_DB: &str = "notion_default_database";
pub const KEY_KNOWLEDGE_PATH: &str = "knowledge_path";
pub const KEY_APOLLO_API: &str = "apollo_api_key";
pub const KEY_LINKEDIN_CLIENT_ID: &str = "linkedin_client_id";
pub const KEY_LINKEDIN_CLIENT_SECRET: &str = "linkedin_client_secret";
pub const KEY_OPENROUTER_API: &str = "openrouter_api_key";

// Interactive Brokers Flex Web Service — personal-workspace-only (Melly).
// Prefixed with the workspace slug so credentials are segregated at the
// settings-file level: other workspaces physically cannot see these keys
// because nothing in the UI writes or reads the `melly_*` namespace.
pub const KEY_IBKR_FLEX_TOKEN: &str = "melly_ibkr_flex_token";
pub const KEY_IBKR_FLEX_QUERY_POSITIONS: &str = "melly_ibkr_flex_query_positions";
pub const KEY_IBKR_FLEX_QUERY_TRADES: &str = "melly_ibkr_flex_query_trades";
pub const KEY_IBKR_FLEX_QUERY_CASH: &str = "melly_ibkr_flex_query_cash";

// Financial Modeling Prep — also personal-workspace-only.
pub const KEY_FMP_API_KEY: &str = "melly_fmp_api_key";

// Background sync toggle keys (default: not set = disabled)
pub const KEY_BG_SYNC_OUTLOOK_EMAIL: &str = "bg_sync_outlook_email";
pub const KEY_BG_SYNC_OUTLOOK_CALENDAR: &str = "bg_sync_outlook_calendar";
pub const KEY_BG_SYNC_NOTION: &str = "bg_sync_notion";
pub const KEY_BG_SYNC_PUBLIC_DATA: &str = "bg_sync_public_data";

/// Key where the list of registered workspace IDs is stored (JSON array of
/// strings). Populated by `settings_register_workspace` — Rust background
/// sync loops iterate over this list so each workspace's bg syncs run
/// against its own Supabase project rather than whichever workspace happened
/// to write global settings last.
pub const KEY_REGISTERED_WORKSPACES: &str = "registered_workspaces";

/// Produce a workspace-scoped settings key. Matches the format emitted by
/// the frontend's `settings_register_workspace` call — keep these two in
/// lock-step when adding new per-workspace settings.
pub fn scoped_key(workspace_id: &str, key: &str) -> String {
    format!("ws:{}:{}", workspace_id, key)
}

/// Read a setting scoped to a specific workspace, with automatic fallback to
/// the global (unscoped) key for backward compat. Returns None if neither is
/// set or is empty.
pub fn get_workspace_setting(workspace_id: &str, key: &str) -> Option<String> {
    let settings = load_settings().ok()?;
    let scoped = scoped_key(workspace_id, key);
    if let Some(v) = settings.keys.get(&scoped) {
        if !v.is_empty() {
            return Some(v.clone());
        }
    }
    settings.keys.get(key).cloned().filter(|v| !v.is_empty())
}

/// Return the list of workspace IDs that have been registered with the
/// Rust side via `settings_register_workspace`. Background sync loops use
/// this to iterate per-workspace. Returns an empty Vec if none registered.
pub fn get_registered_workspaces() -> Vec<String> {
    let settings = match load_settings() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let raw = match settings.keys.get(KEY_REGISTERED_WORKSPACES) {
        Some(s) => s,
        None => return Vec::new(),
    };
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

/// Check if a background sync is enabled (reads settings, returns false if key missing or != "true")
pub fn is_bg_sync_enabled(key: &str) -> bool {
    load_settings()
        .ok()
        .and_then(|s| s.keys.get(key).cloned())
        .map(|v| v == "true")
        .unwrap_or(false)
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub name: String,
    pub description: String,
    pub is_set: bool,
    pub masked_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsStatus {
    pub gamma_api_key: bool,
    pub gemini_api_key: bool,
    pub github_client_id: bool,
    pub github_client_secret: bool,
    pub supabase_url: bool,
    pub supabase_anon_key: bool,
    pub ms_graph_client_id: bool,
    pub ms_graph_tenant_id: bool,
    pub ms_graph_client_secret: bool,
    pub anthropic_api_key: bool,
}

// ============================================================================
// Internal helpers
// ============================================================================

fn get_settings_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let tv_mcp_dir = home.join(".tv-mcp");
    if tv_mcp_dir.join("settings.json").exists() {
        return tv_mcp_dir;
    }
    // Fall back to tv-desktop settings for backward compat
    let tv_desktop_dir = home.join(".tv-desktop");
    if tv_desktop_dir.join("settings.json").exists() {
        return tv_desktop_dir;
    }
    // Default to .tv-mcp for new installs
    tv_mcp_dir
}

fn get_settings_path() -> PathBuf {
    get_settings_dir().join("settings.json")
}

pub fn load_settings() -> CmdResult<Settings> {
    let path = get_settings_path();
    if !path.exists() {
        return Ok(Settings::default());
    }
    let content = fs::read_to_string(&path)?;
    let mut settings: Settings = serde_json::from_str(&content)?;

    // One-time migration: Mumbai → Singapore Supabase (April 2026)
    let old_url = "https://sabrnwuhgkqfwunbrnrt.supabase.co";
    let new_url = "https://cqwcaeffzanfqsxlspig.supabase.co";
    let new_anon_key = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImNxd2NhZWZmemFuZnFzeGxzcGlnIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzUxMzE2MzIsImV4cCI6MjA5MDcwNzYzMn0.4UjeZdVjB7z-_sTWP6BRqHINkpTxA6jhP6ZabvKQC_0";

    if settings.keys.get(KEY_SUPABASE_URL).map(|v| v.as_str()) == Some(old_url) {
        settings.keys.insert(KEY_SUPABASE_URL.to_string(), new_url.to_string());
        settings.keys.insert(KEY_SUPABASE_ANON_KEY.to_string(), new_anon_key.to_string());
        // Persist so this only runs once
        let _ = save_settings(&settings);
    }

    Ok(settings)
}

fn save_settings(settings: &Settings) -> CmdResult<()> {
    let dir = get_settings_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = get_settings_path();
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

// ============================================================================
// Commands - Generic key operations
// ============================================================================

/// Set an API key

pub fn settings_set_key(key_name: String, value: String) -> CmdResult<()> {
    let mut settings = load_settings()?;
    settings.keys.insert(key_name, value);
    save_settings(&settings)
}

/// Get an API key

pub fn settings_get_key(key_name: String) -> CmdResult<Option<String>> {
    let settings = load_settings()?;
    Ok(settings.keys.get(&key_name).cloned())
}

/// Delete an API key

pub fn settings_delete_key(key_name: String) -> CmdResult<()> {
    let mut settings = load_settings()?;
    settings.keys.remove(&key_name);
    save_settings(&settings)
}

/// Check if an API key exists

pub fn settings_has_key(key_name: String) -> CmdResult<bool> {
    let settings = load_settings()?;
    Ok(settings.keys.contains_key(&key_name))
}

/// Get masked value of an API key (for display)

pub fn settings_get_masked_key(key_name: String) -> CmdResult<Option<String>> {
    let settings = load_settings()?;
    Ok(settings.keys.get(&key_name).map(|v| mask_key(v)))
}

// ============================================================================
// Commands - Convenience methods for specific keys
// ============================================================================

/// Get status of all known API keys

pub fn settings_get_status() -> CmdResult<SettingsStatus> {
    let settings = load_settings()?;
    Ok(SettingsStatus {
        gamma_api_key: settings.keys.contains_key(KEY_GAMMA_API),
        gemini_api_key: settings.keys.contains_key(KEY_GEMINI_API),
        github_client_id: settings.keys.contains_key(KEY_GITHUB_CLIENT_ID),
        github_client_secret: settings.keys.contains_key(KEY_GITHUB_CLIENT_SECRET),
        supabase_url: settings.keys.contains_key(KEY_SUPABASE_URL),
        supabase_anon_key: settings.keys.contains_key(KEY_SUPABASE_ANON_KEY),
        ms_graph_client_id: settings.keys.contains_key(KEY_MS_GRAPH_CLIENT_ID),
        ms_graph_tenant_id: settings.keys.contains_key(KEY_MS_GRAPH_TENANT_ID),
        ms_graph_client_secret: settings.keys.contains_key(KEY_MS_GRAPH_CLIENT_SECRET),
        anthropic_api_key: settings.keys.contains_key(KEY_ANTHROPIC_API),
    })
}

/// Get all API key info (for settings UI)

pub fn settings_list_keys() -> CmdResult<Vec<ApiKeyInfo>> {
    let settings = load_settings()?;

    let keys = vec![
        (KEY_GAMMA_API, "Gamma API Key", "For generating presentations"),
        (KEY_GEMINI_API, "Gemini API Key", "For image generation (Nanobanana)"),
        (KEY_GITHUB_CLIENT_ID, "GitHub Client ID", "For OAuth login"),
        (KEY_GITHUB_CLIENT_SECRET, "GitHub Client Secret", "For OAuth login"),
        (KEY_SUPABASE_URL, "Supabase URL", "Database connection"),
        (KEY_SUPABASE_ANON_KEY, "Supabase Anon Key", "Database authentication"),
        (KEY_OPENAI_API, "OpenAI API Key", "For AI features"),
        (KEY_INTERCOM_API, "Intercom API Key", "For Help Center publishing"),
        (KEY_MS_GRAPH_CLIENT_ID, "MS Graph Client ID", "For Outlook email integration"),
        (KEY_MS_GRAPH_TENANT_ID, "MS Graph Tenant ID", "For Outlook email integration"),
        (KEY_MS_GRAPH_CLIENT_SECRET, "MS Graph Client Secret", "For Outlook email integration"),
        (KEY_ANTHROPIC_API, "Anthropic API Key", "For AI email summaries"),
        (KEY_AWS_ACCESS_KEY_ID, "AWS Access Key ID", "For S3 AI publish"),
        (KEY_AWS_SECRET_ACCESS_KEY, "AWS Secret Access Key", "For S3 AI publish"),
        (KEY_GA4_CLIENT_ID, "GA4 Client ID", "Google OAuth2 Client ID for Analytics"),
        (KEY_GA4_CLIENT_SECRET, "GA4 Client Secret", "Google OAuth2 Client Secret for Analytics"),
        (KEY_GA4_PROPERTY_ID, "GA4 Property ID", "GA4 numeric property ID for VAL platform analytics"),
        (KEY_GA4_WEBSITE_PROPERTY_ID, "GA4 Website Property ID", "GA4 numeric property ID for website analytics"),
        (KEY_EMAIL_API_BASE_URL, "Email API Base URL", "Tracking endpoint URL for email open/click/unsubscribe (e.g. https://your-domain.ngrok-free.dev)"),
        (KEY_NOTION_API, "Notion API Key", "For syncing Notion databases to Work Module"),
        (KEY_NOTION_DEFAULT_DB, "Notion Default Database", "Default Notion database ID for pushing tasks without a sync config"),
        (KEY_APOLLO_API, "Apollo API Key", "For prospect search and enrichment"),
        (KEY_LINKEDIN_CLIENT_ID, "LinkedIn Client ID", "For LinkedIn social media integration"),
        (KEY_LINKEDIN_CLIENT_SECRET, "LinkedIn Client Secret", "For LinkedIn social media integration"),
        (KEY_IBKR_FLEX_TOKEN, "IBKR Flex Token", "Flex Web Service token from IBKR Account Management (Melly workspace only)"),
        (KEY_IBKR_FLEX_QUERY_POSITIONS, "IBKR Flex Query — Positions", "Query ID for the daily positions snapshot Flex query"),
        (KEY_IBKR_FLEX_QUERY_TRADES, "IBKR Flex Query — Trades", "Query ID for the executed trades Flex query"),
        (KEY_IBKR_FLEX_QUERY_CASH, "IBKR Flex Query — Cash Activity", "Query ID for the cash transactions / dividends Flex query"),
        (KEY_FMP_API_KEY, "FMP API Key", "Financial Modeling Prep API key for fundamentals, ratios, market data (Melly workspace only)"),
    ];

    let mut result = Vec::new();
    for (name, display_name, description) in keys {
        let value = settings.keys.get(name);
        let is_set = value.is_some();
        let masked_value = value.map(|v| mask_key(v));

        result.push(ApiKeyInfo {
            name: name.to_string(),
            description: format!("{} - {}", display_name, description),
            is_set,
            masked_value,
        });
    }

    Ok(result)
}

// ============================================================================
// Commands - Tool-specific getters (for internal use)
// ============================================================================

/// Get Gamma API key (for gamma commands)

pub fn settings_get_gamma_key() -> CmdResult<Option<String>> {
    settings_get_key(KEY_GAMMA_API.to_string())
}

/// Get Gemini API key (for nanobanana commands)

pub fn settings_get_gemini_key() -> CmdResult<Option<String>> {
    settings_get_key(KEY_GEMINI_API.to_string())
}

/// Get Intercom API key (for help center publishing)

pub fn settings_get_intercom_key() -> CmdResult<Option<String>> {
    settings_get_key(KEY_INTERCOM_API.to_string())
}

/// Get GitHub credentials (for auth)

pub fn settings_get_github_credentials() -> CmdResult<(Option<String>, Option<String>)> {
    let settings = load_settings()?;
    let client_id = settings.keys.get(KEY_GITHUB_CLIENT_ID).cloned();
    let client_secret = settings.keys.get(KEY_GITHUB_CLIENT_SECRET).cloned();
    Ok((client_id, client_secret))
}

/// Get Supabase credentials

pub fn settings_get_supabase_credentials() -> CmdResult<(Option<String>, Option<String>)> {
    let settings = load_settings()?;
    let url = settings.keys.get(KEY_SUPABASE_URL).cloned();
    let anon_key = settings.keys.get(KEY_SUPABASE_ANON_KEY).cloned();
    Ok((url, anon_key))
}

/// Get MS Graph credentials (for Outlook)

pub fn settings_get_ms_graph_credentials() -> CmdResult<(Option<String>, Option<String>, Option<String>)> {
    let settings = load_settings()?;
    let client_id = settings.keys.get(KEY_MS_GRAPH_CLIENT_ID).cloned();
    let tenant_id = settings.keys.get(KEY_MS_GRAPH_TENANT_ID).cloned();
    let client_secret = settings.keys.get(KEY_MS_GRAPH_CLIENT_SECRET).cloned();
    Ok((client_id, tenant_id, client_secret))
}

/// Get Anthropic API key (for AI summaries)

pub fn settings_get_anthropic_key() -> CmdResult<Option<String>> {
    settings_get_key(KEY_ANTHROPIC_API.to_string())
}

/// Get AWS credentials (for S3 sync)

pub fn settings_get_aws_credentials() -> CmdResult<(Option<String>, Option<String>)> {
    let settings = load_settings()?;
    let access_key = settings.keys.get(KEY_AWS_ACCESS_KEY_ID).cloned();
    let secret_key = settings.keys.get(KEY_AWS_SECRET_ACCESS_KEY).cloned();
    Ok((access_key, secret_key))
}

/// Get the settings file path (for importing)

pub fn settings_get_path() -> String {
    get_settings_path().to_string_lossy().to_string()
}

// ============================================================================
// Commands - VAL Sync credentials
// ============================================================================

/// Get credentials for a specific VAL domain
/// Keys: val_email_{domain}, val_password_{domain}

pub fn settings_get_val_credentials(
    domain: String,
) -> CmdResult<(Option<String>, Option<String>)> {
    let settings = load_settings()?;
    let email = settings
        .keys
        .get(&format!("val_email_{}", domain))
        .cloned();
    let password = settings
        .keys
        .get(&format!("val_password_{}", domain))
        .cloned();
    Ok((email, password))
}

/// Import credentials from val-sync .env file
/// Parses VAL_DOMAIN_{DOMAIN}_EMAIL/PASSWORD entries

pub fn settings_import_val_credentials(env_file_path: String) -> CmdResult<Vec<String>> {
    let content = fs::read_to_string(&env_file_path)?;

    let mut settings = load_settings()?;
    let mut imported = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let eq_pos = match line.find('=') {
            Some(p) => p,
            None => continue,
        };
        let key = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'');

        if value.is_empty() {
            continue;
        }

        // Match VAL_DOMAIN_{DOMAIN}_EMAIL or VAL_DOMAIN_{DOMAIN}_PASSWORD
        if let Some(rest) = key.strip_prefix("VAL_DOMAIN_") {
            if let Some(domain_upper) = rest.strip_suffix("_EMAIL") {
                let domain = domain_upper.to_lowercase().replace('_', "-");
                let settings_key = format!("val_email_{}", domain);
                settings.keys.insert(settings_key.clone(), value.to_string());
                imported.push(format!("{} -> {}", key, settings_key));
            } else if let Some(domain_upper) = rest.strip_suffix("_PASSWORD") {
                let domain = domain_upper.to_lowercase().replace('_', "-");
                let settings_key = format!("val_password_{}", domain);
                settings.keys.insert(settings_key.clone(), value.to_string());
                imported.push(format!("{} -> {}", key, settings_key));
            }
        }
    }

    save_settings(&settings)?;
    Ok(imported)
}

// ============================================================================
// Commands - Generic import
// ============================================================================

/// Import settings from a JSON file or env-style file

pub fn settings_import_from_file(file_path: String) -> CmdResult<Vec<String>> {
    let content = fs::read_to_string(&file_path)?;

    let mut imported = Vec::new();
    let mut settings = load_settings()?;

    // Try JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        // Handle nested structure like { "keys": { ... } }
        if let Some(keys) = json.get("keys").and_then(|k| k.as_object()) {
            for (k, v) in keys {
                if let Some(val) = v.as_str() {
                    settings.keys.insert(k.clone(), val.to_string());
                    imported.push(k.clone());
                }
            }
        }
        // Handle flat structure
        else if let Some(obj) = json.as_object() {
            for (k, v) in obj {
                if let Some(val) = v.as_str() {
                    settings.keys.insert(k.clone(), val.to_string());
                    imported.push(k.clone());
                }
            }
        }
    } else {
        // Parse as .env file
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'');

                // Map env var names to our key names
                let mapped_key = match key {
                    "GAMMA_API_KEY" => Some(KEY_GAMMA_API),
                    "GEMINI_API_KEY" => Some(KEY_GEMINI_API),
                    "GITHUB_CLIENT_ID" => Some(KEY_GITHUB_CLIENT_ID),
                    "GITHUB_CLIENT_SECRET" => Some(KEY_GITHUB_CLIENT_SECRET),
                    "SUPABASE_URL" | "NEXT_PUBLIC_SUPABASE_URL" => Some(KEY_SUPABASE_URL),
                    "SUPABASE_ANON_KEY" | "NEXT_PUBLIC_SUPABASE_ANON_KEY" => Some(KEY_SUPABASE_ANON_KEY),
                    "OPENAI_API_KEY" => Some(KEY_OPENAI_API),
                    "INTERCOM_ACCESS_TOKEN" | "INTERCOM_API_KEY" => Some(KEY_INTERCOM_API),
                    "MS_GRAPH_CLIENT_ID" | "AZURE_CLIENT_ID" => Some(KEY_MS_GRAPH_CLIENT_ID),
                    "MS_GRAPH_TENANT_ID" | "AZURE_TENANT_ID" => Some(KEY_MS_GRAPH_TENANT_ID),
                    "MS_GRAPH_CLIENT_SECRET" | "AZURE_CLIENT_SECRET" => Some(KEY_MS_GRAPH_CLIENT_SECRET),
                    "ANTHROPIC_API_KEY" => Some(KEY_ANTHROPIC_API),
                    "OPENROUTER_API_KEY" => Some(KEY_OPENROUTER_API),
                    "AWS_ACCESS_KEY_ID" => Some(KEY_AWS_ACCESS_KEY_ID),
                    "AWS_SECRET_ACCESS_KEY" => Some(KEY_AWS_SECRET_ACCESS_KEY),
                    "LINKEDIN_CLIENT_ID" => Some(KEY_LINKEDIN_CLIENT_ID),
                    "LINKEDIN_CLIENT_SECRET" => Some(KEY_LINKEDIN_CLIENT_SECRET),
                    _ => None,
                };

                if let Some(mapped) = mapped_key {
                    if !value.is_empty() {
                        settings.keys.insert(mapped.to_string(), value.to_string());
                        imported.push(format!("{} -> {}", key, mapped));
                    }
                }
            }
        }
    }

    save_settings(&settings)?;
    Ok(imported)
}

/// Atomically write multiple settings keys at once.
/// Used by workspace switching to update supabase_url + supabase_anon_key (and
/// any workspace-specific API keys) in a single disk write, preventing a race
/// where a Tauri command fires between writing the URL and the key.

pub fn settings_switch_workspace(keys: HashMap<String, String>) -> CmdResult<usize> {
    let mut settings = load_settings()?;
    let count = keys.len();
    for (k, v) in keys {
        settings.keys.insert(k, v);
    }
    save_settings(&settings)?;
    Ok(count)
}

/// Register a workspace's credentials under workspace-scoped keys
/// (`ws:{workspace_id}:{key}`) and add the workspace to the
/// `registered_workspaces` list. Also writes the keys un-scoped so legacy
/// code paths that don't know about workspaces keep working — the last
/// window to switch still "wins" for legacy consumers, but bg sync loops
/// read scoped keys per workspace and are unaffected.
///
/// Caller provides un-prefixed keys; this function handles scoping. For
/// example, pass `{"supabase_url": "...", "supabase_anon_key": "..."}` and
/// they'll land as `ws:{id}:supabase_url` + `ws:{id}:supabase_anon_key`.

pub fn settings_register_workspace(
    workspace_id: String,
    keys: HashMap<String, String>,
) -> CmdResult<usize> {
    let mut settings = load_settings()?;
    let count = keys.len();

    // Write scoped + global copies of each provided key
    for (k, v) in &keys {
        settings.keys.insert(scoped_key(&workspace_id, k), v.clone());
        settings.keys.insert(k.clone(), v.clone());
    }

    // Maintain the list of registered workspace IDs (unique, order preserved)
    let mut registered: Vec<String> = settings
        .keys
        .get(KEY_REGISTERED_WORKSPACES)
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default();
    if !registered.contains(&workspace_id) {
        registered.push(workspace_id.clone());
        settings.keys.insert(
            KEY_REGISTERED_WORKSPACES.to_string(),
            serde_json::to_string(&registered)?,
        );
    }

    save_settings(&settings)?;
    Ok(count)
}

/// Return the list of registered workspace IDs (used by background sync
/// loops to iterate per-workspace).

pub fn settings_list_registered_workspaces() -> CmdResult<Vec<String>> {
    Ok(get_registered_workspaces())
}

/// Export all settings to a JSON file

pub fn settings_export_to_file(file_path: String) -> CmdResult<usize> {
    let settings = load_settings()?;
    let count = settings.keys.len();
    let content = serde_json::to_string_pretty(&settings)?;
    fs::write(&file_path, content)?;
    Ok(count)
}
