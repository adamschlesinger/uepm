# UEPM Phase 1 — Rust Core CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Rust binary `uepm` implementing `init`, `install`, `uninstall`, `update`, and `list` commands that manage Unreal Engine plugins via the npm registry, with no runtime prerequisites.

**Architecture:** Single Rust crate at `uepm/` structured as a library + binary (`src/lib.rs` exposes all modules for testing; `src/main.rs` is the binary entry point). Plugins are downloaded from the npm registry, extracted to `UEPMPlugins/`, and tracked in `uepm.ini` (human-edited) and `uepm.lock` (machine-generated). All commands are async via tokio.

**Tech Stack:** Rust 2021 edition, clap 4 (derive), reqwest 0.12, tokio 1, serde/serde_json, configparser 3, tar + flate2, semver 1, tracing + tracing-subscriber, crossterm 0.27, dotenvy 0.15, thiserror 1, dialoguer 0.11, sha2 + base64 (checksum), mockito + tempfile (tests).

---

## File Map

```
uepm/
  Cargo.toml
  src/
    main.rs          — binary entry: clap, tokio::main, dotenvy, tracing init
    lib.rs           — pub mod declarations
    errors.rs        — UepmError enum (thiserror derives)
    output.rs        — crossterm print_success / print_warn / print_error
    manifest.rs      — ProjectManifest, read/write/create uepm.ini
    lockfile.rs      — LockFile, LockedPlugin, read/write uepm.lock
    uproject.rs      — find/read/write .uproject JSON
    registry.rs      — RegistryClient: fetch package metadata from npm registry
    installer.rs     — download tarball, verify sha512, extract to UEPMPlugins/
    resolver.rs      — recursive dep resolution with conflict detection
    commands/
      mod.rs
      init.rs        — uepm init (project context only in phase 1)
      install.rs     — uepm install
      uninstall.rs   — uepm uninstall
      update.rs      — uepm update
      list.rs        — uepm list
.github/
  workflows/
    release.yml      — cross-compile + GitHub Release on tag push
install.sh           — Unix install script (hosted on GitHub Pages)
install.ps1          — Windows install script (hosted on GitHub Pages)
```

---

### Task 1: Project scaffolding

**Files:**
- Create: `uepm/Cargo.toml`
- Create: `uepm/src/lib.rs`
- Create: `uepm/src/main.rs`
- Create: `uepm/src/commands/mod.rs`

- [ ] **Step 1: Create `uepm/Cargo.toml`**

```toml
[package]
name = "uepm"
version = "2.0.0"
edition = "2021"
description = "Unreal Engine Package Manager"

[[bin]]
name = "uepm"
path = "src/main.rs"

[lib]
name = "uepm"
path = "src/lib.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
configparser = "3"
tar = "0.4"
flate2 = "1"
semver = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
crossterm = "0.27"
dotenvy = "0.15"
thiserror = "1"
dialoguer = "0.11"
sha2 = "0.10"
base64 = "0.22"
bytes = "1"

[dev-dependencies]
mockito = "1"
tempfile = "3"
```

- [ ] **Step 2: Create `uepm/src/lib.rs`**

```rust
pub mod commands;
pub mod errors;
pub mod installer;
pub mod lockfile;
pub mod manifest;
pub mod output;
pub mod registry;
pub mod resolver;
pub mod uproject;
```

- [ ] **Step 3: Create `uepm/src/commands/mod.rs`**

```rust
pub mod init;
pub mod install;
pub mod list;
pub mod uninstall;
pub mod update;
```

- [ ] **Step 4: Create `uepm/src/main.rs`**

```rust
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "uepm", version, about = "Unreal Engine Package Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a project for UEPM
    Init {
        /// Accept detected defaults without prompting
        #[arg(short, long)]
        yes: bool,
    },
    /// Install one or all plugins
    Install {
        /// Packages to install, e.g. @scope/plugin or @scope/plugin@1.0.0
        /// If empty, installs all plugins in uepm.ini
        packages: Vec<String>,
    },
    /// Remove a plugin
    Uninstall { package: String },
    /// Update plugins to latest compatible versions
    Update {
        /// Specific package to update; updates all if omitted
        package: Option<String>,
    },
    /// List installed plugins and compatibility status
    List,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes } => uepm::commands::init::run(yes).await,
        Commands::Install { packages } => uepm::commands::install::run(packages).await,
        Commands::Uninstall { package } => uepm::commands::uninstall::run(package).await,
        Commands::Update { package } => uepm::commands::update::run(package).await,
        Commands::List => uepm::commands::list::run().await,
    };

    if let Err(e) = result {
        uepm::output::print_error(&format!("{e}"));
        std::process::exit(1);
    }
}
```

- [ ] **Step 5: Create placeholder files so the project compiles**

Create `uepm/src/errors.rs`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum UepmError {
    #[error("not yet implemented")]
    Todo,
}
```

Create `uepm/src/output.rs`, `manifest.rs`, `lockfile.rs`, `uproject.rs`, `registry.rs`, `installer.rs`, `resolver.rs` each with a single `// TODO` comment.

Create `uepm/src/commands/init.rs`, `install.rs`, `uninstall.rs`, `update.rs`, `list.rs` each with:
```rust
use crate::errors::UepmError;
pub async fn run(_: impl std::any::Any) -> Result<(), UepmError> { Ok(()) }
```

For `list.rs` and `install.rs` adjust signatures to `run()` and `run(Vec<String>)` respectively.

- [ ] **Step 6: Verify the project compiles**

```bash
cd uepm && cargo build
```

Expected: compiles with warnings but no errors.

- [ ] **Step 7: Commit**

```bash
git add uepm/
git commit -m "chore(rust): Scaffold uepm Rust crate with all dependencies"
```

---

### Task 2: Error types and output helpers

**Files:**
- Modify: `uepm/src/errors.rs`
- Modify: `uepm/src/output.rs`

