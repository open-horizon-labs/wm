//! Compress state.md by synthesizing to higher-level abstractions
//!
//! Takes accumulated tacit knowledge and distills it down by:
//! - Merging related items into broader patterns
//! - Removing obsolete or superseded knowledge
//! - Abstracting specific instances into general principles
//! - Preserving critical constraints and preferences

use crate::state;
use std::process::{Command, Stdio};

/// Run wm compress
pub fn run() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let state_path = state::wm_path("state.md");
    let current_state = std::fs::read_to_string(&state_path)
        .map_err(|e| format!("Failed to read state.md: {}", e))?;

    if current_state.trim().is_empty() {
        println!("Nothing to compress - state.md is empty.");
        return Ok(());
    }

    // Count approximate size for user feedback
    let line_count = current_state.lines().count();
    let char_count = current_state.len();

    state::log(
        "compress",
        &format!(
            "Starting compression of state.md ({} lines, {} chars)",
            line_count, char_count
        ),
    );

    println!("Compressing state.md ({} lines)...", line_count);

    // Call LLM to compress
    let compressed = call_compression(&current_state)?;

    if compressed.was_compressed {
        // Backup old state before overwriting
        let backup_path = state::wm_path("state.md.backup");
        std::fs::write(&backup_path, &current_state)
            .map_err(|e| format!("Failed to write backup: {}", e))?;

        // Write compressed state with atomic rename
        let tmp_path = state::wm_path("state.md.tmp");
        std::fs::write(&tmp_path, &compressed.content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        std::fs::rename(&tmp_path, &state_path)
            .map_err(|e| format!("Failed to rename state file: {}", e))?;

        let new_line_count = compressed.content.lines().count();
        let reduction = if line_count > 0 {
            100 - (new_line_count * 100 / line_count)
        } else {
            0
        };

        state::log(
            "compress",
            &format!(
                "Compressed {} → {} lines ({}% reduction)",
                line_count, new_line_count, reduction
            ),
        );
        println!(
            "Compressed: {} → {} lines ({}% reduction)",
            line_count, new_line_count, reduction
        );
        println!("Backup saved to .wm/state.md.backup");
    } else {
        state::log(
            "compress",
            "No compression possible - state already concise",
        );
        println!("State is already concise - no compression needed.");
    }

    Ok(())
}

struct CompressionResult {
    was_compressed: bool,
    content: String,
}

fn call_compression(current_state: &str) -> Result<CompressionResult, String> {
    // Prevent recursion
    unsafe { std::env::set_var("WM_DISABLED", "1") };

    // AIDEV-NOTE: The compression prompt focuses on synthesis and abstraction,
    // not just deduplication. It references the same tacit knowledge criteria
    // from extract to ensure we preserve the right things.
    let system_prompt = r#"You are compressing accumulated tacit knowledge into a more concise form.

TACIT KNOWLEDGE REMINDER (what we're preserving):
- Rationale behind decisions (WHY this approach)
- Paths rejected and why (judgment in pruning)
- Constraints discovered through friction
- Preferences revealed by corrections
- Patterns followed without stating

COMPRESSION STRATEGIES:
1. MERGE related items into broader principles
   - "Prefers X in context A" + "Prefers X in context B" → "Generally prefers X"

2. ABSTRACT specific instances into general patterns
   - Multiple specific file/function mentions → General architectural preference

3. REMOVE obsolete items
   - Superseded by later, more refined understanding
   - No longer relevant to current codebase state
   - Too specific to be useful in new contexts

4. PRESERVE critical items
   - Hard constraints that caused friction when violated
   - Strong preferences that were corrected multiple times
   - Architectural decisions with clear rationale

5. CONSOLIDATE structure
   - Group related items under clear headings
   - Remove redundant phrasing
   - Keep bullet points concise

THE GOAL: A new Claude session 6 months from now should get the essential wisdom in fewer words. Compress aggressively but preserve meaning.

RESPONSE FORMAT:

If compression was possible, respond:
WAS_COMPRESSED: YES

<compressed markdown content>

If the state is already concise and no meaningful compression is possible, respond:
WAS_COMPRESSED: NO"#;

    let message = format!("CURRENT STATE TO COMPRESS:\n\n{}\n\nOUTPUT:", current_state);

    state::log(
        "compress",
        &format!("Sending {} chars to LLM", message.len()),
    );

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--no-session-persistence")
        .arg("--system-prompt")
        .arg(system_prompt)
        .arg(&message)
        .env("SUPEREGO_DISABLED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn claude CLI: {}", e))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for claude CLI: {}", e))?;

    // Re-enable WM
    unsafe { std::env::remove_var("WM_DISABLED") };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Claude CLI failed (exit {:?}):\nstderr: {}\nstdout: {}",
            output.status.code(),
            stderr,
            stdout
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_compression_response(&stdout)
}

/// Strip markdown prefixes (same as extract.rs)
fn strip_markdown_prefix(line: &str) -> &str {
    line.trim().trim_start_matches(['#', '>', '*']).trim()
}

fn parse_compression_result(result_str: &str) -> CompressionResult {
    let lines: Vec<&str> = result_str.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let stripped = strip_markdown_prefix(line);

        if let Some(value) = stripped.strip_prefix("WAS_COMPRESSED:") {
            let value = value.trim().to_uppercase();
            if value == "YES" || value == "TRUE" {
                let content = lines[i + 1..].join("\n").trim().to_string();
                return CompressionResult {
                    was_compressed: true,
                    content,
                };
            }
            return CompressionResult {
                was_compressed: false,
                content: String::new(),
            };
        }
    }

    // Fallback: no marker found
    state::log("compress", "No WAS_COMPRESSED marker found in response");
    CompressionResult {
        was_compressed: false,
        content: String::new(),
    }
}

fn parse_compression_response(response: &str) -> Result<CompressionResult, String> {
    let cli_response: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| format!("Failed to parse Claude CLI response: {}", e))?;

    let result_str = cli_response
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Claude CLI response missing 'result' field".to_string())?;

    Ok(parse_compression_result(result_str))
}
