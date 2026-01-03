//! Distill command - batch extraction from all sessions
//!
//! Replaces per-turn extraction with on-demand batch distillation.
//! Processes all sessions in ~/.claude/projects/<project-id>/ and extracts
//! tacit knowledge in two passes:
//! - Pass 1: Extract knowledge from each session (yz-fsws)
//! - Pass 2: Categorize into guardrails vs metis (yz-u164)
//!
//! AIDEV-NOTE: This is the CLI scaffold. Actual extraction logic will be
//! implemented in yz-fsws (blocked by this task).

use crate::session::{self, SessionInfo};
use crate::state;

/// Options for the distill command
pub struct DistillOptions {
    /// Preview what would be extracted without writing
    pub dry_run: bool,

    /// Force re-extraction even for already-processed sessions
    pub force: bool,

    /// Push distilled knowledge to Open Horizons via MCP
    pub push_to_oh: bool,

    /// OH context ID to push to (required if push_to_oh is true)
    pub context_id: Option<String>,
}

/// Run the distill command
pub fn run(options: DistillOptions) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Validate options
    if options.push_to_oh && options.context_id.is_none() {
        return Err("--context-id is required when using --push-to-oh".to_string());
    }

    // Discover all sessions for this project
    let project_path = session::current_project_path();
    let sessions = session::discover_sessions(&project_path)?;

    if sessions.is_empty() {
        println!("No sessions found for project.");
        return Ok(());
    }

    println!("Found {} session(s)", sessions.len());

    if options.dry_run {
        println!("\n[DRY RUN] Would process:");
        for session in &sessions {
            print_session_info(session);
        }
        return Ok(());
    }

    // TODO(yz-fsws): Implement Pass 1 - batch extraction from all sessions
    // For each session:
    //   1. Check if already processed (unless --force)
    //   2. Read transcript
    //   3. Extract tacit knowledge
    //   4. Accumulate results

    // TODO(yz-u164): Implement Pass 2 - categorize into guardrails vs metis
    // For accumulated knowledge:
    //   1. Classify each item
    //   2. Write to .wm/distilled/guardrails.md and .wm/distilled/metis.md

    // TODO(yz-pltc): Implement OH integration
    // If --push-to-oh:
    //   1. Call OH MCP to create guardrail/metis candidates
    //   2. Report what was pushed
    // AIDEV-NOTE: OH push will use direct HTTP calls to OH API (not MCP) since wm
    // runs outside Claude Code's MCP context. Requires OH_API_KEY env var.

    println!("\nDistillation not yet implemented.");
    println!("Sessions to process:");
    for session in &sessions {
        print_session_info(session);
    }

    if options.force {
        println!("\n[--force] Would re-extract all sessions regardless of prior processing.");
    }

    if options.push_to_oh {
        println!(
            "\n[--push-to-oh] Would push to OH context: {}",
            options.context_id.as_deref().unwrap_or("(none)")
        );
    }

    Ok(())
}

/// Print session info for display
fn print_session_info(session: &SessionInfo) {
    let size_kb = session.size_bytes / 1024;
    println!(
        "  {} ({} KB, modified {})",
        session.session_id,
        size_kb,
        session.modified_at.format("%Y-%m-%d %H:%M")
    );
}
