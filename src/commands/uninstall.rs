use crate::errors::UepmError;
use crate::manifest::remove_plugin;
use crate::resolver::plugin_dir_name;
use std::path::Path;

pub async fn run(package: String) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    run_uninstall(&package, &project_dir).await
}

pub async fn run_uninstall(package: &str, project_dir: &Path) -> Result<(), UepmError> {
    let uepm_dir = project_dir.join("UEPMPlugins");
    let plugin_dir = uepm_dir.join(plugin_dir_name(package));

    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir)?;
        crate::output::print_success(&format!("Removed {}", plugin_dir_name(package)));
    } else {
        crate::output::print_warn(&format!(
            "{} was not found in UEPMPlugins/",
            plugin_dir_name(package)
        ));
    }

    remove_plugin(project_dir, package)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_uninstall_removes_directory_and_updates_manifest() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();
        std::fs::create_dir(uepm_dir.join("cool-plugin")).unwrap();

        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[plugins]\n@acme/cool-plugin = ^1.0.0\n",
        )
        .unwrap();

        run_uninstall("@acme/cool-plugin", dir.path())
            .await
            .unwrap();

        assert!(!uepm_dir.join("cool-plugin").exists());
        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(!m.plugins.contains_key("@acme/cool-plugin"));
    }
}
