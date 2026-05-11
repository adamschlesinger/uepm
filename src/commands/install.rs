use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile};
use crate::manifest::{add_plugin, read_manifest};
use crate::registry::RegistryClient;
use crate::resolver::resolve_and_install;
use std::collections::HashMap;
use std::path::Path;

pub async fn run(packages: Vec<String>) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    run_install(&packages, &project_dir).await
}

pub async fn run_install(packages: &[String], project_dir: &Path) -> Result<(), UepmError> {
    let uepm_plugins_dir = project_dir.join("UEPMPlugins");
    if !uepm_plugins_dir.exists() {
        std::fs::create_dir_all(&uepm_plugins_dir)?;
    }

    let client = RegistryClient::from_env();
    let token = std::env::var("UEPM_TOKEN").ok();
    let mut lock = read_lockfile(project_dir)?.unwrap_or_default();
    let mut resolved: HashMap<String, String> = HashMap::new();

    if packages.is_empty() {
        let manifest = read_manifest(project_dir)?;
        for (package, range) in &manifest.plugins {
            resolve_and_install(
                package,
                range,
                project_dir,
                &uepm_plugins_dir,
                &mut lock,
                &mut resolved,
                &client,
                token.as_deref(),
            )
            .await?;
        }
    } else {
        for pkg_spec in packages {
            let (package, range) = parse_package_spec(pkg_spec);
            let range = range.unwrap_or("*");

            let meta = client.fetch_metadata_for_version(&package, range).await?;
            let pinned_range = format!("^{}", meta.version);

            resolve_and_install(
                &package,
                &pinned_range,
                project_dir,
                &uepm_plugins_dir,
                &mut lock,
                &mut resolved,
                &client,
                token.as_deref(),
            )
            .await?;

            add_plugin(project_dir, &package, &pinned_range)?;
        }
    }

    write_lockfile(project_dir, &lock)?;
    crate::output::print_success(&format!("Installed {} plugin(s)", resolved.len()));
    Ok(())
}

fn parse_package_spec(spec: &str) -> (String, Option<&str>) {
    if let Some(pos) = spec.rfind('@') {
        if pos > 0 {
            return (spec[..pos].to_string(), Some(&spec[pos + 1..]));
        }
    }
    (spec.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scoped_with_version() {
        let (pkg, ver) = parse_package_spec("@scope/plugin@1.2.0");
        assert_eq!(pkg, "@scope/plugin");
        assert_eq!(ver, Some("1.2.0"));
    }

    #[test]
    fn test_parse_scoped_no_version() {
        let (pkg, ver) = parse_package_spec("@scope/plugin");
        assert_eq!(pkg, "@scope/plugin");
        assert_eq!(ver, None);
    }

    #[test]
    fn test_parse_unscoped() {
        let (pkg, ver) = parse_package_spec("my-plugin@2.0.0");
        assert_eq!(pkg, "my-plugin");
        assert_eq!(ver, Some("2.0.0"));
    }
}
