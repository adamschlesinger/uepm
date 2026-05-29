use crate::context::UEPMContext;
use crate::errors::UepmError;
use crate::installer::{download_and_extract, symlink_local};
use crate::lockfile::{LockFile, LockedPlugin};
use crate::manifest::read_manifest;
use crate::registry::RegistryClient;
use std::collections::HashMap;
use std::path::Path;

/// Mutable session state shared across every recursive `resolve_and_install` call.
/// Borrows the immutable fields from [`UEPMContext`] and owns the per-session maps.
pub struct ResolveContext<'a> {
    pub project_dir: &'a Path,
    pub uepm_plugins_dir: &'a Path,
    pub lock: &'a mut LockFile,
    pub resolved: &'a mut HashMap<String, String>,
    pub client: &'a RegistryClient,
    pub token: Option<&'a str>,
    /// False when `--json` is active — suppresses `print_info` calls that would contaminate stdout.
    pub verbose: bool,
}

impl<'a> ResolveContext<'a> {
    /// Borrow immutable fields from `ctx`; caller supplies the per-session mutable state.
    pub fn new(
        ctx: &'a UEPMContext,
        lock: &'a mut LockFile,
        resolved: &'a mut HashMap<String, String>,
    ) -> Self {
        use crate::context::OutputMode;
        ResolveContext {
            project_dir: &ctx.project_dir,
            uepm_plugins_dir: &ctx.uepm_plugins_dir,
            lock,
            resolved,
            client: &ctx.registry,
            token: ctx.token.as_deref(),
            verbose: ctx.output_mode == OutputMode::Human,
        }
    }
}

/// Derive the `UEPMPlugins/` subdirectory name from a package identifier.
/// `"@acme/cool-plugin"` → `"cool-plugin"`, `"my-plugin"` → `"my-plugin"`.
pub fn plugin_dir_name(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

/// Verify that `resolved_version` satisfies `required_range`.
/// Returns a [`UepmError::VersionConflict`] if not.
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

/// Resolve and install `package` at `range`, then recursively resolve any transitive
/// UEPM dependencies declared in the installed plugin's own `Config/UEPM.ini`.
/// `ctx.resolved` tracks already-installed packages within this session to detect conflicts.
pub async fn resolve_and_install(
    package: &str,
    range: &str,
    ctx: &mut ResolveContext<'_>,
) -> Result<(), UepmError> {
    if let Some(existing) = ctx.resolved.get(package) {
        if !range.starts_with("file:") {
            check_conflict(package, existing, range)?;
        }
        return Ok(());
    }

    let (version, tarball, sha512) = if let Some(rel_path) = range.strip_prefix("file:") {
        let local_path = ctx.project_dir.join(rel_path);
        if ctx.verbose {
            crate::output::print_info(&format!("Installing {package} from {rel_path}"));
        }

        let version = symlink_local(&local_path, package, ctx.uepm_plugins_dir)?;
        (version, range.to_string(), String::new())
    } else {
        let meta = if let Some(locked) = ctx.lock.plugins.get(package) {
            tracing::debug!("Using locked version {} for {}", locked.resolved, package);
            crate::registry::PackageMetadata {
                version: locked.resolved.clone(),
                tarball: locked.tarball.clone(),
                integrity: locked.sha512.clone(),
            }
        } else {
            ctx.client.fetch_metadata_for_version(package, range).await?
        };

        if ctx.verbose {
            crate::output::print_info(&format!("Installing {}@{}", package, meta.version));
        }
        download_and_extract(&meta.tarball, &meta.integrity, package, ctx.uepm_plugins_dir, ctx.token).await?;
        (meta.version, meta.tarball, meta.integrity)
    };

    ctx.resolved.insert(package.to_string(), version.clone());
    ctx.lock.plugins.insert(
        package.to_string(),
        LockedPlugin {
            resolved: version,
            tarball,
            sha512,
            dependencies: HashMap::new(),
        },
    );

    let plugin_dir = ctx.uepm_plugins_dir.join(plugin_dir_name(package));
    resolve_transitive_deps(package, &plugin_dir, ctx).await
}

async fn resolve_transitive_deps(
    package: &str,
    plugin_dir: &Path,
    ctx: &mut ResolveContext<'_>,
) -> Result<(), UepmError> {
    if let Ok(plugin_manifest) = read_manifest(plugin_dir) {
        for (dep_pkg, dep_range) in &plugin_manifest.plugins {
            Box::pin(resolve_and_install(dep_pkg, dep_range, ctx)).await?;
            ctx.lock
                .plugins
                .get_mut(package)
                .expect("package entry was just inserted")
                .dependencies
                .insert(dep_pkg.clone(), dep_range.clone());
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

        let plugin_src = tempdir().unwrap();
        std::fs::write(
            plugin_src.path().join("LocalPlugin.uplugin"),
            r#"{"FileVersion": 3, "VersionName": "2.0.0"}"#,
        )
        .unwrap();

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

        let mut ctx = ResolveContext {
            project_dir: project.path(),
            uepm_plugins_dir: &uepm_dir,
            lock: &mut lock,
            resolved: &mut resolved,
            client: &client,
            token: None,
            verbose: true,
        };

        resolve_and_install("@acme/local-plugin", &format!("file:{plugin_rel}"), &mut ctx)
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
