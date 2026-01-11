//! Codex session discovery
//!
//! Discovers Codex sessions stored in ~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl
//! Unlike Claude Code (which uses project-id directories), Codex sessions embed
//! the cwd in session_meta, so we need to read each file to filter by project.

use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::codex::types::CodexEntry;

/// Information about a discovered Codex session
#[derive(Debug, Clone)]
pub struct CodexSessionInfo {
    /// Session ID (from session_meta or filename)
    pub session_id: String,

    /// Full path to the session JSONL file
    pub session_path: PathBuf,

    /// Working directory where the session ran
    pub cwd: Option<String>,

    /// Last modification time
    pub modified_at: DateTime<Utc>,

    /// File size in bytes
    pub size_bytes: u64,
}

/// Get the Codex sessions root directory (~/.codex/sessions/)
pub fn codex_sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".codex").join("sessions"))
}

/// Discover all Codex sessions, optionally filtered by project path
///
/// If `project_filter` is Some, only sessions where cwd contains the filter string are returned.
/// Returns sessions sorted by modification time (newest first).
pub fn discover_sessions(project_filter: Option<&str>) -> Result<Vec<CodexSessionInfo>, String> {
    let sessions_dir = codex_sessions_dir()
        .ok_or_else(|| "Could not determine Codex sessions directory".to_string())?;

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    // Walk YYYY/MM/DD directory structure
    for year_entry in read_dir_sorted(&sessions_dir)? {
        let year_path = year_entry.path();
        if !year_path.is_dir() {
            continue;
        }

        for month_entry in read_dir_sorted(&year_path)? {
            let month_path = month_entry.path();
            if !month_path.is_dir() {
                continue;
            }

            for day_entry in read_dir_sorted(&month_path)? {
                let day_path = day_entry.path();
                if !day_path.is_dir() {
                    continue;
                }

                // Find rollout-*.jsonl files in this day directory
                for file_entry in read_dir_sorted(&day_path)? {
                    let file_path = file_entry.path();

                    if !is_codex_session_file(&file_path) {
                        continue;
                    }

                    // Try to get session info
                    if let Some(info) = get_session_info(&file_path) {
                        // Apply project filter if specified
                        if let Some(filter) = project_filter {
                            if let Some(ref cwd) = info.cwd {
                                if !cwd.contains(filter) {
                                    continue;
                                }
                            } else {
                                // No cwd info, skip when filtering
                                continue;
                            }
                        }
                        sessions.push(info);
                    }
                }
            }
        }
    }

    // Sort by modification time, newest first
    sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(sessions)
}

/// Check if a path is a Codex session file (rollout-*.jsonl)
fn is_codex_session_file(path: &Path) -> bool {
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    filename.starts_with("rollout-") && filename.ends_with(".jsonl")
}

/// Get session info by reading the first few lines to find session_meta
fn get_session_info(path: &Path) -> Option<CodexSessionInfo> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let size_bytes = metadata.len();
    let modified_at = system_time_to_datetime(modified)?;

    // Extract session ID from filename: rollout-YYYY-MM-DDTHH-MM-SS-UUID.jsonl
    let filename = path.file_stem()?.to_str()?;
    let session_id = filename
        .strip_prefix("rollout-")
        .unwrap_or(filename)
        .to_string();

    // Try to read cwd from session_meta (usually first line)
    let cwd = read_session_cwd(path);

    Some(CodexSessionInfo {
        session_id,
        session_path: path.to_path_buf(),
        cwd,
        modified_at,
        size_bytes,
    })
}

/// Read the cwd from session_meta entry (usually first line)
fn read_session_cwd(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    // Check first 5 lines for session_meta
    for line in reader.lines().take(5) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue, // Skip problematic lines
        };
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<CodexEntry>(&line) {
            if entry.is_session_meta() {
                return entry.session_cwd().map(|s| s.to_string());
            }
        }
    }

    None
}

/// Read directory entries sorted alphabetically
fn read_dir_sorted(path: &Path) -> Result<Vec<std::fs::DirEntry>, String> {
    let mut entries: Vec<_> = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {:?}: {}", path, e))?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by_key(|e| e.path());
    Ok(entries)
}

/// Convert SystemTime to DateTime<Utc>
fn system_time_to_datetime(st: SystemTime) -> Option<DateTime<Utc>> {
    let duration = st.duration_since(std::time::UNIX_EPOCH).ok()?;
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_codex_session_file() {
        assert!(is_codex_session_file(Path::new(
            "rollout-2025-11-03T19-16-00-uuid.jsonl"
        )));
        assert!(!is_codex_session_file(Path::new("other.jsonl")));
        assert!(!is_codex_session_file(Path::new("rollout-test.txt")));
    }

    #[test]
    fn test_codex_sessions_dir() {
        let dir = codex_sessions_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        // Use Path semantics for cross-platform compatibility
        let expected_suffix = Path::new(".codex").join("sessions");
        assert!(path.ends_with(&expected_suffix));
    }
}
