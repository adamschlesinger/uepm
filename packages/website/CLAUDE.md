# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
npm run dev        # Start Astro dev server (localhost:4321)
npm run build      # Production build → dist/
npm run preview    # Preview production build locally
npm run lint       # ESLint on .ts and .astro files
npm run lint:fix   # ESLint with auto-fix
npm run type-check # astro check (type-checks .astro files)
npm test           # vitest --run
```

## Stack

- **Astro** (static output) — page framework
- **Tailwind CSS** — styling
- **React** — used for interactive island components only
- **Shiki** — syntax highlighting for code examples
- **Vitest + jsdom** — unit tests for markup/content assertions

This package is completely independent of `@uepm/core`, `@uepm/init`, and `@uepm/postinstall`. It has no runtime dependency on them.

## Structure

- `src/pages/` — Astro pages (file-based routing; `index.astro` = homepage)
- `src/components/` — Astro and React components
- `src/config/` — **single source of truth** for all site content, URLs, and metadata
- `src/styles/` — global CSS (Tailwind base)
- `src/test/` — Vitest unit tests (run against rendered markup via jsdom)
- `src/utils/` — shared utilities (syntax highlighting, performance helpers)

## Configuration

All URLs and site metadata live in `src/config/site.ts` (`siteConfig`). **Do not hardcode external links anywhere else** — add them to `siteConfig.externalLinks` and reference from there. This is the most common mistake to avoid.

Analytics, site URL, and other environment-specific values are controlled by environment variables (see `src/config/analytics.ts` and `src/config/performance.ts`). The `.env.example` file documents available variables.

## Known Failing Tests

Two tests in `src/test/code-example-copy.test.ts` have been failing since before the initial CLAUDE.md was written and are unrelated to UEPM package logic. Do not treat the test suite as broken solely because of these two tests.

## Astro-Specific Notes

- `.astro` files are not TypeScript — use `astro check` (via `npm run type-check`) rather than `tsc` to catch type errors in them.
- Pages go in `src/pages/`; Astro's file-based routing maps filenames to URLs automatically.
- React components go in `src/components/` and must be explicitly hydrated with an Astro client directive (e.g. `client:load`) when interactivity is needed. Default to static `.astro` components unless interactivity is required.
- The build is fully static (`output: 'static'` in `astro.config.mjs`). No server-side rendering.
