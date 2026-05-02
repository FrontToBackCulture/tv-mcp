// VAL Drive MCP Tools — file/folder operations on the S3-backed Drive layer.
// 7 tools: list-val-drive-folders, list-val-drive-files,
//          check-val-drive-files-all-domains, check-val-drive-file-exists,
//          create-val-drive-folder, rename-val-drive-file, move-val-drive-file.
//
// No delete-* tools by design (policy). Async upload + bulk download skipped
// (binary streams don't fit MCP JSON cleanly).

use crate::modules::val_sync::drive;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

macro_rules! require_str {
    ($args:expr, $key:expr) => {
        match $args.get($key).and_then(|v| v.as_str()) {
            Some(v) => v.to_string(),
            None => {
                return ToolResult::error(format!("'{}' parameter is required", $key));
            }
        }
    };
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list-val-drive-folders".to_string(),
            description:
                "List folders in a VAL Drive path. Defaults to the `val_drive` root if no \
                 `folder_id` is given. Use this to discover the folder tree before drilling into \
                 files via `list-val-drive-files`."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "folder_id": {
                        "type": "string",
                        "description": "Optional parent folder id/path. Defaults to 'val_drive' (root)."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "list-val-drive-files".to_string(),
            description:
                "List files and subfolders in a VAL Drive path for a domain. Shows file names, \
                 sizes, and ages. Use to check for unprocessed files or verify Drive uploads."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": {
                        "type": "string",
                        "description": "VAL domain name (e.g., 'koi', 'suntec')"
                    },
                    "folder": {
                        "type": "string",
                        "description": "Folder path in Drive (e.g., 'val_drive/RevRec/01_SourceReports'). Defaults to 'val_drive'."
                    }
                }),
                vec!["domain".to_string()],
            ),
        },
        Tool {
            name: "check-val-drive-files-all-domains".to_string(),
            description:
                "Sweep VAL Drive across ALL production domains. Scans each domain's val_drive \
                 folders recursively and reports unprocessed files with their age. Files older \
                 than 24h are flagged as stale. Use for morning SOD checks or to verify Drive \
                 uploads are being processed."
                    .to_string(),
            input_schema: InputSchema::empty(),
        },
        Tool {
            name: "check-val-drive-file-exists".to_string(),
            description:
                "Check whether a file exists at a given Drive path. Cheap precondition before \
                 workflows that depend on a specific upload."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "path": {
                        "type": "string",
                        "description": "Full Drive file path (e.g., 'val_drive/RevRec/01_SourceReports/jul-2025.csv')"
                    }
                }),
                vec!["domain".to_string(), "path".to_string()],
            ),
        },
        Tool {
            name: "create-val-drive-folder".to_string(),
            description:
                "Create a new subfolder under an existing Drive folder. Provide the parent path \
                 and the new folder's name; tv-mcp constructs the val-services request shape."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "parent_folder_id": {
                        "type": "string",
                        "description": "Parent folder path (e.g., 'val_drive' or 'val_drive/RevRec')."
                    },
                    "name": {
                        "type": "string",
                        "description": "New folder name (leaf, not a path)."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "parent_folder_id".to_string(),
                    "name".to_string(),
                ],
            ),
        },
        Tool {
            name: "rename-val-drive-file".to_string(),
            description:
                "Rename a Drive file in place. Does not move across folders — use \
                 `move-val-drive-file` for that. Provide the parent folder path, the current file \
                 name, and the new name."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "parent_folder_id": {
                        "type": "string",
                        "description": "Parent folder path (e.g., 'val_drive/RevRec/01_SourceReports')."
                    },
                    "file_name": {
                        "type": "string",
                        "description": "Current file name (leaf, not a full path)."
                    },
                    "new_name": {
                        "type": "string",
                        "description": "New file name."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "parent_folder_id".to_string(),
                    "file_name".to_string(),
                    "new_name".to_string(),
                ],
            ),
        },
        Tool {
            name: "move-val-drive-file".to_string(),
            description:
                "Move a Drive file or folder into another folder. Paths can be given with or \
                 without the `val_drive/` prefix — val-services adds it automatically."
                    .to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "domain": { "type": "string", "description": "VAL domain name" },
                    "source_parent": {
                        "type": "string",
                        "description": "Parent folder of the source (e.g., 'val_drive/RevRec/01_SourceReports' or just 'RevRec/01_SourceReports')."
                    },
                    "source_name": {
                        "type": "string",
                        "description": "Source file or folder name (leaf)."
                    },
                    "target_parent": {
                        "type": "string",
                        "description": "Destination folder path (e.g., 'val_drive/Archive' or 'Archive')."
                    },
                    "type": {
                        "type": "string",
                        "enum": ["file", "folder"],
                        "description": "Whether the source is a 'file' or 'folder'."
                    }
                }),
                vec![
                    "domain".to_string(),
                    "source_parent".to_string(),
                    "source_name".to_string(),
                    "target_parent".to_string(),
                    "type".to_string(),
                ],
            ),
        },
    ]
}

pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "list-val-drive-folders" => {
            let domain = require_str!(args, "domain");
            let folder_id = args.get("folder_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            match drive::val_drive_list_folders(domain, folder_id).await {
                Ok(folders) => match serde_json::to_value(&folders) {
                    Ok(v) => ToolResult::json(&v),
                    Err(e) => ToolResult::error(format!("serialize: {}", e)),
                },
                Err(e) => ToolResult::error(format!("list-val-drive-folders failed: {}", e)),
            }
        }

        "list-val-drive-files" => {
            let domain = require_str!(args, "domain");
            let folder = args
                .get("folder")
                .and_then(|v| v.as_str())
                .unwrap_or("val_drive")
                .to_string();
            crate::server::tools::val_sync::handle_list_drive_files_public(&domain, &folder).await
        }

        "check-val-drive-files-all-domains" => {
            crate::server::tools::val_sync::handle_check_all_domain_drive_files_public().await
        }

        "check-val-drive-file-exists" => {
            let domain = require_str!(args, "domain");
            let path = require_str!(args, "path");
            match drive::val_drive_check_file_exists(domain, path).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("check-val-drive-file-exists failed: {}", e)),
            }
        }

        "create-val-drive-folder" => {
            let domain = require_str!(args, "domain");
            let parent = require_str!(args, "parent_folder_id");
            let name = require_str!(args, "name");
            match drive::val_drive_create_folder(domain, parent, name).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("create-val-drive-folder failed: {}", e)),
            }
        }

        "rename-val-drive-file" => {
            let domain = require_str!(args, "domain");
            let parent = require_str!(args, "parent_folder_id");
            let file_name = require_str!(args, "file_name");
            let new_name = require_str!(args, "new_name");
            match drive::val_drive_rename_file(domain, parent, file_name, new_name).await {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("rename-val-drive-file failed: {}", e)),
            }
        }

        "move-val-drive-file" => {
            let domain = require_str!(args, "domain");
            let source_parent = require_str!(args, "source_parent");
            let source_name = require_str!(args, "source_name");
            let target_parent = require_str!(args, "target_parent");
            let item_type = require_str!(args, "type");
            match drive::val_drive_move_file(
                domain,
                source_parent,
                source_name,
                target_parent,
                item_type,
            )
            .await
            {
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(format!("move-val-drive-file failed: {}", e)),
            }
        }

        _ => ToolResult::error(format!("Unknown val_drive tool: {}", name)),
    }
}
