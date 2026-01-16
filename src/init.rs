//! Initialize .wm/ in current project

use crate::state::{self, wm_dir, wm_path};
use std::fs;

/// Run wm init
pub fn run() -> Result<(), String> {
    if state::is_initialized() {
        return Err("Already initialized: .wm/ exists".to_string());
    }

    // Create .wm/ directory
    fs::create_dir_all(wm_dir()).map_err(|e| format!("Failed to create .wm/: {}", e))?;

    // Create empty state.md (freeform markdown for tacit knowledge)
    fs::write(wm_path("state.md"), "").map_err(|e| format!("Failed to write state.md: {}", e))?;

    // Create checkpoint.json for tracking extraction progress
    fs::write(wm_path("checkpoint.json"), "{\"position\": 0}")
        .map_err(|e| format!("Failed to write checkpoint.json: {}", e))?;

    // Create empty working set
    state::write_working_set("").map_err(|e| format!("Failed to write working_set.md: {}", e))?;

    println!("Initialized .wm/ in current directory");

    Ok(())
}