- [ ] **Step 1: Write `uepm/src/errors.rs`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum UepmError {
    #[error("Registry error: {0}")]
    Registry(#[from] reqwest::Error),

    #[error("Package not found: {package}")]
    PackageNotFound { package: String },

    #[error("Checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Version conflict for {package}: {message}\nHint: pin a version in your uepm.ini")]
    VersionConflict { package: String, message: String },

    #[error("No .uproject file found in {directory}")]
    UprojectNotFound { directory: String },

    #[error("Failed to parse uepm.ini: {0}")]
    ManifestParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid semver range '{range}': {message}")]
    InvalidSemver { range: String, message: String },

    #[error("No version of {package} satisfies range {range}")]
    NoMatchingVersion { package: String, range: String },

    #[error("Interactive terminal required. Run with --yes to use detected defaults.")]
    InteractiveRequired,
}
```

- [ ] **Step 2: Write `uepm/src/output.rs`**

```rust
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{stdout, Write};

pub fn print_success(msg: &str) {
    let _ = execute!(
        stdout(),
        SetForegroundColor(Color::Green),
        Print("✓ "),
        ResetColor,
        Print(msg),
        Print("\n"),
    );
}

pub fn print_warn(msg: &str) {
    let _ = execute!(
        stdout(),
        SetForegroundColor(Color::Yellow),
        Print("⚠ "),
        ResetColor,
        Print(msg),
        Print("\n"),
    );
}

pub fn print_error(msg: &str) {
    let _ = execute!(
        stdout(),
        SetForegroundColor(Color::Red),
        Print("✗ "),
        ResetColor,
        Print(msg),
        Print("\n"),
    );
}

pub fn print_info(msg: &str) {
    let _ = execute!(
        stdout(),
        SetForegroundColor(Color::Cyan),
        Print("ℹ "),
        ResetColor,
        Print(msg),
        Print("\n"),
    );
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cd uepm && cargo build
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add uepm/src/errors.rs uepm/src/output.rs
git commit -m "feat(uepm): Add error types and crossterm output helpers"
```

---

### Task 3: Manifest (`uepm.ini`)

**Files:**
- Modify: `uepm/src/manifest.rs`

- [ ] **Step 1: Write failing tests**

At the bottom of `uepm/src/manifest.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn sample_ini() -> &'static str {
        "[plugins]\n@acme/cool-plugin = ^1.0.0\n@studio/other = ~2.1.0\n\n[settings]\nengine_version = 5.7\n"
    }

    #[test]
    fn test_parse_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("uepm.ini");
        fs::write(&path, sample_ini()).unwrap();
        let m = read_manifest(dir.path()).unwrap();
        assert_eq!(m.plugins.get("@acme/cool-plugin").map(|s| s.as_str()), Some("^1.0.0"));
        assert_eq!(m.plugins.get("@studio/other").map(|s| s.as_str()), Some("~2.1.0"));
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
    }

    #[test]
    fn test_write_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("uepm.ini");
        let mut m = ProjectManifest::default();
        m.plugins.insert("@foo/bar".to_string(), "^1.0.0".to_string());
        m.engine_version = Some("5.3".to_string());
        write_manifest(dir.path(), &m).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("@foo/bar"));
        assert!(content.contains("^1.0.0"));
        assert!(content.contains("engine_version"));
    }

    #[test]
    fn test_create_manifest() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), Some("5.4")).unwrap();
        let m = read_manifest(dir.path()).unwrap();
        assert!(m.plugins.is_empty());
        assert_eq!(m.engine_version.as_deref(), Some("5.4"));
    }

    #[test]
    fn test_missing_manifest_returns_error() {
        let dir = tempdir().unwrap();
        let result = read_manifest(dir.path());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test manifest
```

Expected: compilation errors (functions not defined).

- [ ] **Step 3: Write `uepm/src/manifest.rs`**

```rust
use crate::errors::UepmError;
use configparser::ini::Ini;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub plugins: HashMap<String, String>,  // package_name -> version_range
    pub engine_version: Option<String>,
}

pub fn read_manifest(project_dir: &Path) -> Result<ProjectManifest, UepmError> {
    let path = project_dir.join("uepm.ini");
    let mut config = Ini::new();
    config
        .load(path.to_str().unwrap())
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;

    let plugins = config
        .get_map_ref()
        .get("plugins")
        .map(|section| {
            section
                .iter()
                .filter_map(|(k, v)| v.as_ref().map(|v| (k.clone(), v.trim().to_string())))
                .collect()
        })
        .unwrap_or_default();

    let engine_version = config.get("settings", "engine_version");

    Ok(ProjectManifest {
        plugins,
        engine_version,
    })
}

pub fn write_manifest(project_dir: &Path, manifest: &ProjectManifest) -> Result<(), UepmError> {
    let path = project_dir.join("uepm.ini");
    let mut config = Ini::new();

    for (name, range) in &manifest.plugins {
        config.set("plugins", name, Some(range.clone()));
    }

    if let Some(ref ev) = manifest.engine_version {
        config.set("settings", "engine_version", Some(ev.clone()));
    }

    config
        .write(path.to_str().unwrap())
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;
    Ok(())
}

pub fn create_manifest(project_dir: &Path, engine_version: Option<&str>) -> Result<(), UepmError> {
    let manifest = ProjectManifest {
        plugins: HashMap::new(),
        engine_version: engine_version.map(|s| s.to_string()),
    };
    write_manifest(project_dir, &manifest)
}

pub fn add_plugin(
    project_dir: &Path,
    package: &str,
    range: &str,
) -> Result<(), UepmError> {
    let mut manifest = read_manifest(project_dir)?;
    manifest.plugins.insert(package.to_string(), range.to_string());
    write_manifest(project_dir, &manifest)
}

