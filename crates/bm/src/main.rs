use anyhow::Result;
use clap::Parser;
use clap_complete::CompleteEnv;

use bm::cli::{
    BridgeCommand, BridgeIdentityCommand, BridgeRoomCommand, Cli, Command, DaemonCommand,
    KnowledgeCommand, MembersCommand, ProfilesCommand, ProjectsCommand, RolesCommand,
    TeamsCommand,
};
use bm::commands;

fn main() -> Result<()> {
    CompleteEnv::with_factory(commands::completions::build_cli_with_completions).complete();

    let cli = Cli::parse();

    match cli.command {
        Command::Init {
            non_interactive,
            profile,
            team_name,
            org,
            repo,
            project,
            github_project_board,
            bridge,
            skip_github,
            workzone,
        } => {
            if non_interactive {
                commands::init::run_non_interactive(
                    profile,
                    team_name,
                    org,
                    repo,
                    project,
                    github_project_board,
                    bridge,
                    skip_github,
                    workzone,
                )?;
            } else {
                commands::init::run()?;
            }
        }

        Command::Profiles { command } => match command {
            ProfilesCommand::List => commands::profiles::list()?,
            ProfilesCommand::Describe { profile, show_tags } => {
                commands::profiles::describe(&profile, show_tags)?
            }
            ProfilesCommand::Init { force } => commands::profiles_init::run(force)?,
        },

        Command::Teams { command } => match command {
            TeamsCommand::List => commands::teams::list()?,
            TeamsCommand::Show { name, team } => {
                commands::teams::show(name.as_deref(), team.as_deref())?;
            }
            TeamsCommand::Sync { repos, bridge, all, verbose, team } => {
                let effective_repos = repos || all;
                let effective_bridge = bridge || all;
                commands::teams::sync(effective_repos, effective_bridge, verbose, team.as_deref())?;
            }
        },

        Command::Hire { role, name, team } => {
            commands::hire::run(&role, name.as_deref(), team.as_deref())?;
        }

        Command::Members { command } => match command {
            MembersCommand::List { team } => {
                commands::members::list(team.as_deref())?;
            }
            MembersCommand::Show { member, team } => {
                commands::members::show(&member, team.as_deref())?;
            }
        },

        Command::Roles { command } => match command {
            RolesCommand::List { team } => {
                commands::roles::list(team.as_deref())?;
            }
        },

        Command::Projects { command } => match command {
            ProjectsCommand::List { team } => {
                commands::projects::list(team.as_deref())?;
            }
            ProjectsCommand::Show { project, team } => {
                commands::projects::show(&project, team.as_deref())?;
            }
            ProjectsCommand::Add { url, team } => {
                commands::projects::add(&url, team.as_deref())?;
            }
            ProjectsCommand::Sync { team } => {
                commands::projects::sync(team.as_deref())?;
            }
        },

        Command::Knowledge {
            command,
            team,
            scope,
        } => match command {
            Some(KnowledgeCommand::List { team: t, scope: s }) => {
                let team_flag = t.as_deref().or(team.as_deref());
                let scope_flag = s.as_deref().or(scope.as_deref());
                commands::knowledge::list(team_flag, scope_flag)?;
            }
            Some(KnowledgeCommand::Show { path, team: t }) => {
                let team_flag = t.as_deref().or(team.as_deref());
                commands::knowledge::show(&path, team_flag)?;
            }
            None => {
                commands::knowledge::interactive(team.as_deref(), scope.as_deref())?;
            }
        },

        Command::Bridge { command } => match command {
            BridgeCommand::Start { team } => commands::bridge::start(team.as_deref())?,
            BridgeCommand::Stop { team } => commands::bridge::stop(team.as_deref())?,
            BridgeCommand::Status { team, reveal } => commands::bridge::status(team.as_deref(), reveal)?,
            BridgeCommand::Identity { command } => match command {
                BridgeIdentityCommand::Add { username, team } => {
                    commands::bridge::identity_add(&username, team.as_deref())?
                }
                BridgeIdentityCommand::Rotate { username, team } => {
                    commands::bridge::identity_rotate(&username, team.as_deref())?
                }
                BridgeIdentityCommand::Remove { username, team } => {
                    commands::bridge::identity_remove(&username, team.as_deref())?
                }
                BridgeIdentityCommand::Show { username, reveal, team } => {
                    commands::bridge::identity_show(&username, reveal, team.as_deref())?
                }
                BridgeIdentityCommand::List { team } => {
                    commands::bridge::identity_list(team.as_deref())?
                }
            },
            BridgeCommand::Room { command } => match command {
                BridgeRoomCommand::Create { name, team } => {
                    commands::bridge::room_create(&name, team.as_deref())?
                }
                BridgeRoomCommand::List { team } => {
                    commands::bridge::room_list(team.as_deref())?
                }
            },
        },

        Command::Daemon { command } => match command {
            DaemonCommand::Start {
                team,
                mode,
                port,
                interval,
            } => {
                commands::daemon::start(team.as_deref(), &mode, port, interval)?;
            }
            DaemonCommand::Stop { team } => {
                commands::daemon::stop(team.as_deref())?;
            }
            DaemonCommand::Status { team } => {
                commands::daemon::status(team.as_deref())?;
            }
        },

        Command::DaemonRun {
            team,
            mode,
            port,
            interval,
        } => {
            commands::daemon::run_daemon(&team, &mode, port, interval)?;
        }

        Command::Chat {
            member,
            team,
            hat,
            render_system_prompt,
        } => {
            commands::chat::run(
                &member,
                team.as_deref(),
                hat.as_deref(),
                render_system_prompt,
            )?;
        }

        Command::Minty { team } => {
            commands::minty::run(team.as_deref())?;
        }

        Command::Start {
            member,
            team,
            formation,
            no_bridge,
            bridge_only,
        } => {
            commands::start::run(team.as_deref(), formation.as_deref(), no_bridge, bridge_only, member.as_deref())?;
        }
        Command::Stop { member, team, force, bridge } => {
            commands::stop::run(team.as_deref(), force, member.as_deref(), bridge)?;
        }
        Command::Status { team, verbose } => {
            commands::status::run(team.as_deref(), verbose)?;
        }
        Command::Bootstrap {
            non_interactive,
            name,
            cpus,
            memory,
            disk,
        } => {
            commands::bootstrap::run(non_interactive, name, cpus, &memory, &disk)?;
        }
        Command::Completions { shell } => {
            commands::completions::run(shell)?;
        }
    }

    Ok(())
}
