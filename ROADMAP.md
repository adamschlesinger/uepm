# Roadmap

## Phase 2 — Plugin Authoring

- [ ] `uepm publish` — validate, generate `package.json` from `.uplugin` metadata, and publish to npm registry
- [ ] `uepm new` — scaffold a new plugin directory with `.uplugin`, `Source/`, and `uepm.ini`
- [ ] Engine version detection — locate installed UE builds to pre-fill `engine_version` during `uepm init`

## Discovery & Ecosystem

- [ ] Plugin search — `uepm search <term>` queries the registry for packages with the `uepm` keyword
- [ ] Website listing — curated page showing UEPM-compatible plugins filterable by engine version

## Completed

- [x] Windows support — `copy` install mode produces real files for Perforce / any VCS
- [x] Perforce detection — `P4PORT` / `P4CONFIG` env vars or `.p4config` file walk defaults to `copy` mode
- [x] `uepm uninstall` — removes plugin directory and updates `uepm.ini`
- [x] `uepm list` — shows installed plugins and engine compatibility status
- [x] `uepm update` — re-resolves to latest compatible versions
- [x] Lockfile — `uepm.lock` for reproducible installs
- [x] Recursive dependency resolution with conflict detection
- [x] sha512 integrity verification on every download
