// Seedance video generation via OpenRouter (tv-mcp side).
//
// Mirrors tv-client/src-tauri/src/commands/tools/seedance.rs but exposes the
// pipeline as MCP tools so bots can render videos without the desktop app.
//
// Synchronous render: submit -> poll -> download, all inside a single MCP call.
// Default model is Seedance Fast; the sidecar config can override per-file.

use crate::core::error::{CmdResult, CommandError};
use crate::core::settings::{self, KEY_OPENROUTER_API};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

const OPENROUTER_VIDEOS_URL: &str = "https://openrouter.ai/api/v1/videos";
const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const DISTILL_MODEL: &str = "claude-sonnet-4-6";

const DEFAULT_SEEDANCE_MODEL: &str = "bytedance/seedance-2.0-fast";
const DEFAULT_ASPECT: &str = "16:9";
const DEFAULT_DURATION: u32 = 5;

const POLL_INTERVAL_SECS: u64 = 5;
const DEFAULT_TIMEOUT_SECS: u64 = 600; // 10 min

// ============================================================================
// Sidecar config (.seedance.json)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedanceConfig {
    pub model: String,
    pub prompt: String,
    pub aspect_ratio: String,
    pub duration: u32,
    pub generate_audio: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedanceJobStatus {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub unsigned_urls: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
}

// ============================================================================
// Distill (.md -> prompt) via Anthropic
// ============================================================================

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

const DISTILL_SYSTEM_PROMPT: &str = "You are a video prompt engineer. Convert the user's markdown document into a single cinematic video prompt for a text-to-video model (ByteDance Seedance).

Rules:
- Output ONE paragraph, 1-3 sentences, no markdown, no headings, no lists, no quotes.
- Lead with the subject, then action, then setting, then style/mood/lighting.
- Concrete and visual: describe what is seen and how it moves. No abstract concepts.
- Do not invent narrative details that aren't in the source.
- No camera jargon unless the source explicitly calls for it.
- Output ONLY the prompt text, no preamble or explanation.";

pub async fn distill_md(md_path: &str) -> CmdResult<String> {
    let api_key = settings::settings_get_anthropic_key()?.ok_or_else(|| {
        CommandError::Config("Anthropic API key not configured (anthropic_api_key)".into())
    })?;

    let raw = tokio_fs::read_to_string(md_path).await?;
    let body = strip_frontmatter(&raw);
    if body.trim().is_empty() {
        return Err(CommandError::Config(
            "Markdown file is empty after stripping frontmatter".into(),
        ));
    }

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post(ANTHROPIC_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": DISTILL_MODEL,
            "max_tokens": 512,
            "temperature": 0.4,
            "system": DISTILL_SYSTEM_PROMPT,
            "messages": [
                { "role": "user", "content": body }
            ],
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http {
            status,
            body: text[..text.len().min(500)].to_string(),
        });
    }

    let parsed: AnthropicResponse = response.json().await?;
    let prompt = parsed
        .content
        .iter()
        .map(|b| match b {
            AnthropicContentBlock::Text { text } => text.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if prompt.is_empty() {
        return Err(CommandError::Internal(
            "Anthropic returned an empty distilled prompt".into(),
        ));
    }
    Ok(prompt)
}

fn strip_frontmatter(content: &str) -> &str {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim_start();
        }
    }
    content
}

// ============================================================================
// Sidecar create
// ============================================================================

/// Write a `.seedance.json` next to the given `.md`. If `prompt` is empty,
/// distill from the markdown body via Anthropic. Returns the new sidecar path.
pub async fn create_config(
    md_path: &str,
    prompt_override: Option<String>,
    model: Option<String>,
    aspect_ratio: Option<String>,
    duration: Option<u32>,
    generate_audio: Option<bool>,
) -> CmdResult<String> {
    let prompt = match prompt_override {
        Some(p) if !p.trim().is_empty() => p,
        _ => distill_md(md_path).await?,
    };

    let md = PathBuf::from(md_path);
    let stem = md
        .file_stem()
        .ok_or_else(|| CommandError::Config("Cannot derive stem from md_path".into()))?
        .to_string_lossy()
        .to_string();
    let parent = md.parent().unwrap_or(Path::new("."));
    let sidecar = parent.join(format!("{}.seedance.json", stem));

    let config = SeedanceConfig {
        model: model.unwrap_or_else(|| DEFAULT_SEEDANCE_MODEL.to_string()),
        prompt,
        aspect_ratio: aspect_ratio.unwrap_or_else(|| DEFAULT_ASPECT.to_string()),
        duration: duration.unwrap_or(DEFAULT_DURATION),
        generate_audio: generate_audio.unwrap_or(true),
        source_md: md.file_name().map(|n| n.to_string_lossy().to_string()),
        seed: None,
        resolution: None,
    };

    let body = serde_json::to_string_pretty(&config)?;
    tokio_fs::write(&sidecar, body).await?;
    Ok(sidecar.to_string_lossy().to_string())
}

