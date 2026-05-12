use crate::errors::UepmError;
use crate::installer::{download_and_extract, symlink_local};
use crate::lockfile::{LockFile, LockedPlugin};
use crate::manifest::read_manifest;
use crate::registry::RegistryClient;
use std::collections::HashMap;
use std::path::Path;

pub fn plugin_dir_name(package: &str) -> &str {
    package.split('/').last().unwrap_or(package)
}

pub fn check_conflict(
    package: &str,
    resolved_version: &str,
    required_range: &str,
) -> Result<(), UepmError> {
    let req =
        semver::VersionReq::parse(required_range).map_err(|e| UepmError::InvalidSemver {
            range: required_range.to_string(),
            message: e.to_string(),
        })?;
    let ver =
        semver::Version::parse(resolved_version).map_err(|e| UepmError::InvalidSemver {
            range: resolved_version.to_string(),
            message: e.to_string(),
        })?;

    if !req.matches(&ver) {
        return Err(UepmError::VersionConflict {
            package: package.to_string(),
            message: format!(
                "installed version {resolved_version} does not satisfy required range {required_range}"
            ),
        });
    }
    Ok(())
}

pub async fn resolve_and_install(
    package: &str,
    range: &str,
    project_dir: &Path,
    uepm_plugins_dir: &Path,
    lock: &mut LockFile,
    resolved: &mut HashMap<String, String>,
    client: &RegistryClient,
    token: Option<&str>,
) -> Result<(), UepmError> {
    if let Some(existing) = resolved.get(package) {
        // file: deps never semver-conflict; they're always the local copy
        if !range.starts_with("file:") {
            check_conflict(package, existing, range)?;
        }
        return Ok(());
    }

    // file: local path — bypass registry and lockfile entirely
    if let Some(rel_path) = range.strip_prefix("file:") {
        let local_path = project_dir.join(rel_path);
        crate::output::print_info(&format!("Installing {} from {}", package, rel_path));

        let version = symlink_local(&local_path, package, uepm_plugins_dir)?;
        resolved.insert(package.to_string(), version.clone());
        lock.plugins.insert(
            package.to_string(),
            LockedPlugin {
                resolved: version,
                tarball: range.to_string(),
                sha512: String::new(),
                dependencies: HashMap::new(),
            },
        );

        // Resolve transitive deps from the installed plugin's uepm.ini
        let plugin_dir = uepm_plugins_dir.join(plugin_dir_name(package));
        if let Ok(plugin_manifest) = read_manifest(&plugin_dir) {
            for (dep_pkg, dep_range) in &plugin_manifest.plugins {
                Box::pin(resolve_and_install(
                    dep_pkg,
                    dep_range,
                    project_dir,
                    uepm_plugins_dir,
                    lock,
                    resolved,
                    client,
                    token,
                ))
                .await?;
                lock.plugins
                    .get_mut(package)
                    .unwrap()
                    .dependencies
                    .insert(dep_pkg.clone(), dep_range.clone());
            }
        }

        return Ok(());
    }

    let meta = if let Some(locked) = lock.plugins.get(package) {
        tracing::debug!("Using locked version {} for {}", locked.resolved, package);
        crate::registry::PackageMetadata {
            version: locked.resolved.clone(),
            tarball: locked.tarball.clone(),
            integrity: locked.sha512.clone(),
        }
    } else {
        client.fetch_metadata_for_version(package, range).await?
    };

    crate::output::print_info(&format!("Installing {}@{}", package, meta.version));

    download_and_extract(&meta.tarball, &meta.integrity, package, uepm_plugins_dir, token).await?;

    resolved.insert(package.to_string(), meta.version.clone());

    lock.plugins.insert(
        package.to_string(),
        LockedPlugin {
            resolved: meta.version.clone(),
            tarball: meta.tarball.clone(),
            sha512: meta.integrity.clone(),
            dependencies: HashMap::new(),
        },
    );

    let plugin_dir = uepm_plugins_dir.join(plugin_dir_name(package));
    if let Ok(plugin_manifest) = read_manifest(&plugin_dir) {
        for (dep_package, dep_range) in &plugin_manifest.plugins {
            Box::pin(resolve_and_install(
                dep_package,
                dep_range,
                project_dir,
                uepm_plugins_dir,
                lock,
                resolved,
                client,
                token,
            ))
            .await?;

            lock.plugins
                .get_mut(package)
                .unwrap()
                .dependencies
                .insert(dep_package.clone(), dep_range.clone());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflict_is_ok() {
        let result = check_conflict("@acme/base-plugin", "1.0.0", "^1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_conflict_detected() {
        let result = check_conflict("@acme/base-plugin", "1.0.0", "^2.0.0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("conflict") || err.contains("Conflict") || err.contains("1.0.0"));
    }

    #[tokio::test]
    async fn test_file_dep_copies_local_plugin() {
        use crate::lockfile::LockFile;
        use crate::registry::RegistryClient;
        use tempfile::tempdir;

        // Set up a fake local plugin
        let plugin_src = tempdir().unwrap();
        std::fs::write(
            plugin_src.path().join("LocalPlugin.uplugin"),
            r#"{"FileVersion": 3, "VersionName": "2.0.0"}"#,
        )
        .unwrap();

        // Project dir with the plugin adjacent to it
        let project = tempdir().unwrap();
        let plugin_rel = "LocalPlugin";
        let plugin_abs = project.path().join(plugin_rel);
        std::fs::create_dir(&plugin_abs).unwrap();
        std::fs::copy(
            plugin_src.path().join("LocalPlugin.uplugin"),
            plugin_abs.join("LocalPlugin.uplugin"),
        )
        .unwrap();

        let uepm_dir = project.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let client = RegistryClient::new("http://localhost:1", None);
        let mut lock = LockFile::default();
        let mut resolved = HashMap::new();

        resolve_and_install(
            "@acme/local-plugin",
            &format!("file:{plugin_rel}"),
            project.path(),
            &uepm_dir,
            &mut lock,
            &mut resolved,
            &client,
            None,
        )
        .await
        .unwrap();

        let dest = uepm_dir.join("local-plugin");
        assert!(dest.is_symlink());
        assert!(dest.join("LocalPlugin.uplugin").exists());
        assert_eq!(resolved["@acme/local-plugin"], "2.0.0");
        assert!(lock.plugins["@acme/local-plugin"].tarball.starts_with("file:"));
    }

    #[test]
    fn test_plugin_dir_name_scoped() {
        assert_eq!(plugin_dir_name("@acme/cool-plugin"), "cool-plugin");
    }

    #[test]
    fn test_plugin_dir_name_unscoped() {
        assert_eq!(plugin_dir_name("my-plugin"), "my-plugin");
    }
}
