mod linux;
mod macos;

use anyhow::{bail, Result};

use super::Formation;

/// Creates a local formation appropriate for the current platform.
///
/// On Linux, returns `LinuxLocalFormation` which delegates to existing
/// free functions. On macOS, returns `MacosLocalFormation` which returns
/// "not yet supported" errors.
pub fn create_local_formation(team_name: &str) -> Result<Box<dyn Formation>> {
    if cfg!(target_os = "linux") {
        Ok(Box::new(linux::LinuxLocalFormation::new(team_name)))
    } else if cfg!(target_os = "macos") {
        Ok(Box::new(macos::MacosLocalFormation::new(team_name)))
    } else {
        bail!("Local formation is not supported on this platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_local_formation_returns_formation_on_linux() {
        if !cfg!(target_os = "linux") {
            return; // skip on non-Linux
        }
        let formation = create_local_formation("my-team").unwrap();
        assert_eq!(formation.name(), "local");
    }

    #[test]
    fn create_local_formation_returns_boxed_dyn_formation() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let formation: Box<dyn Formation> = create_local_formation("my-team").unwrap();
        assert_eq!(formation.name(), "local");
    }
}
