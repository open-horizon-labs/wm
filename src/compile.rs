//! Working set compilation
//!
//! Reads distilled knowledge (guardrails + metis) and optional dive context,
//! then combines them into a working set for the current session.
//! All content is pre-curated, no LLM filtering needed.

use crate::state;
use crate::types::HookResponse;

/// Distill directory constant (matches distill.rs)
const DISTILL_DIR: &str = "distill";

/// Run wm compile with optional intent (CLI entry point)
/// AIDEV-NOTE: Returns Ok() instead of Err when not initialized. This is intentional:
/// extract/compile can be triggered automatically by hooks, so they must not spam error
/// logs in projects without .wm/. User-invoked commands like show/status still return
/// Err to inform the user. See also: extract::run().
/// AIDEV-NOTE: Intent parameter is now unused since we don't do LLM filtering.
/// Kept for API compatibility.
pub fn run(_intent: Option<String>) -> Result<(), String> {
    if !state::is_initialized() {
        eprintln!("Not initialized. Run 'wm init' first.");
        return Ok(());
    }

    // Check if compile is paused
    if !state::is_compile_enabled() {
        println!("Compile is paused. Use 'wm resume compile' to enable.");
        return Ok(());
    }

    // Read distilled knowledge (pre-curated, no filtering needed)
    let guardrails = read_distilled_file("guardrails.md");
    let metis = read_distilled_file("metis.md");

    // Check for dive context
    let dive_context = std::fs::read_to_string(state::wm_path("dive_context.md"))
        .or_else(|_| std::fs::read_to_string(state::wm_path("OH_context.md")))
        .unwrap_or_default();

    // Combine all sources
    let combined = combine_context(&dive_context, &guardrails, &metis);

    if combined.trim().is_empty() {
        println!("No distilled knowledge found. Run 'wm distill' first.");
        return Ok(());
    }

    state::write_working_set(&combined)
        .map_err(|e| format!("Failed to write working set: {}", e))?;
    println!("Compiled working set to .wm/working_set.md");
    Ok(())
}

/// Run from post-submit hook - reads intent from stdin, outputs JSON
/// Never blocks - returns empty response on any failure
/// AIDEV-NOTE: Intent is consumed from stdin but not used for filtering since
/// distilled content is pre-curated and always relevant.
pub fn run_hook(session_id: &str) -> Result<(), String> {
    if !state::is_initialized() {
        // Silent success if not initialized
        return Ok(());
    }

    // Check if compile is paused
    if !state::is_compile_enabled() {
        state::log("compile", "Paused via config, returning empty");
        let response = HookResponse {
            additional_context: None,
        };
        let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        println!("{}", json);
        return Ok(());
    }

    state::log("compile", "Hook fired");

    // Consume stdin (intent) but don't use it - distilled content is always relevant
    let _ = read_hook_input();
    state::log("compile", &format!("Session: {}", session_id));

    // Read distilled knowledge (pre-curated, no filtering needed)
    let guardrails = read_distilled_file("guardrails.md");
    let metis = read_distilled_file("metis.md");

    // Check for dive context (curated grounding from dive-prep)
    // Supports both dive_context.md (new) and OH_context.md (legacy)
    let dive_context = std::fs::read_to_string(state::wm_path("dive_context.md"))
        .or_else(|_| std::fs::read_to_string(state::wm_path("OH_context.md")))
        .unwrap_or_default();

    // Log what we found
    if !dive_context.trim().is_empty() {
        state::log("compile", &format!("Dive context: {} bytes", dive_context.len()));
    }
    if !guardrails.trim().is_empty() {
        state::log("compile", &format!("Guardrails: {} bytes", guardrails.len()));
    }
    if !metis.trim().is_empty() {
        state::log("compile", &format!("Metis: {} bytes", metis.len()));
    }

    // Combine all sources (no LLM filtering - all content is pre-curated)
    let final_content = combine_context(&dive_context, &guardrails, &metis);

    let has_content = !final_content.trim().is_empty();

    if !has_content {
        state::log("compile", "No distilled content found, returning empty");
        let response = HookResponse {
            additional_context: None,
        };
        let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        println!("{}", json);
        return Ok(());
    }

    // Write working_set for debugging/inspection
    let _ = state::write_working_set_for_session(session_id, &final_content);

    // Output hook response
    let response = HookResponse {
        additional_context: Some(final_content),
    };

    let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
    state::log("compile", "Complete");
    println!("{}", json);

    Ok(())
}

/// Read intent from hook input (stdin contains JSON with prompt field)
fn read_hook_input() -> Option<String> {
    use std::io::{self, Read};

    let mut buffer = String::new();
    if io::stdin().read_to_string(&mut buffer).is_ok() && !buffer.trim().is_empty() {
        // Try to parse as JSON hook input
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&buffer) {
            return json.get("prompt").and_then(|v| v.as_str()).map(String::from);
        }
        // Fallback: treat raw input as the intent
        Some(buffer.trim().to_string())
    } else {
        None
    }
}

/// Read a distilled file from .wm/distill/
fn read_distilled_file(filename: &str) -> String {
    let path = state::wm_path(DISTILL_DIR).join(filename);
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Combine context sources into a single markdown document
/// Order: dive_context (session-specific grounding) → guardrails → metis
fn combine_context(dive_context: &str, guardrails: &str, metis: &str) -> String {
    let mut sections = Vec::new();

    // Dive context first (session-specific grounding)
    if !dive_context.trim().is_empty() {
        sections.push(dive_context.trim().to_string());
    }

    // Guardrails (hard constraints)
    if !guardrails.trim().is_empty() {
        sections.push(guardrails.trim().to_string());
    }

    // Metis (wisdom/patterns)
    if !metis.trim().is_empty() {
        sections.push(metis.trim().to_string());
    }

    sections.join("\n\n---\n\n")
}
