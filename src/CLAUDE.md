# src/ — Source Module Guide

All modules are declared in `lib.rs` and re-exported for integration testing. The binary entry point is `main.rs`.

## Module Responsibilities

### `context.rs` — Runtime Singleton
`UEPMContext` is built once in `main.rs` and passed by reference into every command handler.

```rust
pub struct UEPMContext {
    pub project_dir: PathBuf,        // cwd at startup
    pub uepm_plugins_dir: PathBuf,   // <project_dir>/UEPMPlugins
    pub registry: RegistryClient,
    pub token: Option<String>,       // from UEPM_TOKEN env var
}
```

Constructors: `UEPMContext::new()` (env vars), `UEPMContext::with_dir(dir)`, `UEPMContext::for_test(dir, url, token)`.

### `manifest.rs` — Config/UEPM.ini
TOML file with three sections (note: TOML section names differ from Rust field names):

| TOML Section | Rust Field | Purpose |
|---|---|---|
| `[Settings]` | `settings` | `EngineVersion`, `CommitPlugins` |
| `[Plugin]` | `package: Option<PackageMetadata>` | Plugin authoring metadata (publish-only) |
| `[Dependencies]` | `plugins: HashMap<String,String>` | `"@scope/name" = "^1.0.0"` |

Key functions: `read_manifest(dir)`, `write_manifest(dir, manifest)`, `write_package_metadata(dir, meta)`.

### `lockfile.rs` — uepm.lock
JSON file: `{ "plugins": { "@scope/name": { "version": "1.0.0", "tarball": "...", "integrity": "sha512-..." } } }`.

Key types: `LockFile`, `LockedPlugin`. Read with `LockFile::load(dir)`, mutate in-place, write with `lock.save(dir)`.

### `registry.rs` — npm Registry Client
`RegistryClient::fetch_package(name)` → `HashMap<String, PackageMetadata>` (all versions).
`RegistryClient::resolve_version(name, range)` → picks highest version satisfying the semver range, preferring `dist-tags.latest`.

Reads `UEPM_REGISTRY` env var (default `https://registry.npmjs.org`).

### `installer.rs` — Tarball Download + Extract
`download_and_extract(url, integrity, pkg_name, plugins_dir, token)`:
1. GET tarball with optional bearer token
2. Verify `sha512-<base64>` integrity string
3. Strip `package/` path prefix while extracting into `UEPMPlugins/<plugin-name>/`

`symlink_local(src_dir, plugins_dir, plugin_name)` — for `local:./path` installs (creates a symlink instead of copying).

### `resolver.rs` — Recursive Dependency Resolution
`ResolveContext<'a>` borrows from `UEPMContext` and owns per-session mutable state (`LockFile`, resolved map).

`resolve_and_install(spec, range, rctx)` — fetches, installs, then reads the installed plugin's `Config/UEPM.ini` to recurse into its `[Dependencies]`. Conflict detection: if a package was already resolved to a different version that doesn't satisfy the new range, returns `UepmError::VersionConflict`.

Helper: `plugin_dir_name("@scope/name")` → `"name"` (used everywhere a filesystem dir name is needed).

### `publisher.rs` — Tarball Builder (Publish)
`build_tarball(dir, pkg_json_bytes)` → `Vec<u8>` — walks the plugin directory (respecting `.npmignore` / `.gitignore`), prepends `package/` to all paths, and gzip-compresses into a `.tgz`.

`list_files(dir)` — dry-run preview of what would be included.

### `uproject.rs` — .uproject Manipulation
`find_uproject(dir)` — searches current dir for `*.uproject`. 
`add_plugin_to_uproject(path, plugin_name)` — injects `{"Name": "...", "Enabled": true}` into the `Plugins` array.

### `ue_install.rs` — Installed Engine Discovery
`find_installed_engines()` → `Vec<InstalledEngine>` — reads `LauncherInstalled.dat` on macOS/Linux; reads Windows registry on Windows (cfg-gated with `winreg`).

### `errors.rs` — Error Types
`UepmError` covers: `Registry`, `PackageNotFound`, `ChecksumMismatch`, `VersionConflict`, `UprojectNotFound`, `ManifestParse`, `Io`, `Json`, `InvalidSemver`, `NoMatchingVersion`, `InteractiveRequired`, `NoPackageMetadata`, `InvalidPackageField`, `PublishFailed`, `TokenRequired`.

### `output.rs` — Colored Terminal Output
Four functions: `print_success` (green ✓), `print_warn` (yellow ⚠), `print_error` (red ✗), `print_info` (cyan). All write to stdout via `crossterm`.

## Cross-Cutting Patterns

- **No global state** — everything flows through `UEPMContext` or explicit parameters.
- **`tracing` not `println!`** for debug output; user-facing output via `output::print_*`.
- **`UepmError` for all fallible operations** — use `?` propagation, never `.unwrap()` in library code.
- **`async fn` everywhere** that touches the network (`registry`, `installer`, all commands).
