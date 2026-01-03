//! Generative LLM extraction from transcript
//!
//! Reads current state + new transcript → LLM generates complete new state
//! Uses session-id filtering to prevent cross-session bleed.
//!
//! AIDEV-NOTE: Previous implementation used fragile byte-position checkpointing
//! which broke on transcript rotation/compaction. Now uses proper JSONL parsing
//! and session-id filtering like superego does.

use crate::llm;
use crate::state;
use crate::transcript::{format_context, get_messages_in_window, get_messages_since, read_transcript};
use chrono::{DateTime, Duration, Utc};
use std::path::Path;

/// Carryover window: how many minutes before last_extracted to re-read for context
/// AIDEV-NOTE: Matches sg's default. Provides continuity without unbounded context growth.
const CARRYOVER_WINDOW_MINUTES: i64 = 5;

/// Run wm extract
/// AIDEV-NOTE: Returns Ok() instead of Err when not initialized. This is intentional:
/// extract/compile can be triggered automatically by hooks (superego calls `wm extract &`),
/// so they must not spam error logs in projects without .wm/. User-invoked commands like
/// show/status still return Err to inform the user. See also: compile::run().
pub fn run(transcript_path: Option<String>, session_id: Option<String>) -> Result<(), String> {
    // AIDEV-NOTE: Deprecation warning - extract is being replaced by distill command
    // which uses batch processing with two passes (extraction then categorization).
    // See epic yz-90jh for the full distillation rewrite plan.
    eprintln!("⚠️  DEPRECATED: 'wm extract' will be replaced by 'wm distill' in a future version.");
    eprintln!("   The new distill command processes all sessions in batch with improved categorization.");
    eprintln!();

    if !state::is_initialized() {
        eprintln!("Not initialized. Run 'wm init' first.");
        return Ok(());
    }

    // Check if extract is paused
    if !state::is_extract_enabled() {
        state::log("extract", "Paused via config, skipping");
        println!("Extract is paused. Use 'wm resume extract' to enable.");
        return Ok(());
    }

    let transcript = find_transcript(transcript_path)?;
    let session = session_id.or_else(|| std::env::var("CLAUDE_SESSION_ID").ok());
    extract_from_transcript(&transcript, session.as_deref())
}

/// Run from hook (called by sg)
pub fn run_hook() -> Result<(), String> {
    if !state::is_initialized() {
        return Ok(()); // Silent success
    }

    // Check if extract is paused
    if !state::is_extract_enabled() {
        state::log("extract", "Paused via config, skipping");
        return Ok(());
    }

    let transcript = find_transcript(None)?;
    let session_id = std::env::var("CLAUDE_SESSION_ID").ok();
    extract_from_transcript(&transcript, session_id.as_deref())
}

/// Find the transcript file
fn find_transcript(explicit_path: Option<String>) -> Result<String, String> {
    if let Some(path) = explicit_path {
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
        return Err(format!("Transcript not found: {}", path));
    }

    // Try environment variable
    if let Ok(path) = std::env::var("CLAUDE_TRANSCRIPT_PATH") {
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
    }

    // Try to find in ~/.claude/projects/
    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude").join("projects");
        if claude_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&claude_dir) {
                let mut transcripts: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().join("transcript.jsonl").exists())
                    .collect();

                transcripts.sort_by_key(|e| {
                    std::fs::metadata(e.path().join("transcript.jsonl"))
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });

                if let Some(latest) = transcripts.last() {
                    return Ok(latest.path().join("transcript.jsonl").display().to_string());
                }
            }
        }
    }

    Err("Could not find transcript. Use --transcript <path> to specify.".to_string())
}

/// Get session-specific state directory
fn session_state_dir(session_id: Option<&str>) -> std::path::PathBuf {
    match session_id {
        Some(sid) => state::wm_path(&format!("sessions/{}", sid)),
        None => state::wm_path(""),
    }
}

