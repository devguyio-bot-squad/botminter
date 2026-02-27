# Research: Embedding Directory Trees in a Rust Binary

> Research for M3 — how `bm` embeds `profiles/` into the binary at compile time and extracts them during `bm init`.

---

## Use Case

`bm init` needs to:
1. List available profiles (embedded in the binary)
2. Extract a chosen profile's entire directory tree to disk (preserving structure)
3. Read individual files (e.g., `botminter.yml`) without extracting

The `profiles/` tree contains YAML, Markdown, and skill files, including hidden directories (`.schema/`).

---

## Approaches Compared

| Criterion | `include_dir` | `rust-embed` | `include_bytes!` | `build.rs` codegen |
|---|---|---|---|---|
| Setup | One macro call | One derive macro | Manual per file | ~50 LOC build script |
| Directory tree API | Native `Dir`/`File` tree | Flat path list | N/A | Flat list |
| List subdirs | `dir.dirs()` | Parse path strings | Manual | Manual |
| Hidden files (`.schema/`) | Yes | Yes | Yes | Yes |
| Non-UTF8 / binary | Yes | Yes | Yes | Yes |
| Compression | Optional feature | Optional feature | Manual | Manual |
| Include/exclude filters | No | Yes | Manual | Full control |
| Debug-mode disk reads | No (always embedded) | Yes by default | No | No |
| Empty dirs preserved | Yes | No | No | No |

---

## Recommendation: `include_dir`

**Rationale:**

1. **Native tree structure.** `Dir` / `File` API directly models what we need — profiles are directories. `rust-embed` flattens everything into path strings.

2. **Simplest API for core operations:**
   - List profiles: `PROFILES.dirs()`
   - Extract to disk: recursive walk writing `DirEntry::File` and `DirEntry::Dir`
   - Read metadata: `PROFILES.get_file("rh-scrum/botminter.yml")`

3. **No surprising debug behavior.** `rust-embed` reads from disk in debug builds by default — confusing for a tool that should always be self-contained.

4. **Empty directory preservation.** Skeleton directories that are intentionally empty (e.g., `knowledge/`) are preserved in the tree.

---

## Implementation

**Cargo.toml:**
```toml
[dependencies]
include_dir = "0.7"
```

**Embedding:**
```rust
use include_dir::{include_dir, Dir, DirEntry};

static PROFILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../profiles");
```

Note: since `bm` lives in `crates/bm/`, the path reaches up two levels to the repo root's `profiles/` directory.

**Listing profiles:**
```rust
pub fn list_profiles() -> Vec<&'static str> {
    PROFILES
        .dirs()
        .filter_map(|d| d.path().file_name()?.to_str())
        .collect()
}
```

**Reading a file without extraction:**
```rust
pub fn get_profile_metadata(profile: &str) -> Option<&'static str> {
    let path = format!("{}/botminter.yml", profile);
    PROFILES.get_file(path).and_then(|f| f.contents_utf8())
}
```

**Extracting a full profile to disk:**
```rust
pub fn extract_profile(name: &str, target: &Path) -> std::io::Result<()> {
    let dir = PROFILES.get_dir(name).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, format!("unknown profile: {}", name))
    })?;
    extract_dir_recursive(dir, target)
}

fn extract_dir_recursive(dir: &Dir<'static>, target: &Path) -> std::io::Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => {
                let dest = target.join(d.path().file_name().unwrap());
                fs::create_dir_all(&dest)?;
                extract_dir_recursive(d, &dest)?;
            }
            DirEntry::File(f) => {
                fs::write(target.join(f.path().file_name().unwrap()), f.contents())?;
            }
        }
    }
    Ok(())
}
```

---

## Gotchas

1. **No file permissions.** Extracted files get default umask permissions. If any files need executable bits, `chmod` after extraction.
2. **No symlink following.** Ensure the embedded tree contains real files only.
3. **Binary size.** All files embedded uncompressed by default. Expected `profiles/` content (YAML, Markdown) is well under 1 MB. Enable `compression` feature if needed.
4. **Recompilation tracking.** Changes to embedded files may not always trigger recompilation. `cargo clean && cargo build` if changes aren't reflected.
5. **Path from workspace crate.** Since `bm` lives in `crates/bm/`, the `include_dir!` path must be `$CARGO_MANIFEST_DIR/../../profiles` to reach the repo root.

---

## Sources

- [include_dir crate](https://crates.io/crates/include_dir)
- [rust-embed crate](https://crates.io/crates/rust-embed)
