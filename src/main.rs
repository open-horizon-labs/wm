use clap::{Parser, Subcommand};
use std::process::ExitCode;

mod compile;
mod compress;
mod extract;
mod init;
mod session;
mod show;
mod state;
mod transcript;
mod types;

#[derive(Parser)]
#[command(name = "wm")]
#[command(about = "Working memory for AI coding assistants")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .wm/ in current project
    Init,

    /// Run LLM extraction from transcript
    Extract {
        /// Path to transcript file
        #[arg(long)]
        transcript: Option<String>,

        /// Claude session ID (for session-scoped extraction)
        #[arg(long)]
        session_id: Option<String>,
    },

    /// Compile working set for current state
    Compile {
        /// User's current message (for intent detection)
        #[arg(long)]
        intent: Option<String>,
    },

    /// Compress state.md by synthesizing to higher-level abstractions
    Compress,

    /// Display state, working set, or sessions
    Show {
        /// What to show: state, working, sessions
        #[arg(default_value = "state")]
        what: String,

        /// Session ID (for session-specific working set)
        #[arg(long)]
        session_id: Option<String>,
    },

    /// Pause extract, compile, or both operations
    Pause {
        /// Operation to pause: extract, compile, or omit for both
        operation: Option<String>,
    },

    /// Resume extract, compile, or both operations
    Resume {
        /// Operation to resume: extract, compile, or omit for both
        operation: Option<String>,
    },

    /// Show current pause/resume status
    Status,

    /// Hook entry points (called by Claude Code hooks)
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
}

#[derive(Subcommand)]
enum HookCommands {
    /// Called by post-submit hook
    Compile {
        /// Claude session ID (required for session-scoped output)
        #[arg(long)]
        session_id: String,
    },

    /// Called by sg after clearing (or manually)
    Extract,
}

fn main() -> ExitCode {
    // Check if disabled
    if std::env::var("WM_DISABLED").is_ok() {
        return ExitCode::SUCCESS;
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => init::run(),
        Commands::Extract {
            transcript,
            session_id,
        } => extract::run(transcript, session_id),
        Commands::Compile { intent } => compile::run(intent),
        Commands::Compress => compress::run(),
        Commands::Show { what, session_id } => show::run(&what, session_id.as_deref()),
        Commands::Pause { operation } => run_pause(operation),
        Commands::Resume { operation } => run_resume(operation),
        Commands::Status => run_status(),
        Commands::Hook { command } => match command {
            HookCommands::Compile { session_id } => compile::run_hook(&session_id),
            HookCommands::Extract => extract::run_hook(),
        },
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run_pause(operation: Option<String>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let mut config = state::read_config();

    match operation.as_deref() {
        Some("extract") => {
            config.operations.extract = false;
            println!("Paused: extract");
        }
        Some("compile") => {
            config.operations.compile = false;
            println!("Paused: compile");
        }
        Some(op) => {
            return Err(format!("Unknown operation: {}. Use 'extract' or 'compile'.", op));
        }
        None => {
            config.operations.extract = false;
            config.operations.compile = false;
            println!("Paused: extract, compile");
        }
    }

    state::write_config(&config).map_err(|e| format!("Failed to write config: {}", e))
}

fn run_resume(operation: Option<String>) -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let mut config = state::read_config();

    match operation.as_deref() {
        Some("extract") => {
            config.operations.extract = true;
            println!("Resumed: extract");
        }
        Some("compile") => {
            config.operations.compile = true;
            println!("Resumed: compile");
        }
        Some(op) => {
            return Err(format!("Unknown operation: {}. Use 'extract' or 'compile'.", op));
        }
        None => {
            config.operations.extract = true;
            config.operations.compile = true;
            println!("Resumed: extract, compile");
        }
    }

    state::write_config(&config).map_err(|e| format!("Failed to write config: {}", e))
}

fn run_status() -> Result<(), String> {
    if !state::is_initialized() {
        return Err("Not initialized. Run 'wm init' first.".to_string());
    }

    let config = state::read_config();

    let extract_status = if config.operations.extract { "running" } else { "paused" };
    let compile_status = if config.operations.compile { "running" } else { "paused" };

    println!("extract: {}", extract_status);
    println!("compile: {}", compile_status);

    Ok(())
}
