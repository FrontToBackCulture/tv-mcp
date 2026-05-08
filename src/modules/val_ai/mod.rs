// VAL AI Package — multi-domain skill assignment, package generation, S3 publish.
//
// Mirror of tv-client's `src-tauri/src/commands/val_sync/{ai_package,s3_sync}.rs`,
// reshaped to be MCP-callable. The bundled `sync_domain_ai_package` function
// runs assign → generate → push for a single domain in one call. The
// orchestrator slash-command in bot-builder loops domains.
//
// Path resolution:
// - Domain `global_path` comes from `~/.tv-desktop/val-sync-config.json` via
//   `val_sync::config::get_domain_config`. Same file tv-client uses.
// - Skills source path is `{knowledge_path}/_skills/`, where `knowledge_path`
//   is read from settings (`KEY_KNOWLEDGE_PATH`).
// - AWS creds come from settings (`aws_access_key_id`, `aws_secret_access_key`).

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{
    load_settings, KEY_AWS_ACCESS_KEY_ID, KEY_AWS_SECRET_ACCESS_KEY, KEY_KNOWLEDGE_PATH,
};
use crate::modules::val_sync::config::get_domain_config;

const S3_BUCKET: &str = "production.thinkval.static";
const S3_REGION: &str = "ap-southeast-1";

const INSTRUCTIONS_TEMPLATE: &str =
    include_str!("../../../resources/domain-ai-instructions.md");

