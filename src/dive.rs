//! Dive prep management - named dive contexts for session grounding
//!
//! Supports multiple named preps (like git branches) stored in .wm/dives/
//! with a "current" prep tracked in config.

use crate::state;
use std::fs;
use std::process::Command;

// ============================================================================
// Prep command - scaffold dive context from local sources
// ============================================================================

/// Prepare a dive session by gathering local context
pub fn prep(intent: Option<&str>, intent_type: &str) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let mut sections = Vec::new();

    // Header
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    sections.push("# Dive Session\n".to_string());
    sections.push(format!("**Intent:** {}", intent_type));
    sections.push(format!("**Started:** {}", now));

    // Focus from intent
    if let Some(intent_str) = intent {
        sections.push(format!("**Focus:** {}", intent_str));
    }

    sections.push(String::new()); // blank line

    // Gather local context
    sections.push("## Context\n".to_string());

    // Project section from CLAUDE.md
    if let Ok(claude_md) = read_claude_md() {
        sections.push("### Project\n".to_string());
        sections.push(claude_md);
        sections.push(String::new());
    }

    // Git state
    if let Ok(git_info) = gather_git_context() {
        sections.push("### Git State\n".to_string());
        sections.push(git_info);
        sections.push(String::new());
    }

    // Workflow based on intent type
    sections.push("## Workflow\n".to_string());
    sections.push(get_workflow(intent_type));

    // Write to dive_context.md
    let content = sections.join("\n");
    let dive_context_path = state::wm_path("dive_context.md");
    fs::write(&dive_context_path, &content)
        .map_err(|e| format!("Failed to write dive_context.md: {}", e))?;

    println!("✓ Dive session prepared");
    println!("  Written to .wm/dive_context.md");
    println!("  Intent: {}", intent_type);
    if let Some(i) = intent {
        println!("  Focus: {}", truncate(i, 60));
    }

    Ok(())
}

fn read_claude_md() -> Result<String, String> {
    // Check current directory and parents for CLAUDE.md
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

    for ancestor in cwd.ancestors() {
        let claude_path = ancestor.join("CLAUDE.md");
        if claude_path.exists() {
            let content = fs::read_to_string(&claude_path).map_err(|e| e.to_string())?;
            // Return first meaningful section (skip to first ## or first 500 chars)
            return Ok(summarize_claude_md(&content));
        }
    }

    Err("CLAUDE.md not found".to_string())
}

fn summarize_claude_md(content: &str) -> String {
    // Extract project overview - first section or first 500 chars
    let lines: Vec<&str> = content.lines().collect();
    let mut summary = Vec::new();
    let mut in_overview = false;
    let mut char_count = 0;

    for line in lines {
        if line.starts_with("# ") {
            in_overview = true;
            continue;
        }
        if line.starts_with("## ") && in_overview {
            // Hit next section, stop
            break;
        }
        if in_overview && char_count < 800 {
            summary.push(line);
            char_count += line.len();
        }
    }

    if summary.is_empty() {
        // Fallback: first 500 chars
        content.chars().take(500).collect()
    } else {
        summary.join("\n").trim().to_string()
    }
}

fn gather_git_context() -> Result<String, String> {
    let mut info = Vec::new();

    // Current branch
    if let Ok(output) = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        && output.status.success()
    {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            info.push(format!("- Branch: `{}`", branch));
        }
    }

    // Status summary
    if let Ok(output) = Command::new("git").args(["status", "--short"]).output()
        && output.status.success()
    {
        let status = String::from_utf8_lossy(&output.stdout);
        let line_count = status.lines().count();
        if line_count > 0 {
            info.push(format!("- {} uncommitted change(s)", line_count));
        } else {
            info.push("- Working tree clean".to_string());
        }
    }

    // Recent commits
    if let Ok(output) = Command::new("git")
        .args(["log", "--oneline", "-3"])
        .output()
        && output.status.success()
    {
        let log = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !log.is_empty() {
            info.push("\nRecent commits:".to_string());
            for line in log.lines() {
                info.push(format!("  {}", line));
            }
        }
    }

    if info.is_empty() {
        Err("Not a git repository".to_string())
    } else {
        Ok(info.join("\n"))
    }
}

