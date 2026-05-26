# Phase 2 — Plugin Authoring: Implementation Plan

## Overview

Phase 2 turns UEPM into an end-to-end tool for plugin **authors**, not just consumers.
Three capabilities are added in sequence:

1. **`[Package]` section** in `Config/UEPM.ini` — metadata that travels with the plugin source
2. **`uepm init` (plugin context)** — detects `.uplugin`, prompts for metadata, writes `[Package]`
3. **`uepm publish`** — validates, builds a `.tgz` tarball in memory, and PUTs directly to the npm registry over HTTP (no Node.js or npm required)

Engine version detection (macOS/Windows UE install scan) is a bonus that pre-fills prompts.

---

## Milestone 1 — `[Package]` section in `Config/UEPM.ini`

### 1.1 Data model (`src/manifest.rs`)

Add a `PackageMetadata` struct and thread it through `ProjectManifest`:

```toml
# Config/UEPM.ini (plugin author's copy)
[Package]
Name       = "@acme/my-plugin"
Version    = "1.2.0"
Description = "Does cool things"
Author     = "ACME Studio"
License    = "MIT"
EngineRange = ">=5.3.0 <6.0.0"
Main       = "MyPlugin.uplugin"

[Plugins]
"@acme/dep-plugin" = "^2.0.0"
```

**`PackageMetadata` fields:**

| Field | Type | Required for publish | Notes |
|---|---|---|---|
| `name` | `String` | ✅ | Must be scoped `@scope/name` |
| `version` | `String` | ✅ | SemVer |
| `description` | `String` | ✅ | |
| `author` | `String` | ✅ | |
| `license` | `String` | ✅ | SPDX string |
| `engine_range` | `String` | ✅ | SemVer range, e.g. `>=5.3.0 <6.0.0` |
| `main` | `String` | ✅ | `.uplugin` filename, e.g. `MyPlugin.uplugin` |

**Changes to `manifest.rs`:**

```rust
#[derive(Debug, Default, Clone)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub engine_range: String,
    pub main: String,           // e.g. "MyPlugin.uplugin"
}

#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub plugins: HashMap<String, String>,
    pub engine_version: Option<String>,
    pub commit_plugins: bool,
    pub package: Option<PackageMetadata>,   // new
}
```

Private TOML repr mirrors this: `[Package]` section → `TomlPackage` struct with `skip_serializing_if = "Option::is_none"` at the `TomlManifest` level so project manifests (no `[Package]`) are not cluttered.

Add helpers:
- `read_package_metadata(dir) -> Result<PackageMetadata, UepmError>` — returns `Err(UepmError::NoPackageMetadata)` if section absent
- `write_package_metadata(dir, meta)` — round-trips only the `[Package]` section
- New error variant: `UepmError::NoPackageMetadata`
- New error variant: `UepmError::InvalidPackageField { field, message }`

**Tests:**
- Round-trip `[Package]` section survives write → read
- Missing `[Package]` returns correct error
- Manifest with only `[Plugins]` is unaffected

---

## Milestone 2 — `uepm init` in plugin context

### 2.1 Context detection

Current `init::run` requires a `.uproject` file. Add a parallel path:

```
If .uplugin found in cwd → run plugin-context init
If .uproject found in cwd → run project-context init (existing)
Else → error
```

Detection order:
1. Scan `cwd` for `*.uplugin` files (like `find_uproject` does for `.uproject`)
2. If found AND `[Package]` already populated → confirm overwrite prompt
3. If not found → existing project flow

### 2.2 Prompt flow (`commands/init.rs`)

New private function `run_plugin_init(plugin_dir, uplugin_path, yes)`:

**Defaults derived from `.uplugin`:**

