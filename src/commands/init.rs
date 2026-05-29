use crate::context::{OutputMode, UEPMContext};
use crate::errors::UepmError;

#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InitOutput {
    Project {
        project_dir: String,
        engine_version: Option<String>,
        commit_plugins: bool,
    },
    Plugin {
        name: String,
        version: String,
        engine_range: String,
        vcs: Option<String>,
    },
}
use crate::manifest::{
    create_manifest, manifest_exists, read_manifest, write_manifest,
    PackageMetadata, write_package_metadata,
};
use crate::ue_install::find_installed_engines;
use crate::uproject::{add_plugin_directory, find_uproject, get_engine_association, is_guid};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::io::Write;
use std::path::{Path, PathBuf};

pub async fn run(ctx: &UEPMContext, yes: bool) -> Result<(), UepmError> {
    if let Some(uplugin_path) = find_uplugin(&ctx.project_dir) {
        run_plugin_init(&ctx.project_dir, &uplugin_path, yes).await?;
        if ctx.output_mode == OutputMode::Json {
            let m = crate::manifest::read_manifest(&ctx.project_dir)?;
            if let Some(pkg) = m.package {
                crate::output::emit_json(&InitOutput::Plugin {
                    name: pkg.name,
                    version: pkg.version,
                    engine_range: pkg.engine_range,
                    vcs: None,
                });
            }
        }
    } else {
        let commit = select_commit_plugins(&ctx.project_dir, yes)?;
        run_init_with_commit(&ctx.project_dir, commit).await?;
        if ctx.output_mode == OutputMode::Json {
            let m = crate::manifest::read_manifest(&ctx.project_dir)?;
            crate::output::emit_json(&InitOutput::Project {
                project_dir: ctx.project_dir.display().to_string(),
                engine_version: m.engine_version.clone(),
                commit_plugins: m.commit_plugins,
            });
        }
    }
    Ok(())
}

// ── Plugin-context init ───────────────────────────────────────────────────────

/// Scan `dir` for a `*.uplugin` file. Returns the first match (sorted), or `None`.
pub fn find_uplugin(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "uplugin").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Derive a URL-safe slug from a display name.
/// `"My Cool Plugin"` → `"my-cool-plugin"`
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Build a default engine semver range from a version string like `"5.7.4"`.
/// Produces `">=5.7.0, <6.0.0"`.
fn engine_range_from_version(v: &str) -> String {
    let parts: Vec<&str> = v.splitn(3, '.').collect();
    let major: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(5);
    let minor = parts.get(1).copied().unwrap_or("0");
    format!(">={}  <{}.0.0", format_args!("{major}.{minor}.0"), major + 1)
        .replacen("  ", ", ", 1)
}

/// Read fields from a `.uplugin` JSON file. Missing keys return empty strings.
fn read_uplugin_fields(path: &Path) -> (String, String, String, String) {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    let friendly_name = json.get("FriendlyName").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let version     = json.get("VersionName").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();
    let description = json.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let author      = json.get("CreatedBy").and_then(|v| v.as_str()).unwrap_or("").to_string();
    (friendly_name, version, description, author)
}

/// Prompt for a single string field, showing a default. With `--yes` uses default directly.
fn prompt_field(label: &str, default: &str, yes: bool) -> Result<String, UepmError> {
    if yes {
        return Ok(default.to_string());
    }
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .default(default.to_string())
        .interact_text()
        .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))
}

/// Detect or prompt for the engine compatibility range.
fn resolve_engine_range(yes: bool) -> Result<String, UepmError> {
    let engines = find_installed_engines();

    if engines.is_empty() {
        return prompt_field("Engine compatibility range", ">=5.3.0, <6.0.0", yes);
    }

    if engines.len() == 1 {
        let default = engine_range_from_version(&engines[0].version);
        return prompt_field("Engine compatibility range", &default, yes);
    }

    // Multiple installs — let the user pick one to base the range on.
    if yes {
        // Use the newest (last after ascending sort).
        let default = engine_range_from_version(&engines.last().unwrap().version);
        return Ok(default);
    }

    let labels: Vec<String> = engines
        .iter()
        .map(|e| format!("{} ({})", e.version, e.path.display()))
        .collect();
    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Base engine compatibility range on")
        .items(&labels)
        .default(labels.len() - 1)
        .interact()
        .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))?;
    Ok(engine_range_from_version(&engines[idx].version))
}

