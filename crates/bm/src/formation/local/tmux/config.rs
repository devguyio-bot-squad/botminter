use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct TmuxConfig;

impl TmuxConfig {
    pub fn config_content() -> &'static str {
        include_str!("tmux.conf")
    }

    pub fn path() -> Result<PathBuf> {
        let config = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config.join("botminter").join("tmux.conf"))
    }

    pub fn ensure_written() -> Result<()> {
        Self::write_to(&Self::path()?)
    }

    fn write_to(path: &Path) -> Result<()> {
        let dir = path
            .parent()
            .context("Config path has no parent directory")?;
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create config dir {}", dir.display()))?;

        let mut tmp = tempfile::NamedTempFile::new_in(dir)
            .context("Failed to create temp config file")?;
        tmp.write_all(Self::config_content().as_bytes())
            .context("Failed to write config content")?;

        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(tmp.path(), perms)
            .context("Failed to set config file permissions")?;

        tmp.persist(path)
            .context("Failed to persist config file")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_content_is_non_empty() {
        let content = TmuxConfig::config_content();
        assert!(!content.is_empty(), "config_content() must return non-empty content");
    }

    #[test]
    fn config_content_contains_remain_on_exit() {
        let content = TmuxConfig::config_content();
        assert!(
            content.contains("remain-on-exit on"),
            "config must contain 'remain-on-exit on', got:\n{content}"
        );
    }

    #[test]
    fn config_content_contains_botminter_branding() {
        let content = TmuxConfig::config_content();
        assert!(
            content.contains("botminter"),
            "config must contain 'botminter' branding, got:\n{content}"
        );
    }

    #[test]
    fn config_content_contains_keybinding_hints() {
        let content = TmuxConfig::config_content();
        assert!(
            content.contains("C-b n") && content.contains("next"),
            "config must contain keybinding hint 'C-b n:next', got:\n{content}"
        );
        assert!(
            content.contains("C-b [") || content.contains("C-b [:scroll"),
            "config must contain keybinding hint for scroll mode, got:\n{content}"
        );
    }

    #[test]
    fn path_ends_with_config_botminter_tmux_conf() {
        let path = TmuxConfig::path().expect("path() should not fail");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with(".config/botminter/tmux.conf"),
            "path must end with '.config/botminter/tmux.conf', got: {path_str}"
        );
    }

    #[test]
    fn write_to_creates_file_with_correct_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tmux.conf");

        TmuxConfig::write_to(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            TmuxConfig::config_content(),
            "written file content must match config_content()"
        );
    }

    #[test]
    fn write_to_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tmux.conf");

        TmuxConfig::write_to(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "config file must have 0600 permissions, got: {mode:04o}"
        );
    }

    #[test]
    fn write_to_leaves_no_tmp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tmux.conf");

        TmuxConfig::write_to(&path).unwrap();

        let tmp_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            tmp_files.is_empty(),
            "no .tmp files should remain after write_to(), found: {:?}",
            tmp_files.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ensure_written_creates_file_at_config_path() {
        TmuxConfig::ensure_written().unwrap();

        let path = TmuxConfig::path().unwrap();
        assert!(
            path.exists(),
            "ensure_written() must create config file at {}",
            path.display()
        );
    }
}
