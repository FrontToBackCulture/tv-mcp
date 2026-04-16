// Diagnostics MCP Tool
// Self-reports version, install type, binary path, auth status, and settings

use crate::core::settings::{self, mask_key};
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "diagnostics".to_string(),
            description: "Report tv-mcp version, install type (standalone vs sidecar), binary path, auth status, and settings. Use this to verify the MCP server is correctly installed and configured.".to_string(),
            input_schema: InputSchema::empty(),
        },
    ]
}

pub async fn call(name: &str, _arguments: Value) -> ToolResult {
    match name {
        "diagnostics" => run_diagnostics().await,
        _ => ToolResult::error(format!("Unknown diagnostics tool: {}", name)),
    }
}

async fn run_diagnostics() -> ToolResult {
    let version = env!("CARGO_PKG_VERSION");

    let binary_path = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(&p).ok())
        .unwrap_or_else(|| PathBuf::from("unknown"));
    let binary_path_str = binary_path.display().to_string();

    let install_type = classify_install(&binary_path_str);

    // Trigger any pending settings migration before reading paths
    let auth_status = check_auth();

    let settings_dir = get_settings_info();

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let report = json!({
        "version": version,
        "binary_path": binary_path_str,
        "install_type": install_type.0,
        "install_type_detail": install_type.1,
        "platform": format!("{}/{}", os, arch),
        "settings_file": settings_dir.0,
        "settings_status": settings_dir.1,
        "auth_status": auth_status.0,
        "auth_detail": auth_status.1,
    });

    let mut lines = Vec::new();
    lines.push(format!("tv-mcp diagnostics"));
    lines.push(format!("─────────────────────────────────"));
    lines.push(format!("Version:        {}", version));
    lines.push(format!("Platform:       {}/{}", os, arch));
    lines.push(format!("Binary:         {}", binary_path_str));
    lines.push(format!("Install type:   {} ({})", install_type.0, install_type.1));
    lines.push(format!("Settings:       {} ({})", settings_dir.0, settings_dir.1));
    lines.push(format!("Auth:           {} ({})", auth_status.0, auth_status.1));
    lines.push(format!("─────────────────────────────────"));

    if install_type.0 == "legacy-sidecar" || install_type.0 == "legacy-tv-desktop" {
        lines.push(String::new());
        lines.push(format!("⚠️  You are running a legacy install."));
        lines.push(format!("Upgrade: download the latest release from"));
        lines.push(format!("https://github.com/FrontToBackCulture/tv-mcp/releases/latest"));
        lines.push(format!("and install to ~/.tv-mcp/bin/tv-mcp"));
    }

    if auth_status.0 == "missing" {
        lines.push(String::new());
        lines.push(format!("⚠️  Workspace not configured. Open TV Client and sign in — "));
        lines.push(format!("credentials will be written to ~/.tv-mcp/settings.json automatically."));
    }

    lines.push(String::new());
    lines.push(format!("Raw: {}", serde_json::to_string_pretty(&report).unwrap_or_default()));

    ToolResult::text(lines.join("\n"))
}

fn classify_install(path: &str) -> (&'static str, &'static str) {
    let path_lower = path.to_lowercase();

    if path_lower.contains(".app/contents") || path_lower.contains("program files") {
        return ("legacy-sidecar", "bundled inside TV Client app — should migrate to standalone");
    }

    if path_lower.contains(".tv-desktop/bin") || path_lower.contains("\\.tv-desktop\\bin") {
        return ("legacy-tv-desktop", "old standalone location — should migrate to ~/.tv-mcp/bin/");
    }

    if path_lower.contains(".tv-mcp/bin") || path_lower.contains("\\.tv-mcp\\bin") {
        return ("standalone", "current recommended install location");
    }

    if path_lower.contains("/target/release") || path_lower.contains("/target/debug") ||
       path_lower.contains("\\target\\release") || path_lower.contains("\\target\\debug") {
        return ("dev-build", "running from cargo build output");
    }

    ("unknown", "unrecognized install location")
}

fn get_settings_info() -> (String, &'static str) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let tv_mcp_path = home.join(".tv-mcp").join("settings.json");
    if tv_mcp_path.exists() {
        return (tv_mcp_path.display().to_string(), "found");
    }

    let tv_desktop_path = home.join(".tv-desktop").join("settings.json");
    if tv_desktop_path.exists() {
        return (tv_desktop_path.display().to_string(), "found (legacy location)");
    }

    (tv_mcp_path.display().to_string(), "not found")
}

fn check_auth() -> (&'static str, String) {
    match settings::load_settings() {
        Ok(settings) => {
            let supabase_url = settings.keys.get("supabase_url").filter(|s| !s.is_empty());
            let supabase_key = settings.keys.get("supabase_anon_key").filter(|s| !s.is_empty());

            match (supabase_url, supabase_key) {
                (Some(url), Some(key)) => {
                    ("configured", format!("workspace: {} | key: {}", url, mask_key(key)))
                }
                (None, _) => {
                    ("missing", "supabase_url not set — no workspace configured".to_string())
                }
                (_, None) => {
                    ("missing", "supabase_anon_key not set".to_string())
                }
            }
        }
        Err(e) => {
            ("error", format!("Failed to load settings: {}", e))
        }
    }
}