/// Read last_extracted timestamp from session state
fn read_last_extracted(session_id: Option<&str>) -> Option<DateTime<Utc>> {
    let state_dir = session_state_dir(session_id);
    let state_path = state_dir.join("extraction_state.json");

    std::fs::read_to_string(state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("last_extracted")?.as_str().map(String::from))
        .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

/// Write last_extracted timestamp to session state
fn write_last_extracted(session_id: Option<&str>, timestamp: DateTime<Utc>) -> Result<(), String> {
    let state_dir = session_state_dir(session_id);

    // Ensure directory exists
    std::fs::create_dir_all(&state_dir)
        .map_err(|e| format!("Failed to create session state dir: {}", e))?;

    let state_path = state_dir.join("extraction_state.json");
    let state = serde_json::json!({
        "last_extracted": timestamp.to_rfc3339(),
    });

    let content = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    std::fs::write(state_path, content).map_err(|e| format!("Failed to write state: {}", e))?;

    Ok(())
}

/// Generative extraction with proper session filtering
/// AIDEV-NOTE: This is the core extraction logic. Key changes from old impl:
/// 1. Parse JSONL properly into typed entries
/// 2. Filter by session_id to prevent cross-session bleed
/// 3. Use timestamp-based cutoff instead of fragile byte position
/// 4. Format context with deduplication (system reminders, tool summaries)
fn extract_from_transcript(transcript_path: &str, session_id: Option<&str>) -> Result<(), String> {
    state::log(
        "extract",
        &format!(
            "Starting extraction from {} (session: {:?})",
            transcript_path, session_id
        ),
    );

    // Capture read time BEFORE reading (for next extraction cutoff)
    let transcript_read_at = Utc::now();

    // Read current state markdown (or empty if first run)
    let current_state = std::fs::read_to_string(state::wm_path("state.md")).unwrap_or_default();

    // Read last extraction timestamp for this session
    let last_extracted = read_last_extracted(session_id);
    state::log(
        "extract",
        &format!("Last extracted: {:?}", last_extracted),
    );

    // Parse transcript JSONL
    let entries = read_transcript(Path::new(transcript_path))
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

    state::log(
        "extract",
        &format!("Parsed {} transcript entries", entries.len()),
    );

    // AIDEV-NOTE: Carryover context - re-read N minutes before last_extracted
    // This provides continuity without unbounded context growth (same pattern as sg)
    let carryover_context = if let Some(cutoff) = last_extracted {
        let window_start = cutoff - Duration::minutes(CARRYOVER_WINDOW_MINUTES);
        let carryover_messages = get_messages_in_window(&entries, window_start, cutoff, session_id);

        if !carryover_messages.is_empty() {
            state::log(
                "extract",
                &format!(
                    "Including {} carryover messages from past {} minutes",
                    carryover_messages.len(),
                    CARRYOVER_WINDOW_MINUTES
                ),
            );
            let formatted = format_context(&carryover_messages);
            if !formatted.trim().is_empty() {
                Some(formatted)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None // First extraction - no carryover
    };

    // Filter to messages since last extraction, for this session only
    let messages = get_messages_since(&entries, last_extracted, session_id);

    if messages.is_empty() {
        state::log("extract", "No new messages for this session, skipping");
        println!("No new transcript content to extract from.");
        return Ok(());
    }

    state::log(
        "extract",
        &format!("Processing {} new messages", messages.len()),
    );

    // Format messages for LLM (with deduplication)
    let formatted_transcript = format_context(&messages);

    if formatted_transcript.trim().is_empty() {
        state::log("extract", "Formatted transcript is empty, skipping");
        println!("No extractable content in new messages.");
        return Ok(());
    }

    // Call LLM with current state + carryover + new transcript → get extraction result
    let extraction = call_generative_extraction(
        &current_state,
        &formatted_transcript,
        carryover_context.as_deref(),
    )?;

    // Only write if there's new knowledge
    if extraction.has_knowledge {
        // Write updated state markdown with atomic rename
        // AIDEV-NOTE: Write to .tmp file then rename to prevent corruption
        // if multiple sessions write concurrently (last writer wins, but no corruption)
        let state_path = state::wm_path("state.md");
        let tmp_path = state::wm_path("state.md.tmp");
        std::fs::write(&tmp_path, &extraction.content)
            .map_err(|e| format!("Failed to write state temp file: {}", e))?;
        std::fs::rename(&tmp_path, &state_path)
            .map_err(|e| format!("Failed to rename state file: {}", e))?;

        state::log(
            "extract",
            &format!("Complete - {} messages processed, knowledge extracted", messages.len()),
        );
        println!(
            "State updated ({} messages processed, session: {})",
            messages.len(),
            session_id.unwrap_or("all")
        );
    } else {
        state::log(
            "extract",
            &format!("Complete - {} messages processed, no new knowledge", messages.len()),
        );
        println!(
            "No new knowledge to extract ({} messages processed, session: {})",
            messages.len(),
            session_id.unwrap_or("all")
        );
    }

    // Update last_extracted for this session regardless of whether we wrote
    // AIDEV-NOTE: Use transcript_read_at (captured before reading) to avoid
    // missing messages that arrived during LLM evaluation. Same fix as sg.
    write_last_extracted(session_id, transcript_read_at)?;

    Ok(())
}

/// Result of extraction - includes flag for whether new knowledge was found
struct ExtractionResult {
    has_knowledge: bool,
    content: String,
}

/// Call LLM with generative approach: current state + transcript → extraction result
/// AIDEV-NOTE: carryover_context provides continuity by including recent messages
/// from before the current extraction window (same pattern as sg)
fn call_generative_extraction(
    current_state: &str,
    new_transcript: &str,
    carryover_context: Option<&str>,
) -> Result<ExtractionResult, String> {
    // AIDEV-NOTE: wm is the RECORDER role - captures learning without authority to enforce.
    // Learning stays "plastic" here until promoted to OH as guardrails/metis.
    // Focus on RATIONALE (why), not just decisions (what).
    // AIDEV-NOTE: Uses text-based markers like sg does - LLMs reliably follow this format
    // and lenient parsing handles markdown wrapping. JSON format was unreliable.
    let system_prompt = r#"You are capturing tacit knowledge that will help future AI sessions.

Tacit knowledge is the wisdom that emerges from HOW someone works, not what they explicitly say. The user might not realize they're teaching you these patterns.

CAPTURE:
- Rationale behind decisions (WHY this approach, not just WHAT was done)
- Paths rejected and why (the judgment in pruning options)
- Constraints discovered through friction
- Preferences revealed by corrections
- Patterns the user follows without stating

EXAMPLES OF GOOD CAPTURE:
- "Prefers asking before implementing when architecture is unclear"
- "Values failing fast over silent error handling"
- "Rejected X approach because Y - prefers Z pattern"

DO NOT CAPTURE:
- What happened ("Fixed X", "Updated Y")
- Explicit requests or questions
- Tool outputs or code snippets
- Anything Claude said

THE TEST: Would a new Claude session find this useful 6 months from now? Is it about HOW to work with this user/codebase, not WHAT happened today?

Most sessions have no tacit insights worth capturing. That's normal.

RESPONSE FORMAT:

If you found tacit knowledge worth capturing, respond:
HAS_KNOWLEDGE: YES

<your markdown content here - existing state + new insights>

If nothing worth capturing, respond:
HAS_KNOWLEDGE: NO"#;

    // Build message with optional carryover context
    let carryover_section = match carryover_context {
        Some(ctx) if !ctx.trim().is_empty() => format!(
            "--- PREVIOUS CONTEXT (for continuity) ---\n{}\n--- END PREVIOUS CONTEXT ---\n\n",
            ctx
        ),
        _ => String::new(),
    };

    let message = format!(
        "CURRENT STATE:\n{}\n\n{}NEW TRANSCRIPT:\n{}\n\nOUTPUT:",
        current_state, carryover_section, new_transcript
    );

    // DEBUG: Log what we're sending
    state::log("extract", &format!("Message length: {} bytes", message.len()));
    state::log("extract", &format!("System prompt length: {} bytes", system_prompt.len()));
    state::log("extract", &format!("Message preview (first 500): {}", &message.chars().take(500).collect::<String>()));

    // Use shared LLM utilities
    let result_str = llm::call_claude(system_prompt, &message)?;
    let response = llm::parse_marker_response(&result_str, "HAS_KNOWLEDGE");

    Ok(ExtractionResult {
        has_knowledge: response.is_positive,
        content: response.content,
    })
}
