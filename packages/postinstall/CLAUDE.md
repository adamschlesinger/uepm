# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
npm run build      # tsc → dist/
npm test           # vitest --run
npm run test:watch # vitest
npm run clean      # rm -rf dist
```

**Important**: `@uepm/core` must be built before these tests run. If core source changed, run `cd ../core && npm run build` first.

## What This Package Does

`@uepm/postinstall` runs automatically after `npm install` via the postinstall script that `@uepm/init` injects. It does two things:

1. **`setup.ts` — `setupPlugins()`**: Reads `package.json`, creates `UEPMPlugins/` directory, creates relative symlinks for every installed Unreal plugin.
2. **`validate.ts` — `validatePlugins()`**: Reads the `.uproject` to get engine version, scans `node_modules` for UEPM-compatible packages, checks semver compatibility.

`index.ts` — `runPostinstall()` calls both in sequence and calls `process.exit(1)` on any thrown error.

## Error Handling Contract

**`setupPlugins` propagates errors** — no outer catch. Failures from `readFile`, `JSON.parse`, or `mkdir` propagate up to `runPostinstall`'s handler. Individual symlink failures are collected and thrown as a batch after the loop (so the user sees all failures, not just the first).

**`unlink` catch blocks** only swallow `ENOENT`. Any other code (`EACCES`, `EPERM`, `EBUSY`) is rethrown so the subsequent `symlink` call produces a meaningful error rather than a misleading `EEXIST`.

**`validatePlugins` warnings** are always printed unconditionally — not gated on `incompatible.length > 0`. Validation errors (missing `.uproject`, bad JSON) go into `result.warnings` and are printed by `runPostinstall`.

**`extractPluginInfo`** returns `null` silently on `ENOENT` (not a plugin directory). Any other error (malformed JSON, `EACCES`) logs a warning with the package path before returning `null`.

## Plugin Detection

A package is identified as an Unreal plugin by `isUnrealPlugin()` in `setup.ts`, which checks for a `unreal` key in `package.json` or the presence of a `.uplugin` file. Do not use package name string matching — it's fragile.

## Engine Version Handling

- GUID-format engine associations (launcher-installed engines, e.g. `{GUID}`) return `null` from `cleanVersionString` and skip compatibility checking entirely.
- Integer-only versions (e.g. `2`) and two-part versions (e.g. `2.5`) are padded to three-part semver before `semver.coerce`.
