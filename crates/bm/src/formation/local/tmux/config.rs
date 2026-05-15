use std::path::PathBuf;

use anyhow::Result;

pub struct TmuxConfig;

impl TmuxConfig {
    pub fn config_content() -> &'static str {
        ""
    }

    pub fn path() -> Result<PathBuf> {
        Ok(PathBuf::from("/dev/null"))
    }

    pub fn ensure_written() -> Result<()> {
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
    fn ensure_written_creates_file_with_correct_content() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("tmux.conf");

        // ensure_written() should write to the real path, not tmp —
        // this test verifies the file at the real path after calling ensure_written().
        // For red phase: this will fail because ensure_written() is a no-op stub.
        TmuxConfig::ensure_written().unwrap();

        let real_path = TmuxConfig::path().unwrap();
        assert!(
            real_path.exists(),
            "ensure_written() must create config file at {}, but it does not exist",
            real_path.display()
        );

        let content = std::fs::read_to_string(&real_path).unwrap();
        assert_eq!(
            content,
            TmuxConfig::config_content(),
            "written file content must match config_content()"
        );
    }

    #[test]
    fn ensure_written_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        TmuxConfig::ensure_written().unwrap();

        let real_path = TmuxConfig::path().unwrap();
        assert!(real_path.exists(), "config file must exist after ensure_written()");

        let metadata = std::fs::metadata(&real_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "config file must have 0600 permissions, got: {mode:04o}"
        );
    }

    #[test]
    fn ensure_written_leaves_no_tmp_files() {
        TmuxConfig::ensure_written().unwrap();

        let real_path = TmuxConfig::path().unwrap();
        if let Some(parent) = real_path.parent() {
            if parent.exists() {
                let tmp_files: Vec<_> = std::fs::read_dir(parent)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .contains(".tmp")
                    })
                    .collect();
                assert!(
                    tmp_files.is_empty(),
                    "no .tmp files should remain after ensure_written(), found: {:?}",
                    tmp_files.iter().map(|e| e.file_name()).collect::<Vec<_>>()
                );
            }
        }
    }
}
