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

    let mut has_content = false;

    // Read dive context if present
    let dive_path = state::wm_path("dive_context.md");
    if let Ok(dive_content) = std::fs::read_to_string(&dive_path) {
        if !dive_content.trim().is_empty() {
            println!("## Dive Context\n");
            println!("{}", dive_content.trim());
            println!();
            has_content = true;
        }
    }

    // Read working set (compiled state)
    let working_content = match session_id {
        Some(id) => {
            let path = state::session_dir(id).join("working_set.md");
            std::fs::read_to_string(&path)
        }
        None => state::read_working_set(),
    };

    if let Ok(content) = working_content {
        if !content.trim().is_empty() {
            if has_content {
                println!("## Compiled Knowledge\n");
            }
            println!("{}", content.trim());
            has_content = true;
        }
    }

    if !has_content {
        println!("_No context loaded._");
        println!();
        println!("To add context:");
        println!("  - Run /dive-prep to create a dive session");
        println!("  - Or wait for wm extract to capture knowledge");
    }

    Ok(())
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