const SKIP_FILES: &[&str] = &[
    "SKILL.md",
    "README.md",
    "AUDIT.md",
    "evals.json",
    "guide.html",
    ".DS_Store",
    ".claude.local.md",
];
const SKIP_DIRS: &[&str] = &[
    "__pycache__",
    ".claude",
    "demo",
    "examples",
    "evals",
    "_catalog",
    "_archive",
    "prompts",
];

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainAiConfig {
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentChange {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageStats {
    pub skills_copied: Vec<String>,
    pub instructions_generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3PushStats {
    pub files_uploaded: usize,
    pub bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDomainAiPackageResult {
    pub domain: String,
    pub assignments: AssignmentChange,
    pub package: PackageStats,
    pub s3: Option<S3PushStats>,
    pub errors: Vec<String>,
}

// ============================================================================
// Public entry point
// ============================================================================

/// Bundled assign → generate → push pipeline for a single domain.
///
/// Exactly one of `add`/`remove` (combined) or `replace` should be set:
/// - `replace = Some(list)` overwrites the assigned skill list verbatim.
/// - `add` and/or `remove` mutate the current list. Both default to empty.
///
/// `skip_push = true` runs assign + generate locally without uploading to S3.
pub async fn sync_domain_ai_package(
    domain: String,
    add: Vec<String>,
    remove: Vec<String>,
    replace: Option<Vec<String>>,
    skip_push: bool,
) -> CmdResult<SyncDomainAiPackageResult> {
    if replace.is_some() && (!add.is_empty() || !remove.is_empty()) {
        return Err(CommandError::Config(
            "`replace` is mutually exclusive with `add`/`remove`".to_string(),
        ));
    }

    let domain_config = get_domain_config(&domain)?;
    let global_path = PathBuf::from(&domain_config.global_path);
    if !global_path.exists() {
        return Err(CommandError::NotFound(format!(
            "Domain folder not found: {}",
            global_path.display()
        )));
    }
    let ai_path = global_path.join("ai");
    fs::create_dir_all(&ai_path)?;

    let skills_path = resolve_skills_path()?;
    if !skills_path.exists() {
        return Err(CommandError::NotFound(format!(
            "Skills directory not found: {}",
            skills_path.display()
        )));
    }

    // ---- Assign ----
    let before = read_ai_config(&ai_path).skills;
    let after = apply_assignment(&before, &add, &remove, replace.as_deref());
    let added: Vec<String> = after.iter().filter(|s| !before.contains(s)).cloned().collect();
    let removed: Vec<String> = before.iter().filter(|s| !after.contains(s)).cloned().collect();
    write_ai_config(&ai_path, &after)?;

    let assignments = AssignmentChange {
        before,
        after: after.clone(),
        added,
        removed,
    };

    // ---- Generate ----
    let mut errors: Vec<String> = Vec::new();
    let all_domain_slugs = list_all_domain_slugs();
    let (skills_copied, instructions_generated) = generate_package(
        &ai_path,
        &skills_path,
        &domain,
        &after,
        &all_domain_slugs,
        &mut errors,
    );
    let package = PackageStats {
        skills_copied,
        instructions_generated,
    };

    // ---- Push ----
    let s3 = if skip_push {
        None
    } else {
        match push_to_s3(&domain, &ai_path).await {
            Ok(stats) => Some(stats),
            Err(e) => {
                errors.push(format!("S3 push failed: {}", e));
                None
            }
        }
    };

    Ok(SyncDomainAiPackageResult {
        domain,
        assignments,
        package,
        s3,
        errors,
    })
}

// ============================================================================
// Path resolution
// ============================================================================

fn resolve_skills_path() -> CmdResult<PathBuf> {
    let settings = load_settings()?;
    let knowledge = settings
        .keys
        .get(KEY_KNOWLEDGE_PATH)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CommandError::Config(
                "knowledge_path not configured in settings — point it at the tv-knowledge root"
                    .to_string(),
            )
        })?;
    Ok(PathBuf::from(knowledge).join("_skills"))
}

/// All domain slugs from val-sync-config.json. Used by the recursive copy to
/// route filename-based domain references (`references/lag.json` is only copied
/// when generating for `lag`).
fn list_all_domain_slugs() -> Vec<String> {
    use crate::modules::val_sync::config::load_config_internal;
    load_config_internal()
        .map(|c| c.domains.into_iter().map(|d| d.domain).collect())
        .unwrap_or_default()
}

// ============================================================================
// Assign
// ============================================================================

fn read_ai_config(ai_path: &Path) -> DomainAiConfig {
    let config_path = ai_path.join("ai_config.json");
    match fs::read_to_string(&config_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => DomainAiConfig::default(),
    }
}

fn write_ai_config(ai_path: &Path, skills: &[String]) -> CmdResult<()> {
    let config = DomainAiConfig {
        skills: skills.to_vec(),
    };
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(ai_path.join("ai_config.json"), json)?;
    Ok(())
}

fn apply_assignment(
    before: &[String],
    add: &[String],
    remove: &[String],
    replace: Option<&[String]>,
) -> Vec<String> {
    if let Some(list) = replace {
        return dedupe_preserve_order(list);
    }
    let mut out: Vec<String> = before.to_vec();
    for slug in add {
        if !out.contains(slug) {
            out.push(slug.clone());
        }
    }
    out.retain(|s| !remove.contains(s));
    dedupe_preserve_order(&out)
}

fn dedupe_preserve_order(list: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for s in list {
        if seen.insert(s.clone()) {
            out.push(s.clone());
        }
    }
    out
}

// ============================================================================
// Generate
// ============================================================================

fn generate_package(
    ai_path: &Path,
    skills_base: &Path,
    domain: &str,
    skills: &[String],
    all_domains: &[String],
    errors: &mut Vec<String>,
) -> (Vec<String>, bool) {
    let ai_skills_path = ai_path.join("skills");
    let ai_tables_path = ai_path.join("tables");

    if ai_tables_path.exists() {
        if let Err(e) = fs::remove_dir_all(&ai_tables_path) {
            errors.push(format!("Failed to clean ai/tables/: {}", e));
        }
    }
    if ai_skills_path.exists() {
        if let Err(e) = fs::remove_dir_all(&ai_skills_path) {
            errors.push(format!("Failed to clean ai/skills/: {}", e));
        }
    }

    let mut skills_copied: Vec<String> = Vec::new();

    for skill in skills {
        let skill_src_dir = skills_base.join(skill);
        let skill_src_md = skill_src_dir.join("SKILL.md");
        if !skill_src_md.exists() {
            errors.push(format!("Skill not found: {}/SKILL.md", skill));
            continue;
        }

        let skill_dir = ai_skills_path.join(skill);
        if let Err(e) = fs::create_dir_all(&skill_dir) {
            errors.push(format!("Failed to create ai/skills/{}/: {}", skill, e));
            continue;
        }

        let dest = skill_dir.join("SKILL.md");
        match fs::read_to_string(&skill_src_md) {
            Ok(content) => {
                let stripped = strip_skill_frontmatter(&content);
                let replaced = stripped.replace("{{DOMAIN}}", domain);
                match fs::write(&dest, &replaced) {
                    Ok(_) => skills_copied.push(skill.clone()),
                    Err(e) => errors.push(format!("Failed to write skill {}: {}", skill, e)),
                }
            }
            Err(e) => errors.push(format!("Failed to read skill {}: {}", skill, e)),
        }

        copy_skill_dir_recursive(&skill_src_dir, &skill_dir, domain, all_domains, skill, errors);
    }

    let instructions_generated = match regenerate_instructions(
        ai_path,
        skills_base,
        domain,
        &skills_copied,
        None,
    ) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e.to_string());
            false
        }
    };

    (skills_copied, instructions_generated)
}

