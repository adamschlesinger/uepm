# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Scripts

| Script | Purpose |
|---|---|
| `publish-local.sh` | Full publish workflow: install → build → test → version bump → npm publish → git tag → GitHub release |
| `test-publish.sh` | Dry-run of the publish workflow — no version changes, no actual publishing |
| `test-published-packages.sh` | Post-publish smoke test: installs the just-published packages from the live npm registry and verifies the CLI (`npx @uepm/init --help`) and postinstall hook work end-to-end |

## Publishing

All three packages (`@uepm/core`, `@uepm/init`, `@uepm/postinstall`) and both sample plugins are versioned in lockstep. `publish-local.sh` bumps them all to the same version and updates cross-package `^version` constraints before publishing.

Run from the **repository root**:
```bash
# Dry run (no changes)
./scripts/publish-local.sh --dry-run

# Patch release
./scripts/publish-local.sh --version-type patch

# Minor or major
./scripts/publish-local.sh --version-type minor
./scripts/publish-local.sh --version-type major
```

Requires `NPM_TOKEN` in a `.env` file at the repo root. The script writes `~/.npmrc` for the publish and removes it when done.

## Preferred Release Path

The canonical release mechanism is the GitHub Actions workflow ("Publish to NPM"). Use `publish-local.sh` only when CI is unavailable or for testing the workflow locally. Use `test-publish.sh` first to catch issues before touching the registry.
