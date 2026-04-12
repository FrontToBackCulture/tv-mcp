// Document Generation MCP Tools
// Generate PDFs for order forms and proposals

use crate::modules::tools::docgen;
use crate::server::protocol::{InputSchema, Tool, ToolResult};
use serde_json::{json, Value};

/// Define document generation tools
pub fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "generate-order-form".to_string(),
            description: "Generate a PDF order form from an order-form-data.md file. The markdown file should contain YAML blocks with order details, customer info, and payment schedules.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "filePath": {
                        "type": "string",
                        "description": "Path to the order-form-data.md file (required)"
                    },
                    "outputPath": {
                        "type": "string",
                        "description": "Output path for the PDF (optional, defaults to same directory with .pdf extension)"
                    }
                }),
                vec!["filePath".to_string()],
            ),
        },
        Tool {
            name: "generate-proposal".to_string(),
            description: "Generate a PDF proposal from a proposal-data.md file. The markdown file should contain YAML blocks with proposal details and content sections.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "filePath": {
                        "type": "string",
                        "description": "Path to the proposal-data.md file (required)"
                    },
                    "outputPath": {
                        "type": "string",
                        "description": "Output path for the PDF (optional, defaults to same directory with .pdf extension)"
                    }
                }),
                vec!["filePath".to_string()],
            ),
        },
        Tool {
            name: "check-document-type".to_string(),
            description: "Check if a file is an order form or proposal data file that can be exported to PDF.".to_string(),
            input_schema: InputSchema::with_properties(
                json!({
                    "filePath": {
                        "type": "string",
                        "description": "Path to check"
                    }
                }),
                vec!["filePath".to_string()],
            ),
        },
    ]
}

/// Call a document generation tool
pub async fn call(name: &str, args: Value) -> ToolResult {
    match name {
        "generate-order-form" => {
            let file_path = match args.get("filePath").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => return ToolResult::error("filePath is required".to_string()),
            };

            let output_path = args.get("outputPath").and_then(|v| v.as_str());

            match docgen::generate_order_form_from_file(file_path, output_path) {
                Ok(path) => ToolResult::json(&json!({
                    "success": true,
                    "outputPath": path,
                    "message": format!("Order form PDF generated: {}", path)
                })),
                Err(e) => ToolResult::error(format!("Failed to generate order form: {}", e)),
            }
        }

        "generate-proposal" => {
            let file_path = match args.get("filePath").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => return ToolResult::error("filePath is required".to_string()),
            };

            let output_path = args.get("outputPath").and_then(|v| v.as_str());

            match docgen::generate_proposal_from_file(file_path, output_path) {
                Ok(path) => ToolResult::json(&json!({
                    "success": true,
                    "outputPath": path,
                    "message": format!("Proposal PDF generated: {}", path)
                })),
                Err(e) => ToolResult::error(format!("Failed to generate proposal: {}", e)),
            }
        }

        "check-document-type" => {
            let file_path = match args.get("filePath").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => return ToolResult::error("filePath is required".to_string()),
            };

            let is_order_form = docgen::is_order_form_file(file_path);
            let is_proposal = docgen::is_proposal_file(file_path);

            let doc_type = if is_order_form {
                "order-form"
            } else if is_proposal {
                "proposal"
            } else {
                "none"
            };

            ToolResult::json(&json!({
                "filePath": file_path,
                "documentType": doc_type,
                "canExportToPdf": is_order_form || is_proposal,
                "exportTool": if is_order_form {
                    "generate-order-form"
                } else if is_proposal {
                    "generate-proposal"
                } else {
                    ""
                }
            }))
        }

        _ => ToolResult::error(format!("Unknown document tool: {}", name)),
    }
}
