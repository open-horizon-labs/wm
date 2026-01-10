//! Core data types for WM

use serde::{Deserialize, Serialize};

/// Hook-specific output for UserPromptSubmit hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook response format (matches Claude Code expectations)
/// AIDEV-NOTE: Must include hookSpecificOutput wrapper for additionalContext to be injected.
/// Plain {"additionalContext":"..."} does NOT work - Claude Code ignores it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Project-level configuration for WM operations
/// Stored in .wm/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub operations: OperationsConfig,

    #[serde(default)]
    pub dive: DiveConfig,
}

/// Configuration for named dive preps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiveConfig {
    /// Name of the currently active dive prep (None = use legacy dive_context.md)
    pub current: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationsConfig {
    #[serde(default = "default_true")]
    pub extract: bool,

    #[serde(default = "default_true")]
    pub compile: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            operations: OperationsConfig::default(),
            dive: DiveConfig::default(),
        }
    }
}

impl Default for OperationsConfig {
    fn default() -> Self {
        Self {
            extract: true,
            compile: true,
        }
    }
}
