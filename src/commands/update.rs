use crate::errors::UepmError;
use crate::lockfile::{write_lockfile, LockFile};
use crate::manifest::read_manifest;
use crate::registry::RegistryClient;
use crate::resolver::resolve_and_install;
use std::collections::HashMap;

pub async fn run(package: Option<String>) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let manifest = read_manifest(&project_dir)?;
    let uepm_dir = project_dir.join("UEPMPlugins");
    let client = RegistryClient::from_env();
    let token = std::env::var("UEPM_TOKEN").ok();

    let mut lock = LockFile::default();
    let mut resolved: HashMap<String, String> = HashMap::new();

    let to_update: Vec<(String, String)> = match package {
        Some(ref pkg) => manifest
            .plugins
            .get(pkg)
            .map(|range| vec![(pkg.clone(), range.clone())])
            .unwrap_or_default(),
        None => manifest.plugins.into_iter().collect(),
    };

    for (pkg, range) in to_update {
        resolve_and_install(
            &pkg,
            &range,
            &project_dir,
            &uepm_dir,
            &mut lock,
            &mut resolved,
            &client,
            token.as_deref(),
        )
        .await?;
    }

    write_lockfile(&project_dir, &lock)?;
    crate::output::print_success(&format!("Updated {} plugin(s)", resolved.len()));
    Ok(())
}
