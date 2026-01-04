//! Distill command - batch extraction from all sessions
//!
//! Replaces per-turn extraction with on-demand batch distillation.
//! Processes all sessions in ~/.claude/projects/<project-id>/ and extracts
//! tacit knowledge in two passes:
//! - Pass 1: Extract knowledge from each session (yz-fsws)
//! - Pass 2: Categorize into guardrails vs metis (yz-u164)
//!
//! AIDEV-NOTE: Pass 1 processes all sessions and accumulates raw extractions.
//! Each session's extraction is cached to support incremental runs (--force overrides).
//! The raw extractions are written to .wm/distill/raw_extractions.md for Pass 2.

use crate::llm;
use crate::oh;
use crate::session::{self, SessionInfo};
use crate::state;
use crate::transcript::{format_context, read_transcript};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Directory for distillation output
const DISTILL_DIR: &str = "distill";

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

    /// Filter to a specific project by name (substring match)
    pub project: Option<String>,
}

/// Cached extraction result for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionExtraction {
    /// Session ID
    session_id: String,

    /// When the extraction was performed
    extracted_at: DateTime<Utc>,

    /// Whether any knowledge was found
    has_knowledge: bool,

    /// The extracted content (if has_knowledge is true)
    content: String,

    /// File size at extraction time (to detect changes)
    /// AIDEV-NOTE: Sessions are append-only JSONL, so size increase = new content.
    /// This heuristic would break for editable files but works for transcripts.
    file_size_bytes: u64,
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

    // Discover sessions, optionally filtered by project
    let sessions = if let Some(ref project_filter) = options.project {
        discover_sessions_by_project_filter(project_filter)?
    } else {
        // Default: current project only
        let project_path = session::current_project_path();
        session::discover_sessions(&project_path)?
    };

    if sessions.is_empty() {
        if let Some(ref filter) = options.project {
            println!("No sessions found for projects matching '{}'.", filter);
        } else {
            println!("No sessions found for project.");
        }
        return Ok(());
    }

    if let Some(ref filter) = options.project {
        println!("Found {} session(s) matching project filter '{}'", sessions.len(), filter);
    } else {
        println!("Found {} session(s)", sessions.len());
    }

    if options.dry_run {
        println!("\n[DRY RUN] Would process:");
        let cache = load_extraction_cache();
        for session in &sessions {
            let status = if options.force {
                "force"
            } else if needs_extraction(&session, &cache) {
                "new/changed"
            } else {
                "cached"
            };
            print_session_info_with_status(&session, status);
        }
        return Ok(());
    }

    // Pass 1: Extract knowledge from each session
    println!("\n=== Pass 1: Extracting knowledge from sessions ===\n");
    let extractions = run_pass1(&sessions, options.force)?;

    // Accumulate raw extractions
    let raw_content = accumulate_extractions(&extractions);

    if raw_content.is_empty() {
        println!("\nNo knowledge extracted from any session.");
        return Ok(());
    }

    // Write raw extractions for Pass 2
    write_raw_extractions(&raw_content)?;
    println!(
        "\nPass 1 complete: {} session(s) with knowledge extracted.",
        extractions.iter().filter(|e| e.has_knowledge).count()
    );
    println!("Raw extractions written to .wm/{}/raw_extractions.md", DISTILL_DIR);

    // Pass 2: Categorize into guardrails vs metis
    println!("\n=== Pass 2: Categorizing into guardrails vs metis ===\n");
    let categorized = run_pass2(&raw_content)?;

    // Push to Open Horizons if requested
    if options.push_to_oh {
        let context_id = options.context_id.as_ref().unwrap(); // Already validated above
        push_to_oh(context_id, &categorized)?;
    }

    Ok(())
}

/// Result of Pass 2 categorization
pub struct CategorizationResult {
    pub guardrails: Vec<String>,
    pub metis: Vec<String>,
}

/// Run Pass 2: categorize raw extractions into guardrails vs metis
/// Returns the categorization result for optional OH push.
fn run_pass2(raw_extractions: &str) -> Result<CategorizationResult, String> {
    let result = call_categorization_llm(raw_extractions)?;

    let guardrail_count = result.guardrails.len();
    let metis_count = result.metis.len();

    // Write guardrails
    if !result.guardrails.is_empty() {
        let content = format_categorized_output("Guardrails", &result.guardrails);
        write_categorized_file("guardrails.md", &content)?;
        println!("  ✓ {} guardrail(s) written to .wm/{}/guardrails.md", guardrail_count, DISTILL_DIR);
    } else {
        println!("  ○ No guardrails identified");
    }

    // Write metis
    if !result.metis.is_empty() {
        let content = format_categorized_output("Metis", &result.metis);
        write_categorized_file("metis.md", &content)?;
        println!("  ✓ {} metis item(s) written to .wm/{}/metis.md", metis_count, DISTILL_DIR);
    } else {
        println!("  ○ No metis items identified");
    }

    println!(
        "\nPass 2 complete: {} guardrail(s), {} metis item(s)",
        guardrail_count, metis_count
    );

    Ok(result)
}

