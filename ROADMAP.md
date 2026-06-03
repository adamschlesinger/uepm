# Roadmap

## Phase 3 — Ecosystem & DX Polish

**Goal:** Make UEPM feel complete to anyone who discovers it. Finishes the surface area v2.0 left thin.

### Discovery & Distribution
- [ ] `uepm search <term>` — query the registry for packages tagged with `uepm`; show name, version, description, engine compatibility
- [ ] `uepm new <name>` — scaffold a new plugin directory with `.uplugin`, `Source/` skeleton, and `Config/UEPM.ini`; runs plugin-context `uepm init` automatically
- [ ] Package manager distribution — Homebrew tap, Chocolatey package, Scoop manifest; automated from existing GitHub Release CI
- [ ] Website plugin directory — live-search index page on the existing site, backed by the npm registry search API, filterable by engine version

### Developer Experience
- [ ] `git:` install source — install directly from a Git repository (`git:https://github.com/org/repo#ref`); pins to a commit SHA in `uepm.lock`; supports private repos via `UEPM_TOKEN` or SSH keys
- [x] `--json` output flag on all commands — structured JSON output for scripting, Editor integration (Phase 4), and CI tooling (Phase 5)
- [ ] `uepm completions <shell>` — emit shell completion scripts for bash, zsh, fish, and PowerShell (via `clap_complete`)
- [ ] `uepm outdated` — compare lockfile versions against latest versions satisfying declared ranges; exits non-zero if anything is behind
- [ ] `uepm info <pkg>` — fetch and display registry metadata: description, author, license, all published versions with engine ranges
- [ ] `uepm doctor` — validate local setup: parseable config, lockfile consistency, `.uproject` wiring, engine compatibility, registry reachability

---

## Phase 4 — Native Unreal Editor Integration

**Goal:** Remove the terminal requirement for day-to-day plugin management. The Editor plugin is itself distributed via UEPM.

- [ ] `@uepm/editor-integration` Editor plugin (C++) — dockable UEPM panel in the Unreal Editor; shells out to the `uepm` binary (using `--json`) for all operations
  - [ ] **Installed view** — list installed plugins with version, engine compatibility badge, update indicator; uninstall/update context menu
  - [ ] **Search view** — debounced search against the registry; one-click install with version picker
  - [ ] **Updates view** — runs `uepm outdated`; bulk "Update All" action
  - [ ] **Log view** — streams subprocess output in real time; persists last N operations
  - [ ] **Editor preferences** — override binary path, registry URL, and token

---

## Phase 5 — CI/CD & Private Registries

**Goal:** The studio engineering lead unlock. UEPM fits existing pipeline and registry workflows without compromise.

### CI/CD Integration
- [ ] `uepm ci` — non-interactive pipeline mode with lockfile-drift detection, clean exit codes (0/1/2/3), no color or spinners
- [ ] GitHub Actions: `uepm/setup-uepm` action on the Actions Marketplace — install binary, configure token from secrets, cache across runs
- [ ] TeamCity + Jenkins documented integration patterns — shell step templates, env var conventions, exit code handling, caching

### Private Registry & Credential Management
- [ ] Per-scope registry routing via `[Registries]` section in `Config/UEPM.ini` — `@studio-scope/*` routes to a private registry; everything else falls through to the default
- [ ] `uepm login [--registry <url>]` — store tokens in OS keychain (Keychain Access / Windows Credential Manager / libsecret) with `~/.uepm/credentials` fallback
- [ ] `uepm logout` — clear stored credentials for a registry
- [ ] Token precedence: `UEPM_TOKEN` env var → keychain → credentials file

---

## Phase 6 — Binary Distribution

**Goal:** Unlock the commercial plugin ecosystem. First-class support for prebuilt binaries alongside (or instead of) source.

- [ ] `Distribution` field in `[Plugin]` — `"source"`, `"binary"`, or `"both"`
- [ ] Binary package format — prebuilt files organized by UE version and platform (`Binaries/Win64/UE5.4/`, etc.)
- [ ] `uepm publish --platform <platform> --engine <ue-version>` — publish platform-specific binary packages
- [ ] Binary-aware install — detect project engine version and host platform; prefer binary, fall back to source; controlled by `PreferBinaries` and `FallbackToSource` settings
- [ ] `uepm pack` — dry-run tarball builder; list included files or write `.tgz` to disk without uploading

---

## Phase 7 — Advanced Dependency Resolution & Workspace Support

**Goal:** Long-tail reliability at scale. Complex plugin graphs and multi-project studio setups.

- [ ] Full backtracking semver resolver — SAT-style solver replacing the current "first match wins" recursive resolver; clean conflict reports naming the conflicting constraints
- [ ] Peer dependencies — `[PeerDependencies]` section; `uepm install` warns on missing or mismatched peers; `uepm doctor` validates
- [ ] Workspace support — `uepm.workspace.ini` for monorepos; unified dependency graph across member projects; hoisted shared plugins; per-project installs

---

## Known Limitations

- [ ] Full semver backtracking resolver — addressed in Phase 7
- [ ] No peer dependency support — addressed in Phase 7
- [ ] No workspace/monorepo support — addressed in Phase 7
- [ ] Binary distribution not yet supported — addressed in Phase 6

---

## Completed

### Phase 2 — Plugin Authoring ✅

- [x] `[Plugin]` section in `Config/UEPM.ini` — plugin authors declare distribution metadata alongside their `[Dependencies]`
- [x] `uepm init` (plugin context) — detects `.uplugin`, derives defaults from `FriendlyName`/`VersionName`/`Description`/`CreatedBy`, prompts for metadata, writes `[Plugin]` section
- [x] Engine version detection — scans Epic `LauncherInstalled.dat` (macOS/Linux) to pre-fill engine range during `uepm init`
- [x] `uepm publish` — validates `[Plugin]`, builds `.tgz` in memory, PUTs directly to registry API; no Node.js or npm required

### Phase 1 — Core CLI ✅

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
