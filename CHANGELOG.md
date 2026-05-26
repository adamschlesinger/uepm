# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

v2.0.0 is a complete rewrite of UEPM in Rust. The binary is fully self-contained —
no Node.js runtime is required for plugin consumers or authors. All registry
communication is handled directly over HTTPS using the npm registry protocol.

### Breaking Changes

- **`Config/UEPM.ini` section `[Package]` renamed to `[Plugin]`** — update any
  existing plugin manifests that declare a `[Package]` section.
- **`Config/UEPM.ini` section `[Plugins]` renamed to `[Dependencies]`** — update
  any existing project or plugin manifests that declare a `[Plugins]` section.
- The tool is now distributed as a compiled binary via GitHub Releases. The previous
  npm-based install (`npm install -g uepm`) no longer applies. Use the new install
  scripts instead (see README).

### Added

**Plugin consumer workflow**

- `uepm install [@scope/plugin[@version]]` — resolves semver ranges against the npm
  registry, downloads and sha512-verifies tarballs, extracts into `UEPMPlugins/`,
  pins new packages to `^<resolved>` in `Config/UEPM.ini`, and writes `uepm.lock`
- `uepm install` (no arguments) — reinstalls everything listed in `Config/UEPM.ini`,
  using `uepm.lock` for reproducible versions
- `uepm uninstall @scope/plugin` — removes the plugin directory and its entry from
  both `Config/UEPM.ini` and `uepm.lock`
- `uepm update [@scope/plugin]` — re-resolves to the latest version satisfying the
  declared range, ignoring the lockfile; updates `uepm.lock`
- `uepm list` — prints all installed plugins, their resolved versions, and whether
  each plugin's declared engine range is compatible with the project's engine version
- `uepm.lock` — deterministic lockfile recording resolved version, tarball URL, and
  sha512 integrity for every installed plugin (including transitives)
- Recursive transitive dependency resolution — if an installed plugin declares its
  own `[Dependencies]`, those are resolved and installed automatically; conflicts
  between required versions are reported as errors
- `file:` protocol — `uepm install file:../my-plugin` creates a symlink in
  `UEPMPlugins/` pointing at a local directory, enabling live development without
  publishing to the registry
- `CommitPlugins` setting in `[Settings]` — when `true`, `UEPMPlugins/` is tracked
  in VCS; when `false` (default), `uepm init` writes `UEPMPlugins/` to `.gitignore`
  or `.p4ignore` automatically
- Perforce detection — `uepm init` writes `.p4ignore` instead of `.gitignore` when
  `P4PORT`/`P4CONFIG` env vars or a `.p4config` file are present
- `.uproject` injection — `uepm init` adds `UEPMPlugins` to the `AdditionalPluginDirectories`
  array in the project's `.uproject` file so Unreal picks up installed plugins

**Plugin author workflow**

- `uepm init` (plugin context) — detects `.uplugin` files and prompts for `[Plugin]`
  metadata (name, version, description, author, license, engine range, main), with
  defaults derived from `.uplugin` fields (`FriendlyName`, `VersionName`, `CreatedBy`,
  `Description`)
- `[Plugin]` section in `Config/UEPM.ini` — plugin authors declare distribution
  metadata alongside `[Dependencies]`; project manifests without a `[Plugin]` section
  are unaffected
- `uepm publish` — validates `[Plugin]`, builds a `.tgz` tarball in memory, computes
  SHA-512 integrity + SHA-1 shasum, and PUTs directly to the npm registry API; no
  Node.js or npm required. Supports `--dry-run`, `--tag`, `--access`, OTP prompting
  on 401, and `UEPM_TOKEN` for authentication
- Engine version detection — scans Epic's `LauncherInstalled.dat` on macOS/Linux (and
  the Windows registry on Windows) to pre-fill the engine compatibility range during
  `uepm init`

**Distribution**

- `install.sh` — curl-pipe-sh installer for macOS and Linux; detects architecture and
  downloads the correct binary from GitHub Releases
- `install.ps1` — PowerShell installer for Windows; updates the current session's
  `PATH` so `uepm` is available immediately without reopening the terminal
- `UEPM_REGISTRY` env var — override the registry base URL (useful for private registries
  or testing)
- `UEPM_TOKEN` env var — bearer token for authenticated registry operations
- `UEPM_VERSION` env var — pin the version installed by `install.sh`/`install.ps1`
  (useful for CI pipelines that need a specific release)

### Changed

- Tool renamed from "Unreal Engine Package Manager" to **Unreal Engine Plugin Manager**
  (binary name `uepm` is unchanged)
- `Config/UEPM.ini` is now included in published tarballs so consumers can resolve
  transitive dependencies after extraction
- Lockfile now records a `dependencies` map per plugin entry for transitive deps

### Fixed

- `uepm list` engine compatibility check now correctly tests whether the project's
  engine version satisfies each plugin's declared engine range (previously always
  reported incompatible)
- `uepm update <plugin>` now preserves all other locked packages; previously wiped
  the entire lockfile and re-fetched fresh versions for everything
- `uepm uninstall` now removes the package entry from `uepm.lock`; previously left
  a stale entry that could resurface on re-install
- Tarball extraction now rejects entries with `..` path components or absolute paths,
  preventing path traversal even from a compromised registry package
- Transitive dependencies are now correctly installed and recorded in `uepm.lock`
  when a registry package declares its own `[Dependencies]`
- `NoMatchingVersion` error now correctly names the package in the message
- `CommitPlugins` is now always persisted on re-init, even when the engine association
  is a launcher GUID rather than a version string
- Engine compatibility ranges use comma-separated form (`>=5.3.0, <6.0.0`) as
  required by the semver crate

---

## [1.1.0] - 2024-01-01

### Added

- Interactive `uepm init` prompts for plugin metadata with `--yes` flag for CI
- Plugin init derives defaults from `.uplugin` fields

## [1.0.0] - 2024-01-01

### Added

- Initial release. Node.js-based CLI with `uepm init`, npm registry install via
  `postinstall` hook, `.uproject` modification, and VCS detection

[Unreleased]: https://github.com/adamschlesinger/uepm/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/adamschlesinger/uepm/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/adamschlesinger/uepm/releases/tag/v1.0.0