/// Call LLM to categorize extractions into guardrails vs metis
fn call_categorization_llm(raw_extractions: &str) -> Result<CategorizationResult, String> {
    // AIDEV-NOTE: Categorization distinguishes between:
    // - Guardrails: Hard constraints that must NEVER be violated (binary enforcement)
    // - Metis: Wisdom/patterns about HOW to work effectively (contextual guidance)
    // The key difference: guardrails are rules, metis is advice.
    let system_prompt = r#"You are categorizing tacit knowledge into two types:

**GUARDRAILS** - Hard constraints that must NEVER be violated:
- Prohibitions: "Never do X", "Always do Y before Z"
- Safety rules: Things that could cause data loss, security issues, or broken builds
- Project-specific requirements that are non-negotiable
- Examples: "Never commit .env files", "Always run tests before pushing", "Never delete migrations"

**METIS** - Wisdom and patterns about HOW to work effectively:
- Preferences: How the user likes things done
- Patterns: Approaches that work well in this codebase
- Context: Understanding about why things are the way they are
- Soft guidance that may have exceptions
- Examples: "Prefer functional approaches", "User likes concise commit messages", "Check existing patterns first"

OUTPUT FORMAT:

GUARDRAILS:
- Item 1
- Item 2
...

METIS:
- Item 1
- Item 2
...

Rules:
1. Each item should be self-contained and actionable
2. Preserve the original meaning but clarify if needed
3. If an item could be both, choose based on severity (safety-critical = guardrail)
4. It's OK to have empty sections if nothing fits that category
5. Combine duplicates, but don't lose distinct nuances"#;

    let message = format!(
        "Categorize these extracted insights:\n\n{}\n\nOUTPUT:",
        raw_extractions
    );

    let result_str = llm::call_claude(system_prompt, &message)?;
    parse_categorization_response(&result_str)
}

/// Parse the categorization response into guardrails and metis
fn parse_categorization_response(response: &str) -> Result<CategorizationResult, String> {
    let mut guardrails = Vec::new();
    let mut metis = Vec::new();
    let mut current_section: Option<&str> = None;

    for line in response.lines() {
        let trimmed = line.trim();

        // Check for section headers
        if trimmed.starts_with("GUARDRAILS:") || trimmed == "GUARDRAILS" {
            current_section = Some("guardrails");
            continue;
        }
        if trimmed.starts_with("METIS:") || trimmed == "METIS" {
            current_section = Some("metis");
            continue;
        }

        // Parse bullet points
        if let Some(section) = current_section {
            if let Some(item) = parse_bullet_item(trimmed) {
                match section {
                    "guardrails" => guardrails.push(item),
                    "metis" => metis.push(item),
                    _ => {}
                }
            }
        }
    }

    Ok(CategorizationResult { guardrails, metis })
}

/// Parse a bullet point item, returning None for empty or non-bullet lines
fn parse_bullet_item(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Skip empty lines and section markers
    if trimmed.is_empty() {
        return None;
    }

    // Remove bullet prefixes
    let content = trimmed
        .trim_start_matches('-')
        .trim_start_matches('*')
        .trim_start_matches('•')
        .trim();

    // Skip if just whitespace after removing bullet
    if content.is_empty() {
        return None;
    }

    Some(content.to_string())
}

/// Format categorized items for output file
fn format_categorized_output(title: &str, items: &[String]) -> String {
    let mut output = format!("# {}\n\n", title);

    for item in items {
        output.push_str(&format!("- {}\n", item));
    }

    output
}

/// Write a categorized output file
fn write_categorized_file(filename: &str, content: &str) -> Result<(), String> {
    let distill_dir = state::wm_path(DISTILL_DIR);
    std::fs::create_dir_all(&distill_dir)
        .map_err(|e| format!("Failed to create distill directory: {}", e))?;

    let path = distill_dir.join(filename);
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

    Ok(())
}