| Prompt | Default source |
|---|---|
| Package name | `@<to-be-filled>/` + `FriendlyName` slugified (`"My Plugin"` → `my-plugin`) |
| Version | `VersionName` from `.uplugin` (fallback `"1.0.0"`) |
| Description | `Description` field from `.uplugin` |
| Author | `CreatedBy` field from `.uplugin` |
| License | `"MIT"` (hardcoded default) |
| Engine range | Detected from installed UE builds (Milestone 2.3), or prompted |
| Main | `<PluginName>.uplugin` filename |

**Prompt implementation** (dialoguer `Input<String>` for each field, `--yes` skips all prompts and uses defaults):

```rust
fn prompt_or_default(prompt: &str, default: &str, yes: bool) -> Result<String, UepmError>
```

On `--yes`, validate that auto-derived defaults are non-empty; error if any required field is blank.

### 2.3 Engine version detection (bonus)

New module `src/ue_install.rs`:

```rust
pub fn find_installed_engines() -> Vec<(String, PathBuf)>  // (version_string, install_path)
```

**macOS:** Read `~/Library/Application Support/Epic/UnrealEngineLauncher/LauncherInstalled.dat` (JSON array of `{ "InstallLocation": "...", "AppVersion": "..." }`) and filter entries where `AppVersion` starts with `UE_`.

**Windows:** Read `HKEY_LOCAL_MACHINE\SOFTWARE\EpicGames\Unreal Engine\<version>` keys (use `winreg` crate, cfg-gated).

Return sorted list of found versions. If exactly one installed, use it as default. If multiple, present a `dialoguer::Select`. If none, fall back to a plain `Input` prompt.

Add `winreg` as optional dependency:
```toml
[target.'cfg(windows)'.dependencies]
winreg = "0.52"
```

### 2.4 `init::run` routing

```rust
pub async fn run(ctx: &UEPMContext, yes: bool) -> Result<(), UepmError> {
    if let Some(uplugin) = find_uplugin(&ctx.project_dir) {
        run_plugin_init(&ctx.project_dir, &uplugin, yes).await
    } else {
        let commit = select_commit_plugins(&ctx.project_dir, yes)?;
        run_init_with_commit(&ctx.project_dir, commit).await
    }
}
```

**Tests:**
- `run_plugin_init` with `--yes` writes correct `[Package]` from `.uplugin` fields
- Prompts pre-filled with `.uplugin` defaults
- Existing `[Package]` preserved unless user confirms overwrite
- `find_installed_engines` returns empty vec when launcher file absent (no panic)

---

## Milestone 3 — `uepm publish` (direct HTTP, no npm required)

> **Design note:** bun recently rewrote their publish command in Rust
> (`src/runtime/cli/publish_command.rs`, MIT licensed). The code is not
> copy-pasteable — it's coupled to ~20 internal bun crates — but reading it
> gives us the exact wire protocol. We implement it ourselves using deps
> UEPM already carries (`reqwest`, `tar`, `flate2`, `sha2`, `base64`,
> `serde_json`). One new dep: `ignore` (the crate backing `ripgrep`) for
> `.gitignore` / `.npmignore` file-walking.

### 3.0 New dep

```toml
ignore = "0.4"   # gitignore/npmignore-aware directory walker
```

### 3.1 CLI addition (`src/main.rs`)

```rust
/// Publish this plugin to the npm registry
Publish {
    /// npm tag (default: "latest")
    #[arg(long, default_value = "latest")]
    tag: String,
    /// Validate and build tarball but do not upload
    #[arg(long)]
    dry_run: bool,
    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
    /// npm access level ("public" or "restricted")
    #[arg(long, default_value = "public")]
    access: String,
},
```

### 3.2 New module `src/publisher.rs` — tarball builder

Self-contained, no registry knowledge. Input: a directory + a files list.
Output: `Vec<u8>` (raw `.tgz` bytes).

#### File inclusion rules (matching npm/bun behavior)

1. If `[Package]` has a `Files` list → use it as an allowlist of globs
2. Otherwise use `ignore` crate walker with `.npmignore` (preferred) or
   `.gitignore` as the exclusion source
