use std::process;

use clap::Parser;

use bm::agent_cli::{AgentCli, AgentCommand, ClaudeCommand, ClaudeHookCommand, InboxCommand, InboxFormat};
use bm::brain::inbox;

fn main() {
    let cli = AgentCli::parse();

    let result = match cli.command {
        AgentCommand::Inbox { command } => run_inbox(command),
        AgentCommand::Claude { command } => run_claude(command),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run_inbox(command: InboxCommand) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = inbox::discover_workspace_root(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a BotMinter workspace (no .botminter.workspace found)"))?;
    let path = inbox::inbox_path(&root);

    match command {
        InboxCommand::Write { message, from } => {
            inbox::write_message(&path, &from, &message)?;
            eprintln!("Message written to inbox.");
        }
        InboxCommand::Read { format } => {
            let result = inbox::read_messages(&path, true)?;
            match format {
                InboxFormat::Json => {
                    let json = serde_json::to_string(&result.messages)?;
                    println!("{json}");
                }
                InboxFormat::Hook => {
                    if let Some(response) = inbox::format_hook_response(&result.messages) {
                        println!("{response}");
                    }
                }
            }
        }
        InboxCommand::Peek => {
            let result = inbox::read_messages(&path, false)?;
            if result.messages.is_empty() {
                println!("No pending messages.");
            } else {
                for msg in &result.messages {
                    println!("[{}] ({}): {}", msg.ts, msg.from, msg.message);
                }
            }
        }
    }

    Ok(())
}

fn run_claude(command: ClaudeCommand) -> anyhow::Result<()> {
    match command {
        ClaudeCommand::Hook { command } => run_claude_hook(command),
    }
}

fn run_claude_hook(command: ClaudeHookCommand) -> anyhow::Result<()> {
    match command {
        ClaudeHookCommand::PostToolUse => {
            // This command NEVER fails — always exits 0.
            // Errors are silently swallowed.
            let _ = try_post_tool_use();
            Ok(())
        }
    }
}

fn try_post_tool_use() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = match inbox::discover_workspace_root(&cwd) {
        Some(r) => r,
        None => return Ok(()),
    };
    let path = inbox::inbox_path(&root);
    let result = inbox::read_messages(&path, true)?;
    if let Some(response) = inbox::format_hook_response(&result.messages) {
        println!("{response}");
    }
    Ok(())
}
