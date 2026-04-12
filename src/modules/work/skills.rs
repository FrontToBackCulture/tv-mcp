// Work Module - Skill Registry Commands

use super::types::*;
use crate::core::error::CmdResult;
use crate::core::supabase::get_client;

/// Register or update a skill in the registry (upsert on slug)

pub async fn work_register_skill(data: RegisterSkill) -> CmdResult<Skill> {
    let client = get_client().await?;

    client.upsert_on("skills", &data, Some("slug")).await
}

/// List all skills in the registry

pub async fn work_list_skills() -> CmdResult<Vec<Skill>> {
    let client = get_client().await?;

    client.select("skills", "order=name.asc").await
}
