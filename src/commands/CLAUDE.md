# src/commands/ — Command Implementation Guide

Each command is its own module with a single `pub async fn run(ctx: &UEPMContext, ...) -> Result<(), UepmError>` entry point. `main.rs` dispatches via `match cli.command { ... }`.

## Command Signatures

```rust
// init.rs
pub async fn run(ctx: &UEPMContext, yes: bool) -> Result<(), UepmError>

// install.rs
pub async fn run(ctx: &UEPMContext, packages: Vec<String>) -> Result<(), UepmError>

// uninstall.rs
pub async fn run(ctx: &UEPMContext, package: String) -> Result<(), UepmError>

// update.rs
pub async fn run(ctx: &UEPMContext, package: Option<String>) -> Result<(), UepmError>

// list.rs
pub async fn run(ctx: &UEPMContext) -> Result<(), UepmError>

// publish.rs
pub async fn run(ctx: &UEPMContext, tag: &str, dry_run: bool, yes: bool, access: &str) -> Result<(), UepmError>
```

## Command Behaviors

### `init`
- Detects context: `.uplugin` present → plugin-init, `.uproject` present → project-init.
- Plugin init: calls `find_uplugin`, runs `run_plugin_init` which prompts for `[Plugin]` fields (or uses `--yes` to skip prompts → `InteractiveRequired` if not a TTY and `--yes` not passed).
- Writes VCS ignore entries (`.gitignore` / `.p4ignore` / `.hgignore`) for `UEPMPlugins/` if not committed.
- Creates `Config/UEPM.ini` + `UEPMPlugins/` directory.

### `install`
- Empty `packages` → installs all from `Config/UEPM.ini` (lockfile-pinned).
- Non-empty → parses each as `@scope/name@range` or `local:./path`.
- Builds a `ResolveContext`, calls `resolve_and_install` for each spec.
- Writes updated manifest + lockfile on success.

### `uninstall`
- Removes `UEPMPlugins/<name>/` (uses `plugin_dir_name` to derive dir from package spec).
- Removes entry from `[Dependencies]` in manifest.
- Does NOT rewrite lockfile (stale entry is harmless; next install will reconcile).

### `update`
- With `package`: re-resolves just that one, ignoring lockfile pin.
- Without `package`: re-resolves all `[Dependencies]`, ignoring lockfile entirely.
- Rewrites `uepm.lock` with fresh versions.

### `list`
- Reads manifest + lockfile; for each locked plugin prints version and whether it's compatible with the project's `EngineVersion`.
- No network calls.

### `publish`
- Validates all `[Plugin]` fields are non-empty and well-formed (semver version, valid engine range).
- `dry_run: true` → calls `list_files` + `build_tarball` but skips HTTP PUT.
- Auth: uses `UEPM_TOKEN`; if 401 response with OTP required header, prompts for OTP and retries once.
- Computes both SHA1 (npm expects it in the manifest) and SHA512 (integrity field).

## Adding a New Command

1. Create `src/commands/<name>.rs` with `pub async fn run(ctx: &UEPMContext, ...) -> Result<(), UepmError>`.
2. Add `pub mod <name>;` to `src/commands/mod.rs`.
3. Add a `Commands::<Name>` variant to the `Commands` enum in `src/main.rs`.
4. Add the match arm in `main.rs`.
5. Add integration test in `tests/<name>_integration.rs`.

## Common Patterns

```rust
// Read manifest — always the first step
let manifest = read_manifest(&ctx.project_dir)?;

// Build resolver state
let mut lock = LockFile::load(&ctx.project_dir)?;
let mut resolved: HashMap<String, String> = HashMap::new();
let mut rctx = ResolveContext::new(ctx, &mut lock, &mut resolved);

// User-facing output
print_success("Installed @acme/plugin@1.0.0");
print_info("Resolving dependencies...");

// Write back
write_manifest(&ctx.project_dir, &manifest)?;
lock.save(&ctx.project_dir)?;
```
