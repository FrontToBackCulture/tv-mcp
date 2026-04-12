// VAL Sync Config - Domain configuration management
// Stores domain configs in ~/.tv-desktop/val-sync-config.json

use crate::core::error::{CmdResult, CommandError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub solution: String,
    #[serde(default, rename = "useCase")]
    pub use_case: Option<String>,
    #[serde(default, rename = "configPath")]
    pub config_path: Option<String>,
    #[serde(default, rename = "metadataTypes")]
    pub metadata_types: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    pub domain: String,
    #[serde(default, rename = "actualDomain")]
    pub actual_domain: Option<String>,
    #[serde(default, rename = "globalPath")]
    pub global_path: String,
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
    #[serde(default, rename = "monitoringPath")]
    pub monitoring_path: Option<String>,
    #[serde(default, rename = "domainType")]
    pub domain_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValSyncConfig {
    pub domains: Vec<DomainConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSummary {
    pub domain: String,
    pub global_path: String,
    pub has_actual_domain: bool,
    pub domain_type: String,
    pub has_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDomain {
    pub domain: String,
    pub domain_type: String,
    pub global_path: String,
    pub has_metadata: bool,
    pub has_actual_domain: bool,
    /// ISO timestamp of the most recent sync operation
    pub last_sync: Option<String>,
    /// Count of total artifacts synced
    pub artifact_count: Option<u32>,
}

impl DomainConfig {
    /// Get the domain used for API calls (actualDomain if set, otherwise domain)
    pub fn api_domain(&self) -> &str {
        self.actual_domain.as_deref().unwrap_or(&self.domain)
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tv-desktop")
        .join("val-sync-config.json")
}

pub fn load_config_internal() -> CmdResult<ValSyncConfig> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(ValSyncConfig { domains: vec![] });
    }
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_config_internal(config: &ValSyncConfig) -> CmdResult<()> {
    let path = get_config_path();
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn get_domain_config(domain: &str) -> CmdResult<DomainConfig> {
    let config = load_config_internal()?;
    config
        .domains
        .into_iter()
        .find(|d| d.domain == domain)
        .ok_or_else(|| CommandError::NotFound(format!("Domain '{}' not found in val-sync config", domain)))
}

/// Resolve ${tv-knowledge} in paths.
/// Uses the explicitly passed path, or falls back to settings.json knowledge_path.
fn resolve_path_variable(path: &str, tv_knowledge_path: Option<&str>) -> String {
    if !path.contains("${tv-knowledge}") {
        return path.to_string();
    }

    let resolved = if let Some(tk_path) = tv_knowledge_path {
        tk_path.to_string()
    } else {
        // Read from settings.json
        crate::core::settings::load_settings()
            .ok()
            .and_then(|s| s.keys.get(crate::core::settings::KEY_KNOWLEDGE_PATH).cloned())
            .filter(|p| !p.is_empty())
            .unwrap_or_default()
    };

    path.replace("${tv-knowledge}", &resolved)
}

// ============================================================================
// Commands
// ============================================================================

/// Load val-sync configuration

pub fn val_sync_load_config() -> CmdResult<ValSyncConfig> {
    load_config_internal()
}

/// Save val-sync configuration

pub fn val_sync_save_config(config: ValSyncConfig) -> CmdResult<()> {
    save_config_internal(&config)
}

/// List all configured domains (summary)

pub fn val_sync_list_domains() -> CmdResult<Vec<DomainSummary>> {
    let config = load_config_internal()?;
    Ok(config
        .domains
        .iter()
        .map(|d| {
            DomainSummary {
                domain: d.domain.clone(),
                global_path: d.global_path.clone(),
                has_actual_domain: d.actual_domain.is_some(),
                domain_type: d.domain_type.clone().unwrap_or_default(),
                has_metadata: true, // sync metadata now lives in Supabase
            }
        })
        .collect())
}

/// Import config from val-sync config.json (with ${tv-knowledge} path resolution)

pub fn val_sync_import_config(
    file_path: String,
    tv_knowledge_path: Option<String>,
) -> CmdResult<ValSyncConfig> {
    let content = fs::read_to_string(&file_path)?;

    // Parse the val-sync config.json format
    let raw: serde_json::Value = serde_json::from_str(&content)?;

    let domains_array = raw
        .get("domains")
        .and_then(|d| d.as_array())
        .ok_or_else(|| CommandError::Parse("Config must have a 'domains' array".to_string()))?;

    let tk_path = tv_knowledge_path.as_deref();
    let mut domains = Vec::new();

    for domain_val in domains_array {
        let domain = domain_val
            .get("domain")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let actual_domain = domain_val
            .get("actualDomain")
            .and_then(|d| d.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let global_path_raw = domain_val
            .get("globalPath")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let global_path = resolve_path_variable(&global_path_raw, tk_path);

        let monitoring_path = domain_val
            .get("monitoringPath")
            .and_then(|d| d.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| resolve_path_variable(s, tk_path));

        let projects = if let Some(projs) = domain_val.get("projects").and_then(|p| p.as_array()) {
            projs
                .iter()
                .filter_map(|p| {
                    let solution = p.get("solution")?.as_str()?.to_string();
                    let use_case = p
                        .get("useCase")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());
                    let config_path = p
                        .get("configPath")
                        .and_then(|c| c.as_str())
                        .map(|s| resolve_path_variable(s, tk_path));
                    let metadata_types = p.get("metadataTypes").cloned().map(|mut mt| {
                        // Resolve paths in metadataTypes values
                        if let Some(obj) = mt.as_object_mut() {
                            for val in obj.values_mut() {
                                if let Some(s) = val.as_str() {
                                    *val = serde_json::Value::String(
                                        resolve_path_variable(s, tk_path),
                                    );
                                }
                            }
                        }
                        mt
                    });

                    Some(ProjectConfig {
                        solution,
                        use_case,
                        config_path,
                        metadata_types,
                    })
                })
                .collect()
        } else {
            vec![]
        };

        if !domain.is_empty() {
            domains.push(DomainConfig {
                domain,
                actual_domain,
                global_path,
                projects,
                monitoring_path,
                domain_type: None,
            });
        }
    }

    let config = ValSyncConfig { domains };
    save_config_internal(&config)?;
    Ok(config)
}

/// Discover domains from the file system at {repo}/0_Platform/domains/
/// Scans all subdirectories and fetches domain_type from Supabase domain_metadata table.

pub async fn val_sync_discover_domains(domains_path: String) -> CmdResult<Vec<DiscoveredDomain>> {
    let base = std::path::Path::new(&domains_path);
    if !base.exists() {
        return Err(CommandError::NotFound(format!("Domains path does not exist: {}", domains_path)));
    }

    // Load existing config to preserve actual_domain aliases and projects
    let existing_config = load_config_internal().unwrap_or(ValSyncConfig { domains: vec![] });
    let existing_map: std::collections::HashMap<String, DomainConfig> = existing_config
        .domains
        .into_iter()
        .map(|d| (d.domain.clone(), d))
        .collect();

    // Fetch domain_type tags from Supabase
    let type_map = fetch_domain_types().await;

    // Scan all subdirectories directly under domains_path
    let entries = fs::read_dir(base)?;
    let mut folder_domains: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let domain_name = entry.file_name().to_string_lossy().to_string();
            if domain_name.starts_with('.') {
                continue;
            }
            let global_path = entry.path().to_string_lossy().to_string();
            folder_domains.push((domain_name, global_path));
        }
    }
    folder_domains.sort_by(|a, b| a.0.cmp(&b.0));

    let mut discovered: Vec<DiscoveredDomain> = Vec::new();
    let mut new_domain_configs: Vec<DomainConfig> = Vec::new();

    for (domain_name, global_path) in folder_domains {
        let domain_type = type_map
            .get(&domain_name)
            .cloned()
            .unwrap_or_else(|| "production".to_string());
        let existing = existing_map.get(&domain_name);
        let has_actual_domain = existing.map_or(false, |d| d.actual_domain.is_some());
        let (last_sync, artifact_count) = super::metadata::read_sync_summary(&domain_name).await;

        discovered.push(DiscoveredDomain {
            domain: domain_name.clone(),
            domain_type: domain_type.clone(),
            global_path: global_path.clone(),
            has_metadata: last_sync.is_some(),
            has_actual_domain,
            last_sync,
            artifact_count,
        });

        if let Some(ex) = existing {
            let mut config = ex.clone();
            config.global_path = global_path;
            config.domain_type = Some(domain_type);
            new_domain_configs.push(config);
        } else {
            new_domain_configs.push(DomainConfig {
                domain: domain_name,
                actual_domain: None,
                global_path,
                projects: vec![],
                monitoring_path: None,
                domain_type: Some(domain_type),
            });
        }
    }

    // Save the updated config so existing auth/sync/extract commands work
    let config = ValSyncConfig {
        domains: new_domain_configs,
    };
    save_config_internal(&config)?;

    Ok(discovered)
}

/// Fetch domain_type mapping from Supabase domain_metadata table.
/// Returns empty map if Supabase is not configured (graceful degradation).
async fn fetch_domain_types() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    let client = match crate::core::supabase::get_client().await {
        Ok(c) => c,
        Err(_) => return map, // Supabase not configured — all domains default to "production"
    };

    #[derive(serde::Deserialize)]
    struct DomainMeta {
        domain: String,
        domain_type: String,
    }

    match client.select::<DomainMeta>("domain_metadata", "").await {
        Ok(rows) => {
            for row in rows {
                map.insert(row.domain, row.domain_type);
            }
        }
        Err(_) => {} // Table may not exist yet — graceful fallback
    }

    map
}

/// Update domain_type in Supabase domain_metadata table.
/// Validates against lookup_values where type = 'domain_type'.

pub async fn val_sync_update_domain_type(domain: String, domain_type: String) -> CmdResult<()> {
    let client = crate::core::supabase::get_client().await?;

    // Validate against lookup_values table
    #[derive(Deserialize)]
    struct LookupRow { value: String }
    let valid: Vec<LookupRow> = client
        .select("lookup_values", "type=eq.domain_type&select=value")
        .await
        .unwrap_or_default();
    let valid_values: Vec<&str> = valid.iter().map(|r| r.value.as_str()).collect();
    if !valid_values.is_empty() && !valid_values.contains(&domain_type.as_str()) {
        return Err(CommandError::Config(format!(
            "Invalid domain_type '{}'. Must be one of: {}",
            domain_type,
            valid_values.join(", ")
        )));
    }

    #[derive(Serialize)]
    struct Row { domain: String, domain_type: String }
    let row = Row { domain, domain_type };
    client
        .upsert_on::<_, serde_json::Value>("domain_metadata", &row, Some("domain"))
        .await?;

    Ok(())
}

// sync metadata functions removed — now uses Supabase via metadata::read_sync_summary()
