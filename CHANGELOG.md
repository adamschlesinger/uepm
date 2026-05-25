# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Renamed tool from "Unreal Engine Package Manager" to **Unreal Engine Plugin Manager**
  (binary name `uepm` is unchanged)
- `Config/UEPM.ini` section `[Package]` renamed to `[Plugin]` — describes the
  plugin being published (name, version, author, etc.)
- `Config/UEPM.ini` section `[Plugins]` renamed to `[Dependencies]` — lists
  plugin dependencies in both project and plugin manifests

### Added

- `uepm init` plugin context: detects `.uplugin` files and prompts for plugin
  metadata (name, version, description, author, license, engine range, main)
  with defaults derived from `.uplugin` fields (`FriendlyName`, `VersionName`,
  `CreatedBy`, `Description`)
- `[Plugin]` section in `Config/UEPM.ini` — plugin authors declare distribution
  metadata alongside `[Dependencies]`; project manifests without a
  `[Plugin]` section are unaffected
- `uepm publish` — validates `[Plugin]`, builds a `.tgz` tarball in memory,
  computes SHA-512 integrity + SHA-1 shasum, and PUTs directly to the npm registry
  API. No Node.js or npm required. Supports `--dry-run`, `--tag`, `--access`,
  OTP prompting on 401, and `UEPM_TOKEN` for authentication
- Engine version detection: scans Epic's `LauncherInstalled.dat` on macOS/Linux
  to pre-fill the engine compatibility range during `uepm init`
- `UEPM_VERSION` env var override in `install.sh` and `install.ps1` for testing
  specific release tags without them being the latest

### Changed

- `Config/UEPM.ini` is now included in published tarballs so consumers can
  resolve transitive dependencies after extraction
- Lockfile now records a `dependencies` map per plugin entry for transitive deps

### Fixed

- Transitive dependencies are now correctly installed and recorded in `uepm.lock`
  when a registry package declares its own `[Dependencies]`
- `NoMatchingVersion` error now correctly names the package in the message
- `commit_plugins` is now always persisted on re-init, even when the engine
  association is a launcher GUID
- Engine compatibility ranges use comma-separated form (`>=5.3.0, <6.0.0`) as
  required by the `semver` crate

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