/// Push categorized items to Open Horizons
fn push_to_oh(context_id: &str, categorized: &CategorizationResult) -> Result<(), String> {
    if categorized.guardrails.is_empty() && categorized.metis.is_empty() {
        println!("\n=== Push to OH ===\n");
        println!("  ○ Nothing to push (no candidates)");
        return Ok(());
    }

    println!("\n=== Push to Open Horizons ===\n");
    println!("  Context: {}", context_id);

    let result = oh::push_candidates(context_id, &categorized.guardrails, &categorized.metis)?;

    // Report results
    if result.guardrails_pushed > 0 {
        println!("  ✓ {} guardrail(s) pushed", result.guardrails_pushed);
    }
    if result.metis_pushed > 0 {
        println!("  ✓ {} metis item(s) pushed", result.metis_pushed);
    }

    // Report errors
    if !result.errors.is_empty() {
        println!("  ✗ {} item(s) failed:", result.errors.len());
        for (content, error) in &result.errors {
            println!("    - \"{}\": {}", content, error);
        }
    }

    let total_pushed = result.guardrails_pushed + result.metis_pushed;
    println!(
        "\nOH push complete: {} item(s) pushed, {} error(s)",
        total_pushed,
        result.errors.len()
    );

    // Return error if all items failed
    if total_pushed == 0 && !result.errors.is_empty() {
        return Err("All items failed to push to OH".to_string());
    }

    Ok(())
}

/// Run Pass 1: extract knowledge from each session
fn run_pass1(sessions: &[SessionInfo], force: bool) -> Result<Vec<SessionExtraction>, String> {
    let mut cache = load_extraction_cache();
    let mut results = Vec::new();
    let mut processed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for session in sessions {
        // Check if we can use cached extraction
        if !force && !needs_extraction(session, &cache) {
            if let Some(cached) = cache.get(&session.session_id) {
                println!("  {} [cached]", session.session_id);
                results.push(cached.clone());
                skipped += 1;
                continue;
            }
        }

        // Extract from this session
        println!("  {} extracting...", session.session_id);
        match extract_from_session(session) {
            Ok(extraction) => {
                let status = if extraction.has_knowledge {
                    "✓ knowledge found"
                } else {
                    "○ no knowledge"
                };
                println!("    {}", status);

                // Cache the result
                cache.insert(session.session_id.clone(), extraction.clone());
                results.push(extraction);
                processed += 1;
            }
            Err(e) => {
                eprintln!("    ✗ error: {}", e);
                // Log to file for later debugging
                log_extraction_error(&session.session_id, &e);
                failed += 1;
                // Continue with other sessions
            }
        }
    }

    // Save updated cache
    save_extraction_cache(&cache)?;

    // Build summary message
    let mut summary_parts = vec![format!("{} session(s) processed", processed)];
    if skipped > 0 {
        summary_parts.push(format!("{} from cache", skipped));
    }
    if failed > 0 {
        summary_parts.push(format!("{} failed", failed));
    }
    println!("\n{}", summary_parts.join(", "));

    if failed > 0 {
        println!("See .wm/{}/errors.log for failure details", DISTILL_DIR);
    }

    Ok(results)
}

/// Check if a session needs extraction (not in cache or file changed)
fn needs_extraction(session: &SessionInfo, cache: &HashMap<String, SessionExtraction>) -> bool {
    match cache.get(&session.session_id) {
        Some(cached) => {
            // Re-extract if file size changed (indicates new content)
            cached.file_size_bytes != session.size_bytes
        }
        None => true,
    }
}

/// Extract knowledge from a single session
fn extract_from_session(session: &SessionInfo) -> Result<SessionExtraction, String> {
    state::log(
        "distill",
        &format!("Extracting from session {}", session.session_id),
    );

    // Read transcript
    let entries = read_transcript(&session.transcript_path)
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

    // Get all messages for this session
    // AIDEV-NOTE: Use .as_str() for proper Option<&str> comparison
    let session_messages: Vec<_> = entries
        .iter()
        .filter(|e| e.session_id() == Some(session.session_id.as_str()))
        .filter(|e| e.is_message() || e.is_summary())
        .collect();

    if session_messages.is_empty() {
        return Ok(SessionExtraction {
            session_id: session.session_id.clone(),
            extracted_at: Utc::now(),
            has_knowledge: false,
            content: String::new(),
            file_size_bytes: session.size_bytes,
        });
    }

    // Format for LLM
    let formatted = format_context(&session_messages);

    if formatted.trim().is_empty() {
        return Ok(SessionExtraction {
            session_id: session.session_id.clone(),
            extracted_at: Utc::now(),
            has_knowledge: false,
            content: String::new(),
            file_size_bytes: session.size_bytes,
        });
    }

    // Call LLM for extraction
    let result = call_extraction_llm(&formatted)?;

    Ok(SessionExtraction {
        session_id: session.session_id.clone(),
        extracted_at: Utc::now(),
        has_knowledge: result.has_knowledge,
        content: result.content,
        file_size_bytes: session.size_bytes,
    })
}