pub fn remove_plugin(project_dir: &Path, package: &str) -> Result<(), UepmError> {
    let mut manifest = read_manifest(project_dir)?;
    manifest.plugins.remove(package);
    write_manifest(project_dir, &manifest)
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test manifest
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add uepm/src/manifest.rs
git commit -m "feat(uepm): Add manifest module for uepm.ini read/write"
```

---

### Task 4: Lockfile (`uepm.lock`)

**Files:**
- Modify: `uepm/src/lockfile.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_lock() -> LockFile {
        let mut lock = LockFile::default();
        lock.plugins.insert(
            "@acme/cool-plugin".to_string(),
            LockedPlugin {
                resolved: "1.0.3".to_string(),
                tarball: "https://registry.npmjs.org/@acme/cool-plugin/-/cool-plugin-1.0.3.tgz"
                    .to_string(),
                sha512: "abc123".to_string(),
                dependencies: std::collections::HashMap::new(),
            },
        );
        lock
    }

    #[test]
    fn test_roundtrip() {
        let dir = tempdir().unwrap();
        let lock = sample_lock();
        write_lockfile(dir.path(), &lock).unwrap();
        let loaded = read_lockfile(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.version, 1);
        let plugin = loaded.plugins.get("@acme/cool-plugin").unwrap();
        assert_eq!(plugin.resolved, "1.0.3");
        assert_eq!(plugin.sha512, "abc123");
    }

    #[test]
    fn test_missing_lockfile_returns_none() {
        let dir = tempdir().unwrap();
        let result = read_lockfile(dir.path()).unwrap();
        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test lockfile
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/lockfile.rs`**

```rust
use crate::errors::UepmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,
    pub plugins: HashMap<String, LockedPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPlugin {
    pub resolved: String,
    pub tarball: String,
    pub sha512: String,
    pub dependencies: HashMap<String, String>,
}

impl Default for LockFile {
    fn default() -> Self {
        LockFile {
            version: 1,
            plugins: HashMap::new(),
        }
    }
}

pub fn read_lockfile(project_dir: &Path) -> Result<Option<LockFile>, UepmError> {
    let path = project_dir.join("uepm.lock");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let lock: LockFile = serde_json::from_str(&content)?;
    Ok(Some(lock))
}

pub fn write_lockfile(project_dir: &Path, lock: &LockFile) -> Result<(), UepmError> {
    let path = project_dir.join("uepm.lock");
    let content = serde_json::to_string_pretty(lock)?;
    std::fs::write(&path, content + "\n")?;
    Ok(())
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test lockfile
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add uepm/src/lockfile.rs
git commit -m "feat(uepm): Add lockfile module for uepm.lock read/write"
```

---

### Task 5: UProject module

**Files:**
- Modify: `uepm/src/uproject.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_uproject(dir: &std::path::Path, content: serde_json::Value) {
        let path = dir.join("TestProject.uproject");
        std::fs::write(path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
    }

    #[test]
    fn test_find_uproject() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), json!({ "EngineAssociation": "5.3" }));
        let path = find_uproject(dir.path()).unwrap();
        assert!(path.ends_with("TestProject.uproject"));
    }

    #[test]
    fn test_find_uproject_missing() {
        let dir = tempdir().unwrap();
        assert!(find_uproject(dir.path()).is_err());
    }

    #[test]
    fn test_add_plugin_directory() {
        let dir = tempdir().unwrap();
        write_uproject(
            dir.path(),
            json!({ "EngineAssociation": "5.3", "AdditionalPluginDirectories": [] }),
        );
        let path = find_uproject(dir.path()).unwrap();
        add_plugin_directory(&path, "UEPMPlugins").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("UEPMPlugins"));
    }

    #[test]
    fn test_add_plugin_directory_idempotent() {
        let dir = tempdir().unwrap();
        write_uproject(
            dir.path(),
            json!({ "EngineAssociation": "5.3", "AdditionalPluginDirectories": ["UEPMPlugins"] }),
        );
        let path = find_uproject(dir.path()).unwrap();
        add_plugin_directory(&path, "UEPMPlugins").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // Should only appear once
        assert_eq!(content.matches("UEPMPlugins").count(), 1);
    }

    #[test]
    fn test_is_guid() {
        assert!(is_guid("{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"));
        assert!(!is_guid("5.3"));
        assert!(!is_guid("5.7"));
    }

    #[test]
    fn test_get_engine_association() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), json!({ "EngineAssociation": "5.7" }));
        let path = find_uproject(dir.path()).unwrap();
        let assoc = get_engine_association(&path).unwrap();
        assert_eq!(assoc, "5.7");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test uproject
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/uproject.rs`**

```rust
use crate::errors::UepmError;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn find_uproject(dir: &Path) -> Result<PathBuf, UepmError> {
    let entries = std::fs::read_dir(dir)?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "uproject")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    found.sort();

    found.into_iter().next().ok_or_else(|| UepmError::UprojectNotFound {
        directory: dir.display().to_string(),
    })
}

pub fn get_engine_association(uproject_path: &Path) -> Result<String, UepmError> {
    let content = std::fs::read_to_string(uproject_path)?;
    let value: Value = serde_json::from_str(&content)?;
    value["EngineAssociation"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| UepmError::ManifestParse("Missing EngineAssociation".to_string()))
}

pub fn add_plugin_directory(uproject_path: &Path, dir_name: &str) -> Result<(), UepmError> {
    let content = std::fs::read_to_string(uproject_path)?;
    let mut value: Value = serde_json::from_str(&content)?;

    let dirs = value
        .as_object_mut()
        .and_then(|obj| {
            obj.entry("AdditionalPluginDirectories")
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
        })
        .ok_or_else(|| UepmError::ManifestParse("Invalid uproject structure".to_string()))?;

    if !dirs.iter().any(|d| d.as_str() == Some(dir_name)) {
        dirs.push(Value::String(dir_name.to_string()));
    }

    let output = serde_json::to_string_pretty(&value)?;
    std::fs::write(uproject_path, output + "\n")?;
    Ok(())
}

