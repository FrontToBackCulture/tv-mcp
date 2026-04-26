// MCP Tools Registry — discovery + metadata for tv-mcp tools
// Syncs the in-process tool catalog into the Supabase `mcp_tools` table.

pub mod sync;
pub mod types;

pub use sync::{sync_mcp_tools, list_mcp_tools};
