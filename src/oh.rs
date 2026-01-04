//! Open Horizons integration for pushing candidates
//!
//! Pushes distilled guardrails and metis candidates to OH API.
//! Uses direct HTTP calls since wm runs outside Claude Code's MCP context.
//!
//! AIDEV-NOTE: This module talks directly to the OH API, not via MCP.
//! Requires OH_API_KEY env var. OH_API_URL defaults to https://app.openhorizons.me

use crate::state;
use serde::{Deserialize, Serialize};

/// Default OH API URL
const DEFAULT_OH_API_URL: &str = "https://app.openhorizons.me";

/// Request body for creating a candidate
#[derive(Debug, Serialize)]
struct CreateCandidateRequest<'a> {
    #[serde(rename = "type")]
    candidate_type: &'a str,
    context_id: &'a str,
    content: &'a str,
    source_type: &'a str,
}

/// Response from creating a candidate
#[derive(Debug, Deserialize)]
struct CreateCandidateResponse {
    candidate_id: String,
}

/// Result of pushing candidates to OH
#[derive(Debug)]
pub struct PushResult {
    /// Number of guardrails successfully pushed
    pub guardrails_pushed: usize,
    /// Number of metis items successfully pushed
    pub metis_pushed: usize,
    /// Errors encountered (item content, error message)
    pub errors: Vec<(String, String)>,
}

/// Push guardrails and metis candidates to Open Horizons
///
/// Returns the number of items successfully pushed and any errors.
pub fn push_candidates(
    context_id: &str,
    guardrails: &[String],
    metis: &[String],
) -> Result<PushResult, String> {
    let api_key = std::env::var("OH_API_KEY")
        .map_err(|_| "OH_API_KEY environment variable not set".to_string())?;

    let api_url = std::env::var("OH_API_URL").unwrap_or_else(|_| DEFAULT_OH_API_URL.to_string());

    state::log(
        "oh",
        &format!(
            "Pushing {} guardrails and {} metis to OH context {}",
            guardrails.len(),
            metis.len(),
            context_id
        ),
    );

    let mut result = PushResult {
        guardrails_pushed: 0,
        metis_pushed: 0,
        errors: Vec::new(),
    };

    // Push guardrails
    for item in guardrails {
        match push_single_candidate(&api_url, &api_key, context_id, "guardrail", item) {
            Ok(candidate_id) => {
                state::log("oh", &format!("Created guardrail candidate: {}", candidate_id));
                result.guardrails_pushed += 1;
            }
            Err(e) => {
                state::log("oh", &format!("Failed to push guardrail: {}", e));
                result.errors.push((truncate_for_error(item), e));
            }
        }
    }

    // Push metis
    for item in metis {
        match push_single_candidate(&api_url, &api_key, context_id, "metis", item) {
            Ok(candidate_id) => {
                state::log("oh", &format!("Created metis candidate: {}", candidate_id));
                result.metis_pushed += 1;
            }
            Err(e) => {
                state::log("oh", &format!("Failed to push metis: {}", e));
                result.errors.push((truncate_for_error(item), e));
            }
        }
    }

    Ok(result)
}

/// Push a single candidate to OH API
fn push_single_candidate(
    api_url: &str,
    api_key: &str,
    context_id: &str,
    candidate_type: &str,
    content: &str,
) -> Result<String, String> {
    let url = format!("{}/api/candidates", api_url.trim_end_matches('/'));

    let request_body = CreateCandidateRequest {
        candidate_type,
        context_id,
        content,
        source_type: "wm_distill",
    };

    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(&request_body)
        .map_err(|e| match e {
            ureq::Error::Status(code, response) => {
                let body = response.into_string().unwrap_or_default();
                format!("HTTP {} - {}", code, body)
            }
            other => format!("Request failed: {}", other),
        })?;

    let response_body: CreateCandidateResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(response_body.candidate_id)
}

/// Truncate content for error messages (first 50 chars)
/// Uses char boundary to avoid UTF-8 panic on multi-byte characters.
fn truncate_for_error(content: &str) -> String {
    let truncated: String = content.chars().take(50).collect();
    if truncated.len() < content.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_error_short() {
        assert_eq!(truncate_for_error("short"), "short");
    }

    #[test]
    fn test_truncate_for_error_long() {
        let long = "a".repeat(100);
        let truncated = truncate_for_error(&long);
        assert!(truncated.ends_with("..."));
        assert_eq!(truncated.len(), 53); // 50 + "..."
    }

    #[test]
    fn test_truncate_for_error_utf8() {
        // Multi-byte UTF-8 characters (emoji are 4 bytes each)
        let emoji_content = "🎉".repeat(60);
        let truncated = truncate_for_error(&emoji_content);
        assert!(truncated.ends_with("..."));
        // 50 emoji chars = 200 bytes, but we should get 50 chars + "..."
        assert_eq!(truncated.chars().count(), 53); // 50 emoji + 3 dots
    }
}