pub fn is_guid(s: &str) -> bool {
    s.starts_with('{') && s.ends_with('}')
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test uproject
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add uepm/src/uproject.rs
git commit -m "feat(uepm): Add uproject module for .uproject read/write"
```

---

### Task 6: Registry client

**Files:**
- Modify: `uepm/src/registry.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn sample_registry_response(pkg: &str, version: &str, tarball_url: &str) -> serde_json::Value {
        serde_json::json!({
            "name": pkg,
            "dist-tags": { "latest": version },
            "versions": {
                version: {
                    "name": pkg,
                    "version": version,
                    "dist": {
                        "tarball": tarball_url,
                        "integrity": "sha512-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn test_fetch_metadata_success() {
        let mut server = Server::new_async().await;
        let pkg = "@acme/cool-plugin";
        let encoded = "%40acme%2Fcool-plugin";
        let tarball_url = format!("{}/tarball.tgz", server.url());
        let body = sample_registry_response(pkg, "1.0.3", &tarball_url);

        let _mock = server
            .mock("GET", format!("/{encoded}").as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let client = RegistryClient::new(&server.url(), None);
        let meta = client.fetch_metadata(pkg).await.unwrap();
        assert_eq!(meta.version, "1.0.3");
        assert_eq!(meta.tarball, tarball_url);
    }

    #[tokio::test]
    async fn test_fetch_metadata_not_found() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/%40acme%2Fmissing")
            .with_status(404)
            .create_async()
            .await;

        let client = RegistryClient::new(&server.url(), None);
        let result = client.fetch_metadata("@acme/missing").await;
        assert!(matches!(result, Err(UepmError::PackageNotFound { .. })));
    }

    #[test]
    fn test_resolve_best_version() {
        let versions = vec!["1.0.0".to_string(), "1.0.3".to_string(), "2.0.0".to_string()];
        let best = resolve_best_version(&versions, "^1.0.0").unwrap();
        assert_eq!(best, "1.0.3");
    }

    #[test]
    fn test_resolve_no_match() {
        let versions = vec!["2.0.0".to_string()];
        let result = resolve_best_version(&versions, "^1.0.0");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test registry
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/registry.rs`**

```rust
use crate::errors::UepmError;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub version: String,
    pub tarball: String,
    pub integrity: String,
}

#[derive(Deserialize)]
struct NpmPackage {
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
    versions: HashMap<String, NpmVersion>,
}

#[derive(Deserialize)]
struct NpmVersion {
    version: String,
    dist: NpmDist,
}

#[derive(Deserialize)]
struct NpmDist {
    tarball: String,
    integrity: String,
}

pub struct RegistryClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl RegistryClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        RegistryClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("UEPM_REGISTRY")
            .unwrap_or_else(|_| "https://registry.npmjs.org".to_string());
        let token = std::env::var("UEPM_TOKEN").ok();
        Self::new(&base_url, token)
    }

    pub async fn fetch_metadata(&self, package: &str) -> Result<PackageMetadata, UepmError> {
        // URL-encode the package name (@scope/name → %40scope%2Fname)
        let encoded = urlencoding::encode(package).into_owned();
        let url = format!("{}/{}", self.base_url, encoded);

        tracing::debug!("Fetching metadata from {url}");

        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(UepmError::PackageNotFound {
                package: package.to_string(),
            });
        }
        resp.error_for_status_ref()?;

        let npm_pkg: NpmPackage = resp.json().await?;

        // Find the latest version
        let latest = npm_pkg
            .dist_tags
            .get("latest")
            .ok_or_else(|| UepmError::PackageNotFound {
                package: package.to_string(),
            })?;

        let version_info =
            npm_pkg
                .versions
                .get(latest)
                .ok_or_else(|| UepmError::PackageNotFound {
                    package: package.to_string(),
                })?;

        Ok(PackageMetadata {
            version: version_info.version.clone(),
            tarball: version_info.dist.tarball.clone(),
            integrity: version_info.dist.integrity.clone(),
        })
    }

    pub async fn fetch_metadata_for_version(
        &self,
        package: &str,
        range: &str,
    ) -> Result<PackageMetadata, UepmError> {
        let encoded = urlencoding::encode(package).into_owned();
        let url = format!("{}/{}", self.base_url, encoded);

        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(UepmError::PackageNotFound {
                package: package.to_string(),
            });
        }
        resp.error_for_status_ref()?;

        let npm_pkg: NpmPackage = resp.json().await?;
        let versions: Vec<String> = npm_pkg.versions.keys().cloned().collect();
        let best = resolve_best_version(&versions, range)?;

        let version_info = npm_pkg.versions.get(&best).unwrap();

        Ok(PackageMetadata {
            version: version_info.version.clone(),
            tarball: version_info.dist.tarball.clone(),
            integrity: version_info.dist.integrity.clone(),
        })
    }
}

pub fn resolve_best_version(
    versions: &[String],
    range: &str,
) -> Result<String, UepmError> {
    let req = semver::VersionReq::parse(range).map_err(|e| UepmError::InvalidSemver {
        range: range.to_string(),
        message: e.to_string(),
    })?;

    let mut matching: Vec<semver::Version> = versions
        .iter()
        .filter_map(|v| semver::Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    matching.sort();

    matching
        .into_iter()
        .last()
        .map(|v| v.to_string())
        .ok_or_else(|| UepmError::NoMatchingVersion {
            package: String::new(),
            range: range.to_string(),
        })
}
```

- [ ] **Step 4: Add `urlencoding` to `Cargo.toml`**

Add to `[dependencies]` in `uepm/Cargo.toml`:
```toml
urlencoding = "2"
```

- [ ] **Step 5: Run tests — verify they pass**

```bash
cd uepm && cargo test registry
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add uepm/src/registry.rs uepm/Cargo.toml
git commit -m "feat(uepm): Add registry client for npm registry API"
```

---

### Task 7: Installer (download + extract)

**Files:**
- Modify: `uepm/src/installer.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::io::Write;
    use tempfile::tempdir;

    fn make_fake_tarball() -> Vec<u8> {
        use flate2::{write::GzEncoder, Compression};
        use tar::Builder;

        let buf = Vec::new();
        let enc = GzEncoder::new(buf, Compression::default());
        let mut builder = Builder::new(enc);

        // Add a fake .uplugin file inside package/ (npm tarballs are in package/)
        let content = b"{\"FileVersion\": 3}";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "package/TestPlugin.uplugin", content.as_ref())
            .unwrap();

        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    fn sha512_b64(data: &[u8]) -> String {
        use base64::{engine::general_purpose, Engine};
        use sha2::{Digest, Sha512};
        let hash = Sha512::digest(data);
        general_purpose::STANDARD.encode(hash)
    }

    #[tokio::test]
    async fn test_extract_installs_to_uepm_plugins() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let tarball = make_fake_tarball();
        let integrity = format!("sha512-{}", sha512_b64(&tarball));

        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/tarball.tgz")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_body(tarball)
            .create_async()
            .await;

        let tarball_url = format!("{}/tarball.tgz", server.url());
        download_and_extract(
            &tarball_url,
            &integrity,
            "@test/test-plugin",
            &uepm_dir,
            None,
        )
        .await
        .unwrap();

        assert!(uepm_dir.join("test-plugin").join("TestPlugin.uplugin").exists());
    }

    #[tokio::test]
    async fn test_checksum_mismatch_aborts() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/tarball.tgz")
            .with_status(200)
            .with_body(b"bad data".as_ref())
            .create_async()
            .await;

        let result = download_and_extract(
            &format!("{}/tarball.tgz", server.url()),
            "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "@test/plugin",
            &uepm_dir,
            None,
        )
        .await;

        assert!(matches!(result, Err(UepmError::ChecksumMismatch { .. })));
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test installer
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/installer.rs`**

```rust
use crate::errors::UepmError;
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use sha2::{Digest, Sha512};
use std::path::Path;

/// Download a tarball, verify its sha512 integrity, and extract it to
/// `uepm_plugins_dir/<plugin_name>/`. npm tarballs contain a `package/`
/// prefix which is stripped during extraction.
pub async fn download_and_extract(
    tarball_url: &str,
    integrity: &str,
    package_name: &str,
    uepm_plugins_dir: &Path,
    token: Option<&str>,
) -> Result<(), UepmError> {
    tracing::debug!("Downloading {tarball_url}");

    let client = reqwest::Client::new();
    let mut req = client.get(tarball_url);
    if let Some(tok) = token {
        req = req.bearer_auth(tok);
    }
    let resp = req.send().await?.error_for_status()?;
    let data: Bytes = resp.bytes().await?;

    verify_integrity(&data, integrity, package_name)?;

    let plugin_dir_name = package_name
        .split('/')
        .last()
        .unwrap_or(package_name);
    let dest = uepm_plugins_dir.join(plugin_dir_name);

    // Remove existing directory to handle updates cleanly
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;

    extract_tarball(&data, &dest)?;

    Ok(())
}

fn verify_integrity(data: &[u8], integrity: &str, package: &str) -> Result<(), UepmError> {
    let b64 = integrity
        .strip_prefix("sha512-")
        .ok_or_else(|| UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: integrity.to_string(),
            actual: "(unparseable integrity string)".to_string(),
        })?;

    let expected = general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: integrity.to_string(),
            actual: "(base64 decode failed)".to_string(),
        })?;

    let actual = Sha512::digest(data);

    if actual.as_slice() != expected.as_slice() {
        return Err(UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: general_purpose::STANDARD.encode(&expected),
            actual: general_purpose::STANDARD.encode(actual),
        });
    }

    Ok(())
}