pub async fn run_plugin_init(
    plugin_dir: &Path,
    uplugin_path: &Path,
    yes: bool,
) -> Result<(), UepmError> {
    // Check interactive terminal early (unless --yes)
    if !yes && !dialoguer::console::Term::stdout().is_term() {
        return Err(UepmError::InteractiveRequired);
    }

    // Warn if [Plugin] already exists
    if manifest_exists(plugin_dir) {
        if let Ok(m) = read_manifest(plugin_dir) {
            if m.package.is_some() && !yes {
                let overwrite = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("[Plugin] section already exists. Overwrite?")
                    .default(false)
                    .interact()
                    .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))?;
                if !overwrite {
                    crate::output::print_info("Aborted — [Plugin] section unchanged");
                    return Ok(());
                }
            }
        }
    }

    // Derive defaults from .uplugin
    let uplugin_stem = uplugin_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let (friendly_name, plugin_version, description, author) = read_uplugin_fields(uplugin_path);

    let name_slug = if !friendly_name.is_empty() {
        slugify(&friendly_name)
    } else {
        slugify(&uplugin_stem)
    };
    let scope_slug = if !author.is_empty() { slugify(&author) } else { "scope".to_string() };
    let default_name = format!("@{}/{}", scope_slug, name_slug);
    let default_main = format!("{uplugin_stem}.uplugin");

    // Prompt each field
    let name        = prompt_field("Package name (@scope/name)", &default_name, yes)?;
    let version     = prompt_field("Version", &plugin_version, yes)?;
    let description = prompt_field("Description", &description, yes)?;
    let author      = prompt_field("Author", &author, yes)?;
    let license     = prompt_field("License", "MIT", yes)?;
    let engine_range = resolve_engine_range(yes)?;
    let main        = prompt_field("Main (.uplugin file)", &default_main, yes)?;

    // Validate required fields
    for (field, val) in &[("name", &name), ("version", &version), ("main", &main)] {
        if val.trim().is_empty() {
            return Err(UepmError::InvalidPackageField {
                field: field.to_string(),
                message: "must not be empty".to_string(),
            });
        }
    }

    let meta = PackageMetadata {
        name,
        version,
        description,
        author,
        license,
        engine_range,
        main,
    };

    // Ensure Config/UEPM.ini exists (may be a fresh plugin tree)
    if !manifest_exists(plugin_dir) {
        create_manifest(plugin_dir, None, false)?;
    }
    write_package_metadata(plugin_dir, &meta)?;

    crate::output::print_success("Plugin initialized for UEPM publishing");
    crate::output::print_info("Run 'uepm publish' to publish to the registry");

    Ok(())
}

/// Walk up from `start` (inclusive) and return `true` as soon as `predicate` matches a directory.
fn find_up(start: &Path, predicate: impl Fn(&Path) -> bool) -> bool {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if predicate(d) {
            return true;
        }
        dir = d.parent();
    }
    false
}

fn detect_p4(project_dir: &Path) -> bool {
    std::env::var("P4PORT").is_ok()
        || std::env::var("P4CONFIG").is_ok()
        || find_up(project_dir, |d| d.join(".p4config").exists())
}

