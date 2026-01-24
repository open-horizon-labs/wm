use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::transcript::types::TranscriptEntry;
use crate::types::{strip_xml_tags, ReadError};

/// Read and parse a transcript JSONL file
///
/// Skips malformed lines rather than failing entirely
pub fn read_transcript(path: &Path) -> Result<Vec<TranscriptEntry>, ReadError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<TranscriptEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                // Log warning but continue - don't fail on malformed lines
                eprintln!(
                    "Warning: skipping malformed line {} in transcript: {}",
                    line_num + 1,
                    e
                );
            }
        }
    }

    Ok(entries)
}

/// Get messages in a time window, optionally filtered by session
pub fn get_messages_in_window<'a>(
    entries: &'a [TranscriptEntry],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    session_id: Option<&str>,
) -> Vec<&'a TranscriptEntry> {
    let session_filter = |e: &&TranscriptEntry| -> bool {
        match session_id {
            Some(sid) => e.session_id() == Some(sid),
            None => true,
        }
    };

    let content_filter = |e: &&TranscriptEntry| e.is_message() || e.is_summary();

    entries
        .iter()
        .filter(content_filter)
        .filter(session_filter)
        .filter(|e| {
            e.timestamp()
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|ts| ts >= start && ts < end)
                .unwrap_or(false)
        })
        .collect()
}

/// Get messages since a given timestamp, optionally filtered by session
/// AIDEV-NOTE: This is the primary context selection method for wm extraction.
/// When session_id is provided, only messages from that session are included
/// to prevent cross-session context bleed.
pub fn get_messages_since<'a>(
    entries: &'a [TranscriptEntry],
    since: Option<DateTime<Utc>>,
    session_id: Option<&str>,
) -> Vec<&'a TranscriptEntry> {
    let session_filter = |e: &&TranscriptEntry| -> bool {
        match session_id {
            Some(sid) => e.session_id() == Some(sid),
            None => true, // No session filter - include all (backward compat)
        }
    };

    // Include messages AND summaries (summaries provide context after compaction)
    let content_filter = |e: &&TranscriptEntry| e.is_message() || e.is_summary();

    match since {
        Some(cutoff) => {
            entries
                .iter()
                .filter(content_filter)
                .filter(session_filter)
                .filter(|e| {
                    // Include if timestamp is after cutoff (or if no timestamp)
                    // Summaries don't have timestamps, so they pass through
                    e.timestamp()
                        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                        .map(|ts| ts > cutoff)
                        .unwrap_or(true)
                })
                .collect()
        }
        None => {
            // No previous extraction - include all messages + summaries (for this session)
            entries
                .iter()
                .filter(content_filter)
                .filter(session_filter)
                .collect()
        }
    }
}

/// Strip ALL <system-reminder>...</system-reminder> blocks
/// AIDEV-NOTE: System reminders contain CLAUDE.md content - already explicit instructions,
/// not tacit knowledge. For extraction, we strip them entirely to avoid redundant capture.
fn strip_system_reminders(text: &str) -> String {
    strip_xml_tags(text, "<system-reminder>", "</system-reminder>")
}