fn extract_tarball(data: &[u8], dest: &Path) -> Result<(), UepmError> {
    let cursor = std::io::Cursor::new(data);
    let decoder = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_path_buf();

        // Strip the leading `package/` prefix that npm tarballs include
        let stripped = entry_path
            .strip_prefix("package")
            .unwrap_or(&entry_path);

        if stripped.as_os_str().is_empty() {
            continue;
        }

        let target = dest.join(stripped);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&target)?;
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test installer
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add uepm/src/installer.rs
git commit -m "feat(uepm): Add installer for tarball download, verify, and extract"
```

---

### Task 8: Resolver

**Files:**
- Modify: `uepm/src/resolver.rs`

The resolver recursively installs deps by:
1. Installing the top-level plugin
2. Reading the installed plugin's `uepm.ini` (if present) for its own deps
3. Recursively installing those, detecting version conflicts

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflict_is_ok() {
        let mut installed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        // @acme/base-plugin 1.0.0 is already installed
        installed.insert("@acme/base-plugin".to_string(), "1.0.0".to_string());

        // Plugin A wants ^1.0.0 of base — compatible
        let result = check_conflict("@acme/base-plugin", "1.0.0", "^1.0.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_conflict_detected() {
        // Plugin A installed base@1.0.0; Plugin B wants ^2.0.0
        let result = check_conflict("@acme/base-plugin", "1.0.0", "^2.0.0");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("conflict") || err.contains("Conflict") || err.contains("1.0.0"));
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
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test resolver
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/resolver.rs`**

```rust
use crate::errors::UepmError;
use crate::installer::download_and_extract;
use crate::lockfile::{LockFile, LockedPlugin};
use crate::manifest::read_manifest;
use crate::registry::RegistryClient;
use std::collections::HashMap;
use std::path::Path;

/// Returns the directory name for a plugin in UEPMPlugins/
/// "@acme/cool-plugin" → "cool-plugin", "my-plugin" → "my-plugin"
pub fn plugin_dir_name(package: &str) -> &str {
    package.split('/').last().unwrap_or(package)
}

/// Check whether installing `resolved_version` of `package` conflicts with
/// an already-resolved version in `installed`.
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

/// Recursively install a plugin and its UEPM dependencies.
/// `resolved` tracks versions already resolved in this install session to detect conflicts.
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
    // If already resolved in this session, check for conflict
    if let Some(existing) = resolved.get(package) {
        check_conflict(package, existing, range)?;
        return Ok(()); // Already installed
    }

    // Check lockfile for reproducible install
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

    // Check conflict with already-resolved plugins
    if let Some(existing) = resolved.get(package) {
        check_conflict(package, &meta.version, range)?;
        let _ = existing; // already installed with compatible version
        return Ok(());
    }

    crate::output::print_info(&format!("Installing {}@{}", package, meta.version));

    download_and_extract(&meta.tarball, &meta.integrity, package, uepm_plugins_dir, token).await?;

    // Record as resolved
    resolved.insert(package.to_string(), meta.version.clone());

    // Update lockfile entry
    let deps_for_lock: HashMap<String, String> = HashMap::new();
    lock.plugins.insert(
        package.to_string(),
        LockedPlugin {
            resolved: meta.version.clone(),
            tarball: meta.tarball.clone(),
            sha512: meta.integrity.clone(),
            dependencies: deps_for_lock,
        },
    );

    // Read the installed plugin's own uepm.ini for its dependencies
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

            // Record dep in lockfile entry
            lock.plugins
                .get_mut(package)
                .unwrap()
                .dependencies
                .insert(dep_package.clone(), dep_range.clone());
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test resolver
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add uepm/src/resolver.rs
git commit -m "feat(uepm): Add recursive dependency resolver with conflict detection"
```

---

### Task 9: `uepm init` command

**Files:**
- Modify: `uepm/src/commands/init.rs`

The init command:
1. Finds `.uproject` in current directory
2. Reads `EngineAssociation` (warns if GUID)
3. Prompts for install mode (select: symlink/copy/none) with VCS-detected default
4. Adds `UEPMPlugins` to `.uproject`
5. Creates `uepm.ini`
6. Creates `UEPMPlugins/` directory