fn detect_git(project_dir: &Path) -> bool {
    find_up(project_dir, |d| d.join(".git").is_dir())
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
        .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))
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
        let mut manifest = read_manifest(project_dir)?;
        if let Some(ver) = engine_version {
            manifest.engine_version = Some(ver.to_string());
        }
        manifest.commit_plugins = commit_plugins;
        write_manifest(project_dir, &manifest)?;
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
            "[Dependencies]\n\"@acme/existing\" = \"^1.0.0\"\n",
        )
        .unwrap();

        run_init_with_commit(dir.path(), false).await.unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@acme/existing"), "init wiped existing plugins");
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
    }

    // ── plugin-context tests ────────────────────────────────────────────

    fn write_uplugin(dir: &std::path::Path, name: &str, fields: serde_json::Value) {
        let path = dir.join(format!("{name}.uplugin"));
        std::fs::write(path, serde_json::to_string_pretty(&fields).unwrap()).unwrap();
    }

    #[test]
    fn test_find_uplugin_finds_file() {
        let dir = tempdir().unwrap();
        write_uplugin(dir.path(), "MyPlugin", json!({}));
        let found = find_uplugin(dir.path());
        assert!(found.is_some());
        assert!(found.unwrap().to_str().unwrap().ends_with("MyPlugin.uplugin"));
    }

    #[test]
    fn test_find_uplugin_returns_none_when_absent() {
        let dir = tempdir().unwrap();
        assert!(find_uplugin(dir.path()).is_none());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Cool Plugin"), "my-cool-plugin");
        assert_eq!(slugify("ACME Studio"), "acme-studio");
        assert_eq!(slugify("already-slugged"), "already-slugged");
        assert_eq!(slugify("  Spaces  "), "spaces");
    }

    #[test]
    fn test_engine_range_from_version() {
        assert_eq!(engine_range_from_version("5.7.4"), ">=5.7.0, <6.0.0");
        assert_eq!(engine_range_from_version("5.3.0"), ">=5.3.0, <6.0.0");
        assert_eq!(engine_range_from_version("4.27.2"), ">=4.27.0, <5.0.0");
    }

    #[tokio::test]
    async fn test_plugin_init_writes_package_section() {
        let dir = tempdir().unwrap();
        write_uplugin(
            dir.path(),
            "MyPlugin",
            json!({
                "FriendlyName": "My Plugin",
                "VersionName": "2.0.0",
                "Description": "Does stuff",
                "CreatedBy": "ACME Studio",
            }),
        );
        let uplugin = dir.path().join("MyPlugin.uplugin");
        run_plugin_init(dir.path(), &uplugin, true).await.unwrap();

        let meta = crate::manifest::read_package_metadata(dir.path()).unwrap();
        assert_eq!(meta.name, "@acme-studio/my-plugin");
        assert_eq!(meta.version, "2.0.0");
        assert_eq!(meta.description, "Does stuff");
        assert_eq!(meta.author, "ACME Studio");
        assert_eq!(meta.license, "MIT");
        assert_eq!(meta.main, "MyPlugin.uplugin");
    }

    #[tokio::test]
    async fn test_plugin_init_creates_manifest_if_absent() {
        let dir = tempdir().unwrap();
        write_uplugin(dir.path(), "FreshPlugin", json!({
            "FriendlyName": "Fresh Plugin",
            "VersionName": "1.0.0",
            "CreatedBy": "Dev",
        }));
        let uplugin = dir.path().join("FreshPlugin.uplugin");
        run_plugin_init(dir.path(), &uplugin, true).await.unwrap();
        assert!(dir.path().join("Config/UEPM.ini").exists());
    }

    #[tokio::test]
    async fn test_plugin_init_preserves_existing_plugins() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Dependencies]\n\"@acme/dep\" = \"^1.0.0\"\n",
        )
        .unwrap();
        write_uplugin(dir.path(), "MyPlugin", json!({
            "FriendlyName": "My Plugin",
            "VersionName": "1.0.0",
            "CreatedBy": "Dev",
        }));
        let uplugin = dir.path().join("MyPlugin.uplugin");
        run_plugin_init(dir.path(), &uplugin, true).await.unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@acme/dep"), "plugins section was wiped");
        assert!(m.package.is_some());
    }

    #[tokio::test]
    async fn test_init_routes_to_plugin_context_when_uplugin_present() {
        let dir = tempdir().unwrap();
        // No .uproject, but has .uplugin — should succeed via plugin path
        write_uplugin(dir.path(), "MyPlugin", json!({
            "FriendlyName": "My Plugin",
            "VersionName": "1.0.0",
            "CreatedBy": "Dev",
        }));
        let ctx = UEPMContext::with_dir(dir.path().to_path_buf());
        run(&ctx, true).await.unwrap();
        assert!(crate::manifest::read_package_metadata(dir.path()).is_ok());
    }

    #[tokio::test]
    async fn test_init_updates_commit_plugins_for_guid_engine() {
        // Regression: commit_plugins was silently dropped when engine is a GUID
        // and the manifest already exists.
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");
        // Pre-create manifest with commit_plugins=false
        crate::manifest::create_manifest(dir.path(), None, false).unwrap();

        run_init_with_commit(dir.path(), true).await.unwrap();

        let m = crate::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.commit_plugins, "commit_plugins should be updated even for GUID engines");
        assert!(m.engine_version.is_none(), "engine_version should remain None for GUIDs");
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
