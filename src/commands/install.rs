use crate::context::UEPMContext;
use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile};
use crate::manifest::{add_plugin, read_manifest};
use crate::resolver::{resolve_and_install, ResolveContext};
use std::collections::HashMap;

/// Entry point for `uepm install`. Delegates to [`run_install`] using the provided context.
pub async fn run(ctx: &UEPMContext, packages: Vec<String>) -> Result<(), UepmError> {
    run_install(ctx, &packages).await
}

/// Install `packages` into `ctx.project_dir`. If `packages` is empty, installs everything
/// listed in `Config/UEPM.ini`. New packages are pinned to `^<resolved>` and written
/// back to the manifest.
pub async fn run_install(ctx: &UEPMContext, packages: &[String]) -> Result<(), UepmError> {
    if !ctx.uepm_plugins_dir.exists() {
        std::fs::create_dir_all(&ctx.uepm_plugins_dir)?;
    }

    let mut lock = read_lockfile(&ctx.project_dir)?.unwrap_or_default();
    let mut resolved: HashMap<String, String> = HashMap::new();
    let mut rctx = ResolveContext::new(ctx, &mut lock, &mut resolved);

    if packages.is_empty() {
        let manifest = read_manifest(&ctx.project_dir)?;
        for (package, range) in &manifest.plugins {
            resolve_and_install(package, range, &mut rctx).await?;
        }
    } else {
        for pkg_spec in packages {
            let (package, range) = parse_package_spec(pkg_spec);
            let range = range.unwrap_or("*");

            let meta = rctx.client.fetch_metadata_for_version(&package, range).await?;
            let pinned_range = format!("^{}", meta.version);

            resolve_and_install(&package, &pinned_range, &mut rctx).await?;
            add_plugin(&ctx.project_dir, &package, &pinned_range)?;
        }
    }

    write_lockfile(&ctx.project_dir, &lock)?;
    crate::output::print_success(&format!("Installed {} plugin(s)", resolved.len()));
    Ok(())
}

fn parse_package_spec(spec: &str) -> (String, Option<&str>) {
    // Only split on the *last* `@` after position 0 so that a leading scope `@` is preserved.
    match spec.rfind('@').filter(|&pos| pos > 0) {
        Some(pos) => (spec[..pos].to_string(), Some(&spec[pos + 1..])),
        None => (spec.to_string(), None),
    }
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
