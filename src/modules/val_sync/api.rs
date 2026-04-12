// VAL Sync API - Generic HTTP client for VAL platform endpoints
// Routes by artifact type to the correct endpoint

use serde_json::Value;
use std::fmt;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug)]
pub enum ValApiError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
    AuthExpired,
}

impl fmt::Display for ValApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValApiError::Http { status, body } => write!(f, "HTTP {}: {}", status, body),
            ValApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ValApiError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ValApiError::AuthExpired => write!(f, "Authentication expired"),
        }
    }
}

impl ValApiError {
    pub fn is_auth_error(&self) -> bool {
        match self {
            ValApiError::AuthExpired => true,
            ValApiError::Http { status, body } => {
                *status == 401
                    || *status == 403
                    || (*status == 500 && {
                        let lower = body.to_lowercase();
                        lower.contains("token not authentic")
                            || lower.contains("jwt expired")
                            || lower.contains("invalid signature")
                    })
            }
            _ => false,
        }
    }
}

// ============================================================================
// API fetch
// ============================================================================

/// Fetch data from VAL API by artifact type.
///
/// - `base_url`: e.g. "https://koi.thinkval.io"
/// - `token`: JWT auth token
/// - `artifact_type`: one of: fields, all-queries, all-workflows, all-dashboards,
///   all-tables, calc-fields, data-model, workflow, dashboard, query
/// - `id`: required for single-item fetches (data-model, workflow, dashboard, query)
pub async fn val_api_fetch(
    base_url: &str,
    token: &str,
    artifact_type: &str,
    id: Option<&str>,
) -> Result<Value, ValApiError> {
    let client = crate::HTTP_CLIENT.clone();

    let (method, path, query_params, body) = match artifact_type {
        "fields" => (
            "GET",
            "/db/admin-fields/getAllFields/".to_string(),
            vec![("convert", "true")],
            None,
        ),
        "all-queries" => (
            "GET",
            "/db/data/v1/listAllDSQueries".to_string(),
            vec![],
            None,
        ),
        "all-workflows" => (
            "GET",
            "/api/v1/workflow/".to_string(),
            vec![],
            None,
        ),
        "all-dashboards" => (
            "GET",
            "/db/dashboard/v2/listAllDashboards".to_string(),
            vec![("type", "all")],
            None,
        ),
        "all-tables" => (
            "GET",
            "/db/admin-management/getFullAdminTree".to_string(),
            vec![],
            None,
        ),
        "calc-fields" => (
            "POST",
            "/db/settings/customGetAdminUiSettings".to_string(),
            vec![],
            Some(serde_json::json!({"type": "workspace_rule_field"})),
        ),
        "data-model" => {
            let table_id = id.ok_or_else(|| {
                ValApiError::Parse("'id' required for data-model fetch".to_string())
            })?;
            (
                "GET",
                "/api/v1/load/loadRepoTableRaw".to_string(),
                vec![("table", table_id)],
                None,
            )
        }
        "workflow" => {
            let wf_id = id.ok_or_else(|| {
                ValApiError::Parse("'id' required for workflow fetch".to_string())
            })?;
            (
                "GET",
                format!("/api/v1/workflow/{}", wf_id),
                vec![],
                None,
            )
        }
        "dashboard" => {
            let db_id = id.ok_or_else(|| {
                ValApiError::Parse("'id' required for dashboard fetch".to_string())
            })?;
            (
                "GET",
                format!("/db/dashboard/v2/getDashboard/{}", db_id),
                vec![],
                None,
            )
        }
        "query" => {
            let q_id = id.ok_or_else(|| {
                ValApiError::Parse("'id' required for query fetch".to_string())
            })?;
            (
                "GET",
                format!("/db/data/v1/getDSQuery/{}", q_id),
                vec![],
                None,
            )
        }
        _ => {
            return Err(ValApiError::Parse(format!(
                "Unknown artifact type: {}",
                artifact_type
            )));
        }
    };

    let url = format!("{}{}", base_url, path);

    let mut request = if method == "POST" {
        client.post(&url)
    } else {
        client.get(&url)
    };

    // Add auth query params
    request = request.query(&[("uuid", "1"), ("token", token)]);

    // Add type-specific query params
    for (k, v) in &query_params {
        request = request.query(&[(k, v)]);
    }

    // Add body for POST requests
    if let Some(body_val) = body {
        request = request
            .header("Content-Type", "application/json")
            .json(&body_val);
    }

    let response = request
        .send()
        .await
        .map_err(|e| ValApiError::Network(e.to_string()))?;

    let status = response.status().as_u16();
    if status == 401 || status == 403 {
        return Err(ValApiError::AuthExpired);
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        let lower = body.to_lowercase();
        if lower.contains("token not authentic")
            || lower.contains("jwt expired")
            || lower.contains("invalid signature")
        {
            return Err(ValApiError::AuthExpired);
        }
        return Err(ValApiError::Http { status, body });
    }

    response
        .json()
        .await
        .map_err(|e| ValApiError::Parse(e.to_string()))
}