fn strip_skill_frontmatter(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    let rest = &content[3..];
    let end = match rest.find("\n---") {
        Some(pos) => pos,
        None => return content.to_string(),
    };
    let frontmatter_block = &rest[..end];
    let body = &rest[end + 4..];

    let mut name = String::new();
    let mut description = String::new();
    for line in frontmatter_block.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("name:") {
            name = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("description:") {
            description = val.trim().trim_matches('"').to_string();
        }
    }

    let mut result = String::from("---\n");
    if !name.is_empty() {
        result.push_str(&format!("name: \"{}\"\n", name));
    }
    if !description.is_empty() {
        result.push_str(&format!("description: \"{}\"\n", description));
    }
    result.push_str("---");
    result.push_str(body);
    result
}

fn copy_skill_dir_recursive(
    src_dir: &Path,
    dest_dir: &Path,
    domain: &str,
    all_domains: &[String],
    skill: &str,
    errors: &mut Vec<String>,
) {
    let entries = match fs::read_dir(src_dir) {
        Ok(e) => e,
        Err(e) => {
            errors.push(format!("Failed to read skill dir {}: {}", skill, e));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if path.is_dir() {
            if SKIP_DIRS.contains(&fname.as_str()) {
                continue;
            }
            // _domains/{domain}/ contents are flattened up to this level so
            // SKILL.md keeps referencing plain paths like `references/foo.md`.
            if fname == "_domains" {
                let domain_subdir = path.join(domain);
                if domain_subdir.is_dir() {
                    copy_skill_dir_recursive(
                        &domain_subdir,
                        dest_dir,
                        domain,
                        all_domains,
                        skill,
                        errors,
                    );
                }
                continue;
            }
            let sub_dest = dest_dir.join(&fname);
            if let Err(e) = fs::create_dir_all(&sub_dest) {
                errors.push(format!("Failed to create {}/{}/: {}", skill, fname, e));
                continue;
            }
            copy_skill_dir_recursive(&path, &sub_dest, domain, all_domains, skill, errors);
        } else {
            if SKIP_FILES.contains(&fname.as_str()) {
                continue;
            }
            if fname.ends_with(".excalidraw") {
                continue;
            }
            // Filename-based domain routing: a file whose stem matches a known
            // domain slug is treated as scoped to that domain.
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if !stem.is_empty()
                && stem != domain
                && all_domains.iter().any(|d| d == stem)
            {
                continue;
            }

            let dest_file = dest_dir.join(&fname);
            let is_text = matches!(
                path.extension().and_then(|e| e.to_str()),
                Some(
                    "md" | "py"
                        | "sql"
                        | "txt"
                        | "json"
                        | "csv"
                        | "html"
                        | "css"
                        | "js"
                        | "ts"
                        | "yaml"
                        | "yml"
                        | "toml"
                        | "sh"
                )
            );
            if is_text {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let replaced = content.replace("{{DOMAIN}}", domain);
                        if let Err(e) = fs::write(&dest_file, &replaced) {
                            errors.push(format!("Failed to write {}/{}: {}", skill, fname, e));
                        }
                    }
                    Err(e) => errors.push(format!("Failed to read {}/{}: {}", skill, fname, e)),
                }
            } else if let Err(e) = fs::copy(&path, &dest_file) {
                errors.push(format!("Failed to copy {}/{}: {}", skill, fname, e));
            }
        }
    }
}

