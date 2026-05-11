# Install Mode Design

**Date:** 2026-05-10
**Status:** Approved

## Overview

UEPM currently creates symbolic links in `UEPMPlugins/` pointing into `node_modules/`. This works for git-based workflows but breaks on Windows (symlinks require elevated permissions or developer mode) and is incompatible with Perforce, the dominant VCS in game development (P4 reconcile walks into junctions; P4 studios expect all project files to be real, checkable-in files).

This feature adds a configurable `installMode` that controls how UEPM places plugin files into `UEPMPlugins/`. The mode is chosen once during `npx @uepm/init` and stored in `package.json`.

## Install Modes

| Mode | Behavior | Best for |
|---|---|---|
| `symlink` | Symbolic links in `UEPMPlugins/` pointing to `node_modules/` (junctions on Windows) | Git workflows |
| `copy` | Plugin files copied from `node_modules/` into `UEPMPlugins/` as real directories | Perforce / any VCS that vendors files |
| `none` | No `UEPMPlugins/` management; no postinstall hook installed | Teams with a custom plugin pipeline |

Default when `installMode` is absent from `package.json`: `symlink` (backward compatibility with existing projects).

## Configuration Schema

`InstallMode` is defined in `@uepm/core/src/types.ts`:

```typescript
export type InstallMode = 'symlink' | 'copy' | 'none';
```

`PackageJson` gains a typed `uepm` field:

```typescript
uepm?: {
  installMode?: InstallMode;
};
```

The value is written during `npx @uepm/init` (project context) and read at runtime by `uepm-postinstall`.

## VCS Detection

`detectInstallMode(directory: string): Promise<InstallMode>` in `@uepm/core` walks up the directory tree checking for VCS indicators and returns the recommended default:

Detection checks in priority order, stopping at the first match:

1. **P4 indicators** → `'copy'`: `P4CONFIG` or `P4PORT` env vars are set, or a `.p4config` file exists in `directory` or any ancestor
2. **Windows** → `'copy'`: `process.platform === 'win32'`
3. **Git** → `'symlink'`: a `.git` directory exists in `directory` or any ancestor
4. **Neither** → `'symlink'`

**Why Windows defaults to `copy`:** Creating symlinks on Windows requires either Administrator privileges or Developer Mode enabled in system settings. In game studio environments, IT policy almost universally blocks Developer Mode. Defaulting Windows users to `copy` avoids a broken default state. Users who explicitly choose `symlink` on Windows and have Developer Mode enabled will get standard relative symlinks — `SymlinkInstallStrategy` makes no platform-specific adjustments.

## Init Flow (Project Context)

Project init gains a single interactive `select` prompt using the `prompts` library (already a dependency of `@uepm/init`). The highlighted default is set by `detectInstallMode()`.

```
? Install mode ›
❯ symlink — symbolic links in UEPMPlugins/ (git workflow)
  copy   — real files in UEPMPlugins/ (Perforce / any VCS)
  none   — UEPM handles init only, no postinstall hook
```

**`--yes` flag**: skips the prompt, uses the detected default.

**Non-TTY without `--yes`**: throws `UEPMError('INTERACTIVE_REQUIRED', ...)` with a suggestion to use `--yes`, same as plugin init.

**Based on selected mode:**
- `symlink` / `copy`: writes `uepm.installMode` to `package.json`, adds `postinstall: "uepm-postinstall"` script, adds `@uepm/postinstall` to `devDependencies`
- `none`: writes `uepm.installMode: "none"` to `package.json`, does **not** add postinstall script or devDependency

`InitOptions` gains `installMode?: InstallMode`.

The prompt logic lives in a new `packages/init/src/project-prompts.ts` (parallel to the existing `plugin-prompts.ts`).

## Strategy Pattern (Postinstall)

### File Structure

```
packages/postinstall/src/
  install-strategy.ts              — InstallStrategy interface
  strategies/
    symlink-strategy.ts            — SymlinkInstallStrategy
    copy-strategy.ts               — CopyInstallStrategy
```

