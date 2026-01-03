//! Dive pack integration with Open Horizons
//!
//! Fetches curated grounding context from OH dive packs and writes to OH_context.md

use crate::state;

/// Load a dive pack from OH and write to OH_context.md
pub fn load(pack_id: &str) -> Result<(), String> {
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

    // Write to OH_context.md
    let oh_context_path = state::wm_path("OH_context.md");
    std::fs::write(&oh_context_path, rendered_md)
        .map_err(|e| format!("Failed to write OH_context.md: {}", e))?;

    println!("✓ Dive pack loaded to .wm/OH_context.md ({} bytes)", rendered_md.len());
    Ok(())
}

/// Clear the current OH context
pub fn clear() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let oh_context_path = state::wm_path("OH_context.md");

    if oh_context_path.exists() {
        std::fs::remove_file(&oh_context_path)
            .map_err(|e| format!("Failed to remove OH_context.md: {}", e))?;
        println!("✓ OH context cleared");
    } else {
        println!("No OH context to clear");
    }

    Ok(())
}

/// Show current OH context
pub fn show() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let oh_context_path = state::wm_path("OH_context.md");

    if oh_context_path.exists() {
        let content = std::fs::read_to_string(&oh_context_path)
            .map_err(|e| format!("Failed to read OH_context.md: {}", e))?;
        println!("{}", content);
    } else {
        println!("No OH context loaded. Use 'wm dive load <pack-id>' to load a dive pack.");
    }

    Ok(())
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

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    config
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Key '{}' not found in config", key))
}
