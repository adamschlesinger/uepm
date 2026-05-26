# docs/ — Documentation Guide

## Structure

```
docs/
└── superpowers/
    ├── plans/      — Implementation plans (completed work)
    └── specs/      — Design specs (approved designs, including future phases)
```

## Plans (Completed Implementation Work)

| File | What it covers |
|---|---|
| `plans/2026-05-11-rust-rewrite-phase1-core-cli.md` | Phase 1 Rust rewrite — core CLI, registry, installer, resolver |
| `plans/2026-05-23-phase2-plugin-authoring-impl.md` | Phase 2 — plugin authoring (`uepm init` for plugins, `uepm publish`) |

## Specs (Design Documents)

| File | What it covers |
|---|---|
| `specs/2026-05-10-rust-rewrite-core-cli-design.md` | Original design for the Rust rewrite of the core CLI |
| `specs/2026-05-11-rust-rewrite-phase2-plugin-authoring-design.md` | Design for the plugin authoring phase |
| `specs/2026-05-26-roadmap-expansion-design.md` | Phases 3–7 roadmap: `uepm search`, `uepm new`, binary distribution, studio features, monetization |

## Smoke Test Checklist

`plans/smoke-test-checklist.md` — manual verification steps for a release build. Run through this before tagging a new version.

## Key Upcoming Features (from Roadmap)

Phase 3 priorities (see `specs/2026-05-26-roadmap-expansion-design.md`):
- `uepm search <term>` — query npm registry for `uepm`-tagged packages
- `uepm new <name>` — scaffold a new plugin directory
- Homebrew/Chocolatey/Scoop automated distribution
- Live plugin directory on the gh-pages website
