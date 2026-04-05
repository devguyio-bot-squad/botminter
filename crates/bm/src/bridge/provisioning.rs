use anyhow::Result;

use super::credential::{CredentialStore, resolve_credential_from_store};
use super::env_var_suffix;
use super::manifest::{BridgeIdentity, BridgeRoom};
use super::Bridge;

/// Result of provisioning a single member.
#[derive(Debug, Clone)]
pub enum ProvisionMemberResult {
    /// Member was already provisioned.
    AlreadyProvisioned,
    /// External bridge member skipped — no credentials found.
    NoCreds,
    /// Member successfully provisioned.
    Provisioned,
    /// Onboard recipe returned no config.
    NoConfig,
    /// Credential stored, but keyring store failed (with warning message).
    ProvisionedWithKeyringWarning(String),
    /// Re-onboarded: member was in state but had no keyring credential.
    ReOnboarded,
    /// Re-onboard attempted but keyring store failed again.
    ReOnboardedWithKeyringWarning(String),
}

/// Result of the full provisioning operation.
#[derive(Debug)]
pub struct ProvisionResult {
    /// Per-member results, in order.
    pub members: Vec<(String, ProvisionMemberResult)>,
    /// Room created during provisioning, if any.
    pub room_created: Option<String>,
}

impl Bridge {
    /// Provisions bridge identities for team members.
    ///
    /// For each member NOT already in state:
    /// - **Local bridges:** invoke the onboard recipe (creates user + returns token).
    /// - **External bridges:** check for existing credential; skip with warning if absent.
    ///
    /// For local bridges with a `verify` recipe, already-provisioned members are
    /// verified against the actual bridge backend (reconciler pattern). If verification
    /// fails, the stale identity is removed and the member is re-onboarded.
    ///
    /// After provisioning, creates a team room if `rooms` is empty (or was cleared
    /// due to verify failures) and the manifest has a room spec.
    /// Caller must call `save()` to persist state changes.
    pub fn provision(&mut self, members: &[super::BridgeMember], cred_store: &dyn CredentialStore) -> Result<ProvisionResult> {
        let mut results = Vec::new();
        // Track whether any verify failed — indicates possible volume loss,
        // which means rooms in state are also stale and need re-creation.
        let mut any_verify_failed = false;

        for member in members {
            let is_reonboard = if self.state.identities.contains_key(&member.name) {
                // Check if credential is actually in the keyring — if not,
                // re-onboard to recover from a previous keyring-locked failure.
                let has_cred = resolve_credential_from_store(&member.name, cred_store)?;
                if has_cred.is_some() || self.manifest.spec.bridge_type == "external" {
                    // External bridges: trust local state (no verify).
                    if self.manifest.spec.bridge_type == "external" {
                        results.push((member.name.clone(), ProvisionMemberResult::AlreadyProvisioned));
                        continue;
                    }
                    // Local bridges with verify recipe: confirm credentials are
                    // valid against the actual bridge backend.
                    if let Some(ref verify_recipe) = self.manifest.spec.identity.verify {
                        let verify_ok = self
                            .invoke_recipe(verify_recipe, &[member.name.as_str()])
                            .is_ok();
                        if verify_ok {
                            // Reconcile mutable fields from current member list.
                            if let Some(identity) = self.state.identities.get_mut(&member.name) {
                                identity.is_operator = member.is_operator;
                            }
                            results.push((
                                member.name.clone(),
                                ProvisionMemberResult::AlreadyProvisioned,
                            ));
                            continue;
                        }
                        // Verify failed — stale credentials (e.g., volume loss).
                        eprintln!(
                            "  verify failed for {} — re-provisioning",
                            member.name
                        );
                        any_verify_failed = true;
                        self.state.identities.remove(&member.name);
                        // Fall through to re-onboard below
                    } else {
                        // No verify recipe — cannot confirm identity is valid.
                        // Remove stale identity and re-onboard.
                        self.state.identities.remove(&member.name);
                    }
                } else {
                    // No credential in keyring — remove stale identity.
                    self.state.identities.remove(&member.name);
                }
                true
            } else {
                false
            };

            if self.manifest.spec.bridge_type == "external" {
                let has_cred = resolve_credential_from_store(&member.name, cred_store)?;
                if has_cred.is_none() {
                    results.push((member.name.clone(), ProvisionMemberResult::NoCreds));
                    continue;
                }
                let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(&member.name));
                std::env::set_var(&env_key, has_cred.as_ref().unwrap());
            }

            let recipe_result = self.invoke_recipe(
                &self.manifest.spec.identity.onboard.clone(),
                &[member.name.as_str()],
            )?;

            if let Some(config) = recipe_result {
                let username = config["username"]
                    .as_str()
                    .unwrap_or(member.name.as_str())
                    .to_string();
                let user_id = config["user_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let token = config["token"].as_str().map(|s| s.to_string());

                self.state.identities.insert(
                    member.name.clone(),
                    BridgeIdentity {
                        username,
                        user_id,
                        token: None,
                        created_at: chrono::Utc::now().to_rfc3339(),
                        is_operator: member.is_operator,
                    },
                );

                let mut keyring_warning = None;
                if let Some(ref tok) = token {
                    if let Err(e) = cred_store.store(&member.name, tok) {
                        keyring_warning = Some(format!(
                            "Could not store credential for {} in keyring: {}. \
                             Set BM_BRIDGE_TOKEN_{} env var instead.",
                            member.name,
                            e,
                            env_var_suffix(&member.name)
                        ));
                    }
                }

                if let Some(warning) = keyring_warning {
                    let result = if is_reonboard {
                        ProvisionMemberResult::ReOnboardedWithKeyringWarning(warning)
                    } else {
                        ProvisionMemberResult::ProvisionedWithKeyringWarning(warning)
                    };
                    results.push((member.name.clone(), result));
                } else {
                    let result = if is_reonboard {
                        ProvisionMemberResult::ReOnboarded
                    } else {
                        ProvisionMemberResult::Provisioned
                    };
                    results.push((member.name.clone(), result));
                }
            } else {
                results.push((member.name.clone(), ProvisionMemberResult::NoConfig));
            }

            if self.manifest.spec.bridge_type == "external" {
                let env_key = format!("BM_BRIDGE_TOKEN_{}", env_var_suffix(&member.name));
                std::env::remove_var(&env_key);
            }
        }

        // If any verify failed (volume loss), rooms in state are also stale.
        // Clear them so the room-creation logic below re-creates them.
        if any_verify_failed && !self.state.rooms.is_empty() {
            eprintln!("  clearing stale rooms from state (verify failures detected)");
            self.state.rooms.clear();
        }

        // Create team room if rooms are empty and manifest has room spec
        let mut room_created = None;
        if self.state.rooms.is_empty() {
            if let Some(ref room_spec) = self.manifest.spec.room {
                let room_name = format!("{}-general", self.team_name);
                let create_recipe = room_spec.create.clone();
                let room_result = self.invoke_recipe(
                    &create_recipe,
                    &[&room_name],
                )?;

                let room_id = room_result
                    .as_ref()
                    .and_then(|v| v["room_id"].as_str())
                    .map(|s| s.to_string());

                self.state.rooms.push(BridgeRoom {
                    name: room_name.clone(),
                    room_id,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    member: None,
                });
                room_created = Some(room_name);
            }
        }

        self.state.bridge_name = Some(self.manifest.metadata.name.clone());
        self.state.bridge_type = Some(self.manifest.spec.bridge_type.clone());

        Ok(ProvisionResult { members: results, room_created })
    }
}
