# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
npm run build      # tsc → dist/
npm test           # vitest --run (all tests once)
npm run test:watch # vitest (watch mode)
npm run clean      # rm -rf dist
```

Run a single test file:
```bash
npx vitest --run src/context-detector.test.ts
```

## What This Package Does

`@uepm/core` is a library — no CLI, no side effects at import. It exposes:
- File managers for `.uproject`, `.uplugin`, and `package.json`
- The `detectContext()` function that determines project-vs-plugin context
- `PluginInitializationStrategy` — the full plugin init orchestration
- Shared types, error classes, and validation utilities

Everything in `src/index.ts` is public API. `test-generators.ts` is intentionally NOT exported from `index.ts` — it is test-only and must be imported directly from source (`@uepm/core/src/test-generators`).

## Key Types

`InitContext` is a discriminated union — narrow on `.type` before accessing variant-specific fields:
```typescript
if (context.type === 'plugin') {
  context.pluginName // string, always present
}
```

`ContextDetectionResult` is a discriminated union on `.success`:
```typescript
if (!result.success) {
  result.error // string
} else {
  result.context // InitContext
}
```

`InitResult.filesCreated` and `filesModified` are always `string[]` (never undefined). Error return paths must include `filesCreated: [], filesModified: []`.

## Error Handling Rules

- Throw `UEPMError` (from `./errors`) for all user-facing errors.
- Use `ExitCode` enum values — never numeric literals like `3` or `1`.
- Use `instanceof UEPMError` for type narrowing — never `error.name === 'UEPMError'`.
- `hasProjectContext`/`hasPluginContext` only swallow `ENOENT`; all other errors propagate.

## Testing Approach

Tests use real temp directories (`fs.mkdtemp`) — no `fs` mocking. `fast-check` property-based tests use shared arbitraries from `src/test-generators.ts`. Property tests are numbered (Property 1, Property 2, …) and reference the feature/requirement they validate.

`plugin-package-json-generator.test.ts` tests the **synchronous** `generatePluginPackageJson`. The **async** `generatePluginPackageJsonWithDevConfig` (which is what `PluginInitializationStrategy` actually calls) has different conditional behaviour and is exercised through `plugin-initialization-strategy.test.ts`.