/// Result of extraction
struct ExtractionResult {
    has_knowledge: bool,
    content: String,
}

/// Call LLM to extract tacit knowledge from transcript
fn call_extraction_llm(transcript: &str) -> Result<ExtractionResult, String> {
    // AIDEV-NOTE: Distill extraction prompt differs from per-turn extract:
    // - We're looking at a complete session, not incremental updates
    // - Focus on extracting standalone insights that can be categorized later
    // - No existing state to merge with - each session is independent
    let system_prompt = r#"You are extracting tacit knowledge from an AI coding session transcript.

Tacit knowledge is wisdom about HOW to work effectively, not WHAT was done. Look for:
- User preferences revealed through corrections or choices
- Patterns in how problems were approached
- Constraints discovered through friction
- Decisions and their rationale (WHY, not just WHAT)
- Quality standards implicit in feedback

OUTPUT FORMAT:

If you found tacit knowledge worth capturing, respond:
HAS_KNOWLEDGE: YES

Then list each insight as a separate bullet point:
- Insight 1
- Insight 2
...

Each insight should be:
- Self-contained (understandable without the transcript)
- About HOW to work, not WHAT happened
- Useful for future AI sessions

If nothing worth capturing, respond:
HAS_KNOWLEDGE: NO

Most sessions have little or no tacit knowledge. That's normal."#;

    let message = format!("TRANSCRIPT:\n{}\n\nOUTPUT:", transcript);

    let result_str = llm::call_claude(system_prompt, &message)?;
    let response = llm::parse_marker_response(&result_str, "HAS_KNOWLEDGE");

    Ok(ExtractionResult {
        has_knowledge: response.is_positive,
        content: response.content,
    })
}

/// Accumulate extractions into a single markdown document
fn accumulate_extractions(extractions: &[SessionExtraction]) -> String {
    let mut output = String::new();

    for extraction in extractions {
        if extraction.has_knowledge && !extraction.content.trim().is_empty() {
            output.push_str(&format!(
                "## Session: {}\n\n{}\n\n",
                extraction.session_id, extraction.content
            ));
        }
    }

    output.trim().to_string()
}

