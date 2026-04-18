// FY Review module — reads fy_snapshots, recognition_schedule,
// fy_reconciliations, fy_close_checklist from the mgmt workspace.

pub mod actions;
pub mod queries;

/// Workspace ID for the mgmt project.
pub const MGMT_WORKSPACE_ID: &str = "b0000000-0000-0000-0000-000000000002";
