//! Tests for ProfileManifest round-trip serialization, especially the `bridge` field.

use bm::profile::ProfileManifest;

const MANIFEST_WITH_BRIDGE: &str = r#"
name: test-profile
display_name: Test Profile
description: A test profile
version: "1.0.0"
schema_version: "1.0.0"
bridge: telegram
"#;

const MANIFEST_WITHOUT_BRIDGE: &str = r#"
name: test-profile
display_name: Test Profile
description: A test profile
version: "1.0.0"
schema_version: "1.0.0"
"#;

#[test]
fn profile_manifest_roundtrip_preserves_bridge_field() {
    let manifest: ProfileManifest =
        serde_yml::from_str(MANIFEST_WITH_BRIDGE).expect("should deserialize");
    assert_eq!(manifest.bridge, Some("telegram".to_string()));

    let serialized = serde_yml::to_string(&manifest).expect("should serialize");
    assert!(
        serialized.contains("bridge: telegram"),
        "serialized output should contain 'bridge: telegram', got:\n{}",
        serialized
    );
}

#[test]
fn profile_manifest_roundtrip_without_bridge_defaults_to_none() {
    let manifest: ProfileManifest =
        serde_yml::from_str(MANIFEST_WITHOUT_BRIDGE).expect("should deserialize");
    assert!(
        manifest.bridge.is_none(),
        "bridge should be None when not present in YAML"
    );

    let serialized = serde_yml::to_string(&manifest).expect("should serialize");
    assert!(
        !serialized.contains("bridge:"),
        "serialized output should NOT contain 'bridge:' when None, got:\n{}",
        serialized
    );
}

const MANIFEST_WITH_OPERATOR: &str = r#"
name: test-profile
display_name: Test Profile
description: A test profile
version: "1.0.0"
schema_version: "1.0.0"
bridge: tuwunel
operator:
  bridge_username: bmadmin
"#;

#[test]
fn profile_manifest_roundtrip_preserves_operator_field() {
    let manifest: ProfileManifest =
        serde_yml::from_str(MANIFEST_WITH_OPERATOR).expect("should deserialize");
    let op = manifest.operator.as_ref().expect("operator should be Some");
    assert_eq!(op.bridge_username, "bmadmin");

    let serialized = serde_yml::to_string(&manifest).expect("should serialize");
    assert!(
        serialized.contains("bridge_username: bmadmin"),
        "serialized output should contain 'bridge_username: bmadmin', got:\n{}",
        serialized
    );
}

#[test]
fn profile_manifest_without_operator_defaults_to_none() {
    let manifest: ProfileManifest =
        serde_yml::from_str(MANIFEST_WITHOUT_BRIDGE).expect("should deserialize");
    assert!(
        manifest.operator.is_none(),
        "operator should be None when not present in YAML"
    );

    let serialized = serde_yml::to_string(&manifest).expect("should serialize");
    assert!(
        !serialized.contains("operator"),
        "serialized output should NOT contain 'operator' when None, got:\n{}",
        serialized
    );
}

#[test]
fn workspace_rs_has_no_stale_push_flag_references() {
    let workspace_src =
        std::fs::read_to_string("src/workspace.rs").expect("should read workspace.rs");
    assert!(
        !workspace_src.contains("--push"),
        "workspace.rs should not contain stale '--push' flag references"
    );
}
