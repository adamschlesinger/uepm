# Roadmap

## Phase 2 — Plugin Authoring ✅

- [x] `[Plugin]` section in `Config/UEPM.ini` — plugin authors declare distribution metadata alongside their `[Dependencies]`
- [x] `uepm init` (plugin context) — detects `.uplugin`, derives defaults from `FriendlyName`/`VersionName`/`Description`/`CreatedBy`, prompts for metadata, writes `[Plugin]` section
- [x] Engine version detection — scans Epic `LauncherInstalled.dat` (macOS/Linux) to pre-fill engine range during `uepm init`
- [x] `uepm publish` — validates `[Plugin]`, builds `.tgz` in memory, PUTs directly to registry API; no Node.js or npm required

## Phase 3 — Ecosystem

- [ ] `uepm new` — scaffold a new plugin from scratch with `.uplugin`, `Source/`, and `Config/UEPM.ini`
- [ ] Plugin search — `uepm search <term>` queries the registry for packages with the `uepm` keyword
- [ ] Website listing — curated page showing UEPM-compatible plugins filterable by engine version
- [ ] Package manager distribution — Homebrew tap, Chocolatey package, Scoop manifest

## Known Limitations

- [ ] Full semver graph resolution — current resolver does recursive install with explicit conflict detection; a full npm-style backtracking solver would handle more complex dependency graphs

## Completed

- [x] `uepm init` (project context) — creates `Config/UEPM.ini`, `UEPMPlugins/`, modifies `.uproject`, writes `.gitignore`/`.p4ignore` based on VCS detection
- [x] `uepm install` — resolves, downloads, sha512-verifies, and extracts plugins; recursive transitive dep resolution
- [x] `uepm uninstall` — removes plugin directory and updates `Config/UEPM.ini`
- [x] `uepm update` — re-resolves to latest compatible versions ignoring lockfile
- [x] `uepm list` — shows installed plugins, resolved versions, and engine compatibility status
- [x] `file:` protocol — symlinks local plugin directories for live development without publishing
- [x] Lockfile — `uepm.lock` for reproducible installs with sha512 integrity
- [x] Recursive dependency resolution with conflict detection
- [x] `CommitPlugins` setting — auto-adds `UEPMPlugins/` to `.gitignore` or `.p4ignore`
- [x] Perforce detection — `P4PORT`/`P4CONFIG` env vars or `.p4config` file walk
- [x] Standalone Rust binary — no Node.js required for plugin consumers
