// VAL Sync Auth - JWT token management and VAL platform login
// Tokens cached in ~/.tv-desktop/val-tokens.json

use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::load_settings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub domain: String,
    pub authenticated: bool,
    pub token_preview: Option<String>,
    pub expires_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    data: Option<LoginData>,
}

#[derive(Debug, Deserialize)]
struct LoginData {
    #[serde(default)]
    user: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwtPayload {
    #[serde(default)]
    exp: Option<u64>,
}

// ============================================================================
// Internal helpers
// ============================================================================

fn get_tokens_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tv-desktop")
        .join("val-tokens.json")
}

fn load_tokens() -> CmdResult<HashMap<String, String>> {
    let path = get_tokens_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_tokens(tokens: &HashMap<String, String>) -> CmdResult<()> {
    let path = get_tokens_path();
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }
    let content = serde_json::to_string_pretty(tokens)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Decode JWT payload (base64) and check expiration
fn is_token_valid(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }

    // Decode payload (second part)
    let payload_b64 = parts[1];
    // JWT uses base64url encoding — add padding if needed
    let padded = match payload_b64.len() % 4 {
        2 => format!("{}==", payload_b64),
        3 => format!("{}=", payload_b64),
        _ => payload_b64.to_string(),
    };
    let padded = padded.replace('-', "+").replace('_', "/");

    let decoded = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &padded,
    ) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let payload: JwtPayload = match serde_json::from_slice(&decoded) {
        Ok(p) => p,
        Err(_) => return false,
    };

    if let Some(exp) = payload.exp {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Allow 60s buffer
        exp > now + 60
    } else {
        // No expiration claim — assume valid
        true
    }
}

/// Extract expiration time from JWT as ISO string
fn get_token_expiry(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let payload_b64 = parts[1];
    let padded = match payload_b64.len() % 4 {
        2 => format!("{}==", payload_b64),
        3 => format!("{}=", payload_b64),
        _ => payload_b64.to_string(),
    };
    let padded = padded.replace('-', "+").replace('_', "/");

    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &padded,
    )
    .ok()?;

    let payload: JwtPayload = serde_json::from_slice(&decoded).ok()?;
    payload.exp.map(|exp| {
        chrono::DateTime::from_timestamp(exp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_else(|| format!("epoch:{}", exp))
    })
}

fn token_preview(token: &str) -> String {
    if token.len() <= 20 {
        "***".to_string()
    } else {
        format!("{}...{}", &token[..10], &token[token.len() - 6..])
    }
}

/// Get per-domain credentials from settings.json
fn get_domain_credentials(domain: &str) -> CmdResult<(String, String)> {
    let settings = load_settings()?;
    let email_key = format!("val_email_{}", domain);
    let password_key = format!("val_password_{}", domain);

    let email = settings
        .keys
        .get(&email_key)
        .cloned()
        .ok_or_else(|| CommandError::Config(format!("No email configured for domain '{}'. Set key '{}'", domain, email_key)))?;
    let password = settings
        .keys
        .get(&password_key)
        .cloned()
        .ok_or_else(|| CommandError::Config(format!("No password configured for domain '{}'. Set key '{}'", domain, password_key)))?;

    Ok((email, password))
}

/// Login to VAL and return JWT token
async fn login_to_val(api_domain: &str, email: &str, password: &str) -> CmdResult<String> {
    let url = format!("https://{}.thinkval.io/api/v1/users/login", api_domain);

    let client = crate::HTTP_CLIENT.clone();

    let body = serde_json::json!({
        "email": email,
        "password": password,
        "rememberMe": false,
        "loginID": email
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http { status: status.as_u16(), body: format!("Login failed for {}: {}", api_domain, text) });
    }

    let data: LoginResponse = response.json().await?;

    // Token can be at response.data.user or response.user
    let token = data
        .data
        .and_then(|d| d.user)
        .or(data.user)
        .ok_or_else(|| CommandError::Internal(format!("No token in login response for {}", api_domain)))?;

    Ok(token)
}

