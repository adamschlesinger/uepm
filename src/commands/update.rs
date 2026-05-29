use crate::context::{OutputMode, UEPMContext};
use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile, LockFile};
use crate::manifest::read_manifest;
use crate::resolver::{resolve_and_install, ResolveContext};
use std::collections::HashMap;

#[derive(serde::Serialize)]
struct UpdateResult {
    name: String,
    from: Option<String>,
    to: String,
}

/// Re-resolve and reinstall plugins, ignoring the lockfile so fresh versions are fetched.
/// If `package` is `Some`, updates only that plugin; otherwise updates all.
pub async fn run(ctx: &UEPMContext, package: Option<String>) -> Result<(), UepmError> {
    let mut manifest = read_manifest(&ctx.project_dir)?;

    // Snapshot old lock before any modifications — needed for UpdateResult.from.
    let old_lock = read_lockfile(&ctx.project_dir)?.unwrap_or_default();

    // When updating a single package, seed the new lock with all existing
    // locked entries *except* the one being updated, so other packages
    // remain at their locked versions.
    let mut lock = if let Some(ref pkg) = package {
        let mut seeded = old_lock.clone();
        seeded.plugins.remove(pkg);
        seeded
    } else {
        LockFile::default()
    };

    let mut resolved: HashMap<String, String> = HashMap::new();
    let mut rctx = ResolveContext::new(ctx, &mut lock, &mut resolved);

    if let Some(pkg) = package {
        if let Some(range) = manifest.plugins.remove(&pkg) {
            resolve_and_install(&pkg, &range, &mut rctx).await?;
        }
    } else {
        for (pkg, range) in &manifest.plugins {
            resolve_and_install(pkg, range, &mut rctx).await?;
        }
    }

    write_lockfile(&ctx.project_dir, &lock)?;

    if ctx.output_mode == OutputMode::Json {
        let mut results: Vec<UpdateResult> = resolved
            .iter()
            .map(|(name, to)| UpdateResult {
                name: name.clone(),
                from: old_lock.plugins.get(name).map(|p| p.resolved.clone()),
                to: to.clone(),
            })
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        crate::output::emit_json(&results);
    } else {
        crate::output::print_success(&format!("Updated {} plugin(s)", resolved.len()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{LockedPlugin, write_lockfile};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn locked_plugin(version: &str) -> LockedPlugin {
        LockedPlugin {
            resolved: version.to_string(),
            tarball: format!("https://example.com/{version}.tgz"),
            sha512: "sha512-abc".to_string(),
            dependencies: HashMap::new(),
        }
    }

    /// When updating a single package that doesn't exist in the manifest (noop resolve),
    /// the other packages already in uepm.lock must not be wiped.
    #[tokio::test]
    async fn test_single_package_update_preserves_other_locked_packages() {
        let dir = tempdir().unwrap();
        let ctx = UEPMContext::with_dir(dir.path().to_path_buf());
        std::fs::create_dir(&ctx.uepm_plugins_dir).unwrap();
        std::fs::create_dir(dir.path().join("Config")).unwrap();

        // Manifest has only plugin-b; we will ask to "update" plugin-a (absent)
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Dependencies]\n\"@acme/plugin-b\" = \"^2.0.0\"\n",
        )
        .unwrap();

        // Pre-seed the lockfile with both packages already locked
        let mut initial_lock = LockFile::default();
        initial_lock.plugins.insert("@acme/plugin-a".to_string(), locked_plugin("1.0.0"));
        initial_lock.plugins.insert("@acme/plugin-b".to_string(), locked_plugin("2.0.0"));
        write_lockfile(dir.path(), &initial_lock).unwrap();

        // Update only plugin-a (absent from manifest → no network call, noop resolve)
        run(&ctx, Some("@acme/plugin-a".to_string())).await.unwrap();

        let lock = read_lockfile(dir.path()).unwrap().unwrap();
        assert!(
            lock.plugins.contains_key("@acme/plugin-b"),
            "plugin-b must survive a single-package update targeting plugin-a"
        );
    }
}
