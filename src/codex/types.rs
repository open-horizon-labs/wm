//! Codex session entry types
//!
//! Codex sessions are JSONL files with entries of these types:
//! - session_meta: Session metadata (id, cwd, cli_version)
//! - event_msg: Events (user_message, agent_message, agent_reasoning, token_count)
//! - response_item: Response content (message, reasoning, function_call, function_call_output)
//! - turn_context: Turn boundary markers

use serde::{Deserialize, Serialize};

/// A single entry in the Codex transcript JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexEntry {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub payload: serde_json::Value,
}

#[allow(dead_code)]
impl CodexEntry {
    /// Check if this is a session metadata entry
    pub fn is_session_meta(&self) -> bool {
        self.entry_type == "session_meta"
    }

    /// Check if this is an event message
    pub fn is_event_msg(&self) -> bool {
        self.entry_type == "event_msg"
    }

    /// Check if this is a response item
    pub fn is_response_item(&self) -> bool {
        self.entry_type == "response_item"
    }

    /// Check if this is a turn context marker
    pub fn is_turn_context(&self) -> bool {
        self.entry_type == "turn_context"
    }

    /// Get the working directory from session_meta
    pub fn session_cwd(&self) -> Option<&str> {
        if self.is_session_meta() {
            self.payload.get("cwd").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Get the session ID from session_meta
    pub fn session_id(&self) -> Option<&str> {
        if self.is_session_meta() {
            self.payload.get("id").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Get the payload type for event_msg or response_item
    fn payload_type(&self) -> Option<&str> {
        self.payload.get("type").and_then(|v| v.as_str())
    }

    /// Check if this is a user message (event_msg with type=user_message)
    pub fn is_user_message(&self) -> bool {
        self.is_event_msg() && self.payload_type() == Some("user_message")
    }

    /// Check if this is an agent message (event_msg with type=agent_message)
    pub fn is_agent_message(&self) -> bool {
        self.is_event_msg() && self.payload_type() == Some("agent_message")
    }

    /// Check if this is agent reasoning (event_msg with type=agent_reasoning)
    pub fn is_agent_reasoning(&self) -> bool {
        self.is_event_msg() && self.payload_type() == Some("agent_reasoning")
    }

    /// Check if this is a token count event (skip these)
    pub fn is_token_count(&self) -> bool {
        self.is_event_msg() && self.payload_type() == Some("token_count")
    }

    /// Check if this is a function call (response_item with type=function_call)
    pub fn is_function_call(&self) -> bool {
        self.is_response_item() && self.payload_type() == Some("function_call")
    }

    /// Check if this is a function call output (response_item with type=function_call_output)
    pub fn is_function_call_output(&self) -> bool {
        self.is_response_item() && self.payload_type() == Some("function_call_output")
    }

    /// Check if this is a message response item
    pub fn is_message_item(&self) -> bool {
        self.is_response_item() && self.payload_type() == Some("message")
    }

    /// Check if this entry is relevant for knowledge extraction
    pub fn is_relevant(&self) -> bool {
        self.is_user_message()
            || self.is_agent_message()
            || self.is_agent_reasoning()
            || self.is_function_call()
            || self.is_function_call_output()
    }

    /// Extract user message text
    pub fn user_message_text(&self) -> Option<&str> {
        if self.is_user_message() {
            self.payload.get("message").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Extract agent message text
    pub fn agent_message_text(&self) -> Option<&str> {
        if self.is_agent_message() {
            self.payload.get("message").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Extract agent reasoning text
    pub fn agent_reasoning_text(&self) -> Option<&str> {
        if self.is_agent_reasoning() {
            self.payload.get("text").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Extract function call name
    pub fn function_call_name(&self) -> Option<&str> {
        if self.is_function_call() {
            self.payload.get("name").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Extract function call arguments (as string)
    pub fn function_call_args(&self) -> Option<&str> {
        if self.is_function_call() {
            self.payload.get("arguments").and_then(|v| v.as_str())
        } else {
            None
        }
    }

    /// Extract function call output
    pub fn function_call_output(&self) -> Option<String> {
        if self.is_function_call_output() {
            self.payload.get("output").map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_meta() {
        let json = r#"{"timestamp":"2025-11-04T00:16:00.093Z","type":"session_meta","payload":{"id":"test-id","cwd":"/test/path"}}"#;
        let entry: CodexEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_session_meta());
        assert_eq!(entry.session_cwd(), Some("/test/path"));
        assert_eq!(entry.session_id(), Some("test-id"));
    }

    #[test]
    fn test_parse_user_message() {
        let json = r#"{"timestamp":"2025-11-04T00:16:00.102Z","type":"event_msg","payload":{"type":"user_message","message":"Hello world","images":[]}}"#;
        let entry: CodexEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_user_message());
        assert_eq!(entry.user_message_text(), Some("Hello world"));
    }

    #[test]
    fn test_parse_agent_reasoning() {
        let json = r#"{"timestamp":"2025-11-04T00:16:08.855Z","type":"event_msg","payload":{"type":"agent_reasoning","text":"**Thinking about this**"}}"#;
        let entry: CodexEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_agent_reasoning());
        assert_eq!(entry.agent_reasoning_text(), Some("**Thinking about this**"));
    }

    #[test]
    fn test_parse_function_call() {
        let json = r#"{"timestamp":"2025-11-04T00:16:08.870Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"ls\"]}","call_id":"test"}}"#;
        let entry: CodexEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_function_call());
        assert_eq!(entry.function_call_name(), Some("shell"));
    }

    #[test]
    fn test_is_relevant() {
        let user_msg = r#"{"timestamp":"t","type":"event_msg","payload":{"type":"user_message","message":"test"}}"#;
        let token_count = r#"{"timestamp":"t","type":"event_msg","payload":{"type":"token_count","info":null}}"#;

        let user_entry: CodexEntry = serde_json::from_str(user_msg).unwrap();
        let token_entry: CodexEntry = serde_json::from_str(token_count).unwrap();

        assert!(user_entry.is_relevant());
        assert!(!token_entry.is_relevant());
    }
}
