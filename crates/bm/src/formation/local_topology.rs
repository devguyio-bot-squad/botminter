use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::state::{self, RuntimeState};
use crate::topology::{self, Endpoint, MemberTopology, Topology};

/// Writes a local topology file after starting members.
///
/// Builds a `Topology` from the current runtime state for the given team
/// and saves it to disk. Only members belonging to `team_name` are included.
pub fn write_local_topology(
    workzone: &Path,
    team_name: &str,
    state: &RuntimeState,
) -> Result<()> {
    let team_prefix = format!("{}/", team_name);
    let mut members = HashMap::new();

    for (key, rt) in &state.members {
        if !key.starts_with(&team_prefix) {
            continue;
        }
        let member_name = key.strip_prefix(&team_prefix).unwrap_or(key);
        members.insert(
            member_name.to_string(),
            MemberTopology {
                status: if state::is_alive(rt.pid) {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                },
                endpoint: Endpoint::Local {
                    pid: rt.pid,
                    workspace: rt.workspace.clone(),
                },
            },
        );
    }

    let topo = Topology {
        formation: "local".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        members,
    };

    let topo_path = topology::topology_path(workzone, team_name);
    topology::save(&topo_path, &topo)?;

    Ok(())
}