- [ ] **Step 1: Write failing tests**

```rust
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
    fn test_detect_mode_p4_env() {
        std::env::set_var("P4PORT", "perforce:1666");
        let mode = detect_install_mode(std::path::Path::new("."));
        std::env::remove_var("P4PORT");
        assert_eq!(mode, InstallMode::Copy);
    }

    #[test]
    fn test_detect_mode_windows_is_copy() {
        // On non-Windows CI, test the platform detection logic directly
        if cfg!(windows) {
            let mode = detect_install_mode(std::path::Path::new("."));
            assert_eq!(mode, InstallMode::Copy);
        }
    }

    #[tokio::test]
    async fn test_init_creates_uepm_ini_and_modifies_uproject() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "5.7");

        run_init_with_mode(dir.path(), InstallMode::Symlink).await.unwrap();

        // uepm.ini created
        assert!(dir.path().join("uepm.ini").exists());
        let m = uepm::manifest::read_manifest(dir.path()).unwrap();
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));

        // .uproject modified
        let uproject = dir.path().join("Test.uproject");
        let content = std::fs::read_to_string(uproject).unwrap();
        assert!(content.contains("UEPMPlugins"));

        // UEPMPlugins/ created
        assert!(dir.path().join("UEPMPlugins").is_dir());
    }

    #[tokio::test]
    async fn test_init_skips_engine_version_for_guid() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), "{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}");

        run_init_with_mode(dir.path(), InstallMode::Copy).await.unwrap();

        let m = uepm::manifest::read_manifest(dir.path()).unwrap();
        assert!(m.engine_version.is_none());
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test commands::init
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/commands/init.rs`**

```rust
use crate::errors::UepmError;
use crate::manifest::create_manifest;
use crate::uproject::{add_plugin_directory, find_uproject, get_engine_association, is_guid};
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum InstallMode {
    Symlink,
    Copy,
    None,
}

pub fn detect_install_mode(project_dir: &Path) -> InstallMode {
    // P4 indicators take highest priority
    if std::env::var("P4PORT").is_ok() || std::env::var("P4CONFIG").is_ok() {
        return InstallMode::Copy;
    }
    // Walk up looking for .p4config
    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".p4config").exists() {
            return InstallMode::Copy;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    // Windows defaults to copy (symlinks require Developer Mode)
    if cfg!(windows) {
        return InstallMode::Copy;
    }

    // Git indicator → symlink
    let mut dir = Some(project_dir.to_path_buf());
    while let Some(d) = dir {
        if d.join(".git").is_dir() {
            return InstallMode::Symlink;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    InstallMode::Symlink
}

pub async fn run(yes: bool) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let mode = select_install_mode(&project_dir, yes)?;
    run_init_with_mode(&project_dir, mode).await
}

fn select_install_mode(project_dir: &Path, yes: bool) -> Result<InstallMode, UepmError> {
    let default = detect_install_mode(project_dir);
    if yes {
        return Ok(default);
    }
    if !dialoguer::console::Term::stdout().is_term() {
        return Err(UepmError::InteractiveRequired);
    }

    let options = ["symlink — symbolic links in UEPMPlugins/ (git workflow)",
                   "copy   — real files in UEPMPlugins/ (Perforce / any VCS)",
                   "none   — UEPM handles init only, no postinstall hook"];
    let default_idx = match default {
        InstallMode::Symlink => 0,
        InstallMode::Copy => 1,
        InstallMode::None => 2,
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Install mode")
        .default(default_idx)
        .items(&options)
        .interact()
        .map_err(|e| UepmError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    Ok(match selection {
        0 => InstallMode::Symlink,
        1 => InstallMode::Copy,
        _ => InstallMode::None,
    })
}

pub async fn run_init_with_mode(project_dir: &Path, _mode: InstallMode) -> Result<(), UepmError> {
    // Find .uproject
    let uproject_path = find_uproject(project_dir)?;

    // Read engine association
    let engine_assoc = get_engine_association(&uproject_path)?;
    let engine_version = if is_guid(&engine_assoc) {
        crate::output::print_warn(
            "Engine is a launcher-installed GUID — engine_version will be omitted from uepm.ini"
        );
        None
    } else {
        Some(engine_assoc.as_str())
    };

    // Add UEPMPlugins to .uproject
    add_plugin_directory(&uproject_path, "UEPMPlugins")?;

    // Create uepm.ini
    create_manifest(project_dir, engine_version)?;

    // Create UEPMPlugins/ directory
    let uepm_plugins = project_dir.join("UEPMPlugins");
    if !uepm_plugins.exists() {
        std::fs::create_dir_all(&uepm_plugins)?;
    }

    crate::output::print_success("Project initialized for UEPM");
    crate::output::print_info("Run 'uepm install @scope/plugin' to add your first plugin");

    Ok(())
}
```

- [ ] **Step 4: Add `console` feature to `dialoguer` in `Cargo.toml`**

The `dialoguer::console::Term` used above is from the `console` crate re-exported by `dialoguer`. No change needed to Cargo.toml — it's included by default with `dialoguer`.

- [ ] **Step 5: Run tests — verify they pass**

```bash
cd uepm && cargo test commands::init
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add uepm/src/commands/init.rs
git commit -m "feat(uepm): Add uepm init command with VCS-detected install mode prompt"
```

---

### Task 10: `uepm install` command

**Files:**
- Modify: `uepm/src/commands/install.rs`

- [ ] **Step 1: Write failing integration test**

Create `uepm/tests/install_integration.rs`:

