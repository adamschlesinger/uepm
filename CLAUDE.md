# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # debug build
cargo build --release                # release binary → target/release/uepm
cargo test                           # all tests
cargo test registry                  # tests matching "registry" (module filter)
cargo test install_integration       # integration test in tests/
cargo clippy -- -D warnings          # lint (CI enforced)
cargo fmt                            # format
RUST_LOG=debug uepm install …        # enable tracing output
```

## Architecture

UEPM is a standalone Rust binary that manages Unreal Engine plugins via the npm registry. No Node.js required at runtime.

**Entry points:**
- `src/main.rs` — clap CLI, tokio main, dotenvy + tracing init
- `src/lib.rs` — re-exports all modules for integration testing

**Core flow:** `uepm init` → creates `Config/UEPM.ini` + `UEPMPlugins/` → `uepm install @scope/pkg` → registry fetch → sha512-verified tarball extract → `uepm.lock` written.

### Modules

| Module | Purpose |
|---|---|
| `context` | `UEPMContext` — constructed once at startup, passed to every command. Holds `project_dir`, `uepm_plugins_dir`, `RegistryClient`, and optional `token`. |
| `manifest` | Read/write `Config/UEPM.ini` via `toml` + serde. Sections: `[Settings]`, `[Plugin]`, `[Dependencies]`. Exposes `ProjectManifest` + `PackageMetadata`. |
| `lockfile` | Read/write `uepm.lock` (JSON). Reproducible installs via locked tarballs. |
| `uproject` | Find, read, and modify `.uproject` JSON (plugin directory injection). |
| `registry` | `RegistryClient` — fetch npm package metadata, semver range resolution. |
| `installer` | Download tarball, verify sha512 integrity, extract stripping `package/` prefix. `symlink_local` for `local:./path` installs. |
| `resolver` | `ResolveContext` + recursive install with conflict detection. Reads plugin's own `Config/UEPM.ini` for transitive deps. |
| `output` | crossterm-colored `print_success` / `print_warn` / `print_error` / `print_info`. |
| `errors` | `UepmError` enum (thiserror derives). |
| `ue_install` | `find_installed_engines()` — scans Epic `LauncherInstalled.dat` (macOS/Linux) and Windows registry (cfg-gated) for installed UE builds. |
| `publisher` | Tarball builder: `build_tarball(dir, pkg_json_bytes)` → raw `.tgz` bytes. `list_files(dir)` for dry-run. |
| `commands/init` | Routes to plugin-context init (`.uplugin` found) or project-context init (`.uproject` found). Plugin path: `find_uplugin`, `run_plugin_init`. VCS detection + ignore file writes. |
| `commands/install` | Parse `@scope/pkg@ver` or `local:./path` specs, resolve+install, update `Config/UEPM.ini` + `uepm.lock`. |
| `commands/uninstall` | Remove `UEPMPlugins/<name>/`, update `Config/UEPM.ini`. |
| `commands/update` | Re-resolve all/one plugin ignoring lockfile, rewrite `uepm.lock`. |
| `commands/list` | Read manifest + lockfile, print compatibility status. |
| `commands/publish` | Validate `[Plugin]` section, build `.tgz` in memory, compute SHA1+SHA512, PUT to registry. OTP retry on 401. |

### Key files

```
Cargo.toml                        — single crate, lib + bin targets
src/main.rs                       — binary (clap Commands enum matches to commands::*)
src/lib.rs                        — pub mod declarations
src/context.rs                    — UEPMContext: runtime singleton injected into every command
tests/install_integration.rs      — end-to-end install test (mockito + tempfile)
tests/publish_integration.rs      — end-to-end publish test (mockito + tempfile)
install.sh                        — Unix install script (curl | sh)
install.ps1                       — Windows install script (irm | iex)
.github/workflows/release.yml     — cross-compile on tag push, creates GitHub Release
.github/workflows/test.yml        — CI: cargo test + cargo clippy on every PR
```

### Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `UEPM_REGISTRY` | `https://registry.npmjs.org` | Override registry URL (used in tests via `mockito`) |
| `UEPM_TOKEN` | — | Bearer token for private registries + publish auth |
| `RUST_LOG` | — | Tracing verbosity, e.g. `debug`, `uepm=trace` |

### Testing

- Unit tests live in each module (`#[cfg(test)] mod tests`)
- `tests/install_integration.rs` is the integration test — uses `mockito::Server` for a fake npm registry and `tempfile::tempdir` for isolation
- `UEPM_REGISTRY` env var overrides the registry URL in tests

## Before Every Commit

Re-evaluate whether the README needs updating to reflect the changes being committed — especially the Commands table, Project files section, and any env var or install flow changes.
