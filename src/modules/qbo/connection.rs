// QBO connection status + sync runs

use super::MGMT_WORKSPACE_ID;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client_for_workspace;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub realm_id: String,
    pub company_name: Option<String>,
    pub expires_at: String,
    pub environment: String,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRun {
    pub id: String,
    pub entity_type: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub records_processed: Option<i64>,
    pub error: Option<String>,
    pub cursor: Option<String>,
    pub triggered_by: Option<String>,
}

pub async fn qbo_connection_status() -> CmdResult<Option<ConnectionInfo>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let query = "status=eq.active&order=updated_at.desc&limit=1";
    let rows: Vec<ConnectionInfo> = client.select("qbo_connection_info", query).await?;
    Ok(rows.into_iter().next())
}

pub async fn qbo_list_sync_runs(limit: Option<i32>) -> CmdResult<Vec<SyncRun>> {
    let client = get_client_for_workspace(MGMT_WORKSPACE_ID).await?;
    let limit = limit.unwrap_or(20);
    let query = format!("order=started_at.desc&limit={}", limit);
    client.select("qbo_sync_runs", &query).await
}
