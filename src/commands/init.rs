use crate::errors::UepmError;
use crate::manifest::{create_manifest, read_manifest, write_manifest};
use crate::uproject::{add_plugin_directory, find_uproject, get_engine_association, is_guid};
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum InstallMode {
    Symlink,
    Copy,
    None,
}

pub fn detect_install_mode(project_dir: &Path) -> InstallMode {
    if std::env::var("P4PORT").is_ok() || std::env::var("P4CONFIG").is_ok() {
        return InstallMode::Copy;
    }
    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".p4config").exists() {
            return InstallMode::Copy;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    if cfg!(windows) {
        return InstallMode::Copy;
    }

    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".git").is_dir() {
            return InstallMode::Symlink;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    InstallMode::Symlink
}

pub async fn run(yes: bool) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let mode = select_install_mode(&project_dir, yes)?;
    run_init_with_mode(&project_dir, mode).await
}

fn select_install_mode(project_dir: &Path, yes: bool) -> Result<InstallMode, UepmError> {
    let default = detect_install_mode(project_dir);
    if yes {
        return Ok(default);
    }
    if !dialoguer::console::Term::stdout().is_term() {
        return Err(UepmError::InteractiveRequired);
    }

    let options = [
        "symlink — symbolic links in UEPMPlugins/ (git workflow)",
        "copy   — real files in UEPMPlugins/ (Perforce / any VCS)",
        "none   — UEPM handles init only, no postinstall hook",
    ];
    let default_idx = match default {
        InstallMode::Symlink => 0,
        InstallMode::Copy => 1,
        InstallMode::None => 2,
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Install mode")
        .default(default_idx)
        .items(&options)
        .interact()
        .map_err(|e| UepmError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    Ok(match selection {
        0 => InstallMode::Symlink,
        1 => InstallMode::Copy,
        _ => InstallMode::None,
    })
}

pub async fn run_init_with_mode(project_dir: &Path, _mode: InstallMode) -> Result<(), UepmError> {
    let uproject_path = find_uproject(project_dir)?;

    let engine_assoc = get_engine_association(&uproject_path)?;
    let engine_version = if is_guid(&engine_assoc) {
        crate::output::print_warn(
            "Engine is a launcher-installed GUID — engine_version will be omitted from uepm.ini",
        );
        None
    } else {
        Some(engine_assoc.as_str())
    };

    add_plugin_directory(&uproject_path, "UEPMPlugins")?;

    // Create uepm.ini if missing; if it already exists, only update engine_version
    let uepm_ini = project_dir.join("uepm.ini");
    if uepm_ini.exists() {
        if let Some(ver) = engine_version {
            let mut manifest = read_manifest(project_dir)?;
            manifest.engine_version = Some(ver.to_string());
            write_manifest(project_dir, &manifest)?;
        }
    } else {
        create_manifest(project_dir, engine_version)?;
    }

    let uepm_plugins = project_dir.join("UEPMPlugins");
    if !uepm_plugins.exists() {
        std::fs::create_dir_all(&uepm_plugins)?;
    }

    crate::output::print_success("Project initialized for UEPM");
    crate::output::print_info("Run 'uepm install @scope/plugin' to add your first plugin");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_uproject(dir: &std::path::Path, engine: &str) {
        let path = dir.join("Test.uproject");
        std::fs::write(
            path,
            serde_json::to_string_pretty(&json!({ "EngineAssociation": engine })).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_detect_mode_p4_env() {
        std::env::set_var("P4PORT", "perforce:1666");
        let mode = detect_install_mode(std::path::Path::new("."));
        std::env::remove_var("P4PORT");
        assert_eq!(mode, InstallMode::Copy);
    }

    #[test]
    fn test_detect_mode_windows_is_copy() {
        if cfg!(windows) {
            let mode = detect_install_mode(std::path::Path::new("."));
            assert_eq!(mode, InstallMode::Copy);
        }
    }

    #[tokio::test]
    async fn test_init_creates_uepm_ini_and_modifies_uproject() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");

        run_init_with_mode(dir.path(), InstallMode::Symlink)
            .await
            .unwrap();

        assert!(dir.path().join("uepm.ini").exists());
        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));

        let uproject = dir.path().join("Test.uproject");
        let content = std::fs::read_to_string(uproject).unwrap();
        assert!(content.contains("UEPMPlugins"));

        assert!(dir.path().join("UEPMPlugins").is_dir());
    }

    #[tokio::test]
    async fn test_init_preserves_existing_plugins() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");
        // Pre-populate uepm.ini with plugins
        std::fs::write(
            dir.path().join("uepm.ini"),
            "[plugins]\n@acme/existing = ^1.0.0\n",
        )
        .unwrap();

        run_init_with_mode(dir.path(), InstallMode::Symlink)
            .await
            .unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@acme/existing"), "init wiped existing plugins");
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
    }

    #[tokio::test]
    async fn test_init_skips_engine_version_for_guid() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");

        run_init_with_mode(dir.path(), InstallMode::Copy)
            .await
            .unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.engine_version.is_none());
    }
}
