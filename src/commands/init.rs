use crate::errors::UepmError;
use crate::manifest::{create_manifest, manifest_exists, read_manifest, write_manifest};
use crate::uproject::{add_plugin_directory, find_uproject, get_engine_association, is_guid};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::io::Write;
use std::path::Path;

pub async fn run(yes: bool) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let commit = select_commit_plugins(&project_dir, yes)?;
    run_init_with_commit(&project_dir, commit).await
}

fn detect_p4(project_dir: &Path) -> bool {
    if std::env::var("P4PORT").is_ok() || std::env::var("P4CONFIG").is_ok() {
        return true;
    }
    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".p4config").exists() {
            return true;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    false
}

fn detect_git(project_dir: &Path) -> bool {
    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".git").is_dir() {
            return true;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    false
}

fn select_commit_plugins(project_dir: &Path, yes: bool) -> Result<bool, UepmError> {
    // P4 detected → default commit=true; otherwise default commit=false (ignore like node_modules)
    let default = detect_p4(project_dir);

    if yes {
        return Ok(default);
    }
    if !dialoguer::console::Term::stdout().is_term() {
        return Err(UepmError::InteractiveRequired);
    }

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Commit UEPMPlugins to version control? (no = add to .gitignore/.p4ignore)")
        .default(default)
        .interact()
        .map_err(|e| UepmError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
}

pub async fn run_init_with_commit(project_dir: &Path, commit_plugins: bool) -> Result<(), UepmError> {
    let uproject_path = find_uproject(project_dir)?;

    let engine_assoc = get_engine_association(&uproject_path)?;
    let engine_version = if is_guid(&engine_assoc) {
        crate::output::print_warn(
            "Engine is a launcher-installed GUID — engine_version will be omitted from Config/UEPM.ini",
        );
        None
    } else {
        Some(engine_assoc.as_str())
    };

    add_plugin_directory(&uproject_path, "UEPMPlugins")?;

    if manifest_exists(project_dir) {
        if let Some(ver) = engine_version {
            let mut manifest = read_manifest(project_dir)?;
            manifest.engine_version = Some(ver.to_string());
            manifest.commit_plugins = commit_plugins;
            write_manifest(project_dir, &manifest)?;
        }
    } else {
        create_manifest(project_dir, engine_version, commit_plugins)?;
    }

    let uepm_plugins = project_dir.join("UEPMPlugins");
    if !uepm_plugins.exists() {
        std::fs::create_dir_all(&uepm_plugins)?;
    }

    if !commit_plugins {
        if detect_git(project_dir) {
            append_ignore(project_dir.join(".gitignore"), "UEPMPlugins/")?;
        }
        if detect_p4(project_dir) {
            append_ignore(project_dir.join(".p4ignore"), "UEPMPlugins/")?;
        }
    }

    crate::output::print_success("Project initialized for UEPM");
    crate::output::print_info("Run 'uepm install @scope/plugin' to add your first plugin");

    Ok(())
}

fn append_ignore(path: std::path::PathBuf, entry: &str) -> Result<(), UepmError> {
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        if content.lines().any(|l| l.trim() == entry) {
            return Ok(());
        }
        let mut f = std::fs::OpenOptions::new().append(true).open(&path)?;
        writeln!(f, "\n# UEPM\n{entry}")?;
    } else {
        std::fs::write(&path, format!("# UEPM\n{entry}\n"))?;
    }
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
    fn test_detect_p4_env() {
        std::env::set_var("P4PORT", "perforce:1666");
        assert!(detect_p4(std::path::Path::new(".")));
        std::env::remove_var("P4PORT");
    }

    #[tokio::test]
    async fn test_init_creates_manifest_and_modifies_uproject() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");

        run_init_with_commit(dir.path(), false).await.unwrap();

        assert!(dir.path().join("Config/UEPM.ini").exists());
        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
        assert!(!m.commit_plugins);

        let content = std::fs::read_to_string(dir.path().join("Test.uproject")).unwrap();
        assert!(content.contains("UEPMPlugins"));

        assert!(dir.path().join("UEPMPlugins").is_dir());
    }

    #[tokio::test]
    async fn test_init_writes_gitignore_when_not_committing() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");
        // Simulate a git repo
        std::fs::create_dir(dir.path().join(".git")).unwrap();

        run_init_with_commit(dir.path(), false).await.unwrap();

        let gitignore = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("UEPMPlugins/"));
    }

    #[tokio::test]
    async fn test_init_no_gitignore_when_committing() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");
        std::fs::create_dir(dir.path().join(".git")).unwrap();

        run_init_with_commit(dir.path(), true).await.unwrap();

        assert!(!dir.path().join(".gitignore").exists());
    }

    #[tokio::test]
    async fn test_init_preserves_existing_plugins() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[plugins]\n@acme/existing = ^1.0.0\n",
        )
        .unwrap();

        run_init_with_commit(dir.path(), false).await.unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@acme/existing"), "init wiped existing plugins");
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
    }

    #[tokio::test]
    async fn test_init_skips_engine_version_for_guid() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");

        run_init_with_commit(dir.path(), false).await.unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.engine_version.is_none());
    }
}
