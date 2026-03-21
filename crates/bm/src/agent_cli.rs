use clap::{Parser, Subcommand, ValueEnum};

/// Agent-facing tools for BotMinter workspaces.
#[derive(Parser)]
#[command(name = "bm-agent", version, about)]
pub struct AgentCli {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Brain-to-loop messaging
    Inbox {
        #[command(subcommand)]
        command: InboxCommand,
    },
    /// Claude Code specific tools
    Claude {
        #[command(subcommand)]
        command: ClaudeCommand,
    },
}

#[derive(Subcommand)]
pub enum InboxCommand {
    /// Send a message to the loop's inbox
    Write {
        /// Message text
        message: String,
        /// Sender identity
        #[arg(long, default_value = "brain")]
        from: String,
    },
    /// Read and consume pending messages
    Read {
        /// Output format
        #[arg(long, default_value = "hook")]
        format: InboxFormat,
    },
    /// View pending messages without consuming
    Peek,
}

#[derive(Subcommand)]
pub enum ClaudeCommand {
    /// Claude Code hook handlers
    Hook {
        #[command(subcommand)]
        command: ClaudeHookCommand,
    },
}

#[derive(Subcommand)]
pub enum ClaudeHookCommand {
    /// PostToolUse hook — checks inbox, returns additionalContext
    PostToolUse,
}

#[derive(Clone, ValueEnum)]
pub enum InboxFormat {
    Json,
    Hook,
}