### Interface

```typescript
export interface InstallStrategy {
  install(
    pluginName: string,
    sourcePath: string,
    uepmPluginsDir: string
  ): Promise<void>;

  cleanup(
    installedPluginNames: string[],
    uepmPluginsDir: string
  ): Promise<void>;
}
```

`cleanup` runs once per postinstall invocation after all `install` calls. It removes entries in `UEPMPlugins/` that are no longer in `installedPluginNames`. Both strategies implement it — `SymlinkInstallStrategy` removes stale symlinks, `CopyInstallStrategy` removes stale directories.

### SymlinkInstallStrategy

Extracts the current symlink logic from `setup.ts` with no platform-specific changes. Creates a relative `'dir'` symlink on all platforms. If the developer is on Windows without Developer Mode, they will get the OS-level `EPERM` error — which is correct behaviour, since they explicitly chose `symlink` mode against the detected default of `copy`.

### CopyInstallStrategy

Uses `fs.cp(sourcePath, targetPath, { recursive: true })` to copy plugin files. Before copying, removes any existing directory at `targetPath` (handles updates cleanly). Cleanup compares `UEPMPlugins/` entries against `installedPluginNames` and removes any that are not in the list.

### `setup.ts` orchestration

```typescript
// Read mode from package.json (default: 'symlink')
const mode = packageJson.uepm?.installMode ?? 'symlink';

if (mode === 'none') return; // nothing to do

if (!['symlink', 'copy'].includes(mode)) {
  throw new UEPMError('INVALID_CONFIG', `Unknown installMode: "${mode}". Expected symlink, copy, or none.`);
}

const strategy = mode === 'copy' ? new CopyInstallStrategy() : new SymlinkInstallStrategy();

// ... per-plugin loop calling strategy.install() ...

await strategy.cleanup(linkedPluginNames, uepmPluginsDir);
```

## Error Handling

- **Unknown `installMode`**: `UEPMError` with the invalid value and a list of valid options
- **Copy failures**: collected and thrown as a batch after the loop (same pattern as current symlink batch errors)
- **Cleanup**: only removes entries confirmed to be stale (in `UEPMPlugins/` but not in current installed plugin list) — never blindly deletes
- **Windows symlink failure (`EPERM`)**: surfaces directly with the OS error — the developer chose `symlink` mode explicitly on a platform where it requires Developer Mode

## Testing

**`detectInstallMode`** (new test file in `@uepm/core`):
- P4 env var set → returns `'copy'`
- `.p4config` file in ancestor directory → returns `'copy'`
- `process.platform === 'win32'` (mocked) → returns `'copy'`
- `.git` directory present → returns `'symlink'`
- Neither → returns `'symlink'`
- P4 indicators present alongside `.git` → returns `'copy'` (P4 takes precedence)
- Windows with `.git` present → returns `'copy'` (Windows checked before git)

**`SymlinkInstallStrategy`**:
- Existing `setup.test.ts` symlink tests migrate here

**`CopyInstallStrategy`**:
- Real temp dirs throughout (no mocking)
- `install`: files are actually present in `UEPMPlugins/` after call
- `install` on existing directory: files updated to new version
- `cleanup`: stale directory removed; directory for currently-installed plugin preserved

**Project init prompt** (new tests in `packages/init`):
- `--yes` uses VCS-detected default without prompting
- Non-TTY without `--yes` → error with `--yes` suggestion
- `symlink` mode → postinstall script and devDep added, `uepm.installMode` written
- `copy` mode → same as symlink
- `none` mode → no postinstall script, no devDep, `uepm.installMode: "none"` written
- Re-init with `--force` re-prompts for mode (same as first run — `--force` doesn't preserve existing values)

## Backward Compatibility

Existing projects without `uepm.installMode` in `package.json` continue to work as before — the postinstall hook defaults to `symlink` behavior. No migration required.
