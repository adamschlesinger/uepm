# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
npm run build      # tsc → dist/
npm test           # vitest --run
npm run test:watch # vitest
npm run clean      # rm -rf dist

# Run a single test file
npx vitest --run src/index.test.ts
```

**Important**: `@uepm/core` must be built before these tests run. If core source changed, run `cd ../core && npm run build` first.

## What This Package Does

`@uepm/init` is the CLI tool users run via `npx @uepm/init`. It:
1. Detects context (project vs plugin) by looking for `.uproject` / `.uplugin` files
2. Dispatches to `PluginInitializationStrategy` (from core) for plugin context
3. Runs the inline `initializeProject()` function for project context

## Call Chain

```
bin/uepm-init.js
  → cli.ts          (Commander setup, --help/--version, error formatting)
  → command-registry.ts  (maps string command names to Command implementations)
  → InitCommand.execute()  (init-command.ts — translates CLI options → InitOptions, maps exit codes)
  → init()          (index.ts — detects context, dispatches strategy)
```

To add a new CLI command, implement the `Command` interface from `command-registry.ts` and register it in `cli.ts`.

## Types

`InitOptions` and `InitResult` are defined in `@uepm/core` and re-exported here. Do not redefine them locally. Import via:
```typescript
import { InitOptions, InitResult } from '@uepm/core';
```

All return paths from `init()` must include `filesCreated: []` and `filesModified: []` — even error exits — because `InitResult` requires those fields.

## Error Handling

`init()` catches `UEPMError` and converts it to `{ success: false, message: ... }` including the `suggestion` field. Generic errors get wrapped in a "Unexpected error during initialization:" message. `InitCommand.execute()` maps the `success` boolean to exit codes 0/1.