3. **Always include** regardless of ignore rules: `*.uplugin`, `package.json`,
   `README*`, `LICENSE*`, `CHANGELOG*`
4. **Always exclude**: `node_modules/`, `.git/`, `UEPMPlugins/`,
   `Config/UEPM.ini`, `*.lock`

#### Tarball layout

Every entry prefixed `package/` (npm convention):
```
package/MyPlugin.uplugin
package/Source/MyPlugin/...
package/Resources/Icon128.png
package/package.json          ← ephemeral, written by us
```

```rust
pub fn build_tarball(plugin_dir: &Path, package_json: &[u8]) -> Result<Vec<u8>, UepmError>
```

- Walk with `ignore::WalkBuilder`, apply inclusion rules
- Build tar archive via `tar::Builder` writing into a `Vec<u8>` wrapped in
  `flate2::write::GzEncoder` (level 9, matching npm)
- Inject `package/package.json` from the `package_json` argument (not from
  disk — keeps the directory clean)
- Return raw compressed bytes

### 3.3 New module `src/commands/publish.rs`

#### Step 1 — Load and validate `[Package]`

- `read_package_metadata(&ctx.project_dir)` → error if absent
- Validate: all required fields non-empty, `name` matches `@scope/name`,
  `version` valid SemVer, `engine_range` parses, `main` file exists on disk

#### Step 2 — Show publish summary + confirm

```
  Publishing @acme/my-plugin@1.2.0
  Engine range : >=5.3.0 <6.0.0
  Main         : MyPlugin.uplugin
  Registry     : https://registry.npmjs.org
  Tag          : latest
  Access       : public

  Continue? [y/N]
```

Skip with `--yes` or `--dry-run`.

#### Step 3 — Build in-memory `package.json`

```rust
fn build_package_json(meta: &PackageMetadata) -> serde_json::Value
```

```json
{
  "name": "@acme/my-plugin",
  "version": "1.2.0",
  "description": "Does cool things",
  "main": "MyPlugin.uplugin",
  "author": "ACME Studio",
  "license": "MIT",
  "unreal": {
    "engineVersion": ">=5.3.0 <6.0.0",
    "pluginName": "MyPlugin"
  },
  "keywords": ["unreal", "unreal-engine", "plugin", "uepm"]
}
```

`pluginName` = `meta.main` with `.uplugin` stripped. No `files` field needed
— we control what goes in the tarball directly.

This JSON is **never written to disk**. It's kept as `Vec<u8>` and injected
into the tarball as `package/package.json` in memory.

#### Step 4 — Build tarball + compute digests

```rust
let tarball: Vec<u8> = publisher::build_tarball(&ctx.project_dir, &pkg_json_bytes)?;

// SHA1 for legacy `dist.shasum` — add `sha1 = "0.10"` to Cargo.toml
use sha1::{Sha1, Digest as _};
let shasum: String = format!("{:x}", Sha1::digest(&tarball));

// SHA512 for `dist.integrity` — sha2 already in Cargo.toml
use sha2::{Sha512, Digest as _};
let integrity: String = format!(
    "sha512-{}",
    base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&tarball))
);
```

Note: `sha1 = "0.10"` needs adding to `Cargo.toml` (same family as `sha2`, ~10KB).

If `--dry-run`: print summary, show file list, exit here — no upload.

#### Step 5 — Construct and PUT publish request

