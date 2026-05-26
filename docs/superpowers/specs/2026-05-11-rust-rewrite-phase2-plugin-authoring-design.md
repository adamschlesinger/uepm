# UEPM Phase 2 — Plugin Authoring Design

**Date:** 2026-05-11
**Branch:** rust-rewrite
**Status:** Approved
**Depends on:** `2026-05-10-rust-rewrite-core-cli-design.md`

## Overview

Phase 2 adds plugin authoring support to the UEPM Rust CLI: `uepm init` gains a plugin context that generates `uepm.ini` with package metadata from the `.uplugin` file, and `uepm publish` validates and publishes the plugin to the npm registry.

Plugin packages are still distributed via the npm registry so that the existing ecosystem of npm-published UEPM plugins continues to work. `uepm publish` wraps `npm publish` rather than calling the registry API directly; npm is a plugin-author prerequisite (not a consumer prerequisite).

## `uepm.ini` `[package]` Section

A plugin author's `uepm.ini` contains a `[package]` section alongside `[plugins]`. Consumer project `uepm.ini` files never have a `[package]` section.

```ini
[package]
name = @acme/cool-plugin
version = 1.0.3
description = A cool Unreal Engine plugin
author = Acme Studio
license = MIT
engine_version = ^5.0.0
keywords = unreal, unreal-engine, plugin, uepm
files = CoolPlugin.uplugin, Source/**, Resources/**, Content/**

[plugins]
@acme/base-plugin = ^1.0.0
```

`keywords` and `files` are comma-separated strings. `uepm publish` splits them into JSON arrays when generating `package.json`.

**Critical:** `uepm publish` always adds `uepm.ini` to the `files` list before publishing so that UEPM can read plugin dependencies during `uepm install` (phase 1 step 6).

## `uepm init` — Plugin Context

Triggered when the current directory contains a `.uplugin` file and no `.uproject` file (same detection logic as the TypeScript version). If both exist, project context takes priority with a warning.

### Interactive Prompts

Six prompts in order, with defaults derived from `.uplugin` metadata:

| Prompt | Default source |
|---|---|
| `package name:` | `CreatedBy` → npm scope (lowercased, non-alphanumeric → hyphens) + filename kebab-case |
| `version:` | `VersionName` → `Version.toString()` padded to semver → `"1.0.0"` |
| `description:` | `Description` → `FriendlyName` → `""` |
| `author:` | `CreatedBy` → `""` |
| `license:` | `"MIT"` |
| `engine version (semver range):` | `EngineVersion` from `.uplugin` → `"^5.0.0"` |

`--yes` skips all prompts and uses derived defaults. Non-TTY without `--yes` exits with `UepmError::InteractiveRequired` and a suggestion to use `--yes`.

### Output

Writes `uepm.ini` with the confirmed values. The `files` field is pre-populated with the `.uplugin` filename and standard plugin directories (`Source/**`, `Resources/**`, `Content/**`). The `[plugins]` section is written empty.

Also appends `package.json` to `.gitignore` (creating the file if absent), since `package.json` is a generated publish artifact that should not be checked in.

If `uepm.ini` already exists and `--force` is not set, exits with a message indicating the plugin is already initialized. With `--force`, re-prompts and overwrites.

## `uepm publish`

Requires `uepm.ini` with a `[package]` section in the current directory.

### Step 1: Validate `uepm.ini`

Required fields: `name`, `version`, `license`, `engine_version`. Any missing → `UepmError::MissingPackageField` naming the absent field with a suggestion to run `uepm init`.

### Step 2: Validate files on disk

Expand each entry in `files` as a glob pattern. Any pattern with zero matches → `UepmError::MissingFiles` listing which patterns failed. At minimum, the `.uplugin` file must match.

### Step 3: Verify `.uplugin` exists

The plugin name from `name` (part after `/`) with `.uplugin` extension must exist in the current directory. → `UepmError::MissingUplugin` if absent.

### Step 4: Generate `package.json`

Constructed from `[package]` fields. `uepm.ini` is appended to the `files` array if not already present. The `unreal` section is populated:

```json
{
  "name": "@acme/cool-plugin",
  "version": "1.0.3",
  "description": "A cool Unreal Engine plugin",
  "author": "Acme Studio",
  "license": "MIT",
  "main": "CoolPlugin.uplugin",
  "files": ["CoolPlugin.uplugin", "Source/**", "Resources/**", "Content/**", "uepm.ini"],
  "keywords": ["unreal", "unreal-engine", "plugin", "uepm"],
  "unreal": {
    "engineVersion": "^5.0.0",
    "pluginName": "CoolPlugin"
  }
}
```

Written to disk at `./package.json`.

### Step 5: Run `npm publish`

Invoked as a subprocess: `npm publish --access public`. If `npm` is not found on PATH → `UepmError::NpmNotFound` with message: *"uepm publish requires npm. Install Node.js from https://nodejs.org"*.

If npm exits non-zero → `UepmError::PublishFailed` surfacing npm's stderr directly.

### Step 6: Clean up

`package.json` is removed from disk regardless of whether publish succeeded or failed. Uses a Rust `Drop` guard or explicit cleanup in both success and error paths to ensure this always runs.

## Codebase Changes

Phase 2 adds one new command file and extends `init.rs`:

```
uepm/src/
  commands/
    init.rs         — extended: add plugin context branch (was project-only in phase 1)
    publish.rs      — new: uepm publish
  manifest.rs       — extended: read/write [package] section
```

A `PluginManifest` struct is added to `manifest.rs` alongside the existing `ProjectManifest`:

```rust
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub engine_version: String,
    pub keywords: Vec<String>,
    pub files: Vec<String>,
}
```

## Error Handling

New `UepmError` variants (additions to phase 1 enum):

| Variant | Trigger |
|---|---|
| `MissingPackageField(String)` | Required `[package]` field absent |
| `MissingFiles(Vec<String>)` | `files` globs with no matches |
| `MissingUplugin` | `.uplugin` file not found |
| `NpmNotFound` | `npm` not on PATH |
| `PublishFailed(String)` | npm exited non-zero; carries npm's stderr |
| `AlreadyInitialized` | `uepm.ini` exists and `--force` not set |

## Testing

**`uepm init` (plugin context):**
- Unit tests for default derivation: scope from `CreatedBy`, kebab-case conversion, version padding, fallback chains for description and author
- Integration test: real temp directory with a minimal `.uplugin` → `--yes` → verify `uepm.ini` written correctly
- Non-TTY without `--yes` → `InteractiveRequired` error
- `--force` on already-initialized plugin → re-generates `uepm.ini`
- `package.json` added to `.gitignore`

**`uepm publish` validation:**
- Each missing required field produces the correct `MissingPackageField` error
- Missing files glob → `MissingFiles` with correct paths listed
- Missing `.uplugin` → `MissingUplugin`

**`uepm publish` generation:**
- Generated `package.json` has correct fields from `uepm.ini`
- `uepm.ini` is present in the `files` array
- `package.json` is removed after both success and failure paths (cleanup verified)

**`uepm publish` subprocess:**
- Integration test with a mock `npm` binary on PATH that records arguments and returns 0
- Integration test with a mock `npm` that returns 1 → `PublishFailed` with captured stderr

## Roadmap

- **Direct registry API** — replace `npm publish` subprocess with direct HTTP calls to the npm registry, making `uepm publish` fully standalone with no npm prerequisite for plugin authors
