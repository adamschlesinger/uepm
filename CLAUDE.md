# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # debug build
cargo build --release                # release binary → target/release/uepm
cargo test                           # all tests
cargo test registry                  # tests matching "registry" (module filter)
cargo test install_integration       # integration test in tests/
```

## Architecture

UEPM is a standalone Rust binary that manages Unreal Engine plugins via the npm registry. No Node.js required at runtime.

**Entry points:**
- `src/main.rs` — clap CLI, tokio main, dotenvy + tracing init
- `src/lib.rs` — re-exports all modules for integration testing

**Core flow:** `uepm init` → creates `uepm.ini` + `UEPMPlugins/` → `uepm install @scope/pkg` → registry fetch → sha512-verified tarball extract → `uepm.lock` written.

### Modules

| Module | Purpose |
|---|---|
| `manifest` | Read/write `uepm.ini` via `configparser`. `[settings]` + `[plugins]` sections. |
| `lockfile` | Read/write `uepm.lock` (JSON). Reproducible installs via locked tarballs. |
| `uproject` | Find, read, and modify `.uproject` JSON (plugin directory injection). |
| `registry` | `RegistryClient` — fetch npm package metadata, semver range resolution. |
| `installer` | Download tarball, verify sha512 integrity, extract stripping `package/` prefix. |
| `resolver` | Recursive install with conflict detection. Reads plugin's own `uepm.ini` for transitive deps. |
| `output` | crossterm-colored `print_success` / `print_warn` / `print_error` / `print_info`. |
| `errors` | `UepmError` enum (thiserror derives). |
| `commands/init` | VCS detection (`P4PORT`, `.p4config`, `.git`), `InstallMode` enum, dialoguer select. |
| `commands/install` | Parse `@scope/pkg@ver` specs, resolve+install, update `uepm.ini` + `uepm.lock`. |
| `commands/uninstall` | Remove `UEPMPlugins/<name>/`, update `uepm.ini`. |
| `commands/update` | Re-resolve all/one plugin ignoring lockfile, rewrite `uepm.lock`. |
| `commands/list` | Read manifest + lockfile, print compatibility status. |

### Key files

```
Cargo.toml              — single crate, lib + bin targets
src/main.rs             — binary (clap Commands enum matches to commands::*)
src/lib.rs              — pub mod declarations
tests/install_integration.rs  — end-to-end test using mockito + tempfile
install.sh              — Unix install script (curl | sh)
install.ps1             — Windows install script (irm | iex)
.github/workflows/release.yml  — cross-compile on tag push, creates GitHub Release
```

### Testing

- Unit tests live in each module (`#[cfg(test)] mod tests`)
- `tests/install_integration.rs` is the integration test — uses `mockito::Server` for a fake npm registry and `tempfile::tempdir` for isolation
- `UEPM_REGISTRY` env var overrides the registry URL in tests