fn read_skill_description(skills_path: &Path, slug: &str) -> Option<String> {
    let registry_path = skills_path.join("registry.json");
    let raw = fs::read_to_string(&registry_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    parsed
        .get("skills")?
        .get(slug)?
        .get("description")?
        .as_str()
        .map(|s| s.to_string())
}

fn regenerate_instructions(
    ai_path: &Path,
    skills_path: &Path,
    domain: &str,
    skills: &[String],
    template_override: Option<&str>,
) -> CmdResult<bool> {
    let instructions_path = ai_path.join("instructions.md");

    let skill_list = skills
        .iter()
        .map(|s| match read_skill_description(skills_path, s) {
            Some(d) => format!("- `skills/{}/SKILL.md` — {}", s, d),
            None => format!("- `skills/{}/SKILL.md`", s),
        })
        .collect::<Vec<_>>()
        .join("\n");

    let template = template_override.unwrap_or(INSTRUCTIONS_TEMPLATE);
    let mut content = template
        .replace("{{DOMAIN}}", domain)
        .replace("{{SKILL_LIST}}", &skill_list);

    let custom_path = ai_path.join("custom.md");
    if let Ok(custom_raw) = fs::read_to_string(&custom_path) {
        let custom = custom_raw.trim();
        if !custom.is_empty() {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str("\n---\n\n## Custom Instructions\n\n");
            content.push_str(custom);
            content.push('\n');
        }
    }

    fs::write(&instructions_path, &content)?;
    Ok(true)
}

// ============================================================================
// S3 push
// ============================================================================

async fn push_to_s3(domain: &str, ai_path: &Path) -> CmdResult<S3PushStats> {
    let start = Instant::now();
    let settings = load_settings()?;
    let access_key = settings
        .keys
        .get(KEY_AWS_ACCESS_KEY_ID)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CommandError::Config(
                "AWS Access Key ID not configured in settings.json".to_string(),
            )
        })?;
    let secret_key = settings
        .keys
        .get(KEY_AWS_SECRET_ACCESS_KEY)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CommandError::Config(
                "AWS Secret Access Key not configured in settings.json".to_string(),
            )
        })?;

    let client = build_s3_client(access_key, secret_key);
    let s3_prefix = format!("solutions/{}/", domain);

    delete_s3_prefix(&client, &s3_prefix).await?;

    let mut local_files: HashMap<String, u64> = HashMap::new();
    collect_local_files(ai_path, ai_path, &mut local_files);

    let mut files_uploaded = 0usize;
    let mut bytes = 0u64;
    for (rel_path, size) in &local_files {
        let full_path = ai_path.join(rel_path);
        let body = tokio::fs::read(&full_path)
            .await
            .map_err(|e| CommandError::Io(format!("Failed to read {}: {}", rel_path, e)))?;
        let s3_key = format!("{}{}", s3_prefix, rel_path);
        client
            .put_object()
            .bucket(S3_BUCKET)
            .key(&s3_key)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(|e| CommandError::Network(format!("Failed to upload {}: {}", rel_path, e)))?;
        files_uploaded += 1;
        bytes += size;
    }

    Ok(S3PushStats {
        files_uploaded,
        bytes,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn build_s3_client(access_key: &str, secret_key: &str) -> aws_sdk_s3::Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "tv-mcp-settings");
    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(S3_REGION))
        .credentials_provider(creds)
        .build();
    aws_sdk_s3::Client::from_conf(config)
}

fn collect_local_files(base: &Path, dir: &Path, out: &mut HashMap<String, u64>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden files, ai_config.json (local-only), and custom.md (author source).
        if name.starts_with('.') || name == "ai_config.json" || name == "custom.md" {
            continue;
        }
        if path.is_dir() {
            collect_local_files(base, &path, out);
        } else {
            let rel = path
                .strip_prefix(base)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if !rel.is_empty() {
                out.insert(rel, size);
            }
        }
    }
}

async fn delete_s3_prefix(client: &aws_sdk_s3::Client, prefix: &str) -> CmdResult<()> {
    let objects = list_s3_keys(client, prefix).await?;
    if objects.is_empty() {
        return Ok(());
    }
    for chunk in objects.chunks(1000) {
        let ids: Vec<ObjectIdentifier> = chunk
            .iter()
            .map(|key| {
                ObjectIdentifier::builder()
                    .key(key)
                    .build()
                    .expect("ObjectIdentifier build")
            })
            .collect();
        let delete = Delete::builder()
            .set_objects(Some(ids))
            .quiet(true)
            .build()
            .map_err(|e| CommandError::Internal(format!("Failed to build delete request: {}", e)))?;
        client
            .delete_objects()
            .bucket(S3_BUCKET)
            .delete(delete)
            .send()
            .await
            .map_err(|e| CommandError::Network(format!("Failed to delete S3 objects: {}", e)))?;
    }
    Ok(())
}

