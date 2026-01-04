//! Shared LLM utilities for calling Claude CLI
//!
//! AIDEV-NOTE: Extracted from extract.rs and distill.rs to avoid duplication.
//! Both modules use the same pattern: call Claude CLI with a system prompt,
//! parse the response using text-based markers (HAS_KNOWLEDGE, HAS_RELEVANT, etc).

use crate::state;
use std::process::{Command, Stdio};

/// Result of calling the LLM with a marker-based response format
#[derive(Debug)]
pub struct MarkerResponse {
    /// Whether the marker indicated yes/true
    pub is_positive: bool,

    /// Content after the marker line (if positive)
    pub content: String,
}

/// Drop guard that restores an environment variable when dropped
/// AIDEV-NOTE: Ensures env vars are restored even if the LLM call panics or
/// if future code changes add early returns via `?`. More robust than manual cleanup.
/// Preserves original value if the env var was already set.
struct EnvGuard {
    var_name: &'static str,
    original_value: Option<String>,
}

impl EnvGuard {
    fn new(var_name: &'static str, value: &str) -> Self {
        // Capture original value before overwriting
        let original_value = std::env::var(var_name).ok();
        // SAFETY: Single-threaded, setting recursion prevention flag
        unsafe { std::env::set_var(var_name, value) };
        Self { var_name, original_value }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: Single-threaded, restoring previous state
        match &self.original_value {
            Some(val) => unsafe { std::env::set_var(self.var_name, val) },
            None => unsafe { std::env::remove_var(self.var_name) },
        }
    }
}

/// Call Claude CLI with a system prompt and message
///
/// Returns the raw result string from the Claude CLI JSON response.
/// Sets WM_DISABLED and SUPEREGO_DISABLED to prevent recursion.
pub fn call_claude(system_prompt: &str, message: &str) -> Result<String, String> {
    // Prevent recursion using drop guards - env vars are restored even on panic/early return
    let _wm_guard = EnvGuard::new("WM_DISABLED", "1");
    let _sg_guard = EnvGuard::new("SUPEREGO_DISABLED", "1");

    call_claude_inner(system_prompt, message)
}

/// Inner implementation of call_claude (without env var management)
fn call_claude_inner(system_prompt: &str, message: &str) -> Result<String, String> {
    state::log("llm", &format!("Calling Claude CLI (message: {} bytes)", message.len()));

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--no-session-persistence")
        .arg("--system-prompt")
        .arg(system_prompt)
        .arg(message)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn claude CLI: {}", e))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Claude CLI failed (exit {:?}):\nstderr: {}\nstdout: {}",
            output.status.code(),
            stderr,
            stdout
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse Claude CLI JSON wrapper to extract result field
    let cli_response: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse Claude CLI response: {}", e))?;

    cli_response
        .get("result")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "Claude CLI response missing 'result' field".to_string())
}

/// Parse a marker-based response (e.g., "HAS_KNOWLEDGE: YES\n<content>")
///
/// The marker format is: `MARKER_NAME: YES|NO|TRUE|FALSE`
/// If positive, content is everything after the marker line.
/// If no marker found, returns negative with empty content.
pub fn parse_marker_response(text: &str, marker_name: &str) -> MarkerResponse {
    let lines: Vec<&str> = text.lines().collect();
    let marker_prefix = format!("{}:", marker_name);

    for (i, line) in lines.iter().enumerate() {
        let stripped = strip_markdown_prefix(line);

        if let Some(value) = stripped.strip_prefix(&marker_prefix) {
            let value = value.trim().to_uppercase();
            if value == "YES" || value == "TRUE" {
                let content = lines[i + 1..].join("\n").trim().to_string();
                return MarkerResponse {
                    is_positive: true,
                    content,
                };
            }
            // NO or FALSE
            return MarkerResponse {
                is_positive: false,
                content: String::new(),
            };
        }
    }

    // Fallback: no marker found
    state::log(
        "llm",
        &format!("No {} marker found in response, treating as negative", marker_name),
    );
    MarkerResponse {
        is_positive: false,
        content: String::new(),
    }
}

/// Strip markdown prefixes from a line for lenient marker matching
/// AIDEV-NOTE: Copied from superego's pattern - LLMs sometimes wrap markers in markdown.
fn strip_markdown_prefix(line: &str) -> &str {
    line.trim().trim_start_matches(['#', '>', '*']).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_marker_yes() {
        let text = "HAS_KNOWLEDGE: YES\n- First insight\n- Second insight";
        let result = parse_marker_response(text, "HAS_KNOWLEDGE");
        assert!(result.is_positive);
        assert_eq!(result.content, "- First insight\n- Second insight");
    }

    #[test]
    fn test_parse_marker_no() {
        let text = "HAS_KNOWLEDGE: NO";
        let result = parse_marker_response(text, "HAS_KNOWLEDGE");
        assert!(!result.is_positive);
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_parse_marker_with_markdown() {
        let text = "## HAS_RELEVANT: TRUE\nSome content here";
        let result = parse_marker_response(text, "HAS_RELEVANT");
        assert!(result.is_positive);
        assert_eq!(result.content, "Some content here");
    }

    #[test]
    fn test_parse_marker_not_found() {
        let text = "No markers here";
        let result = parse_marker_response(text, "HAS_KNOWLEDGE");
        assert!(!result.is_positive);
        assert!(result.content.is_empty());
    }
}
