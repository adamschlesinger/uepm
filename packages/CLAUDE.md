# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace Layout

All packages are npm workspaces declared in the root `package.json`. Three packages are published to npm; one is the website; one is test infrastructure.

| Directory | Package | Published |
|---|---|---|
| `core/` | `@uepm/core` | Yes |
| `init/` | `@uepm/init` | Yes |
| `postinstall/` | `@uepm/postinstall` | Yes |
| `website/` | `@uepm/website` | No |

`samples/tests` is also a workspace (structural validation tests for the sample project) but is not published.

## Cross-Package Conventions

**Dependency direction**: `init` and `postinstall` depend on `core`. Nothing depends on `init` or `postinstall`. The website is fully independent.

**Building**: `core` must be built before `init` or `postinstall` tests run, because those packages resolve `@uepm/core` from `dist/` (not TypeScript source). After changing `core`, run `cd packages/core && npm run build` before running tests in other packages.

**Error handling**: All errors that should surface to the user must use `UEPMError` from `@uepm/core/src/errors`. Use `instanceof UEPMError` for checks — never `error.name === 'UEPMError'`. Use `ExitCode` enum values — never raw numeric literals.

**Types**: `InitOptions` and `InitResult` are defined once in `@uepm/core` and re-exported from `@uepm/init`. Do not duplicate them.

**Symlinks**: All `fs.symlink` calls must use relative paths computed via `path.relative(path.dirname(linkPath), targetPath)`.

## Versioning

All three published packages are versioned in lockstep (same version number). The `scripts/publish-local.sh` script handles bumping all packages together and updating cross-package dependency constraints.