// ============================================================================
// OpenRouter submit / poll / download
// ============================================================================

fn read_config(config_path: &str) -> CmdResult<SeedanceConfig> {
    let raw = std::fs::read_to_string(config_path)?;
    let cfg: SeedanceConfig = serde_json::from_str(&raw)?;
    if cfg.prompt.trim().is_empty() {
        return Err(CommandError::Config("Seedance config has no prompt".into()));
    }
    Ok(cfg)
}

fn openrouter_key() -> CmdResult<String> {
    settings::settings_get_key(KEY_OPENROUTER_API.to_string())?.ok_or_else(|| {
        CommandError::Config("OpenRouter API key not configured (openrouter_api_key)".into())
    })
}

pub async fn submit(config_path: &str) -> CmdResult<SeedanceJobStatus> {
    let cfg = read_config(config_path)?;
    let api_key = openrouter_key()?;

    let mut body = json!({
        "model": cfg.model,
        "prompt": cfg.prompt,
        "aspect_ratio": cfg.aspect_ratio,
        "duration": cfg.duration,
        "generate_audio": cfg.generate_audio,
    });
    if let Some(seed) = cfg.seed {
        body["seed"] = json!(seed);
    }
    if let Some(resolution) = cfg.resolution.as_deref() {
        body["resolution"] = json!(resolution);
    }

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .post(OPENROUTER_VIDEOS_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http {
            status,
            body: text[..text.len().min(800)].to_string(),
        });
    }

    let job: SeedanceJobStatus = response.json().await?;
    Ok(job)
}

pub async fn poll(job_id: &str) -> CmdResult<SeedanceJobStatus> {
    let api_key = openrouter_key()?;
    let url = format!("{}/{}", OPENROUTER_VIDEOS_URL, job_id);

    let client = crate::HTTP_CLIENT.clone();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http {
            status,
            body: text[..text.len().min(800)].to_string(),
        });
    }

    let job: SeedanceJobStatus = response.json().await?;
    Ok(job)
}

async fn download(video_url: &str, config_path: &str) -> CmdResult<String> {
    let cfg_path = PathBuf::from(config_path);
    let parent = cfg_path.parent().unwrap_or(Path::new("."));
    let stem = cfg_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".seedance.json"))
        .unwrap_or("video")
        .to_string();
    let dest = parent.join(format!("{}.mp4", stem));

    let client = crate::HTTP_CLIENT.clone();
    let mut response = client.get(video_url).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(CommandError::Http {
            status,
            body: text[..text.len().min(500)].to_string(),
        });
    }

    let mut file = tokio_fs::File::create(&dest).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    Ok(dest.to_string_lossy().to_string())
}

/// One-shot synchronous render: submit -> poll -> download. Returns the saved
/// `.mp4` path next to the sidecar.
pub async fn render(config_path: &str, timeout_secs: Option<u64>) -> CmdResult<RenderResult> {
    let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let max_polls = timeout / POLL_INTERVAL_SECS;

    let submitted = submit(config_path).await?;
    let job_id = submitted.id.clone();

    let mut last_status = submitted.status.clone();
    let mut video_url: Option<String> = None;

    for _ in 0..max_polls {
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        let status = poll(&job_id).await?;
        last_status = status.status.clone();

        match status.status.as_str() {
            "completed" => {
                video_url = status.unsigned_urls.into_iter().next();
                break;
            }
            "failed" | "cancelled" | "expired" => {
                return Err(CommandError::Internal(format!(
                    "Render {}: {}",
                    status.status,
                    status.error.unwrap_or_default()
                )));
            }
            _ => continue, // pending, in_progress
        }
    }

    let url = video_url.ok_or_else(|| {
        CommandError::Internal(format!(
            "Render timed out after {}s (last status: {})",
            timeout, last_status
        ))
    })?;

    let saved_path = download(&url, config_path).await?;
    Ok(RenderResult {
        job_id,
        saved_path,
        status: "completed".into(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderResult {
    pub job_id: String,
    pub saved_path: String,
    pub status: String,
}
