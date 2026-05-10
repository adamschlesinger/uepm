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

### Init Package Flow (`packages/init/src/`)

`cli.ts` → `command-registry.ts` → `InitCommand` (init-command.ts) → `init()` (index.ts) → calls `detectContext()` then dispatches to `PluginInitializationStrategy` or project init logic.

See `packages/core/CLAUDE.md` for a detailed breakdown of core internals.

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
- **fast-check** for property-based tests; shared arbitraries live in `core/src/test-generators.ts` (not part of the public API — import directly from source in tests)
- Sample project at `samples/project/` with live `.uproject` for integration testing
- **Build order matters**: `@uepm/core` must be built before running `npm test` in `packages/init` or `packages/postinstall`. Running `npm test` from the repo root builds all packages first via the `build` workspace script, so this only matters when running per-package tests directly after changing core.

## Keeping CLAUDE.md Files Current

Each significant directory has its own `CLAUDE.md`. When making changes to a package, update the corresponding file if the commands, architecture, or key constraints described there change. Files to keep in sync:

- `CLAUDE.md` — root overview and monorepo commands
- `packages/CLAUDE.md` — workspace layout and cross-package conventions
- `packages/core/CLAUDE.md` — core types, exported API, test arbitraries
- `packages/init/CLAUDE.md` — CLI flow, command registration
- `packages/postinstall/CLAUDE.md` — setup/validate split, error handling contract
- `packages/website/CLAUDE.md` — Astro/Tailwind/React stack, env vars, known failing tests
- `samples/CLAUDE.md` — sample project and plugin structure, validation tests
- `scripts/CLAUDE.md` — publish and release workflow
