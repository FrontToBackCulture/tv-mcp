// Work Module - User Commands

use super::types::*;
use crate::core::error::{CmdResult, CommandError};
use crate::core::supabase::get_client;

/// List all users (humans and bots)

pub async fn work_list_users() -> CmdResult<Vec<User>> {
    let client = get_client().await?;

    client.select("users", "order=name.asc").await
}

/// List only human users

pub async fn work_list_humans() -> CmdResult<Vec<User>> {
    let client = get_client().await?;

    client.select("users", "type=eq.human&order=name.asc").await
}

/// List only bot users

pub async fn work_list_bots() -> CmdResult<Vec<User>> {
    let client = get_client().await?;

    client.select("users", "type=eq.bot&order=name.asc").await
}

/// Get a single user by ID

pub async fn work_get_user(user_id: String) -> CmdResult<User> {
    let client = get_client().await?;

    let query = format!("id=eq.{}", user_id);

    client
        .select_single("users", &query)
        .await?
        .ok_or_else(|| CommandError::NotFound(format!("User not found: {}", user_id)))
}

/// Find user by email

pub async fn work_find_user_by_email(email: String) -> CmdResult<Option<User>> {
    let client = get_client().await?;

    let query = format!("email=eq.{}", email);
    client.select_single("users", &query).await
}

/// Find user by GitHub username

pub async fn work_find_user_by_github(github_username: String) -> CmdResult<Option<User>> {
    let client = get_client().await?;

    let query = format!("github_username=eq.{}", github_username);
    client.select_single("users", &query).await
}

/// Find bot by folder ID

pub async fn work_find_bot_by_folder(bot_folder_id: String) -> CmdResult<Option<User>> {
    let client = get_client().await?;

    let query = format!("type=eq.bot&bot_folder_id=eq.{}", bot_folder_id);
    client.select_single("users", &query).await
}