Exact JSON body structure (derived from bun's `construct_publish_request_body`):

```json
{
  "_id": "@acme/my-plugin",
  "name": "@acme/my-plugin",
  "dist-tags": { "latest": "1.2.0" },
  "versions": {
    "1.2.0": {
      ...package.json fields...,
      "dist": {
        "integrity": "sha512-<base64>",
        "shasum": "<sha1-hex>",
        "tarball": "<registry>/@acme/my-plugin/-/my-plugin-1.2.0.tgz"
      }
    }
  },
  "access": "public",
  "_attachments": {
    "my-plugin-1.2.0.tgz": {
      "content_type": "application/octet-stream",
      "data": "<base64-encoded tarball>",
      "length": <byte_length>
    }
  }
}
```

The tarball attachment filename uses the **unscoped** package name
(`my-plugin-1.2.0.tgz`, not `@acme-my-plugin-1.2.0.tgz`) — matching npm.

```rust
async fn upload(registry: &str, body: &[u8], token: &str) -> Result<(), UepmError> {
    let url = format!("{}/{}", registry, urlencoding::encode(name));
    client
        .put(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .body(body.to_vec())
        .send()
        .await?;
    // check status, parse error JSON on non-2xx
}
```

#### Step 6 — 401 / OTP retry

If response is 401 and `WWW-Authenticate` contains `"otp"` or body contains
`"one-time pass"`: prompt for OTP via `dialoguer::Input`, add
`npm-otp: <value>` header, retry once. Otherwise surface the registry's
error message from the JSON body (`{"error": "..."}`).

New error variants:
```rust
UepmError::NoPackageMetadata,
UepmError::InvalidPackageField { field: String, message: String },
UepmError::PublishFailed(String),   // registry error message
UepmError::TokenRequired,           // UEPM_TOKEN not set
```

**Tests:**

- `build_tarball` produces a valid `.tgz` with all entries prefixed `package/`
  and `package/package.json` injected
- `build_package_json` output matches expected shape
- Validation rejects missing `name`, invalid SemVer, missing `main` file
- `upload` PUT body structure matches expected JSON (mockito)
- Dry-run exits before any HTTP call
- 401+OTP flow: mockito returns 401 with `otp` header, assert retry with
  `npm-otp` header (non-interactive: feed OTP via stdin mock)

---

## Milestone 4 — README + ROADMAP updates

- Add `uepm init` plugin-context description to **Quick Start**
- Add `uepm publish` to the **Commands** table
- Add `[Package]` section to **Publishing plugins**, replacing the manual `package.json` example
- Move Phase 2 items from `[ ]` to `[x]` in `ROADMAP.md`
- Update `CLAUDE.md` architecture table to include `commands/publish`

---

## Implementation order

```
M1  src/manifest.rs              [Package] struct + TOML round-trip + tests
M2a src/ue_install.rs            find_installed_engines() + tests
M2b src/commands/init.rs         uplugin detection + plugin-context prompts + tests
M3a src/errors.rs                new error variants
M3b src/commands/publish.rs      full publish flow + tests
M3c src/main.rs                  wire Publish command
M4  README.md, ROADMAP.md, CLAUDE.md
```

Each milestone should compile and pass `cargo test` before moving to the next.

---

## Open questions

1. **Registry target** — `UEPM_REGISTRY` already controls installs; should it also control `publish`? Almost certainly yes, so private/enterprise registries work end-to-end.
2. **`Files` override in `[Package]`** — should authors be able to specify an explicit `Files = ["MyPlugin.uplugin", "Source/**/*", "Shaders/**/*"]` in `Config/UEPM.ini` to override the auto-detected list?
3. **SHA1 dep** — `sha2` crate does SHA-256/512 but not SHA-1. Options: add `sha1` crate (tiny), use the `sha2` crate's `Sha1` re-export if available, or just omit `shasum` (some registries accept integrity-only). npm still validates it, so we should include it.
4. **Engine version detection scope** — macOS-only for v2.0, or also attempt Windows `winreg` scan?
5. **`ignore` crate vs. manual glob** — the `ignore` crate is the right call for correct `.gitignore` semantics, but it's ~200KB compiled. Alternative: hand-roll a simpler glob matcher scoped to npm's actual rule set (which is much simpler than full gitignore). Worth deciding before starting M3.
