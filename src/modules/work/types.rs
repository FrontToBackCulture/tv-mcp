// Work Module Types
// Data structures for projects, tasks, milestones, initiatives, etc.

use serde::{Deserialize, Serialize};

// ============================================================================
// Projects
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_task_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>, // on_track | at_risk | off_track
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>, // 0-4
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>, // planned | active | completed | paused
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,

    // Unified project type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>, // work | deal

    // Deal fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_solution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_expected_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_actual_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_proposal_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_order_form_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_lost_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_won_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stage_changed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stale_snoozed_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_contact_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_notes: Option<String>,

    // Nested data (from joins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statuses: Option<Vec<TaskStatus>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_count: Option<i32>,

    // Project sessions/artifacts/context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<ProjectSession>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<ProjectArtifact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ProjectContext>,

    // Deal nested data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<Box<Company>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<Contact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateProject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
    // Unified fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_solution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_expected_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    // Deal fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_solution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_expected_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_actual_close: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_proposal_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_order_form_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_lost_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_won_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_notes: Option<String>,
    /// Manually set deal_stage_changed_at (use with preserve_stage_date to override automatic update)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_stage_changed_at: Option<String>,
    /// If true, don't update deal_stage_changed_at when stage changes
    #[serde(skip_serializing)]
    pub preserve_stage_date: Option<bool>,
}

// ============================================================================
// Task Statuses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(rename = "type")]
    pub status_type: String, // backlog | unstarted | started | review | completed | canceled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

// ============================================================================
// Tasks
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub status_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_number: Option<i32>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>, // 0=none, 1=urgent, 2=high, 3=medium, 4=low
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    // Bot-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
    // CRM associations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>, // general, target, prospect, follow_up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type_changed_at: Option<String>,
    // Triage fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_score: Option<i32>, // 0-100 priority score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_action: Option<String>, // do_now, do_this_week, defer, delegate, kill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_triaged_at: Option<String>,
    // Notion sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notion_page_id: Option<String>,
    // Nested data (from joins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<Box<Project>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<TaskAssignee>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
}

// Junction table wrapper for task_assignees join
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignee {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub project_id: String,
    pub status_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing)]
    pub assignee_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing)]
    pub assignee_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    // Triage fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_triaged_at: Option<String>,
}

// ============================================================================
// Milestones
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMilestone {
    pub project_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMilestone {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
}

// ============================================================================
// Initiatives
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Initiative {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>, // planned | active | completed | paused
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>, // on_track | at_risk | off_track
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    // Nested data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<Project>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInitiative {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInitiative {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
}

// ============================================================================
// Labels
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabel {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================================
// Users
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    #[serde(rename = "type")]
    pub user_type: String, // human | bot
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    // Human fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_username: Option<String>,
    // Bot fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
}

// ============================================================================
// Project Updates
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUpdate {
    pub id: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectUpdate {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
}

// ============================================================================
// Initiative-Project Junction
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeProject {
    pub initiative_id: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

// ============================================================================
// Project Session/Artifact/Context
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSession {
    pub id: String,
    pub project_id: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decisions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_steps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectArtifact {
    pub id: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub reference: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_decisions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

// ============================================================================
// Skills
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_demo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_examples: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_deck: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_guide: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_work: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSkill {
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

// ============================================================================
// CRM Types (re-exported from work module for unified access)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referred_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub company_id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

// ============================================================================
// Pipeline Stats
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub stage: String,
    pub count: i32,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub by_stage: Vec<PipelineStage>,
    pub total_value: f64,
    pub total_deals: i32,
}

// ============================================================================
// Session/Artifact/Context create/update types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectSession {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decisions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_steps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectSession {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decisions: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_steps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectArtifact {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub reference: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertProjectContext {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_decisions: Option<serde_json::Value>,
}