async fn list_s3_keys(client: &aws_sdk_s3::Client, prefix: &str) -> CmdResult<Vec<String>> {
    let mut keys = Vec::new();
    let mut continuation_token: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(S3_BUCKET).prefix(prefix);
        if let Some(token) = &continuation_token {
            req = req.continuation_token(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| CommandError::Network(format!("Failed to list S3 objects: {}", e)))?;
        for obj in resp.contents() {
            if let Some(key) = obj.key() {
                keys.push(key.to_string());
            }
        }
        match resp.next_continuation_token() {
            Some(token) => continuation_token = Some(token.to_string()),
            None => break,
        }
    }
    Ok(keys)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn unique_tmp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("val_ai_test_{}_{}", label, nanos));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn apply_replace_overrides_everything() {
        let before = vec!["a".to_string(), "b".to_string()];
        let after = apply_assignment(&before, &[], &[], Some(&["x".to_string(), "y".to_string()]));
        assert_eq!(after, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn apply_add_is_idempotent() {
        let before = vec!["a".to_string(), "b".to_string()];
        let after = apply_assignment(&before, &["b".to_string(), "c".to_string()], &[], None);
        assert_eq!(after, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn apply_remove_is_idempotent() {
        let before = vec!["a".to_string(), "b".to_string()];
        let after = apply_assignment(&before, &[], &["b".to_string(), "z".to_string()], None);
        assert_eq!(after, vec!["a".to_string()]);
    }

    #[test]
    fn apply_add_then_remove() {
        let before = vec!["a".to_string()];
        let after = apply_assignment(
            &before,
            &["b".to_string(), "c".to_string()],
            &["a".to_string()],
            None,
        );
        assert_eq!(after, vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn strip_frontmatter_keeps_name_and_desc() {
        let raw = "---\nname: foo\ndescription: bar baz\nextra: ignored\n---\n\nbody";
        let out = strip_skill_frontmatter(raw);
        assert!(out.starts_with("---\nname: \"foo\"\ndescription: \"bar baz\"\n---"));
        assert!(out.contains("\nbody"));
        assert!(!out.contains("extra: ignored"));
    }

    #[test]
    fn regenerate_uses_custom_md_when_present() {
        let dir = unique_tmp_dir("custom");
        let ai_path = dir.join("ai");
        fs::create_dir_all(&ai_path).unwrap();
        fs::write(ai_path.join("custom.md"), "Be helpful.\nNo apologies.").unwrap();

        let template = "# {{DOMAIN}}\n{{SKILL_LIST}}\n";
        regenerate_instructions(&ai_path, &dir.join("nope"), "demo", &[], Some(template)).unwrap();

        let out = fs::read_to_string(ai_path.join("instructions.md")).unwrap();
        assert!(out.contains("# demo"));
        assert!(out.contains("---\n\n## Custom Instructions"));
        assert!(out.contains("Be helpful."));
        // custom.md must be untouched
        let custom = fs::read_to_string(ai_path.join("custom.md")).unwrap();
        assert_eq!(custom, "Be helpful.\nNo apologies.");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn copy_flattens_matching_domain_subdir() {
        let dir = unique_tmp_dir("flatten");
        let src = dir.join("src");
        let dest = dir.join("dest");
        fs::create_dir_all(src.join("references/_domains/lag")).unwrap();
        fs::create_dir_all(src.join("references/_domains/koi")).unwrap();
        fs::write(src.join("references/shared.md"), "for {{DOMAIN}}").unwrap();
        fs::write(src.join("references/_domains/lag/lag-only.md"), "lag {{DOMAIN}}").unwrap();
        fs::write(src.join("references/_domains/koi/koi-only.md"), "koi").unwrap();
        fs::create_dir_all(&dest).unwrap();

        let all = vec!["lag".to_string(), "koi".to_string()];
        let mut errors: Vec<String> = Vec::new();
        copy_skill_dir_recursive(&src, &dest, "lag", &all, "test-skill", &mut errors);

        assert!(errors.is_empty(), "errors: {:?}", errors);
        let shared = fs::read_to_string(dest.join("references/shared.md")).unwrap();
        assert_eq!(shared, "for lag");
        let lag = fs::read_to_string(dest.join("references/lag-only.md")).unwrap();
        assert_eq!(lag, "lag lag");
        assert!(!dest.join("references/_domains").exists());
        assert!(!dest.join("references/koi-only.md").exists());
        fs::remove_dir_all(&dir).ok();
    }
}
