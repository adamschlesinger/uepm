use crate::context::UEPMContext;
use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile};
use crate::manifest::remove_plugin;
use crate::resolver::plugin_dir_name;

/// Entry point for `uepm uninstall`. Delegates to [`run_uninstall`] using the provided context.
pub async fn run(ctx: &UEPMContext, package: String) -> Result<(), UepmError> {
    run_uninstall(ctx, &package).await
}

/// Remove `package` from `UEPMPlugins/` and from `Config/UEPM.ini`.
pub async fn run_uninstall(ctx: &UEPMContext, package: &str) -> Result<(), UepmError> {
    let dir_name = plugin_dir_name(package);
    let plugin_dir = ctx.uepm_plugins_dir.join(dir_name);

    if plugin_dir.exists() || plugin_dir.symlink_metadata().is_ok() {
        if plugin_dir.is_symlink() {
            std::fs::remove_file(&plugin_dir)?;
        } else {
            std::fs::remove_dir_all(&plugin_dir)?;
        }
        crate::output::print_success(&format!("Removed {dir_name}"));
    } else {
        crate::output::print_warn(&format!("{dir_name} was not found in UEPMPlugins/"));
    }

    remove_plugin(&ctx.project_dir, package)?;

    // Remove from lockfile if present
    if let Some(mut lock) = read_lockfile(&ctx.project_dir)? {
        lock.plugins.remove(package);
        write_lockfile(&ctx.project_dir, &lock)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{LockedPlugin, LockFile, write_lockfile};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_uninstall_removes_directory_and_updates_manifest() {
        let dir = tempdir().unwrap();
        let ctx = UEPMContext::with_dir(dir.path().to_path_buf());
        std::fs::create_dir(&ctx.uepm_plugins_dir).unwrap();
        std::fs::create_dir(ctx.uepm_plugins_dir.join("cool-plugin")).unwrap();

        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
        )
        .unwrap();

        run_uninstall(&ctx, "@acme/cool-plugin").await.unwrap();

        assert!(!ctx.uepm_plugins_dir.join("cool-plugin").exists());
        let m = crate::manifest::read_manifest(&ctx.project_dir).unwrap();
        assert!(!m.plugins.contains_key("@acme/cool-plugin"));
    }

    #[tokio::test]
    async fn test_uninstall_removes_package_from_lockfile() {
        let dir = tempdir().unwrap();
        let ctx = UEPMContext::with_dir(dir.path().to_path_buf());
        std::fs::create_dir(&ctx.uepm_plugins_dir).unwrap();
        std::fs::create_dir(ctx.uepm_plugins_dir.join("cool-plugin")).unwrap();

        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
        )
        .unwrap();

        // Pre-populate uepm.lock
        let mut lock = LockFile::default();
        lock.plugins.insert(
            "@acme/cool-plugin".to_string(),
            LockedPlugin {
                resolved: "1.0.3".to_string(),
                tarball: "https://example.com/cool-plugin-1.0.3.tgz".to_string(),
                sha512: "sha512-abc".to_string(),
                dependencies: HashMap::new(),
            },
        );
        write_lockfile(dir.path(), &lock).unwrap();

        run_uninstall(&ctx, "@acme/cool-plugin").await.unwrap();

        let updated_lock = read_lockfile(dir.path()).unwrap().unwrap();
        assert!(
            !updated_lock.plugins.contains_key("@acme/cool-plugin"),
            "uninstalled plugin must be removed from uepm.lock"
        );
    }
}
