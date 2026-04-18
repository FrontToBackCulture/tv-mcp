// QBO module — reads the mgmt workspace's qbo_* mirror tables.
//
// All queries target the mgmt workspace Supabase project (`b0000000-...-002`)
// via `get_client_for_workspace`. The bot running tv-mcp must have
// workspace_memberships to that workspace, and the workspace jwt_secret must
// be configured in the gateway. See plans/finance-workspace/phase-5-*.sql.

pub mod connection;
pub mod entities;
pub mod reports;
pub mod sync;
pub mod transactions;

/// Workspace ID for the mgmt project — all QBO reads target this workspace.
pub const MGMT_WORKSPACE_ID: &str = "b0000000-0000-0000-0000-000000000002";