```rust
use std::io::Write;
use mockito::Server;
use tempfile::tempdir;
use serde_json::json;

fn make_fake_tarball() -> Vec<u8> {
    use flate2::{write::GzEncoder, Compression};
    use tar::Builder;
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut builder = Builder::new(enc);
    let content = b"{\"FileVersion\": 3, \"Version\": 1}";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, "package/CoolPlugin.uplugin", content.as_ref()).unwrap();
    let enc = builder.into_inner().unwrap();
    enc.finish().unwrap()
}

fn sha512_integrity(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine};
    use sha2::{Digest, Sha512};
    format!("sha512-{}", general_purpose::STANDARD.encode(Sha512::digest(data)))
}

#[tokio::test]
async fn test_install_single_plugin() {
    let dir = tempdir().unwrap();
    let uepm_dir = dir.path().join("UEPMPlugins");
    std::fs::create_dir(&uepm_dir).unwrap();

    // Write uepm.ini and uepm.lock
    std::fs::write(
        dir.path().join("uepm.ini"),
        "[plugins]\n",
    ).unwrap();

    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let mut server = Server::new_async().await;

    let tarball_url = format!("{}/tarball.tgz", server.url());
    let meta = json!({
        "name": "@acme/cool-plugin",
        "dist-tags": { "latest": "1.0.0" },
        "versions": {
            "1.0.0": {
                "name": "@acme/cool-plugin",
                "version": "1.0.0",
                "dist": {
                    "tarball": tarball_url,
                    "integrity": integrity,
                }
            }
        }
    });

    let _meta_mock = server
        .mock("GET", "/%40acme%2Fcool-plugin")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta.to_string())
        .create_async()
        .await;

    let _tarball_mock = server
        .mock("GET", "/tarball.tgz")
        .with_status(200)
        .with_body(tarball)
        .create_async()
        .await;

    std::env::set_var("UEPM_REGISTRY", server.url());

    uepm::commands::install::run_install(
        &["@acme/cool-plugin".to_string()],
        dir.path(),
    ).await.unwrap();

    std::env::remove_var("UEPM_REGISTRY");

    // Plugin extracted
    assert!(uepm_dir.join("cool-plugin").join("CoolPlugin.uplugin").exists());

    // uepm.ini updated
    let manifest = uepm::manifest::read_manifest(dir.path()).unwrap();
    assert!(manifest.plugins.contains_key("@acme/cool-plugin"));

    // uepm.lock written
    let lock = uepm::lockfile::read_lockfile(dir.path()).unwrap().unwrap();
    assert!(lock.plugins.contains_key("@acme/cool-plugin"));
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd uepm && cargo test install_integration
```

Expected: compilation error (run_install not defined).

- [ ] **Step 3: Write `uepm/src/commands/install.rs`**

```rust
use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile, LockFile};
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
        // Install all from uepm.ini
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

            // Resolve to get the actual version for uepm.ini
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

            // Add to uepm.ini
            add_plugin(project_dir, &package, &pinned_range)?;
        }
    }

    write_lockfile(project_dir, &lock)?;
    crate::output::print_success(&format!("Installed {} plugin(s)", resolved.len()));
    Ok(())
}

/// Parse "@scope/plugin@1.2.0" into ("@scope/plugin", Some("1.2.0"))
/// or "@scope/plugin" into ("@scope/plugin", None)
fn parse_package_spec(spec: &str) -> (String, Option<&str>) {
    // Find the @ that separates package name from version (not the leading @ of scope)
    // "@scope/plugin@1.2.0" → split at the last '@'
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
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd uepm && cargo test install
```

Expected: all tests pass (unit + integration).

- [ ] **Step 5: Commit**

```bash
git add uepm/src/commands/install.rs uepm/tests/install_integration.rs
git commit -m "feat(uepm): Add uepm install command with full resolution flow"
```

---

### Task 11: `uepm uninstall`, `uepm update`, `uepm list`

**Files:**
- Modify: `uepm/src/commands/uninstall.rs`
- Modify: `uepm/src/commands/update.rs`
- Modify: `uepm/src/commands/list.rs`

- [ ] **Step 1: Write tests for all three commands**

In `uepm/src/commands/uninstall.rs`:
```rust
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

        std::fs::write(
            dir.path().join("uepm.ini"),
            "[plugins]\n@acme/cool-plugin = ^1.0.0\n",
        ).unwrap();

        run_uninstall("@acme/cool-plugin", dir.path()).await.unwrap();

        assert!(!uepm_dir.join("cool-plugin").exists());
        let m = uepm::manifest::read_manifest(dir.path()).unwrap();
        assert!(!m.plugins.contains_key("@acme/cool-plugin"));
    }
}
```

In `uepm/src/commands/list.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_returns_installed_plugins() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();
        std::fs::create_dir(uepm_dir.join("cool-plugin")).unwrap();
        std::fs::write(
            uepm_dir.join("cool-plugin").join("CoolPlugin.uplugin"),
            "{}",
        ).unwrap();

        std::fs::write(
            dir.path().join("uepm.ini"),
            "[plugins]\n@acme/cool-plugin = ^1.0.0\n[settings]\nengine_version = 5.7\n",
        ).unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "@acme/cool-plugin");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd uepm && cargo test uninstall && cargo test list
```

Expected: compilation errors.

- [ ] **Step 3: Write `uepm/src/commands/uninstall.rs`**

```rust
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
```

- [ ] **Step 4: Write `uepm/src/commands/update.rs`**

```rust
use crate::errors::UepmError;
use crate::lockfile::{read_lockfile, write_lockfile, LockFile};
use crate::manifest::read_manifest;
use crate::registry::RegistryClient;
use crate::resolver::resolve_and_install;
use std::collections::HashMap;
use std::path::Path;

pub async fn run(package: Option<String>) -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let manifest = read_manifest(&project_dir)?;
    let uepm_dir = project_dir.join("UEPMPlugins");
    let client = RegistryClient::from_env();
    let token = std::env::var("UEPM_TOKEN").ok();

    // For update, ignore the lockfile so fresh versions are resolved
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
```

- [ ] **Step 5: Write `uepm/src/commands/list.rs`**

