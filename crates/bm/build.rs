use std::fs;
use std::path::{Path, PathBuf};

/// Profiles to include in release builds (comma-separated).
const SHIPPED_PROFILES: &str = "scrum-compact";

/// Bridges to strip from profile manifests in release builds (comma-separated).
const STRIPPED_BRIDGES: &[&str] = &["rocketchat"];

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let profiles_src = manifest_dir.join("../../profiles");
    let profile = std::env::var("PROFILE").unwrap(); // "debug" or "release"

    // When the `console` feature is enabled, warn if console assets are missing.
    // This does NOT fail the build — developers without Node.js can still build
    // (the `#[allow_missing = true]` on ConsoleAssets handles that at compile time).
    if std::env::var("CARGO_FEATURE_CONSOLE").is_ok() {
        let console_build = manifest_dir.join("../../console/build");
        let has_assets = console_build.is_dir()
            && fs::read_dir(&console_build)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false);
        if !has_assets {
            println!(
                "cargo:warning=Console feature is enabled but console/build/ is missing or empty. \
                 The web console will not be available at runtime. \
                 Run 'npm ci && npm run build' in console/ to build assets."
            );
        }
        println!("cargo:rerun-if-changed=../../console/build");
    }

    let profiles_dir = if profile == "release" {
        let staging = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("profiles-staging");
        stage_release_profiles(&profiles_src, &staging);
        staging
    } else {
        // Dev build: use all profiles as-is
        profiles_src
    };

    let profiles_dir = fs::canonicalize(&profiles_dir)
        .unwrap_or_else(|e| panic!("Failed to canonicalize profiles dir {:?}: {}", profiles_dir, e));

    println!(
        "cargo:rustc-env=BM_PROFILES_DIR={}",
        profiles_dir.display()
    );
    println!("cargo:rerun-if-changed=../../profiles");
}

/// Copies only shipped profiles to the staging directory, then strips
/// excluded bridges from their botminter.yml manifests.
fn stage_release_profiles(src: &Path, staging: &Path) {
    if staging.exists() {
        fs::remove_dir_all(staging).unwrap();
    }
    fs::create_dir_all(staging).unwrap();

    for name in SHIPPED_PROFILES.split(',').map(str::trim) {
        let profile_src = src.join(name);
        if !profile_src.is_dir() {
            panic!("Shipped profile '{}' not found at {:?}", name, profile_src);
        }
        let profile_dst = staging.join(name);
        copy_dir_recursive(&profile_src, &profile_dst);
        strip_bridges(&profile_dst.join("botminter.yml"));
    }
}

/// Removes excluded bridge entries from a botminter.yml file.
fn strip_bridges(manifest_path: &Path) {
    let content = fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", manifest_path, e));

    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        // Check if this is a bridge entry to strip: "  - name: <bridge>"
        let should_strip = STRIPPED_BRIDGES.iter().any(|b| {
            line.trim() == format!("- name: {}", b)
        });

        if should_strip {
            // Skip this line and all indented lines that follow (display_name, description, type)
            i += 1;
            while i < lines.len() {
                let next = lines[i];
                if next.starts_with("    ") && !next.trim().starts_with("- name:") {
                    i += 1;
                } else {
                    break;
                }
            }
        } else {
            result.push(line);
            i += 1;
        }
    }

    let output = result.join("\n");
    // Preserve trailing newline if original had one
    let output = if content.ends_with('\n') {
        format!("{}\n", output)
    } else {
        output
    };
    fs::write(manifest_path, output)
        .unwrap_or_else(|e| panic!("Failed to write {:?}: {}", manifest_path, e));
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}
