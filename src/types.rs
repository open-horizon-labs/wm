//! Core data types for WM

use serde::{Deserialize, Serialize};

/// Hook response format (matches Claude Code expectations)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Project-level configuration for WM operations
/// Stored in .wm/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub operations: OperationsConfig,
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