fn get_workflow(intent_type: &str) -> String {
    match intent_type {
        "fix" => r#"1. Understand the issue
2. Write failing test (if applicable)
3. Implement fix
4. Run tests
5. Commit with clear message
6. PR for review"#
            .to_string(),

        "plan" => r#"1. Review available context
2. Identify options and trade-offs
3. Draft plan with concrete steps
4. Surface risks and dependencies
5. Document decision rationale"#
            .to_string(),

        "review" => r#"1. Gather recent work artifacts
2. Identify patterns, learnings, surprises
3. Surface insights worth capturing
4. Document findings"#
            .to_string(),

        "ship" => r#"1. Verify all tests pass
2. Check constraints and guardrails
3. Review changes for completeness
4. Create PR with full context
5. Address review feedback
6. Deploy when approved"#
            .to_string(),

        _ => r#"1. Understand the problem space
2. Read relevant code/docs
3. Ask clarifying questions
4. Document findings
5. Identify next steps"#
            .to_string(), // explore (default)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ============================================================================
// Named prep management
// ============================================================================

/// List all dive preps, marking the current one
pub fn list() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let preps = state::list_dive_preps().map_err(|e| format!("Failed to list preps: {}", e))?;
    let current = state::current_dive();

    if preps.is_empty() {
        println!("No dive preps found. Create one with 'wm dive new <name>'");
        return Ok(());
    }

    for prep in preps {
        let marker = if current.as_ref() == Some(&prep) {
            "* "
        } else {
            "  "
        };
        println!("{}{}", marker, prep);
    }

    Ok(())
}

/// Create a new named dive prep
pub fn new(name: &str, content: Option<&str>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Validate name (kebab-case, no special chars)
    if !is_valid_prep_name(name) {
        return Err(format!(
            "Invalid prep name '{}'. Use lowercase letters, numbers, and hyphens only.",
            name
        ));
    }

    let path = state::dive_prep_path(name);
    if path.exists() {
        return Err(format!(
            "Prep '{}' already exists. Use 'wm dive switch {}' to activate it.",
            name, name
        ));
    }

    state::ensure_dive_dir().map_err(|e| format!("Failed to create dives directory: {}", e))?;

    let default_content = format!(
        "# Dive: {}\n\nIntent: \n\n## Focus\n\n## Constraints\n",
        name
    );
    let initial_content = content.unwrap_or(&default_content);
    fs::write(&path, initial_content).map_err(|e| format!("Failed to create prep: {}", e))?;

    println!("✓ Created dive prep '{}' at .wm/dives/{}.md", name, name);
    println!("  Switch to it: wm dive switch {}", name);

    Ok(())
}

/// Switch to a named dive prep (set as current)
pub fn switch(name: &str) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let path = state::dive_prep_path(name);
    if !path.exists() {
        return Err(format!(
            "Prep '{}' not found. Create it with 'wm dive new {}'",
            name, name
        ));
    }

    state::set_current_dive(Some(name)).map_err(|e| format!("Failed to update config: {}", e))?;

    println!("✓ Switched to dive prep '{}'", name);

    Ok(())
}

/// Delete a named dive prep
pub fn delete(name: &str) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let path = state::dive_prep_path(name);
    if !path.exists() {
        return Err(format!("Prep '{}' not found.", name));
    }

    fs::remove_file(&path).map_err(|e| format!("Failed to delete prep: {}", e))?;

    // If this was the current prep, clear it
    if state::current_dive().as_deref() == Some(name) {
        state::set_current_dive(None).map_err(|e| format!("Failed to update config: {}", e))?;
        println!("✓ Deleted dive prep '{}' (was current, now cleared)", name);
    } else {
        println!("✓ Deleted dive prep '{}'", name);
    }

    Ok(())
}

/// Save current dive_context.md as a named prep
pub fn save(name: &str) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Validate name
    if !is_valid_prep_name(name) {
        return Err(format!(
            "Invalid prep name '{}'. Use lowercase letters, numbers, and hyphens only.",
            name
        ));
    }

    // Read from legacy location
    let legacy_path = state::wm_path("dive_context.md");
    let content = fs::read_to_string(&legacy_path)
        .or_else(|_| fs::read_to_string(state::wm_path("OH_context.md")))
        .map_err(|_| "No dive context found. Use /dive-prep to create one first.".to_string())?;

    let target_path = state::dive_prep_path(name);
    if target_path.exists() {
        return Err(format!(
            "Prep '{}' already exists. Delete it first or choose a different name.",
            name
        ));
    }

    state::ensure_dive_dir().map_err(|e| format!("Failed to create dives directory: {}", e))?;

    fs::write(&target_path, &content).map_err(|e| format!("Failed to save prep: {}", e))?;

    // Set as current
    state::set_current_dive(Some(name)).map_err(|e| format!("Failed to update config: {}", e))?;

    println!("✓ Saved current dive context as '{}' (now active)", name);

    Ok(())
}

/// Show current prep name
pub fn current() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    match state::current_dive() {
        Some(name) => println!("{}", name),
        None => println!("(none - using legacy dive_context.md if present)"),
    }

    Ok(())
}