/// Load extraction cache from disk
fn load_extraction_cache() -> HashMap<String, SessionExtraction> {
    let cache_path = state::wm_path(DISTILL_DIR).join("cache.json");

    std::fs::read_to_string(&cache_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Save extraction cache to disk
fn save_extraction_cache(cache: &HashMap<String, SessionExtraction>) -> Result<(), String> {
    let distill_dir = state::wm_path(DISTILL_DIR);
    std::fs::create_dir_all(&distill_dir)
        .map_err(|e| format!("Failed to create distill directory: {}", e))?;

    let cache_path = distill_dir.join("cache.json");
    let content = serde_json::to_string_pretty(cache)
        .map_err(|e| format!("Failed to serialize cache: {}", e))?;

    std::fs::write(cache_path, content).map_err(|e| format!("Failed to write cache: {}", e))?;

    Ok(())
}

/// Write raw extractions to file
fn write_raw_extractions(content: &str) -> Result<(), String> {
    let distill_dir = state::wm_path(DISTILL_DIR);
    std::fs::create_dir_all(&distill_dir)
        .map_err(|e| format!("Failed to create distill directory: {}", e))?;

    let path = distill_dir.join("raw_extractions.md");
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write raw extractions: {}", e))?;

    Ok(())
}

/// Log an extraction error to the errors log file
fn log_extraction_error(session_id: &str, error: &str) {
    use chrono::Local;
    use std::fs::OpenOptions;
    use std::io::Write;

    let distill_dir = state::wm_path(DISTILL_DIR);

    // Ensure directory exists
    if std::fs::create_dir_all(&distill_dir).is_err() {
        return; // Silently fail - this is best-effort logging
    }

    let log_path = distill_dir.join("errors.log");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    // Collapse multi-line errors to single line for parseable log format
    let error_oneline = error.replace('\n', " | ");
    let line = format!("[{}] Session {}: {}\n", timestamp, session_id, error_oneline);

    // Append to log file, ignore errors (logging should never fail the operation)
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}

/// Print session info with status
fn print_session_info_with_status(session: &SessionInfo, status: &str) {
    let size_kb = session.size_bytes / 1024;
    println!(
        "  {} ({} KB, {}) [{}]",
        session.session_id,
        size_kb,
        session.modified_at.format("%Y-%m-%d %H:%M"),
        status
    );
}

/// Discover sessions from projects matching a filter string
fn discover_sessions_by_project_filter(filter: &str) -> Result<Vec<SessionInfo>, String> {
    if filter.trim().is_empty() {
        return Err("Project filter cannot be empty".to_string());
    }

    let matching_projects = session::find_projects_by_filter(filter)?;

    if matching_projects.is_empty() {
        return Err(format!(
            "No projects found matching '{}'. Use 'wm show sessions' to list available projects.",
            filter
        ));
    }

    // If multiple matches, show which projects we're processing
    if matching_projects.len() > 1 {
        println!(
            "Matched {} projects:",
            matching_projects.len()
        );
        for p in &matching_projects {
            println!("  {} ({} sessions)", p.project_id, p.session_count);
        }
        println!();
    } else {
        println!("Project: {}", matching_projects[0].project_id);
    }

    // Collect sessions from all matching projects
    let mut all_sessions = Vec::new();
    for project in matching_projects {
        let sessions = session::discover_sessions_in_dir(&project.project_dir)?;
        all_sessions.extend(sessions);
    }

    // Sort by modification time, newest first
    all_sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(all_sessions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_categorization_response_basic() {
        let response = r#"GUARDRAILS:
- Never commit .env files
- Always run tests before pushing

METIS:
- Prefer functional approaches when possible
- Check existing patterns before adding new code"#;

        let result = parse_categorization_response(response).unwrap();

        assert_eq!(result.guardrails.len(), 2);
        assert_eq!(result.guardrails[0], "Never commit .env files");
        assert_eq!(result.guardrails[1], "Always run tests before pushing");

        assert_eq!(result.metis.len(), 2);
        assert_eq!(result.metis[0], "Prefer functional approaches when possible");
        assert_eq!(
            result.metis[1],
            "Check existing patterns before adding new code"
        );
    }

    #[test]
    fn test_parse_categorization_response_with_colons() {
        let response = r#"GUARDRAILS:
- Never do this: commit secrets
- Always do that: run linter

METIS:
- User preference: concise messages"#;

        let result = parse_categorization_response(response).unwrap();

        assert_eq!(result.guardrails.len(), 2);
        assert_eq!(result.guardrails[0], "Never do this: commit secrets");

        assert_eq!(result.metis.len(), 1);
        assert_eq!(result.metis[0], "User preference: concise messages");
    }

    #[test]
    fn test_parse_categorization_response_empty_sections() {
        let response = r#"GUARDRAILS:

METIS:
- Only metis here"#;

        let result = parse_categorization_response(response).unwrap();

        assert_eq!(result.guardrails.len(), 0);
        assert_eq!(result.metis.len(), 1);
    }

    #[test]
    fn test_parse_categorization_response_asterisk_bullets() {
        let response = r#"GUARDRAILS:
* Item with asterisk

METIS:
* Another asterisk item"#;

        let result = parse_categorization_response(response).unwrap();

        assert_eq!(result.guardrails.len(), 1);
        assert_eq!(result.guardrails[0], "Item with asterisk");
    }

    #[test]
    fn test_parse_bullet_item() {
        assert_eq!(parse_bullet_item("- item"), Some("item".to_string()));
        assert_eq!(parse_bullet_item("* item"), Some("item".to_string()));
        assert_eq!(parse_bullet_item("• item"), Some("item".to_string()));
        assert_eq!(parse_bullet_item("  - indented"), Some("indented".to_string()));
        assert_eq!(parse_bullet_item(""), None);
        assert_eq!(parse_bullet_item("  "), None);
        assert_eq!(parse_bullet_item("-"), None);
    }

    #[test]
    fn test_format_categorized_output() {
        let items = vec!["First item".to_string(), "Second item".to_string()];
        let output = format_categorized_output("Test", &items);

        assert!(output.starts_with("# Test\n\n"));
        assert!(output.contains("- First item\n"));
        assert!(output.contains("- Second item\n"));
    }
}
