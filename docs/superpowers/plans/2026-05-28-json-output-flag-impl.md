# `--json` Output Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a global `--json` flag to all `uepm` commands so each command emits structured JSON on stdout instead of colored human-readable output; errors also emit JSON on stdout.

**Architecture:** `OutputMode` enum (Human | Json, default Human) added to `UEPMContext` as a plain field — no constructor signature changes. `main.rs` sets the field after construction and enforces `--yes` when `--json` is used with interactive commands. Each command defines a typed `Serialize` struct and calls `output::emit_json` at the end.

**Tech Stack:** Rust, clap 4 (global arg), serde + serde_json (already present), assert_cmd (new dev-dep for JSON integration tests), mockito + tempfile (already present).

---

## File Map

| Action | File | Change |
|---|---|---|
| Modify | `Cargo.toml` | Add `assert_cmd = "2"` to `[dev-dependencies]` |
| Modify | `src/context.rs` | Add `OutputMode` enum; add `output_mode` field to `UEPMContext` |
| Modify | `src/output.rs` | Add `emit_json<T: Serialize>` helper |
| Modify | `src/main.rs` | Add `--json` global flag; set `output_mode`; enforce `--yes`; update error handler |
| Modify | `src/commands/list.rs` | Derive `Serialize` on `PluginEntry`; add JSON emit branch |
| Modify | `src/commands/install.rs` | Add `InstallResult`; snapshot pre-install lock keys; add JSON emit |
| Modify | `src/commands/uninstall.rs` | Add `UninstallResult`; add JSON emit branch |
| Modify | `src/commands/update.rs` | Snapshot old lock before resolution; add `UpdateResult`; add JSON emit |
| Modify | `src/commands/publish.rs` | Add `PublishResult`; add JSON emit branch |
| Modify | `src/commands/init.rs` | Add `InitOutput` enum; read manifest after init; add JSON emit in `run()` |
| Create | `tests/json_output.rs` | Integration tests for JSON mode via `assert_cmd` |

---

## Task 1: Add `assert_cmd` dev-dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, find `[dev-dependencies]` and add:

```toml
[dev-dependencies]
mockito = "1"
tempfile = "3"
assert_cmd = "2"
```

- [ ] **Step 2: Verify it resolves**

```
cargo check
```
Expected: no errors. If assert_cmd version conflicts, try `assert_cmd = "2.0"`.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add assert_cmd dev-dep for JSON integration tests"
```

---

## Task 2: Add `OutputMode` to `UEPMContext`

**Files:**
- Modify: `src/context.rs`

- [ ] **Step 1: Add `OutputMode` enum and field**

Open `src/context.rs`. Replace the entire file contents with:

```rust
use crate::errors::UepmError;
use crate::registry::RegistryClient;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Human,
    Json,
}

/// Resolved project context constructed once at startup and passed to every command.
pub struct UEPMContext {
    pub project_dir: PathBuf,
    /// `<project_dir>/UEPMPlugins`
    pub uepm_plugins_dir: PathBuf,
    pub registry: RegistryClient,
    pub token: Option<String>,
    pub output_mode: OutputMode,
}

impl UEPMContext {
    /// Build context from the current working directory and environment variables.
    /// Reads `UEPM_REGISTRY` and `UEPM_TOKEN`.
    pub fn new() -> Result<Self, UepmError> {
        Ok(Self::with_dir(std::env::current_dir()?))
    }

    /// Build context from an explicit project directory. Useful in tests.
    pub fn with_dir(project_dir: PathBuf) -> Self {
        let uepm_plugins_dir = project_dir.join("UEPMPlugins");
        let registry = RegistryClient::from_env();
        let token = std::env::var("UEPM_TOKEN").ok();
        Self { project_dir, uepm_plugins_dir, registry, token, output_mode: OutputMode::Human }
    }

