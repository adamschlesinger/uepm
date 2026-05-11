# UEPM Core CLI — Rust Rewrite Design

**Date:** 2026-05-10
**Branch:** rust-rewrite
**Status:** Approved

## Overview

UEPM is being rewritten as a standalone Rust binary distributed via a `curl | sh` install script. The TypeScript packages (`@uepm/init`, `@uepm/postinstall`, `@uepm/core`) remain on `main` as the 1.x release line. This spec covers the core CLI only. Plugin authoring (`uepm init` in a plugin directory, `uepm publish`) is phase 2.

**Why Rust:**
- Truly standalone — no Node.js, npm, or runtime prerequisites on end-user machines
- Cross-compiles cleanly to Windows, macOS, and Linux
- Strong async HTTP, tarball, and filesystem libraries available

**Distribution:** Plugin packages continue to be published to the npm registry. The UEPM CLI fetches from the npm registry API directly — no npm CLI required.

## Command Surface

```
uepm init                         Create uepm.ini, add UEPMPlugins to .uproject
uepm install                      Install all plugins listed in uepm.ini
uepm install @scope/plugin        Add to uepm.ini and install
uepm install @scope/plugin@1.2.0  Install a specific version
uepm uninstall @scope/plugin      Remove from uepm.ini and delete from UEPMPlugins/
uepm update                       Update all plugins to latest compatible versions
uepm update @scope/plugin         Update one plugin
uepm list                         Show installed plugins, resolved versions, and engine compatibility
```

## File Formats

### `uepm.ini` (human-edited, checked into VCS)

```ini
[plugins]
@acme/cool-plugin = ^1.0.0
@studio/other-plugin = ~2.1.0

[settings]
engine_version = 5.7
```

`engine_version` is written by `uepm init` (read from the `.uproject` `EngineAssociation` field) and used for compatibility validation on every command. Both files live at the project root alongside the `.uproject` file.

### `uepm.lock` (machine-generated JSON, checked into VCS)

```json
{
  "version": 1,
  "plugins": {
    "@acme/cool-plugin": {
      "resolved": "1.0.3",
      "tarball": "https://registry.npmjs.org/@acme/cool-plugin/-/cool-plugin-1.0.3.tgz",
      "sha512": "abc123...",
      "dependencies": {
        "@acme/base-plugin": "1.0.1"
      }
    }
  }
}
```

`uepm.lock` is the source of truth for reproducible installs. `uepm install` (no args) installs exactly what the lockfile specifies; `uepm install @scope/plugin` resolves and updates the lockfile.

## Install Flow

`uepm install @scope/plugin` runs these steps in order:

1. **Resolve version** — fetch package metadata from `https://registry.npmjs.org/@scope/plugin` and find the latest version satisfying the semver range in `uepm.ini`
2. **Check lock** — if `uepm.lock` already has this plugin at the resolved version with a matching checksum, skip download
3. **Download tarball** — fetch from the `dist.tarball` URL in the registry metadata
4. **Verify checksum** — sha512 must match `dist.shasum` from registry metadata; abort on mismatch
5. **Extract** — unpack directly to `UEPMPlugins/<plugin-name>/` where `<plugin-name>` is the part after `/` in the scoped package name (e.g. `@acme/cool-plugin` → `UEPMPlugins/cool-plugin/`)
6. **Resolve dependencies** — read the installed plugin's `uepm.ini` if present; recursively install any declared deps not already in `UEPMPlugins/`; if a version conflict is detected (same package required at incompatible versions by two different plugins), abort and print the full conflict chain with a suggestion to pin a version in the user's `uepm.ini`
7. **Update lockfile** — write resolved version, tarball URL, checksum, and resolved dep versions to `uepm.lock`
8. **Update `uepm.ini`** — add the package and range if not already present

`uepm install` (no args) reads `uepm.ini` and runs steps 2–7 for each listed plugin. If `uepm.lock` exists, locked versions are used for reproducibility. If no lockfile exists (first run), full resolution runs and the lockfile is created.

### `uepm init` flow

