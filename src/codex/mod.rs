//! Codex session parsing module
//!
//! Provides functionality to discover and parse OpenAI Codex CLI sessions
//! stored in ~/.codex/sessions/ for knowledge extraction.

pub mod reader;
pub mod session;
pub mod types;

pub use reader::{format_context, read_codex_session};
pub use session::{discover_sessions, CodexSessionInfo};
