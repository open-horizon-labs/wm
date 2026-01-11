//! Codex transcript reader
//!
//! Reads and parses Codex JSONL session files, formats for LLM extraction.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::codex::types::CodexEntry;

/// Error type for Codex transcript reading
#[derive(Debug)]
pub enum CodexReadError {
    IoError(std::io::Error),
}

impl std::fmt::Display for CodexReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodexReadError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for CodexReadError {}

impl From<std::io::Error> for CodexReadError {
    fn from(e: std::io::Error) -> Self {
        CodexReadError::IoError(e)
    }
}

/// Read and parse a Codex session JSONL file
///
/// Skips malformed lines rather than failing entirely (graceful failure).
pub fn read_codex_session(path: &Path) -> Result<Vec<CodexEntry>, CodexReadError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<CodexEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                // Log warning but continue - don't fail on malformed lines
                eprintln!(
                    "Warning: skipping malformed line {} in Codex session: {}",
                    line_num + 1,
                    e
                );
            }
        }
    }

    Ok(entries)
}

/// Format Codex entries for context extraction (for sending to extraction LLM)
///
/// Formats relevant entries into a human-readable transcript similar to
/// the Claude Code format_context function.
pub fn format_context(entries: &[CodexEntry]) -> String {
    let mut output = String::new();

    for entry in entries {
        if !entry.is_relevant() {
            continue;
        }

        if let Some(text) = entry.user_message_text() {
            let cleaned = strip_environment_context(text);
            if !cleaned.is_empty() {
                output.push_str("USER: ");
                output.push_str(&cleaned);
                output.push_str("\n\n");
            }
        } else if let Some(text) = entry.agent_message_text() {
            if !text.is_empty() {
                output.push_str("ASSISTANT: ");
                output.push_str(text);
                output.push_str("\n\n");
            }
        } else if let Some(text) = entry.agent_reasoning_text() {
            if !text.is_empty() {
                output.push_str("THINKING: ");
                output.push_str(text);
                output.push_str("\n\n");
            }
        } else if entry.is_function_call() {
            if let Some(name) = entry.function_call_name() {
                output.push_str("TOOL: ");
                output.push_str(name);

                // Add summary of arguments for context
                if let Some(args) = entry.function_call_args() {
                    let summary = summarize_tool_args(name, args);
                    if !summary.is_empty() {
                        output.push('(');
                        output.push_str(&summary);
                        output.push(')');
                    }
                }
                output.push_str("\n");
            }
        } else if entry.is_function_call_output() {
            if let Some(output_text) = entry.function_call_output() {
                // Truncate very long outputs (respecting UTF-8 boundaries)
                let truncated = if output_text.len() > 500 {
                    let truncate_at = output_text
                        .char_indices()
                        .take_while(|(i, _)| *i < 500)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(0);
                    format!("{}...[truncated]", &output_text[..truncate_at])
                } else {
                    output_text
                };
                output.push_str("TOOL_RESULT: ");
                output.push_str(&truncated);
                output.push_str("\n\n");
            }
        }
    }

    output
}

/// Strip <environment_context>...</environment_context> blocks from user messages
/// These contain cwd, sandbox settings, etc. - not relevant for knowledge extraction.
fn strip_environment_context(text: &str) -> String {
    const OPEN: &str = "<environment_context>";
    const CLOSE: &str = "</environment_context>";

    let mut result = String::with_capacity(text.len());
    let mut search_start = 0;

    while let Some(open_offset) = text[search_start..].find(OPEN) {
        let open_pos = search_start + open_offset;
        result.push_str(&text[search_start..open_pos]);

        let after_open = open_pos + OPEN.len();
        if let Some(close_offset) = text[after_open..].find(CLOSE) {
            search_start = after_open + close_offset + CLOSE.len();
        } else {
            search_start = text.len();
            break;
        }
    }

    result.push_str(&text[search_start..]);
    result.trim().to_string()
}

/// Summarize tool arguments for context
fn summarize_tool_args(tool_name: &str, args: &str) -> String {
    // Parse args JSON and extract key info
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(args);
    let args_obj = match parsed {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    match tool_name {
        "shell" => {
            // Extract command from shell tool
            args_obj
                .get("command")
                .and_then(|c| {
                    if let Some(arr) = c.as_array() {
                        // Command is array like ["zsh", "-lc", "actual command"]
                        arr.last().and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        c.as_str().map(|s| s.to_string())
                    }
                })
                .unwrap_or_default()
        }
        "read_file" | "write_file" => args_obj
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "edit_file" => args_obj
            .get("target_file")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_environment_context() {
        let text = "<environment_context>\n  <cwd>/test</cwd>\n</environment_context>\n\nActual message";
        let cleaned = strip_environment_context(text);
        assert_eq!(cleaned, "Actual message");
    }

    #[test]
    fn test_strip_environment_context_no_tag() {
        let text = "Just a normal message";
        let cleaned = strip_environment_context(text);
        assert_eq!(cleaned, "Just a normal message");
    }

    #[test]
    fn test_summarize_shell_args() {
        let args = r#"{"command":["zsh","-lc","ls -la"],"workdir":"/test"}"#;
        let summary = summarize_tool_args("shell", args);
        assert_eq!(summary, "ls -la");
    }

    #[test]
    fn test_format_context_basic() {
        let entries = vec![
            serde_json::from_str::<CodexEntry>(
                r#"{"timestamp":"t","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#,
            )
            .unwrap(),
            serde_json::from_str::<CodexEntry>(
                r#"{"timestamp":"t","type":"event_msg","payload":{"type":"agent_message","message":"Hi there"}}"#,
            )
            .unwrap(),
        ];

        let formatted = format_context(&entries);
        assert!(formatted.contains("USER: Hello"));
        assert!(formatted.contains("ASSISTANT: Hi there"));
    }
}
