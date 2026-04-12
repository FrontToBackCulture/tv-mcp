// SES campaign sending engine
// Replaces the tv-api sendCampaign — runs entirely in-process via Tauri command.

use aws_sdk_ses::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_ses::types::RawMessage;
use aws_sdk_ses::primitives::Blob;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::{get_client, SupabaseClient};
use serde::{Deserialize, Serialize};

const SES_REGION: &str = "ap-southeast-1";
const S3_BUCKET: &str = "production.thinkval.static";
const S3_REGION: &str = "ap-southeast-1";

/// Build an SES client from stored credentials
fn build_ses_client(access_key: &str, secret_key: &str) -> aws_sdk_ses::Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "tv-client-settings");
    let config = aws_sdk_ses::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(SES_REGION))
        .credentials_provider(creds)
        .build();
    aws_sdk_ses::Client::from_conf(config)
}

/// Build an S3 client from stored credentials
fn build_s3_client(access_key: &str, secret_key: &str) -> aws_sdk_s3::Client {
    let creds = aws_sdk_s3::config::Credentials::new(access_key, secret_key, None, None, "tv-client-settings");
    let config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(S3_REGION))
        .credentials_provider(creds)
        .build();
    aws_sdk_s3::Client::from_conf(config)
}

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Campaign {
    id: String,
    subject: String,
    from_name: String,
    from_email: String,
    html_body: Option<String>,
    content_path: Option<String>,
    report_path: Option<String>,
    report_url: Option<String>,
    bcc_email: Option<String>,
    group_id: Option<String>,
    status: String,
    tokens: Option<serde_json::Value>,
    send_channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Contact {
    id: String,
    email: String,
    name: Option<String>,
    edm_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContactLink {
    crm_contacts: Option<Contact>,
}

#[derive(Debug, Deserialize)]
struct EventRow {
    id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendCampaignResult {
    pub sent: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendTestResult {
    pub success: bool,
    pub error: Option<String>,
}

// ── S3 report upload ─────────────────────────────────────────────

/// Upload a report file to S3 with a UUID-based path and return the public URL
async fn upload_report_to_s3(
    s3: &aws_sdk_s3::Client,
    report_path: &str,
    knowledge_path: &str,
    campaign_id: &str,
) -> CmdResult<String> {
    let full_path = std::path::Path::new(knowledge_path).join(report_path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| CommandError::Internal(format!(
            "Failed to read report file {}: {}", full_path.display(), e
        )))?;

    // Generate unguessable S3 key: reports/{campaign_id}/{random}/report.html
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    chrono::Utc::now().timestamp_nanos_opt().hash(&mut hasher);
    campaign_id.hash(&mut hasher);
    report_path.hash(&mut hasher);
    let uuid = format!("{:016x}", hasher.finish());
    let file_name = std::path::Path::new(report_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("report.html");
    let s3_key = format!("email-reports/{}/{}/{}", campaign_id, uuid, file_name);

    s3.put_object()
        .bucket(S3_BUCKET)
        .key(&s3_key)
        .body(aws_sdk_s3::primitives::ByteStream::from(content.into_bytes()))
        .content_type("text/html; charset=utf-8")
        .cache_control("public, max-age=86400")
        .send()
        .await
        .map_err(|e| CommandError::Network(format!("S3 upload failed: {:?}", e)))?;

    let url = format!("https://s3.{}.amazonaws.com/{}/{}", S3_REGION, S3_BUCKET, s3_key);
    Ok(url)
}

// ── Resolve campaign HTML body ────────────────────────────────────

/// Get the HTML body for a campaign, reading from content_path file if set.
/// Automatically inlines CSS `<style>` blocks into element `style` attributes
/// so emails render correctly in clients that strip `<style>` tags (e.g. Gmail).
fn resolve_html_body(campaign: &Campaign, knowledge_path: Option<&str>) -> CmdResult<String> {
    let raw = if let Some(content_path) = &campaign.content_path {
        if let Some(kp) = knowledge_path {
            let full_path = std::path::Path::new(kp).join(content_path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Failed to read content_path {}: {}", full_path.display(), e);
                    campaign.html_body.clone()
                        .ok_or_else(|| CommandError::Internal("Campaign has no HTML body or content file".into()))?
                }
            }
        } else {
            campaign.html_body.clone()
                .ok_or_else(|| CommandError::Internal("Campaign has no HTML body or content file".into()))?
        }
    } else {
        campaign.html_body.clone()
            .ok_or_else(|| CommandError::Internal("Campaign has no HTML body or content file".into()))?
    };

    // Inline CSS for email client compatibility
    let inliner = css_inline::CSSInliner::options()
        .keep_style_tags(false)
        .build();
    match inliner.inline(&raw) {
        Ok(inlined) => Ok(inlined),
        Err(e) => {
            eprintln!("CSS inlining failed, using raw HTML: {}", e);
            Ok(raw)
        }
    }
}

// ── Token replacement ──────────────────────────────────────────────

fn replace_tokens(
    html: &str,
    contact: &Contact,
    event_id: &str,
    campaign_id: &str,
    api_base_url: &str,
    subject: &str,
    report_url: Option<&str>,
    custom_tokens: Option<&serde_json::Value>,
) -> String {
    let mut result = html.to_string();

    // Replace {{first_name}} — extract first name from full name
    let first_name = contact.name.as_deref()
        .and_then(|n| n.split_whitespace().next())
        .unwrap_or("there");
    result = result.replace("{{first_name}}", first_name);

    // Replace {{subject}} — templates use this in hero headings
    result = result.replace("{{subject}}", subject);

    // Replace {{report_url}} — from dedicated column or tokens JSON
    if let Some(url) = report_url {
        result = result.replace("{{report_url}}", url);
    }

    // Replace {{unsubscribe_url}}
    let unsub_url = format!(
        "{}/email/unsubscribe?cid={}&mid={}",
        api_base_url, contact.id, campaign_id
    );
    result = result.replace("{{unsubscribe_url}}", &unsub_url);

    // Replace custom tokens from campaign.tokens JSONB
    if let Some(serde_json::Value::Object(map)) = custom_tokens {
        for (key, val) in map {
            // Skip system tokens that are always computed — but allow report_url
            // from tokens if the dedicated column didn't provide it
            if matches!(key.as_str(), "first_name" | "subject" | "unsubscribe_url") {
                continue;
            }
            if key == "report_url" && report_url.is_some() {
                continue;
            }
            if let Some(s) = val.as_str() {
                result = result.replace(&format!("{{{{{}}}}}", key), s);
            }
        }
    }

    // Inject open tracking pixel before </body>
    let open_pixel = format!(
        r#"<img src="{}/email/track/open?eid={}" width="1" height="1" style="max-height:0;overflow:hidden;mso-hide:all" alt="" />"#,
        api_base_url, event_id
    );
    if result.contains("</body>") {
        result = result.replace("</body>", &format!("{}</body>", open_pixel));
    } else {
        result.push_str(&open_pixel);
    }

    // Rewrite links for click tracking (skip unsubscribe links)
    result = rewrite_links(&result, event_id, api_base_url);

    result
}

/// Simple token replacement for preview (no tracking injection)
fn replace_tokens_preview(
    html: &str,
    first_name: &str,
    subject: &str,
    report_url: Option<&str>,
    custom_tokens: Option<&serde_json::Value>,
) -> String {
    let mut result = html.to_string();
    result = result.replace("{{first_name}}", first_name);
    result = result.replace("{{subject}}", subject);
    result = result.replace("{{unsubscribe_url}}", "#unsubscribe");
    if let Some(url) = report_url {
        result = result.replace("{{report_url}}", url);
    } else {
        result = result.replace("{{report_url}}", "#report");
    }

    // Replace custom tokens
    if let Some(serde_json::Value::Object(map)) = custom_tokens {
        for (key, val) in map {
            if matches!(key.as_str(), "first_name" | "subject" | "unsubscribe_url") {
                continue;
            }
            if key == "report_url" && report_url.is_some() {
                continue;
            }
            if let Some(s) = val.as_str() {
                result = result.replace(&format!("{{{{{}}}}}", key), s);
            }
        }
    }

    result
}

/// Rewrite href="https://..." links to go through click tracker
fn rewrite_links(html: &str, event_id: &str, api_base_url: &str) -> String {
    let re = regex::Regex::new(r#"href="(https?://[^"]+)""#).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let url = &caps[1];
        // Don't wrap unsubscribe links
        if url.contains("/email/unsubscribe") {
            return caps[0].to_string();
        }
        let encoded = urlencoding::encode(url);
        format!(
            r#"href="{}/email/track/click?eid={}&url={}""#,
            api_base_url, event_id, encoded
        )
    })
    .to_string()
}

// ── Upload report command ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UploadReportResult {
    pub url: String,
}


pub async fn email_upload_report(
    campaign_id: String,
    knowledge_path: String,
) -> CmdResult<UploadReportResult> {
    let settings = crate::core::settings::load_settings()?;
    let access_key = settings
        .keys
        .get("aws_access_key_id")
        .ok_or_else(|| CommandError::Config("AWS Access Key ID not configured".into()))?;
    let secret_key = settings
        .keys
        .get("aws_secret_access_key")
        .ok_or_else(|| CommandError::Config("AWS Secret Access Key not configured".into()))?;

    let db = get_client().await?;

    // Fetch campaign to get report_path
    let campaign: Campaign = db
        .select_single::<Campaign>(
            "email_campaigns",
            &format!("id=eq.{}&select=*", campaign_id),
        )
        .await?
        .ok_or_else(|| CommandError::NotFound("Campaign not found".into()))?;

    let report_path = campaign
        .report_path
        .as_ref()
        .ok_or_else(|| CommandError::Internal("Campaign has no report_path set".into()))?;

    let s3 = build_s3_client(access_key, secret_key);
    let url = upload_report_to_s3(&s3, report_path, &knowledge_path, &campaign_id).await?;

    // Save report_url and upload timestamp to campaign
    let now = chrono::Utc::now().to_rfc3339();
    db.update::<serde_json::Value, serde_json::Value>(
        "email_campaigns",
        &format!("id=eq.{}", campaign_id),
        &serde_json::json!({ "report_url": url, "report_uploaded_at": now }),
    )
    .await?;

    Ok(UploadReportResult { url })
}

// ── Clear report command ─────────────────────────────────────────


pub async fn email_clear_report(campaign_id: String) -> CmdResult<()> {
    let db = get_client().await?;

    // Fetch campaign to get report_url for S3 deletion
    let campaign: Campaign = db
        .select_single::<Campaign>(
            "email_campaigns",
            &format!("id=eq.{}&select=*", campaign_id),
        )
        .await?
        .ok_or_else(|| CommandError::NotFound("Campaign not found".into()))?;

    // Delete from S3 if uploaded
    if let Some(ref url) = campaign.report_url {
        // Extract S3 key from URL: https://s3.{region}.amazonaws.com/{bucket}/{key}
        let prefix = format!("https://s3.{}.amazonaws.com/{}/", S3_REGION, S3_BUCKET);
        if let Some(key) = url.strip_prefix(&prefix) {
            let settings = crate::core::settings::load_settings()?;
            if let (Some(ak), Some(sk)) = (
                settings.keys.get("aws_access_key_id"),
                settings.keys.get("aws_secret_access_key"),
            ) {
                let s3 = build_s3_client(ak, sk);
                // Best-effort delete — don't fail if S3 delete fails
                let _ = s3.delete_object()
                    .bucket(S3_BUCKET)
                    .key(key)
                    .send()
                    .await;
            }
        }
    }

    // Clear report fields in DB
    db.update::<serde_json::Value, serde_json::Value>(
        "email_campaigns",
        &format!("id=eq.{}", campaign_id),
        &serde_json::json!({ "report_path": null, "report_url": null, "report_uploaded_at": null }),
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionalResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

/// Send a single transactional email via SES.
/// Called by the MCP tool handler — not a Tauri command.
pub async fn send_transactional_email(
    to: &str,
    subject: &str,
    html_body: &str,
    from_name: Option<&str>,
    from_email: Option<&str>,
) -> CmdResult<SendTransactionalResult> {
    let settings = crate::core::settings::load_settings()?;
    let access_key = settings
        .keys
        .get("aws_access_key_id")
        .ok_or_else(|| CommandError::Config("AWS Access Key ID not configured".into()))?;
    let secret_key = settings
        .keys
        .get("aws_secret_access_key")
        .ok_or_else(|| CommandError::Config("AWS Secret Access Key not configured".into()))?;

    let ses = build_ses_client(access_key, secret_key);

    let sender_name = from_name.unwrap_or("ThinkVAL");
    let sender_email = from_email.unwrap_or("hello@thinkval.com");
    let boundary = format!("----=_Part_{}", chrono::Utc::now().timestamp_millis());

    let raw_email = format!(
        "From: {} <{}>\r\n\
         To: {}\r\n\
         Subject: {}\r\n\
         MIME-Version: 1.0\r\n\
         Content-Type: multipart/alternative; boundary=\"{}\"\r\n\
         \r\n\
         --{}\r\n\
         Content-Type: text/html; charset=UTF-8\r\n\
         Content-Transfer-Encoding: 7bit\r\n\
         \r\n\
         {}\r\n\
         \r\n\
         --{}--",
        sender_name, sender_email, to, subject, boundary, boundary, html_body, boundary,
    );

    match ses
        .send_raw_email()
        .raw_message(
            RawMessage::builder()
                .data(Blob::new(raw_email.as_bytes()))
                .build()
                .map_err(|e| CommandError::Internal(format!("Failed to build message: {}", e)))?,
        )
        .send()
        .await
    {
        Ok(output) => Ok(SendTransactionalResult {
            success: true,
            message_id: Some(output.message_id().to_string()),
            error: None,
        }),
        Err(e) => {
            let msg = if let Some(service_err) = e.as_service_error() {
                format!(
                    "SES: {}",
                    service_err
                        .meta()
                        .message()
                        .unwrap_or(&format!("{}", service_err))
                )
            } else {
                format!("SES error: {:?}", e)
            };
            Ok(SendTransactionalResult {
                success: false,
                message_id: None,
                error: Some(msg),
            })
        }
    }
}