1. Find `.uproject` in current directory
2. Read `EngineAssociation` field; if it is a GUID (launcher-installed engine), warn that `engine_version` could not be determined and leave it blank in `uepm.ini`
3. Add `UEPMPlugins` to `AdditionalPluginDirectories` in `.uproject` if not present
4. Create `uepm.ini` with empty `[plugins]` and `engine_version` from step 2 (omitted if GUID)
5. Create `UEPMPlugins/` directory if not present

## Codebase Structure

Single binary crate at `uepm/` in the repo root.

```
uepm/
  Cargo.toml
  src/
    main.rs              — CLI entry point (clap)
    commands/
      init.rs
      install.rs
      uninstall.rs
      update.rs
      list.rs
    registry.rs          — npm registry HTTP client (reqwest)
    resolver.rs          — recursive dependency resolution with conflict detection
    installer.rs         — tarball download, checksum verify, extract (tar + flate2)
    manifest.rs          — uepm.ini read/write (configparser)
    lockfile.rs          — uepm.lock read/write (serde_json)
    uproject.rs          — .uproject JSON read/write
    errors.rs            — typed UepmError enum
```

### Dependencies

| Crate | Purpose |
|---|---|
| `clap` | CLI argument parsing |
| `reqwest` | Async HTTP for registry and tarball downloads |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Lockfile serialisation |
| `configparser` | INI parsing for `uepm.ini` |
| `tar` + `flate2` | Tarball extraction |
| `semver` | Version range matching |
| `tracing` + `tracing-subscriber` | Structured logging (`RUST_LOG` env var) |
| `crossterm` | Cross-platform terminal styling |
| `dotenvy` | `.env` file loading for `UEPM_REGISTRY`, `UEPM_TOKEN`, `RUST_LOG` |

### Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `UEPM_REGISTRY` | `https://registry.npmjs.org` | Registry URL (override for private registries) |
| `UEPM_TOKEN` | — | Auth token for private registries |
| `RUST_LOG` | `warn` | `tracing-subscriber` log level |

## Build & Release Pipeline

GitHub Actions builds on every tag push (`v*`), producing four binaries:

| Target | Artifact |
|---|---|
| `x86_64-unknown-linux-gnu` | `uepm-linux-x86_64` |
| `x86_64-pc-windows-msvc` | `uepm-windows-x86_64.exe` |
| `aarch64-apple-darwin` | `uepm-macos-arm64` |
| `x86_64-apple-darwin` | `uepm-macos-x86_64` |

Binaries are attached to the GitHub Release. Install scripts are hosted on GitHub Pages at `uepm.dev`.

**`install.sh` (Unix):**
1. Detect OS and architecture
2. Download matching binary from the latest GitHub Release
3. Install to `~/.uepm/bin/`
4. Append `~/.uepm/bin` to PATH in `~/.bashrc` / `~/.zshrc`

**`install.ps1` (Windows):**
1. Detect architecture
2. Download `.exe` from latest GitHub Release
3. Install to `%LOCALAPPDATA%\uepm\bin\`
4. Add to user PATH via registry

## Error Handling

`UepmError` is a typed enum with variants for:
- Registry unreachable / HTTP error
- Package not found
- Checksum mismatch (with expected vs actual)
- Version conflict (with full conflict chain and suggestion to pin)
- `.uproject` not found
- `uepm.ini` parse failure
- Filesystem permission denied

All errors print with `crossterm`-styled output and a suggestion where applicable. `tracing` logs full detail at `DEBUG` level.

## Testing

- **Unit tests** — `manifest.rs`, `lockfile.rs`, `resolver.rs`, `uproject.rs`: pure logic, no I/O
- **Integration tests** — a local mock HTTP server (`mockito` or `axum`) serving the npm registry API and tarballs; the existing sample plugins (`samples/plugins/example-plugin`, `samples/plugins/dependency-plugin`) serve as fixtures
- **End-to-end** — `uepm install` + `uepm list` + `uepm uninstall` against the mock registry in a temp directory

## Roadmap (Out of Scope for This Spec)

- **Full semver dependency resolution** — npm-style full graph resolution with backtracking (currently: recursive install with explicit conflict detection)
- **Package manager distribution** — Homebrew tap, Chocolatey package, Scoop manifest once the binary is stable
- **Plugin authoring** — `uepm init` in plugin directory, `uepm publish` wrapping npm publish