    /// Build context with explicit registry URL and token — no env var reads.
    /// Use this in integration tests to avoid races between parallel test threads.
    pub fn for_test(project_dir: PathBuf, registry_url: &str, token: Option<String>) -> Self {
        let uepm_plugins_dir = project_dir.join("UEPMPlugins");
        let registry = RegistryClient::new(registry_url, token.clone());
        Self { project_dir, uepm_plugins_dir, registry, token, output_mode: OutputMode::Human }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build. The new field has a default so the existing struct literals in tests still work (we're not using struct update syntax anywhere — the constructors are the only construction sites).

- [ ] **Step 3: Run existing tests to confirm no regressions**

```
cargo test
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/context.rs
git commit -m "feat: add OutputMode enum and field to UEPMContext"
```

---

## Task 3: Add `emit_json` helper to `output.rs`

**Files:**
- Modify: `src/output.rs`

- [ ] **Step 1: Add the helper**

Open `src/output.rs`. Add at the top after the existing `use` statements:

```rust
use serde::Serialize;
```

Then add at the end of the file:

```rust
/// Serialize `value` as compact JSON and print to stdout.
/// Called by commands when `ctx.output_mode == OutputMode::Json`.
pub fn emit_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string(value).expect("serialize"));
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Commit**

```bash
git add src/output.rs
git commit -m "feat: add emit_json helper to output module"
```

---

## Task 4: Wire `--json` in `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add the global flag and update error handler**

Replace the entire contents of `src/main.rs` with:

```rust
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use uepm::context::{OutputMode, UEPMContext};

#[derive(Parser)]
#[command(name = "uepm", version, about = "Unreal Engine Plugin Manager")]
struct Cli {
    /// Emit structured JSON on stdout instead of colored human-readable output
    #[arg(long, global = true)]
    json: bool,
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
        /// If empty, installs all plugins in Config/UEPM.ini
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
    /// Publish this plugin to the npm registry
    Publish {
        /// npm dist-tag (default: latest)
        #[arg(long, default_value = "latest")]
        tag: String,
        /// Validate and build tarball but do not upload
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
        /// Registry access level
        #[arg(long, default_value = "public")]
        access: String,
    },
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        .init();

    let cli = Cli::parse();

    // Enforce --yes when --json is used with interactive commands.
    if cli.json {
        let needs_yes = matches!(
            &cli.command,
            Commands::Init { yes: false } | Commands::Publish { yes: false, .. }
        );
        if needs_yes {
            println!("{}", serde_json::json!({"error": "pass --yes when using --json"}));
            std::process::exit(1);
        }
    }

    let mut ctx = match UEPMContext::new() {
        Ok(c) => c,
        Err(e) => {
            if cli.json {
                println!("{}", serde_json::json!({"error": e.to_string()}));
            } else {
                uepm::output::print_error(&format!("{e}"));
            }
            std::process::exit(1);
        }
    };
    ctx.output_mode = if cli.json { OutputMode::Json } else { OutputMode::Human };

    let result = match cli.command {
        Commands::Init { yes } => uepm::commands::init::run(&ctx, yes).await,
        Commands::Install { packages } => uepm::commands::install::run(&ctx, packages).await,
        Commands::Uninstall { package } => uepm::commands::uninstall::run(&ctx, package).await,
        Commands::Update { package } => uepm::commands::update::run(&ctx, package).await,
        Commands::List => uepm::commands::list::run(&ctx).await,
        Commands::Publish { tag, dry_run, yes, access } => {
            uepm::commands::publish::run(&ctx, &tag, dry_run, yes, &access).await
        }
    };

    if let Err(e) = result {
        if ctx.output_mode == OutputMode::Json {
            println!("{}", serde_json::json!({"error": e.to_string()}));
        } else {
            uepm::output::print_error(&format!("{e}"));
        }
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Run tests**

```
cargo test
```
Expected: all existing tests pass.

- [ ] **Step 4: Smoke test the flag exists**

```
cargo run -- --help
```
Expected: `--json` appears in the output.

- [ ] **Step 5: Smoke test --json validation**

```
cargo run -- --json init
```
Expected: `{"error":"pass --yes when using --json"}` printed, non-zero exit.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire --json global flag and --yes enforcement in main.rs"
```

---

## Task 5: JSON output for `list`

**Files:**
- Modify: `src/commands/list.rs`

- [ ] **Step 1: Derive `Serialize` on `PluginEntry`**

Open `src/commands/list.rs`. Change the `PluginEntry` struct definition from:

```rust
#[derive(Debug)]
pub struct PluginEntry {
```

to:

```rust
#[derive(Debug, serde::Serialize)]
pub struct PluginEntry {
```

- [ ] **Step 2: Add JSON emit to `run()`**

Replace the `run()` function with:

```rust
pub async fn run(ctx: &UEPMContext) -> Result<(), UepmError> {
    let plugins = list_plugins(&ctx.project_dir)?;

    if ctx.output_mode == crate::context::OutputMode::Json {
        crate::output::emit_json(&plugins);
        return Ok(());
    }

    if plugins.is_empty() {
        crate::output::print_info(
            "No plugins installed. Run 'uepm install @scope/plugin' to add one.",
        );
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
```

- [ ] **Step 3: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 4: Run tests**

```
cargo test list
```
Expected: all list tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/list.rs
git commit -m "feat: add --json output to uepm list"
```

---

## Task 6: JSON output for `install`

**Files:**
- Modify: `src/commands/install.rs`

- [ ] **Step 1: Add `InstallResult` and update `run_install`**

Open `src/commands/install.rs`. Add after the existing `use` lines:

```rust
use crate::context::OutputMode;
use std::collections::HashSet;
```

Add the output struct after the `use` block:

```rust
#[derive(serde::Serialize)]
struct InstallResult {
    name: String,
    version: String,
    tarball: String,
    fresh: bool,
}
```

Replace `run_install` with:

```rust
pub async fn run_install(ctx: &UEPMContext, packages: &[String]) -> Result<(), UepmError> {
    if !ctx.uepm_plugins_dir.exists() {
        std::fs::create_dir_all(&ctx.uepm_plugins_dir)?;
    }

    let mut lock = read_lockfile(&ctx.project_dir)?.unwrap_or_default();
    // Snapshot the names already locked before this invocation to track freshness.
    let pre_install_names: HashSet<String> = lock.plugins.keys().cloned().collect();

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

    if ctx.output_mode == OutputMode::Json {
        let results: Vec<InstallResult> = resolved
            .keys()
            .map(|name| {
                let locked = lock.plugins.get(name);
                InstallResult {
                    name: name.clone(),
                    version: locked.map(|p| p.resolved.clone()).unwrap_or_default(),
                    tarball: locked.map(|p| p.tarball.clone()).unwrap_or_default(),
                    fresh: !pre_install_names.contains(name),
                }
            })
            .collect();
        crate::output::emit_json(&results);
    } else {
        crate::output::print_success(&format!("Installed {} plugin(s)", resolved.len()));
    }

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Run tests**

```
cargo test install
```
Expected: all install tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: add --json output to uepm install"
```

---

## Task 7: JSON output for `uninstall`

**Files:**
- Modify: `src/commands/uninstall.rs`

- [ ] **Step 1: Add `UninstallResult` and update `run_uninstall`**

Open `src/commands/uninstall.rs`. Add after the existing `use` lines:

```rust
use crate::context::OutputMode;
```

Add the output struct:

```rust
#[derive(serde::Serialize)]
struct UninstallResult {
    removed: String,
}
```

Replace `run_uninstall` with:

```rust
pub async fn run_uninstall(ctx: &UEPMContext, package: &str) -> Result<(), UepmError> {
    let dir_name = plugin_dir_name(package);
    let plugin_dir = ctx.uepm_plugins_dir.join(&dir_name);

    if plugin_dir.exists() || plugin_dir.symlink_metadata().is_ok() {
        if plugin_dir.is_symlink() {
            std::fs::remove_file(&plugin_dir)?;
        } else {
            std::fs::remove_dir_all(&plugin_dir)?;
        }
        if ctx.output_mode != OutputMode::Json {
            crate::output::print_success(&format!("Removed {dir_name}"));
        }
    } else if ctx.output_mode != OutputMode::Json {
        crate::output::print_warn(&format!("{dir_name} was not found in UEPMPlugins/"));
    }

    remove_plugin(&ctx.project_dir, package)?;

    if let Some(mut lock) = read_lockfile(&ctx.project_dir)? {
        lock.plugins.remove(package);
        write_lockfile(&ctx.project_dir, &lock)?;
    }

    if ctx.output_mode == OutputMode::Json {
        crate::output::emit_json(&UninstallResult { removed: package.to_string() });
    }

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Run tests**

```
cargo test uninstall
```
Expected: all uninstall tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/uninstall.rs
git commit -m "feat: add --json output to uepm uninstall"
```

---

## Task 8: JSON output for `update`

**Files:**
- Modify: `src/commands/update.rs`

- [ ] **Step 1: Add `UpdateResult` and snapshot old lock**

Open `src/commands/update.rs`. Add after the existing `use` lines:

```rust
use crate::context::OutputMode;
```

Add the output struct:

```rust
#[derive(serde::Serialize)]
struct UpdateResult {
    name: String,
    from: Option<String>,
    to: String,
}
```

Replace `run()` with:

```rust
pub async fn run(ctx: &UEPMContext, package: Option<String>) -> Result<(), UepmError> {
    let mut manifest = read_manifest(&ctx.project_dir)?;

    // Snapshot old lock before any modifications — needed for UpdateResult.from.
    let old_lock = read_lockfile(&ctx.project_dir)?.unwrap_or_default();

    let mut lock = if let Some(ref pkg) = package {
        let mut existing = read_lockfile(&ctx.project_dir)?.unwrap_or_default();
        existing.plugins.remove(pkg);
        existing
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
        let results: Vec<UpdateResult> = resolved
            .iter()
            .map(|(name, to)| UpdateResult {
                name: name.clone(),
                from: old_lock.plugins.get(name).map(|p| p.resolved.clone()),
                to: to.clone(),
            })
            .collect();
        crate::output::emit_json(&results);
    } else {
        crate::output::print_success(&format!("Updated {} plugin(s)", resolved.len()));
    }

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Run tests**

```
cargo test update
```
Expected: all update tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/update.rs
git commit -m "feat: add --json output to uepm update"
```

---

## Task 9: JSON output for `publish`

**Files:**
- Modify: `src/commands/publish.rs`

- [ ] **Step 1: Add `PublishResult` and emit at end**

Open `src/commands/publish.rs`. Add after the existing `use` lines:

```rust
use crate::context::OutputMode;
```

Add the output struct:

```rust
#[derive(serde::Serialize)]
struct PublishResult {
    name: String,
    version: String,
    registry: String,
    dry_run: bool,
}
```

In `src/commands/publish.rs`, find the line near the end of `run()` that reads:

```rust
crate::output::print_success(&format!("Published {}@{} → {registry}", meta.name, meta.version));
Ok(())
```

Replace those two lines with:

```rust
if ctx.output_mode == OutputMode::Json {
    crate::output::emit_json(&PublishResult {
        name: meta.name.clone(),
        version: meta.version.clone(),
        registry: registry.clone(),
        dry_run,
    });
} else {
    crate::output::print_success(&format!(
        "Published {}@{} → {registry}",
        meta.name, meta.version
    ));
}
Ok(())
```

`meta`, `registry`, and `dry_run` are already in scope at that point in `run()`. `meta` is `read_package_metadata(...)` at the top, `registry` is the trimmed base URL string, `dry_run` is a parameter.

- [ ] **Step 2: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 3: Run tests**

```
cargo test publish
```
Expected: all publish tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/publish.rs
git commit -m "feat: add --json output to uepm publish"
```

---

## Task 10: JSON output for `init`

**Files:**
- Modify: `src/commands/init.rs`

- [ ] **Step 1: Add `InitOutput` enum**

Open `src/commands/init.rs`. Add after the existing `use` block (before the `pub async fn run` definition):

```rust
use crate::context::OutputMode;

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
```

- [ ] **Step 2: Update `run()` to emit JSON after init**

Replace the `run()` function with:

```rust
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
```

- [ ] **Step 3: Verify it compiles**

```
cargo build
```
Expected: clean build.

- [ ] **Step 4: Run tests**

```
cargo test init
```
Expected: all init tests pass.

- [ ] **Step 5: Run all tests**

```
cargo test
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/commands/init.rs
git commit -m "feat: add --json output to uepm init"
```

---

## Task 11: JSON integration tests — local commands

These tests use `assert_cmd` to spawn the compiled binary. They do not need a registry mock because these commands only read/write local files.

**Files:**
- Create: `tests/json_output.rs`

- [ ] **Step 1: Create the test file with list and uninstall tests**

Create `tests/json_output.rs`:

```rust
use assert_cmd::Command;
use std::collections::HashMap;
use tempfile::tempdir;
use uepm::lockfile::{LockedPlugin, LockFile, write_lockfile};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn locked_plugin(version: &str, tarball: &str) -> LockedPlugin {
    LockedPlugin {
        resolved: version.to_string(),
        tarball: tarball.to_string(),
        sha512: "sha512-abc".to_string(),
        dependencies: HashMap::new(),
    }
}

fn uepm() -> Command {
    Command::cargo_bin("uepm").unwrap()
}

// ── list --json ───────────────────────────────────────────────────────────────

#[test]
fn test_list_json_returns_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Settings]\nEngineVersion = \"5.3.0\"\n\n[Dependencies]\n\"@acme/cool-plugin\" = \">=5.0.0, <6.0.0\"\n",
    ).unwrap();

    let mut lock = LockFile::default();
    lock.plugins.insert(
        "@acme/cool-plugin".to_string(),
        locked_plugin("1.2.3", "https://example.com/tarball.tgz"),
    );
    write_lockfile(dir.path(), &lock).unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code should be 0");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("stdout must be valid JSON");
    assert!(json.is_array(), "list --json must emit an array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "@acme/cool-plugin");
    assert_eq!(arr[0]["version"], "1.2.3");
    assert_eq!(arr[0]["compatible"], true);
}

#[test]
fn test_list_json_empty_returns_empty_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json, serde_json::json!([]));
}

// ── uninstall --json ──────────────────────────────────────────────────────────

#[test]
fn test_uninstall_json_returns_removed_field() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins/cool-plugin")).unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
    ).unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "uninstall", "@acme/cool-plugin"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["removed"], "@acme/cool-plugin");
}

// ── update --json (empty manifest, no network) ────────────────────────────────

#[test]
fn test_update_json_no_plugins_returns_empty_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "update"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json, serde_json::json!([]));
}
```

- [ ] **Step 2: Run the new tests**

```
cargo test --test json_output
```
Expected: all 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/json_output.rs
git commit -m "test: JSON integration tests for list, uninstall, update"
```

---

## Task 12: JSON integration tests — `install` (3 cases with mockito)

**Files:**
- Modify: `tests/json_output.rs`

These tests need a live mockito server to serve registry metadata and a tarball.

- [ ] **Step 1: Add helpers and install tests to `tests/json_output.rs`**

Add the following to the end of `tests/json_output.rs`:

```rust
// ── install --json ────────────────────────────────────────────────────────────
// These tests need a mock registry served over localhost.

use base64::{engine::general_purpose, Engine};
use mockito::Server;
use sha2::{Digest, Sha512};

fn make_fake_tarball() -> Vec<u8> {
    use flate2::{write::GzEncoder, Compression};
    use tar::Builder;
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(enc);
    let content = b"{\"FileVersion\": 3}";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, "package/CoolPlugin.uplugin", content.as_ref()).unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

fn sha512_integrity(data: &[u8]) -> String {
    format!("sha512-{}", general_purpose::STANDARD.encode(Sha512::digest(data)))
}

fn registry_meta(pkg: &str, version: &str, tarball_url: &str, integrity: &str) -> serde_json::Value {
    serde_json::json!({
        "name": pkg,
        "dist-tags": { "latest": version },
        "versions": {
            version: {
                "name": pkg,
                "version": version,
                "dist": { "tarball": tarball_url, "integrity": integrity }
            }
        }
    })
}

#[tokio::test]
async fn test_install_json_single_package_fresh() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let tarball_url = format!("{}/tarball.tgz", server.url());
    let meta = registry_meta("@acme/cool-plugin", "1.0.0", &tarball_url, &integrity);

    let _meta_mock = server.mock("GET", "/%40acme%2Fcool-plugin")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(meta.to_string()).create_async().await;
    let _tarball_mock = server.mock("GET", "/tarball.tgz")
        .with_status(200).with_body(tarball).create_async().await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install", "@acme/cool-plugin"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array(), "install --json must emit an array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "@acme/cool-plugin");
    assert_eq!(arr[0]["version"], "1.0.0");
    assert_eq!(arr[0]["fresh"], true, "new package must be fresh:true");
}

#[tokio::test]
async fn test_install_json_multiple_packages_is_array() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let tarball_a = make_fake_tarball();
    let integrity_a = sha512_integrity(&tarball_a);
    let tarball_b = make_fake_tarball();
    let integrity_b = sha512_integrity(&tarball_b);

    let url_a = format!("{}/tarball_a.tgz", server.url());
    let url_b = format!("{}/tarball_b.tgz", server.url());

    let _ma = server.mock("GET", "/%40acme%2Fplugin-a")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(registry_meta("@acme/plugin-a", "1.0.0", &url_a, &integrity_a).to_string())
        .create_async().await;
    let _ta = server.mock("GET", "/tarball_a.tgz")
        .with_status(200).with_body(tarball_a).create_async().await;
    let _mb = server.mock("GET", "/%40acme%2Fplugin-b")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(registry_meta("@acme/plugin-b", "2.0.0", &url_b, &integrity_b).to_string())
        .create_async().await;
    let _tb = server.mock("GET", "/tarball_b.tgz")
        .with_status(200).with_body(tarball_b).create_async().await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install", "@acme/plugin-a", "@acme/plugin-b"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array(), "must be array for multiple packages");
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_install_json_already_locked_fresh_false() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    // Manifest declares the plugin
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
    ).unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins/cool-plugin")).unwrap();

    // Pre-populate uepm.lock so the plugin is already locked
    let mut lock = LockFile::default();
    lock.plugins.insert(
        "@acme/cool-plugin".to_string(),
        locked_plugin("1.0.0", &format!("{}/tarball.tgz", server.url())),
    );
    write_lockfile(dir.path(), &lock).unwrap();

    // The registry mock still needs to exist (resolver will check it for range satisfaction)
    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let tarball_url = format!("{}/tarball.tgz", server.url());
    let meta = registry_meta("@acme/cool-plugin", "1.0.0", &tarball_url, &integrity);
    let _mm = server.mock("GET", "/%40acme%2Fcool-plugin")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(meta.to_string()).create_async().await;
    let _tm = server.mock("GET", "/tarball.tgz")
        .with_status(200).with_body(tarball).create_async().await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["fresh"], false, "already-locked package must be fresh:false");
}
```

- [ ] **Step 2: Run the new install tests**

```
cargo test --test json_output test_install_json
```
Expected: all 3 tests pass.

- [ ] **Step 3: Run all json_output tests**

```
cargo test --test json_output
```
Expected: all 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/json_output.rs
git commit -m "test: JSON integration tests for install (single, multi, already-locked)"
```

---

## Task 13: JSON integration test — error shape

**Files:**
- Modify: `tests/json_output.rs`

- [ ] **Step 1: Add error shape test**

Add to the end of `tests/json_output.rs`:

```rust
// ── Error shape ───────────────────────────────────────────────────────────────

#[test]
fn test_error_emits_json_on_stdout_with_nonzero_exit() {
    let dir = tempdir().unwrap();
    // No Config/UEPM.ini — list will fail with a manifest parse error.

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "non-zero exit expected on error");
    assert!(output.stdout.len() > 0, "error must be on stdout, not stderr");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("error output must be valid JSON");
    assert!(json["error"].is_string(), "error JSON must have an 'error' string field");
    assert!(json.as_object().unwrap().len() == 1, "error JSON must have exactly one field");
}

#[test]
fn test_json_without_yes_on_init_exits_with_error_json() {
    let dir = tempdir().unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "init"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"].as_str().unwrap().contains("--yes"));
}
```

- [ ] **Step 2: Run the error tests**

```
cargo test --test json_output test_error
```
Expected: both tests pass.

- [ ] **Step 3: Run the full test suite**

```
cargo test
```
Expected: all tests pass, no regressions.

- [ ] **Step 4: Commit**

```bash
git add tests/json_output.rs
git commit -m "test: JSON error shape integration tests"
```

---

## Done

All commands now support `--json`. The next Phase 3 features (`uepm search`, `uepm info`, `uepm outdated`) will include `--json` from the start using the pattern established here.
