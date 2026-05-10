# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Root (all packages)
npm install          # Install all workspace dependencies
npm run build        # Build all packages
npm test             # Run all tests
npm run clean        # Clean all build artifacts and node_modules

# Per-package (run from packages/<name>/ or use --workspace)
npm run build        # Compile TypeScript → dist/
npm test             # Run vitest once
npm run test:watch   # Run vitest in watch mode
npm run clean        # Remove dist/

# Run a single test file
cd packages/core && npx vitest --run src/context-detector.test.ts
```

## Architecture

UEPM is a monorepo that brings NPM-based distribution to Unreal Engine plugins. The core flow:

1. `npx @uepm/init` — run once in a `.uproject` directory. Modifies the project file to add `node_modules` to plugin search paths, generates/updates `package.json`.
2. `npm install @scope/plugin` — standard NPM install.
3. `uepm-postinstall` — postinstall hook that creates symlinks in `UEPMPlugins/` and validates engine version compatibility.

### Packages

| Package | Purpose |
|---|---|
| `@uepm/core` | Shared types, file managers, and utilities |
| `@uepm/init` | CLI tool (`npx @uepm/init`) using Commander.js |
| `@uepm/postinstall` | Postinstall hook: symlink setup + validation |
| `packages/website` | Astro marketing site (independent of above) |

### Core Package Internals (`packages/core/src/`)

- **`types.ts`** — All shared interfaces: `UProjectFile`, `UPluginFile`, `PackageJson`, `InitContext`, etc.
- **`context-detector.ts`** — Detects whether CWD is a project (`.uproject`) or plugin (`.uplugin`) context; drives init branching.
- **`uproject-manager.ts`** / **`uplugin-manager.ts`** — Read/write `.uproject` / `.uplugin` JSON files; `uplugin-manager` also extracts plugin metadata.
- **`package-json-manager.ts`** — Read/write/check existence of `package.json`.
- **`plugin-package-json-generator.ts`** — Generates plugin-specific `package.json` content from `.uplugin` metadata; handles merge with existing files.
- **`plugin-initialization-strategy.ts`** — `PluginInitializationStrategy` class: orchestrates the full plugin init flow (read uplugin → generate config → merge/create package.json).
- **`errors.ts`** — `UEPMError` class with typed exit codes; factory functions for common error cases.
- **`validation.ts`** — Engine version semver compatibility checking (uses `semver` package).
- **`test-generators.ts`** — `fast-check` arbitraries shared across test suites for property-based tests.

### Init Package Flow (`packages/init/src/`)

`cli.ts` → `command-registry.ts` → `InitCommand` (init-command.ts) → `init()` (index.ts) → calls `detectContext()` then dispatches to `PluginInitializationStrategy` or project init logic.

### Plugin Package Structure

UEPM plugins require a `package.json` with:
```json
{
  "main": "PluginName.uplugin",
  "unreal": { "engineVersion": ">=5.0.0 <6.0.0", "pluginName": "PluginName" },
  "keywords": ["unreal", "unreal-engine", "plugin", "uepm"]
}
```

### Testing Approach

- **Vitest** for all unit tests; config at `packages/<name>/vitest.config.ts`
- **fast-check** for property-based tests; shared arbitraries live in `core/src/test-generators.ts`
- Sample project at `samples/project/` with live `.uproject` for integration testing