/// Ensure we have a valid token for a domain, logging in if needed.
/// Returns (token, api_domain).
pub async fn ensure_auth(domain: &str) -> CmdResult<(String, String)> {
    let domain_config = super::config::get_domain_config(domain)?;
    let api_domain = domain_config.api_domain().to_string();

    // Check cached token
    let tokens = load_tokens()?;
    if let Some(token) = tokens.get(domain) {
        if is_token_valid(token) {
            return Ok((token.clone(), api_domain));
        }
    }

    // Need to login — get credentials from settings
    let (email, password) = get_domain_credentials(domain)?;
    let token = login_to_val(&api_domain, &email, &password).await?;

    // Cache the token
    let mut tokens = load_tokens()?;
    tokens.insert(domain.to_string(), token.clone());
    save_tokens(&tokens)?;

    Ok((token, api_domain))
}

/// Same as ensure_auth but clears token first and retries login.
/// Used after auth errors (401/403).
pub async fn reauth(domain: &str) -> CmdResult<(String, String)> {
    // Clear existing token
    let mut tokens = load_tokens()?;
    tokens.remove(domain);
    save_tokens(&tokens)?;

    // Re-login
    ensure_auth(domain).await
}

// ============================================================================
// Commands
// ============================================================================

/// Login to a VAL domain using stored credentials

pub async fn val_sync_login(domain: String) -> CmdResult<AuthResult> {
    match ensure_auth(&domain).await {
        Ok((token, _api_domain)) => Ok(AuthResult {
            domain: domain.clone(),
            authenticated: true,
            token_preview: Some(token_preview(&token)),
            expires_at: get_token_expiry(&token),
            message: format!("Authenticated to {}", domain),
        }),
        Err(e) => Ok(AuthResult {
            domain,
            authenticated: false,
            token_preview: None,
            expires_at: None,
            message: e.to_string(),
        }),
    }
}

/// Login with explicitly provided credentials (for first-time setup / testing)

pub async fn val_sync_login_with_credentials(
    domain: String,
    email: String,
    password: String,
) -> CmdResult<AuthResult> {
    let domain_config = super::config::get_domain_config(&domain)?;
    let api_domain = domain_config.api_domain().to_string();

    match login_to_val(&api_domain, &email, &password).await {
        Ok(token) => {
            // Cache the token
            let mut tokens = load_tokens()?;
            tokens.insert(domain.clone(), token.clone());
            save_tokens(&tokens)?;

            Ok(AuthResult {
                domain: domain.clone(),
                authenticated: true,
                token_preview: Some(token_preview(&token)),
                expires_at: get_token_expiry(&token),
                message: format!("Authenticated to {} with provided credentials", domain),
            })
        }
        Err(e) => Ok(AuthResult {
            domain,
            authenticated: false,
            token_preview: None,
            expires_at: None,
            message: e.to_string(),
        }),
    }
}

/// Check auth status for a domain (does not login)

pub fn val_sync_check_auth(domain: String) -> CmdResult<AuthResult> {
    let tokens = load_tokens()?;
    match tokens.get(&domain) {
        Some(token) => {
            let valid = is_token_valid(token);
            Ok(AuthResult {
                domain: domain.clone(),
                authenticated: valid,
                token_preview: Some(token_preview(token)),
                expires_at: get_token_expiry(token),
                message: if valid {
                    format!("Valid token for {}", domain)
                } else {
                    format!("Expired token for {}", domain)
                },
            })
        }
        None => Ok(AuthResult {
            domain: domain.clone(),
            authenticated: false,
            token_preview: None,
            expires_at: None,
            message: format!("No token cached for {}", domain),
        }),
    }
}

/// Clear cached token for a domain

pub fn val_sync_clear_token(domain: String) -> CmdResult<()> {
    let mut tokens = load_tokens()?;
    tokens.remove(&domain);
    save_tokens(&tokens)
}
