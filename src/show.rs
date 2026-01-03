//! Display commands for state and working set

use crate::session;
use crate::state;

/// Run wm show <what> [--session-id ID]
pub fn run(what: &str, session_id: Option<&str>) -> Result<(), String> {
    match what {
        "state" => show_state(),
        "working" => show_working(session_id),
        "sessions" => show_sessions(),
        _ => Err(format!(
            "Unknown target: {}. Use: state, working, sessions",
            what
        )),
    }
}

fn show_state() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let path = state::wm_path("state.md");
    match std::fs::read_to_string(&path) {
        Ok(content) if content.trim().is_empty() => {
            println!("_No knowledge captured yet. Run 'wm extract' after some conversations._");
            Ok(())
        }
        Ok(content) => {
            println!("{}", content);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("_No state.md found. Run 'wm init' first._");
            Ok(())
        }
        Err(e) => Err(format!("Failed to read state.md: {}", e)),
    }
}

fn show_working(session_id: Option<&str>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Read from session-specific path if provided, otherwise global
    let content = match session_id {
        Some(id) => {
            let path = state::session_dir(id).join("working_set.md");
            std::fs::read_to_string(&path)
        }
        None => state::read_working_set(),
    };

    match content {
        Ok(c) if c.trim().is_empty() => {
            println!("_No working set compiled yet. Run 'wm compile' first._");
            Ok(())
        }
        Ok(c) => {
            println!("{}", c);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match session_id {
                Some(id) => println!("_No working set for session {}. Run 'wm hook compile --session-id {}' first._", id, id),
                None => println!("_No working set compiled yet. Run 'wm compile' first._"),
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to read working set: {}", e)),
    }
}

fn show_sessions() -> Result<(), String> {
    let project_path = session::current_project_path();
    let sessions = session::discover_sessions(&project_path)?;

    if sessions.is_empty() {
        println!("_No Claude sessions found for this project._");
        return Ok(());
    }

    println!("# Claude Sessions ({})", sessions.len());
    println!();

    for s in &sessions {
        // Check if we have local state for this session
        let has_local_state = state::session_dir(&s.session_id)
            .join("extraction_state.json")
            .exists();
        let marker = if has_local_state { "●" } else { "○" };

        // Format size in human-readable form
        let size = format_size(s.size_bytes);

        // Format timestamp
        let time = s.modified_at.format("%Y-%m-%d %H:%M");

        println!("{} {} ({}, {})", marker, s.session_id, size, time);
    }

    println!();
    println!("● = has wm state, ○ = not yet processed");

    Ok(())
}

/// Format bytes in human-readable form
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