/// Show dive prep content (current or specific)
pub fn show(name: Option<&str>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let content = match name {
        Some(n) => {
            // Show specific prep
            let path = state::dive_prep_path(n);
            fs::read_to_string(&path).map_err(|_| format!("Prep '{}' not found.", n))?
        }
        None => {
            // Show current prep or legacy fallback
            match state::current_dive() {
                Some(current_name) => {
                    let path = state::dive_prep_path(&current_name);
                    fs::read_to_string(&path).map_err(|_| {
                        format!(
                            "Current prep '{}' not found (may have been deleted).",
                            current_name
                        )
                    })?
                }
                None => {
                    // Legacy fallback
                    fs::read_to_string(state::wm_path("dive_context.md"))
                        .or_else(|_| fs::read_to_string(state::wm_path("OH_context.md")))
                        .map_err(|_| "No dive context loaded. Use 'wm dive new <name>' or /dive-prep to create one.".to_string())?
                }
            }
        }
    };

    println!("{}", content);
    Ok(())
}

// ============================================================================
// Legacy OH integration (kept for backwards compatibility)
// ============================================================================

/// Load a dive pack from OH and optionally save as named prep
pub fn load(pack_id: &str, save_as: Option<&str>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Get OH API configuration
    let api_url = std::env::var("OH_API_URL")
        .or_else(|_| load_config_value("api_url"))
        .unwrap_or_else(|_| "https://app.openhorizons.me".to_string());

    let api_key = std::env::var("OH_API_KEY")
        .or_else(|_| load_config_value("api_key"))
        .map_err(|_| {
            "OH API key not found. Set OH_API_KEY or configure ~/.config/openhorizons/config.json"
                .to_string()
        })?;

    // Fetch the dive pack
    let url = format!("{}/api/dive-packs/{}", api_url, pack_id);

    let output = std::process::Command::new("curl")
        .arg("-s")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to fetch dive pack: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to fetch dive pack: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse dive pack response: {}", e))?;

    // Check for error response
    if let Some(error) = response.get("error") {
        return Err(format!("OH API error: {}", error));
    }

    // Extract rendered_md from the dive pack
    let rendered_md = response
        .get("rendered_md")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Dive pack missing rendered_md field".to_string())?;

    // Save as named prep if requested, otherwise write to legacy location
    if let Some(name) = save_as {
        if !is_valid_prep_name(name) {
            return Err(format!(
                "Invalid prep name '{}'. Use lowercase letters, numbers, and hyphens only.",
                name
            ));
        }

        state::ensure_dive_dir().map_err(|e| format!("Failed to create dives directory: {}", e))?;

        let path = state::dive_prep_path(name);
        fs::write(&path, rendered_md).map_err(|e| format!("Failed to write dive prep: {}", e))?;

        state::set_current_dive(Some(name))
            .map_err(|e| format!("Failed to update config: {}", e))?;

        println!(
            "✓ Dive pack loaded as '{}' ({} bytes)",
            name,
            rendered_md.len()
        );
    } else {
        // Legacy: write to dive_context.md
        let dive_context_path = state::wm_path("dive_context.md");
        fs::write(&dive_context_path, rendered_md)
            .map_err(|e| format!("Failed to write dive_context.md: {}", e))?;

        println!(
            "✓ Dive pack loaded to .wm/dive_context.md ({} bytes)",
            rendered_md.len()
        );
        println!("  Tip: Use --name <name> to save as a named prep");
    }

    Ok(())
}

/// Clear the current dive context
pub fn clear() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    // Clear current prep setting
    let had_current = state::current_dive().is_some();
    if had_current {
        state::set_current_dive(None).map_err(|e| format!("Failed to update config: {}", e))?;
    }

    // Also remove legacy file if present
    let dive_context_path = state::wm_path("dive_context.md");
    let had_legacy = dive_context_path.exists();
    if had_legacy {
        fs::remove_file(&dive_context_path)
            .map_err(|e| format!("Failed to remove dive_context.md: {}", e))?;
    }

    if had_current || had_legacy {
        println!("✓ Dive context cleared");
    } else {
        println!("No dive context to clear");
    }

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Validate prep name (kebab-case: lowercase letters, numbers, hyphens)
fn is_valid_prep_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    // Must start with a letter
    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }

    // Only lowercase letters, numbers, and hyphens
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Load a config value from ~/.config/openhorizons/config.json
fn load_config_value(key: &str) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let config_path = std::path::PathBuf::from(home)
        .join(".config")
        .join("openhorizons")
        .join("config.json");

    if !config_path.exists() {
        return Err("Config file not found".to_string());
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    config
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Key '{}' not found in config", key))
}
