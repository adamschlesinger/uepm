# Changelog

All notable changes to UEPM are documented here.

---

## [2.0.0] — unreleased

Complete rewrite in Rust. No Node.js required at runtime.

### Added

**Plugin authoring**
- `uepm init` now detects `.uplugin` files and enters **plugin context**: prompts for
  package metadata (name, version, description, author, license, engine range, main)
  with defaults derived from `.uplugin` fields (`FriendlyName`, `VersionName`,
  `CreatedBy`, `Description`)
- `[Package]` section in `Config/UEPM.ini` — plugin authors declare distribution
  metadata alongside their `[Plugins]` dependencies; project manifests with no
  `[Package]` section are unaffected
- `uepm publish` — validates `[Package]`, builds a `.tgz` tarball in memory,
  computes SHA-512 integrity + SHA-1 shasum, and PUTs directly to the npm registry
  API. No Node.js or npm required. Supports `--dry-run`, `--tag`, `--access`, OTP
  prompting on 401, and `UEPM_TOKEN` for auth
- Engine version detection: scans Epic's `LauncherInstalled.dat` on macOS/Linux to
  pre-fill the engine compatibility range during `uepm init`

**Install scripts**
- `UEPM_VERSION` env var override in `install.sh` and `install.ps1` — allows testing
  a specific release tag without it being the latest

### Changed
- Manifest file is now `Config/UEPM.ini` (TOML format) instead of `uepm.ini`
- `CommitPlugins` replaces the old install mode setting
- Scoped package names required for all plugins (e.g. `@scope/name`)
- Lockfile format version bumped to 1; now records transitive `dependencies` map
  per plugin entry
- `Config/UEPM.ini` is included in published tarballs so consumers can resolve
  transitive dependencies after extraction

### Fixed
- Transitive dependencies are now correctly installed and recorded in `uepm.lock`
  when a registry package declares its own `[Plugins]` dependencies
- `NoMatchingVersion` error now correctly names the package
- `commit_plugins` is now always written on re-init, even when the engine
  association is a launcher GUID
- Engine compatibility ranges use comma-separated form (`>=5.3.0, <6.0.0`) as
  required by the `semver` crate

---

## [1.1.0] — 2024 (TypeScript)

- Interactive `uepm init` prompts for plugin metadata with `--yes` flag for CI
- Plugin init derives defaults from `.uplugin` fields

## [1.0.0] — 2024 (TypeScript)

Initial release. Node.js-based CLI with `uepm init`, npm registry install via
`postinstall` hook, `.uproject` modification, and VCS detection.

> **Note:** The 1.x TypeScript implementation has been superseded by the 2.0.0
> Rust rewrite. The source is preserved in `archive/` on the `main` branch.