/// Extract key identifier from tool input (file path, command, pattern)
fn tool_summary(name: &str, input: Option<&serde_json::Value>) -> String {
    let input = match input {
        Some(v) => v,
        None => return String::new(),
    };
    match name {
        "Edit" | "Write" | "Read" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Glob" | "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

/// Format messages for context (for sending to extraction LLM)
pub fn format_context(messages: &[&TranscriptEntry]) -> String {
    let mut output = String::new();

    for entry in messages {
        match entry {
            TranscriptEntry::Summary { .. } => {
                if let Some(text) = entry.summary_text() {
                    output.push_str("SUMMARY: ");
                    output.push_str(text);
                    output.push_str("\n\n");
                }
            }
            TranscriptEntry::User { .. } => {
                // Include tool results (what Claude read/executed)
                let tool_results = entry.tool_results();
                if !tool_results.is_empty() {
                    for (_id, content) in &tool_results {
                        output.push_str("TOOL_RESULT: ");
                        output.push_str(content);
                        output.push_str("\n\n");
                    }
                }

                if let Some(text) = entry.user_text() {
                    let cleaned = strip_system_reminders(&text);
                    if !cleaned.is_empty() {
                        output.push_str("USER: ");
                        output.push_str(&cleaned);
                        output.push_str("\n\n");
                    }
                }
            }
            TranscriptEntry::Assistant { .. } => {
                let tool_uses = entry.tool_uses();

                // Include thinking if present (shows Claude's reasoning)
                if let Some(thinking) = entry.assistant_thinking() {
                    output.push_str("THINKING: ");
                    output.push_str(&thinking);
                    output.push_str("\n\n");
                }

                if !tool_uses.is_empty() {
                    output.push_str("TOOLS: ");
                    for (name, input) in &tool_uses {
                        output.push_str(name);
                        let summary = tool_summary(name, *input);
                        if !summary.is_empty() {
                            output.push('(');
                            output.push_str(&summary);
                            output.push(')');
                        }
                        output.push(' ');
                    }
                    output.push('\n');
                }

                if let Some(text) = entry.assistant_text() {
                    output.push_str("ASSISTANT: ");
                    output.push_str(&text);
                    output.push_str("\n\n");
                } else if !tool_uses.is_empty() {
                    output.push('\n');
                }
            }
            _ => {}
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_entry() {
        let json = r#"{"type":"user","uuid":"abc","parentUuid":null,"sessionId":"sess-1","timestamp":"2025-01-15T10:00:00Z","message":{"role":"user","content":"hello"}}"#;
        let entry: TranscriptEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_user());
        assert_eq!(entry.session_id(), Some("sess-1"));
        assert_eq!(entry.user_text(), Some("hello".to_string()));
    }

    #[test]
    fn test_parse_assistant_entry() {
        let json = r#"{"type":"assistant","uuid":"def","parentUuid":"abc","sessionId":"sess-1","timestamp":"2025-01-15T10:00:01Z","message":{"role":"assistant","content":[{"type":"text","text":"hi there"}]}}"#;
        let entry: TranscriptEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_assistant());
        assert_eq!(entry.assistant_text(), Some("hi there".to_string()));
    }

    #[test]
    fn test_parse_unknown_type() {
        let json = r#"{"type":"some-new-type","data":"whatever"}"#;
        let entry: TranscriptEntry = serde_json::from_str(json).unwrap();
        assert!(matches!(entry, TranscriptEntry::Unknown));
    }

    #[test]
    fn test_strip_system_reminders_single() {
        let text = "Hello <system-reminder>workflow stuff</system-reminder> world";
        assert_eq!(strip_system_reminders(text), "Hello  world");
    }

    #[test]
    fn test_strip_system_reminders_multiple() {
        let text = "<system-reminder>first</system-reminder>content<system-reminder>second</system-reminder>";
        assert_eq!(strip_system_reminders(text), "content");
    }

    #[test]
    fn test_strip_system_reminders_none() {
        let text = "Just normal text";
        assert_eq!(strip_system_reminders(text), "Just normal text");
    }

    #[test]
    fn test_session_filtering() {
        let msg_s1 = r#"{"type":"user","uuid":"a","sessionId":"s1","timestamp":"2025-01-15T10:00:00Z","message":{"role":"user","content":"Session 1"}}"#;
        let msg_s2 = r#"{"type":"user","uuid":"b","sessionId":"s2","timestamp":"2025-01-15T10:00:00Z","message":{"role":"user","content":"Session 2"}}"#;

        let entries: Vec<TranscriptEntry> = vec![
            serde_json::from_str(msg_s1).unwrap(),
            serde_json::from_str(msg_s2).unwrap(),
        ];

        // Filter by session s1
        let result = get_messages_since(&entries, None, Some("s1"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_text(), Some("Session 1".to_string()));

        // Filter by session s2
        let result = get_messages_since(&entries, None, Some("s2"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_text(), Some("Session 2".to_string()));

        // No filter - get both
        let result = get_messages_since(&entries, None, None);
        assert_eq!(result.len(), 2);
    }
}