```rust
use crate::errors::UepmError;
use crate::manifest::read_manifest;
use crate::lockfile::read_lockfile;
use crate::resolver::plugin_dir_name;
use std::path::Path;

#[derive(Debug)]
pub struct PluginEntry {
    pub name: String,
    pub resolved_version: Option<String>,
    pub engine_range: String,
    pub compatible: Option<bool>,
}

pub async fn run() -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let plugins = list_plugins(&project_dir)?;

    if plugins.is_empty() {
        crate::output::print_info("No plugins installed. Run 'uepm install @scope/plugin' to add one.");
        return Ok(());
    }

    for p in &plugins {
        let version = p.resolved_version.as_deref().unwrap_or("unknown");
        let compat = match p.compatible {
            Some(true) => "✓ compatible",
            Some(false) => "✗ incompatible",
            None => "? unknown",
        };
        crate::output::print_info(&format!(
            "{} @ {} — engine: {} [{}]",
            p.name, version, p.engine_range, compat
        ));
    }
    Ok(())
}

pub fn list_plugins(project_dir: &Path) -> Result<Vec<PluginEntry>, UepmError> {
    let manifest = read_manifest(project_dir)?;
    let lock = read_lockfile(project_dir)?.unwrap_or_default();

    let engine_req: Option<semver::VersionReq> = manifest
        .engine_version
        .as_deref()
        .and_then(|v| semver::VersionReq::parse(v).ok());

    let mut entries = Vec::new();

    for (name, engine_range) in &manifest.plugins {
        let resolved = lock.plugins.get(name).map(|lp| lp.resolved.clone());

        let compatible = resolved.as_deref().and_then(|v| {
            let ver = semver::Version::parse(v).ok()?;
            let req = semver::VersionReq::parse(engine_range).ok()?;
            let _ = engine_req.as_ref()?; // only check if we know engine version
            Some(req.matches(&ver))
        });

        entries.push(PluginEntry {
            name: name.clone(),
            resolved_version: resolved,
            engine_range: engine_range.clone(),
            compatible,
        });
    }

    Ok(entries)
}
```

- [ ] **Step 6: Run all tests**

```bash
cd uepm && cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add uepm/src/commands/uninstall.rs uepm/src/commands/update.rs uepm/src/commands/list.rs
git commit -m "feat(uepm): Add uepm uninstall, update, and list commands"
```

---

### Task 12: GitHub Actions release pipeline and install scripts

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `install.sh`
- Create: `install.ps1`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: uepm-linux-x86_64
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            artifact: uepm-windows-x86_64.exe
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: uepm-macos-arm64
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: uepm-macos-x86_64

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        working-directory: uepm
        run: cargo build --release --target ${{ matrix.target }}

      - name: Rename binary (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cp uepm/target/${{ matrix.target }}/release/uepm ${{ matrix.artifact }}

      - name: Rename binary (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          Copy-Item "uepm/target/${{ matrix.target }}/release/uepm.exe" "${{ matrix.artifact }}"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/*
          generate_release_notes: true
```

- [ ] **Step 2: Create `install.sh`**

```bash
#!/usr/bin/env sh
set -e

REPO="bad-planning/uepm"
INSTALL_DIR="$HOME/.uepm/bin"

# Detect OS and arch
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64) ARTIFACT="uepm-linux-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      arm64)  ARTIFACT="uepm-macos-arm64" ;;
      x86_64) ARTIFACT="uepm-macos-x86_64" ;;
      *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

# Get latest release version
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

echo "Installing uepm $LATEST..."

# Download
URL="https://github.com/$REPO/releases/download/$LATEST/$ARTIFACT"
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$INSTALL_DIR/uepm"
chmod +x "$INSTALL_DIR/uepm"

# Add to PATH if not already there
add_to_path() {
  PROFILE="$1"
  if [ -f "$PROFILE" ] && ! grep -q 'uepm/bin' "$PROFILE"; then
    echo '' >> "$PROFILE"
    echo '# UEPM' >> "$PROFILE"
    echo 'export PATH="$HOME/.uepm/bin:$PATH"' >> "$PROFILE"
    echo "Added ~/.uepm/bin to PATH in $PROFILE"
  fi
}

add_to_path "$HOME/.bashrc"
add_to_path "$HOME/.zshrc"

echo ""
echo "✓ uepm installed to $INSTALL_DIR/uepm"
echo "  Restart your shell or run: export PATH=\"\$HOME/.uepm/bin:\$PATH\""
echo ""
echo "  Get started: uepm init"
```

- [ ] **Step 3: Create `install.ps1`**

```powershell
$ErrorActionPreference = 'Stop'

$Repo = "bad-planning/uepm"
$InstallDir = "$env:LOCALAPPDATA\uepm\bin"

# Detect architecture
$Arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else {
    Write-Error "Only 64-bit Windows is supported."
    exit 1
}
$Artifact = "uepm-windows-$Arch.exe"

# Get latest release
$Latest = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name
Write-Host "Installing uepm $Latest..."

# Download
$Url = "https://github.com/$Repo/releases/download/$Latest/$Artifact"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Invoke-WebRequest -Uri $Url -OutFile "$InstallDir\uepm.exe"

# Add to user PATH if not already present
$CurrentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*uepm\bin*") {
    [System.Environment]::SetEnvironmentVariable(
        "Path",
        "$CurrentPath;$InstallDir",
        "User"
    )
    Write-Host "Added $InstallDir to user PATH"
}

Write-Host ""
Write-Host "✓ uepm installed to $InstallDir\uepm.exe"
Write-Host "  Restart your terminal to use uepm"
Write-Host ""
Write-Host "  Get started: uepm init"
```

- [ ] **Step 4: Verify the full build works**

```bash
cd uepm && cargo build --release
```

Expected: binary produced at `uepm/target/release/uepm`.

- [ ] **Step 5: Run the full test suite one final time**

```bash
cd uepm && cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release.yml install.sh install.ps1
git commit -m "feat(uepm): Add GitHub Actions release pipeline and install scripts"
```

---

## Self-Review Notes

**Spec coverage:**
- ✅ All 5 commands: init, install, uninstall, update, list
- ✅ uepm.ini read/write (manifest.rs)
- ✅ uepm.lock read/write (lockfile.rs)
- ✅ .uproject read/write (uproject.rs)
- ✅ npm registry client with version resolution (registry.rs)
- ✅ Tarball download + sha512 verification + extraction (installer.rs)
- ✅ Recursive dep resolution with conflict detection (resolver.rs)
- ✅ VCS detection for install mode (init.rs)
- ✅ UEPM_REGISTRY, UEPM_TOKEN, RUST_LOG env vars via dotenvy
- ✅ All 4 build targets in release.yml
- ✅ install.sh (Unix) + install.ps1 (Windows)
- ✅ Unit tests for pure logic modules
- ✅ Integration tests with mockito mock server

**Missing from spec that's needed:** `urlencoding` crate added to Cargo.toml (Task 6 Step 4) — not in original spec dependency list but required for npm package URL encoding.
