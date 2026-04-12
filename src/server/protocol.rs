// MCP Protocol Types
// JSON-RPC 2.0 message types for the Model Context Protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// JSON-RPC Types
// ============================================================================

/// JSON-RPC request from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC response to Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// Standard JSON-RPC error codes
#[allow(dead_code)]
pub const PARSE_ERROR: i32 = -32700;
#[allow(dead_code)]
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
#[allow(dead_code)]
pub const INVALID_PARAMS: i32 = -32602;
#[allow(dead_code)]
pub const INTERNAL_ERROR: i32 = -32603;

// ============================================================================
// MCP Protocol Types
// ============================================================================

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: InputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl InputSchema {
    pub fn empty() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
        }
    }

    pub fn with_properties(properties: Value, required: Vec<String>) -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: if required.is_empty() { None } else { Some(required) },
        }
    }
}

/// Tool call parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    pub fn text(text: String) -> Self {
        Self {
            content: vec![Content::text(text)],
            is_error: None,
        }
    }

    pub fn json<T: Serialize>(value: &T) -> Self {
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
        Self::text(text)
    }

    pub fn error(message: String) -> Self {
        Self {
            content: vec![Content::text(message)],
            is_error: Some(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl Content {
    pub fn text(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
        }
    }
}

/// List tools result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -------------------------------------------------------
    // JsonRpcResponse
    // -------------------------------------------------------

    #[test]
    fn success_response_has_result_and_no_error() {
        let resp = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(json!(1)));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn error_response_has_error_and_no_result() {
        let resp = JsonRpcResponse::error(Some(json!(2)), METHOD_NOT_FOUND, "not found");
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, METHOD_NOT_FOUND);
        assert_eq!(err.message, "not found");
    }

    #[test]
    fn success_response_serializes_without_error_field() {
        let resp = JsonRpcResponse::success(Some(json!(1)), json!("ok"));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn error_response_serializes_without_result_field() {
        let resp = JsonRpcResponse::error(None, INTERNAL_ERROR, "boom");
        let s = serde_json::to_string(&resp).unwrap();
        assert!(!s.contains("\"result\""));
    }

    // -------------------------------------------------------
    // InputSchema
    // -------------------------------------------------------

    #[test]
    fn empty_schema_has_object_type_and_no_properties() {
        let schema = InputSchema::empty();
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_none());
        assert!(schema.required.is_none());
    }

    #[test]
    fn with_properties_stores_required_fields() {
        let schema = InputSchema::with_properties(
            json!({"name": {"type": "string"}}),
            vec!["name".to_string()],
        );
        assert!(schema.properties.is_some());
        assert_eq!(schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn with_properties_empty_required_becomes_none() {
        let schema = InputSchema::with_properties(json!({}), vec![]);
        assert!(schema.required.is_none());
    }

    // -------------------------------------------------------
    // ToolResult
    // -------------------------------------------------------

    #[test]
    fn tool_result_text_is_not_error() {
        let r = ToolResult::text("hello".to_string());
        assert!(r.is_error.is_none());
        assert_eq!(r.content.len(), 1);
        assert_eq!(r.content[0].text, "hello");
        assert_eq!(r.content[0].content_type, "text");
    }

    #[test]
    fn tool_result_error_is_flagged() {
        let r = ToolResult::error("bad".to_string());
        assert_eq!(r.is_error, Some(true));
        assert_eq!(r.content[0].text, "bad");
    }

    #[test]
    fn tool_result_json_serializes_value() {
        let data = vec!["a", "b"];
        let r = ToolResult::json(&data);
        assert!(r.is_error.is_none());
        let parsed: Vec<String> = serde_json::from_str(&r.content[0].text).unwrap();
        assert_eq!(parsed, vec!["a", "b"]);
    }

    // -------------------------------------------------------
    // Round-trip serialization
    // -------------------------------------------------------

    #[test]
    fn jsonrpc_request_round_trips() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(42)),
            method: "tools/call".to_string(),
            params: Some(json!({"name": "list-crm-companies"})),
        };
        let s = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.method, "tools/call");
        assert_eq!(parsed.id, Some(json!(42)));
    }

    #[test]
    fn tool_definition_round_trips() {
        let tool = Tool {
            name: "test-tool".to_string(),
            description: "A test".to_string(),
            input_schema: InputSchema::with_properties(
                json!({"x": {"type": "integer"}}),
                vec!["x".to_string()],
            ),
        };
        let s = serde_json::to_string(&tool).unwrap();
        let parsed: Tool = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.name, "test-tool");
        assert!(s.contains("inputSchema")); // camelCase
    }

    #[test]
    fn tool_call_params_parses_from_json() {
        let json_str = r#"{"name": "list-tasks", "arguments": {"project_id": "abc"}}"#;
        let params: ToolCallParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(params.name, "list-tasks");
        assert_eq!(params.arguments["project_id"], "abc");
    }

    #[test]
    fn tool_call_params_defaults_arguments_to_null() {
        let json_str = r#"{"name": "list-tasks"}"#;
        let params: ToolCallParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(params.name, "list-tasks");
        assert!(params.arguments.is_null());
    }
}
