//! Dive prep management - named dive contexts for session grounding
//!
//! Supports multiple named preps (like git branches) stored in .wm/dives/
//! with a "current" prep tracked in config.

use crate::state;
use std::fs;

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
        return Err(format!("Prep '{}' already exists. Use 'wm dive switch {}' to activate it.", name, name));
    }

    state::ensure_dive_dir().map_err(|e| format!("Failed to create dives directory: {}", e))?;

    let default_content = format!("# Dive: {}\n\nIntent: \n\n## Focus\n\n## Constraints\n", name);
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

    state::set_current_dive(Some(name))
        .map_err(|e| format!("Failed to update config: {}", e))?;

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
        state::set_current_dive(None)
            .map_err(|e| format!("Failed to update config: {}", e))?;
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
    state::set_current_dive(Some(name))
        .map_err(|e| format!("Failed to update config: {}", e))?;

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
            fs::read_to_string(&path)
                .map_err(|_| format!("Prep '{}' not found.", n))?
        }
        None => {
            // Show current prep or legacy fallback
            match state::current_dive() {
                Some(current_name) => {
                    let path = state::dive_prep_path(&current_name);
                    fs::read_to_string(&path)
                        .map_err(|_| format!("Current prep '{}' not found (may have been deleted).", current_name))?
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
        .map_err(|_| "OH API key not found. Set OH_API_KEY or configure ~/.config/openhorizons/config.json".to_string())?;

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
        return Err(format!("Failed to fetch dive pack: {}", String::from_utf8_lossy(&output.stderr)));
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

        println!("✓ Dive pack loaded as '{}' ({} bytes)", name, rendered_md.len());
    } else {
        // Legacy: write to dive_context.md
        let dive_context_path = state::wm_path("dive_context.md");
        fs::write(&dive_context_path, rendered_md)
            .map_err(|e| format!("Failed to write dive_context.md: {}", e))?;

        println!("✓ Dive pack loaded to .wm/dive_context.md ({} bytes)", rendered_md.len());
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
        state::set_current_dive(None)
            .map_err(|e| format!("Failed to update config: {}", e))?;
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
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    config
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Key '{}' not found in config", key))
}
