use clap::{Parser, Subcommand};
use std::process::ExitCode;

mod compile;
mod compress;
mod dive;
mod extract;
mod init;
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

    /// Manage Open Horizons dive pack context
    Dive {
        #[command(subcommand)]
        command: DiveCommands,
    },

    /// Hook entry points (called by Claude Code hooks)
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
}

#[derive(Subcommand)]
enum DiveCommands {
    /// Load a dive pack from OH and write to OH_context.md
    Load {
        /// Dive pack ID to load
        pack_id: String,
    },

    /// Clear the current OH context
    Clear,

    /// Show current OH context
    Show,
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
        Commands::Dive { command } => match command {
            DiveCommands::Load { pack_id } => dive::load(&pack_id),
            DiveCommands::Clear => dive::clear(),
            DiveCommands::Show => dive::show(),
        },
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
