use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::{bail, Context, Result};

use crate::bridge;
use crate::config;
use crate::formation::{self, CredentialDomain};
use crate::git::manifest_flow::credential_keys;
use crate::workspace;

/// Handles `bm credentials export -o <file> [-t team]`.
///
/// Reads all members' GitHub App and bridge credentials from the keyring
/// and writes them to a YAML file with 0600 permissions.
pub fn export(output: &str, team_flag: Option<&str>) -> Result<()> {
    let cfg = config::load()?;
    let team = config::resolve_team(&cfg, team_flag)?;
    let team_repo = team.path.join("team");

    // Enumerate hired members from team repo
    let members = workspace::list_member_dirs(&team_repo.join("members"))?;
    if members.is_empty() {
        bail!("No members found in team '{}'. Nothing to export.", team.name);
    }

    let formation = formation::create_local_formation(&team.name)?;

    // Discover bridge (may be None if no bridge configured)
    let bridge_dir = bridge::discover(&team_repo, &team.name)?;
    let bridge_name = bridge_dir.as_ref().and_then(|dir| {
        bridge::load_manifest(dir).ok().map(|m| m.metadata.name)
    });

    // Build YAML document
    let mut members_yaml = serde_json::Map::new();

    for member in &members {
        let mut member_entry = serde_json::Map::new();

        // ── GitHub App credentials ──────────────────────────────────
        let app_store = formation.credential_store(CredentialDomain::GitHubApp {
            team_name: team.name.clone(),
            member_name: member.clone(),
        })?;

        let app_id = app_store.retrieve(&credential_keys::app_id(member))?;
        let client_id = app_store.retrieve(&credential_keys::client_id(member))?;
        let private_key = app_store.retrieve(&credential_keys::private_key(member))?;
        let installation_id = app_store.retrieve(&credential_keys::installation_id(member))?;

        if let (Some(aid), Some(cid), Some(pk), Some(iid)) =
            (app_id, client_id, private_key, installation_id)
        {
            let mut github_app = serde_json::Map::new();
            github_app.insert("app_id".into(), serde_json::Value::String(aid));
            github_app.insert("client_id".into(), serde_json::Value::String(cid));
            github_app.insert("private_key".into(), serde_json::Value::String(pk));
            github_app.insert("installation_id".into(), serde_json::Value::String(iid));
            member_entry.insert(
                "github_app".into(),
                serde_json::Value::Object(github_app),
            );
        } else {
            eprintln!(
                "Warning: Incomplete GitHub App credentials for member '{}' (skipping App section).",
                member
            );
        }

        // ── Bridge credentials ──────────────────────────────────────
        if let Some(ref bname) = bridge_name {
            let bstate_path = bridge::state_path(&cfg.workzone, &team.name);
            let bridge_store = formation.credential_store(CredentialDomain::Bridge {
                team_name: team.name.clone(),
                bridge_name: bname.clone(),
                state_path: bstate_path,
            })?;

            // Bridge credential store uses member name directly as key
            if let Some(token) = bridge_store.retrieve(member)? {
                let mut bridge_section = serde_json::Map::new();
                bridge_section.insert("token".into(), serde_json::Value::String(token));
                member_entry.insert(
                    "bridge".into(),
                    serde_json::Value::Object(bridge_section),
                );
            }
        }

        if !member_entry.is_empty() {
            members_yaml.insert(member.clone(), serde_json::Value::Object(member_entry));
        }
    }

    let doc = serde_json::json!({
        "team": team.name,
        "members": serde_json::Value::Object(members_yaml),
    });

    let yaml_content = serde_yml::to_string(&doc)
        .context("Failed to serialize credentials to YAML")?;

    // Write file
    fs::write(output, &yaml_content)
        .with_context(|| format!("Failed to write credentials file: {output}"))?;

    // Set 0600 permissions
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(output, perms)
        .with_context(|| format!("Failed to set permissions on: {output}"))?;

    eprintln!(
        "⚠️  SECURITY WARNING: {} contains sensitive credentials (private keys, tokens).\n\
         Transfer securely and delete after import on the new machine.\n\
         File permissions set to 0600 (owner read/write only).",
        output
    );

    println!("Exported credentials for {} member(s) to '{}'.", members.len(), output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::{InMemoryKeyValueCredentialStore, KeyValueCredentialStore};
    use crate::git::manifest_flow::{self, PreGeneratedCredentials};

    #[test]
    fn export_yaml_format_matches_design() {
        // Build the same JSON structure the export function builds
        let mut members_yaml = serde_json::Map::new();

        let mut superman_entry = serde_json::Map::new();
        let mut github_app = serde_json::Map::new();
        github_app.insert("app_id".into(), serde_json::Value::String("123456".into()));
        github_app.insert("client_id".into(), serde_json::Value::String("Iv1.abc".into()));
        github_app.insert("private_key".into(), serde_json::Value::String("-----BEGIN RSA PRIVATE KEY-----\nfake\n-----END RSA PRIVATE KEY-----".into()));
        github_app.insert("installation_id".into(), serde_json::Value::String("789012".into()));
        superman_entry.insert("github_app".into(), serde_json::Value::Object(github_app));

        let mut bridge_section = serde_json::Map::new();
        bridge_section.insert("token".into(), serde_json::Value::String("syt_xxx".into()));
        superman_entry.insert("bridge".into(), serde_json::Value::Object(bridge_section));

        members_yaml.insert("superman".into(), serde_json::Value::Object(superman_entry));

        let doc = serde_json::json!({
            "team": "my-team",
            "members": serde_json::Value::Object(members_yaml),
        });

        let yaml = serde_yml::to_string(&doc).unwrap();

        // Parse back and verify structure
        let parsed: serde_json::Value = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed["team"].as_str().unwrap(), "my-team");

        let superman = &parsed["members"]["superman"];
        assert_eq!(superman["github_app"]["app_id"].as_str().unwrap(), "123456");
        assert_eq!(superman["github_app"]["client_id"].as_str().unwrap(), "Iv1.abc");
        assert!(superman["github_app"]["private_key"].as_str().unwrap().contains("BEGIN RSA PRIVATE KEY"));
        assert_eq!(superman["github_app"]["installation_id"].as_str().unwrap(), "789012");
        assert_eq!(superman["bridge"]["token"].as_str().unwrap(), "syt_xxx");
    }

    #[test]
    fn round_trip_store_and_read_back() {
        let store = InMemoryKeyValueCredentialStore::new();
        let creds = PreGeneratedCredentials {
            app_id: "111".into(),
            client_id: "Iv1.xyz".into(),
            private_key: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----".into(),
            installation_id: "222".into(),
        };
        manifest_flow::store_pregenerated_credentials(&store, "member-a", &creds).unwrap();

        // Read back using credential_keys
        assert_eq!(
            store.retrieve(&credential_keys::app_id("member-a")).unwrap(),
            Some("111".to_string())
        );
        assert_eq!(
            store.retrieve(&credential_keys::client_id("member-a")).unwrap(),
            Some("Iv1.xyz".to_string())
        );
        assert_eq!(
            store.retrieve(&credential_keys::private_key("member-a")).unwrap(),
            Some("-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----".to_string())
        );
        assert_eq!(
            store.retrieve(&credential_keys::installation_id("member-a")).unwrap(),
            Some("222".to_string())
        );
    }

    #[test]
    fn export_file_permissions() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        fs::write(path, "test").unwrap();
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).unwrap();

        let meta = fs::metadata(path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }
}
