mod show;
mod sync;

use anyhow::Result;
use comfy_table::{ContentArrangement, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Table};

use crate::config;
use crate::profile;

pub use show::format_team_summary;

/// Handles `bm teams list` — displays a table of all registered teams.
pub fn list() -> Result<()> {
    let cfg = config::load()?;

    if cfg.teams.is_empty() {
        println!("No teams registered. Run `bm init` to create one.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec!["Team", "Profile", "GitHub", "Members", "Projects", "Default"]);

    for team in &cfg.teams {
        let is_default = cfg.default_team.as_ref() == Some(&team.name);
        let team_repo = team.path.join("team");
        let member_count = profile::discover_member_dirs(&team_repo).len();
        let project_count = profile::read_team_projects(&team_repo).len();
        table.add_row(vec![
            team.name.as_str(),
            team.profile.as_str(),
            team.github_repo.as_str(),
            &member_count.to_string(),
            &project_count.to_string(),
            if is_default { "✔" } else { "" },
        ]);
    }

    println!("{table}");
    Ok(())
}

pub use show::show;
pub use sync::sync;
