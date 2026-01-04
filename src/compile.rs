//! Working set compilation
//!
//! Reads state.md + intent → LLM filters for relevance → outputs working_set.md
//! Acts as working memory: surfaces what's relevant RIGHT NOW for the task

use crate::llm;
use crate::state;
use crate::types::HookResponse;

/// Run wm compile with optional intent
/// AIDEV-NOTE: Returns Ok() instead of Err when not initialized. This is intentional:
/// extract/compile can be triggered automatically by hooks, so they must not spam error
/// logs in projects without .wm/. User-invoked commands like show/status still return
/// Err to inform the user. See also: extract::run().
pub fn run(intent: Option<String>) -> Result<(), String> {
    if !state::is_initialized() {
        eprintln!("Not initialized. Run 'wm init' first.");
        return Ok(());
    }

    // Check if compile is paused
    if !state::is_compile_enabled() {
        println!("Compile is paused. Use 'wm resume compile' to enable.");
        return Ok(());
    }

    let state = std::fs::read_to_string(state::wm_path("state.md")).unwrap_or_default();

    if state.trim().is_empty() {
        println!("No knowledge in state.md yet. Run 'wm extract' first.");
        return Ok(());
    }

    let result = compile_with_llm(&state, intent.as_deref())?;

    if result.has_relevant {
        state::write_working_set(&result.content)
            .map_err(|e| format!("Failed to write working set: {}", e))?;
        println!("Compiled working set to .wm/working_set.md");
    } else {
        println!("No relevant knowledge for this intent.");
    }
    Ok(())
}

/// Run from post-submit hook - reads intent from stdin, outputs JSON
/// Never blocks - returns empty response on any failure
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

    let intent = read_hook_input();
    state::log(
        "compile",
        &format!(
            "Intent: {:?}, Session: {}",
            intent.as_deref().unwrap_or("(none)").chars().take(50).collect::<String>(),
            session_id
        ),
    );

    let state_content = std::fs::read_to_string(state::wm_path("state.md")).unwrap_or_default();

    // Check for dive context (curated grounding from dive-prep)
    // Supports both dive_context.md (new) and OH_context.md (legacy)
    let dive_context = std::fs::read_to_string(state::wm_path("dive_context.md"))
        .or_else(|_| std::fs::read_to_string(state::wm_path("OH_context.md")))
        .unwrap_or_default();
    let has_dive_context = !dive_context.trim().is_empty();
    if has_dive_context {
        state::log("compile", &format!("Dive context loaded: {} bytes", dive_context.len()));
    }

    // If no state and no dive context, return empty response
    if state_content.trim().is_empty() && !has_dive_context {
        state::log("compile", "No state or dive context, returning empty");
        let response = HookResponse {
            additional_context: None,
        };
        let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        println!("{}", json);
        return Ok(());
    }

    // If we have dive context but no state, just inject dive context directly (no LLM call needed)
    if state_content.trim().is_empty() && has_dive_context {
        state::log("compile", "Injecting dive context directly (no state to filter)");
        let _ = state::write_working_set_for_session(session_id, &dive_context);
        let response = HookResponse {
            additional_context: Some(dive_context),
        };
        let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
        println!("{}", json);
        return Ok(());
    }

    state::log("compile", &format!("State size: {} bytes", state_content.len()));

    // Try LLM call, but don't fail the hook if it errors
    let compile_result = match compile_with_llm(&state_content, intent.as_deref()) {
        Ok(result) => {
            state::log("compile", &format!("LLM returned has_relevant={}, {} bytes",
                result.has_relevant, result.content.len()));
            result
        }
        Err(e) => {
            state::log("compile", &format!("LLM error: {}", e));
            CompileResult { has_relevant: false, content: String::new() }
        }
    };

    // Combine dive context (always included if present) with filtered state
    let final_content = if has_dive_context {
        if compile_result.has_relevant {
            // Both dive context and relevant state - combine them
            format!("{}\n\n---\n\n## Working Memory\n\n{}", dive_context, compile_result.content)
        } else {
            // Only dive context
            dive_context.clone()
        }
    } else {
        compile_result.content.clone()
    };

    let has_content = !final_content.trim().is_empty();

    // Only write working_set if there's content
    if has_content {
        let _ = state::write_working_set_for_session(session_id, &final_content);
    }

    // Output hook response
    let response = HookResponse {
        additional_context: if has_content {
            Some(final_content)
        } else {
            None
        },
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

/// Result of compilation - includes flag for whether relevant knowledge was found
struct CompileResult {
    has_relevant: bool,
    content: String,
}

/// Use LLM to filter state for relevance to intent
fn compile_with_llm(state: &str, intent: Option<&str>) -> Result<CompileResult, String> {
    let intent_text = intent.unwrap_or("general coding task");

    // AIDEV-NOTE: This prompt must FILTER, not ANSWER. Previous version caused
    // the LLM to synthesize explanations when user prompt looked like a question.
    // AIDEV-NOTE: Uses text-based markers like sg does - more reliable than JSON.
    let system_prompt = r#"You are a relevance filter for an AI assistant's working memory.

Given accumulated knowledge and the user's current message, SELECT items that are relevant to their task.

DO NOT:
- Answer the user's question
- Synthesize new explanations
- Add commentary or analysis
- Reformat or summarize the knowledge

ONLY output knowledge items from the accumulated state that apply to the current task.
Copy relevant sections verbatim or near-verbatim.

RESPONSE FORMAT (text-based, not JSON):

If knowledge is relevant, respond:
HAS_RELEVANT: YES

<the relevant knowledge items as markdown>

If nothing is relevant, respond:
HAS_RELEVANT: NO

That's it. Just the marker line, then content (if YES). No JSON, no code fences, no explanation."#;

    let message = format!(
        "ACCUMULATED KNOWLEDGE:\n{}\n\nUSER'S CURRENT INTENT:\n{}\n\nRELEVANT KNOWLEDGE:",
        state, intent_text
    );

    // Use shared LLM utilities (handles env var guards, CLI invocation, JSON parsing)
    let result_str = llm::call_claude(system_prompt, &message)?;
    let response = llm::parse_marker_response(&result_str, "HAS_RELEVANT");

    Ok(CompileResult {
        has_relevant: response.is_positive,
        content: response.content,
    })
}
