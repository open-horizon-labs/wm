//! Core data types for WM

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

// =============================================================================
// Shared Utilities
// =============================================================================

/// Convert SystemTime to DateTime<Utc>
///
/// Shared utility used by session discovery in both Claude Code and Codex modules.
pub fn system_time_to_datetime(st: SystemTime) -> Option<DateTime<Utc>> {
    let duration = st.duration_since(std::time::UNIX_EPOCH).ok()?;
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
}

/// Strip all occurrences of an XML-style tag from text
///
/// Used by both transcript readers to strip context/reminder tags.
/// Example: `strip_xml_tags(text, "<system-reminder>", "</system-reminder>")`
pub fn strip_xml_tags(text: &str, open_tag: &str, close_tag: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut search_start = 0;

    while let Some(open_offset) = text[search_start..].find(open_tag) {
        let open_pos = search_start + open_offset;
        result.push_str(&text[search_start..open_pos]);

        let after_open = open_pos + open_tag.len();
        if let Some(close_offset) = text[after_open..].find(close_tag) {
            search_start = after_open + close_offset + close_tag.len();
        } else {
            search_start = text.len();
            break;
        }
    }

    result.push_str(&text[search_start..]);
    result.trim().to_string()
}

// =============================================================================
// Shared Error Type
// =============================================================================

/// Unified error type for transcript/session reading
///
/// Replaces both TranscriptError and CodexReadError.
#[derive(Debug)]
pub enum ReadError {
    Io(std::io::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        ReadError::Io(e)
    }
}

// =============================================================================
// Session Types
// =============================================================================

/// Claude Code session info
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub transcript_path: PathBuf,
    pub modified_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Codex session info
#[derive(Debug, Clone)]
pub struct CodexSessionInfo {
    pub session_id: String,
    pub session_path: PathBuf,
    pub cwd: Option<String>,
    pub modified_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Hook-specific output for UserPromptSubmit hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook response format (matches Claude Code expectations)
/// AIDEV-NOTE: Must include hookSpecificOutput wrapper for additionalContext to be injected.
/// Plain {"additionalContext":"..."} does NOT work - Claude Code ignores it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Project-level configuration for WM operations
/// Stored in .wm/config.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub operations: OperationsConfig,

    #[serde(default)]
    pub dive: DiveConfig,
}

/// Configuration for named dive preps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiveConfig {
    /// Name of the currently active dive prep (None = use legacy dive_context.md)
    pub current: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationsConfig {
    #[serde(default = "default_true")]
    pub extract: bool,

    #[serde(default = "default_true")]
    pub compile: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OperationsConfig {
    fn default() -> Self {
        Self {
            extract: true,
            compile: true,
        }
    }
}
