//! Session discovery - find all transcripts in ~/.claude/projects/<project-id>/
//!
//! Claude Code stores transcripts as JSONL files with UUID names directly in the project
//! directory. The project-id is derived from the absolute path with slashes replaced by dashes.
//!
//! AIDEV-NOTE: This module is foundational for the distill command which needs to process
//! all sessions in batch. Used by yz-yb9q (distill CLI).

use std::path::{Path, PathBuf};

// Re-export SessionInfo for backward compatibility
pub use crate::types::SessionInfo;
use crate::types::system_time_to_datetime;

/// Compute project-id from a project path
/// Converts absolute path to Claude's project-id format: slashes become dashes
///
/// Example: /Users/drazen/playground/ai-omnibus/wm -> -Users-drazen-playground-ai-omnibus-wm
pub fn compute_project_id(project_path: &Path) -> String {
    // Get absolute path
    let abs_path = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.to_path_buf());

    // Convert to string and replace / with -
    let path_str = abs_path.to_string_lossy();
    path_str.replace('/', "-")
}

/// Get the Claude projects directory (~/.claude/projects/)
pub fn claude_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude").join("projects"))
}

/// Get the project directory for a given project path
/// Returns None if the directory doesn't exist
pub fn get_project_dir(project_path: &Path) -> Option<PathBuf> {
    let projects_dir = claude_projects_dir()?;
    let project_id = compute_project_id(project_path);
    let project_dir = projects_dir.join(&project_id);

    if project_dir.exists() {
        Some(project_dir)
    } else {
        None
    }
}

/// Discover all session transcripts for a project
///
/// Returns sessions sorted by modification time (newest first)
pub fn discover_sessions(project_path: &Path) -> Result<Vec<SessionInfo>, String> {
    let project_dir = get_project_dir(project_path)
        .ok_or_else(|| format!("No Claude project directory found for {:?}", project_path))?;

    discover_sessions_in_dir(&project_dir)
}

/// Discover all session transcripts in a specific directory
pub fn discover_sessions_in_dir(project_dir: &Path) -> Result<Vec<SessionInfo>, String> {
    let entries = std::fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    let mut sessions: Vec<SessionInfo> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();

            // Only consider .jsonl files
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                return None;
            }

            // Session ID is the filename without extension
            let session_id = path.file_stem()?.to_str()?.to_string();

            // Get metadata for timestamps and size
            let metadata = std::fs::metadata(&path).ok()?;
            let modified = metadata.modified().ok()?;
            let size_bytes = metadata.len();

            // Convert SystemTime to DateTime<Utc>
            let modified_at = system_time_to_datetime(modified)?;

            Some(SessionInfo {
                session_id,
                transcript_path: path,
                modified_at,
                size_bytes,
            })
        })
        .collect();

    // Sort by modification time, newest first
    sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(sessions)
}

/// Get the current project path
/// Uses CLAUDE_PROJECT_DIR if set, otherwise current working directory
pub fn current_project_path() -> PathBuf {
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        PathBuf::from(project_dir)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Information about a Claude project directory
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// Project ID (directory name in ~/.claude/projects/)
    pub project_id: String,

    /// Full path to the project directory
    pub project_dir: PathBuf,

    /// Number of session files
    pub session_count: usize,
}

/// List all Claude projects in ~/.claude/projects/
///
/// Returns projects sorted alphabetically by project_id
pub fn list_all_projects() -> Result<Vec<ProjectInfo>, String> {
    let projects_dir = claude_projects_dir()
        .ok_or_else(|| "Could not determine Claude projects directory".to_string())?;

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&projects_dir)
        .map_err(|e| format!("Failed to read projects directory: {}", e))?;

    let mut projects: Vec<ProjectInfo> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();

            // Only consider directories
            if !path.is_dir() {
                return None;
            }

            let project_id = path.file_name()?.to_str()?.to_string();

            // Count .jsonl files (sessions)
            let session_count = std::fs::read_dir(&path)
                .ok()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
                .count();

            Some(ProjectInfo {
                project_id,
                project_dir: path,
                session_count,
            })
        })
        .collect();

    // Sort alphabetically
    projects.sort_by(|a, b| a.project_id.cmp(&b.project_id));

    Ok(projects)
}

/// Find projects matching a filter string (case-insensitive substring match)
pub fn find_projects_by_filter(filter: &str) -> Result<Vec<ProjectInfo>, String> {
    let all_projects = list_all_projects()?;
    let filter_lower = filter.to_lowercase();

    Ok(all_projects
        .into_iter()
        .filter(|p| p.project_id.to_lowercase().contains(&filter_lower))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_project_id() {
        let path = Path::new("/Users/drazen/playground/ai-omnibus/wm");
        let id = compute_project_id(path);
        // Note: canonicalize may resolve symlinks differently, but the format should be correct
        assert!(id.starts_with("-"));
        assert!(id.contains("-wm"));
        assert!(!id.contains("/"));
    }

    #[test]
    fn test_claude_projects_dir() {
        let dir = claude_projects_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.ends_with(".claude/projects") || path.to_string_lossy().contains(".claude"));
    }
}
