# GitHub Actions

## Workflows

### `test.yml` — Continuous Integration

Triggers on push to `main` and pull requests to `main`. Runs `cargo test` on Linux.

### `release.yml` — Cross-Compile Release

Triggers on tag push matching `v*`. Builds binaries for:

| Target | Artifact |
|---|---|
| `x86_64-unknown-linux-gnu` | `uepm-linux-x86_64` |
| `x86_64-pc-windows-msvc` | `uepm-windows-x86_64.exe` |
| `aarch64-apple-darwin` | `uepm-macos-arm64` |
| `x86_64-apple-darwin` | `uepm-macos-x86_64` |

Creates a GitHub Release with all four artifacts and auto-generated release notes.

## Cutting a release

```sh
git tag v2.1.0
git push origin v2.1.0
```

The `release.yml` workflow runs automatically. The `install.sh` and `install.ps1` scripts always fetch the latest release tag, so no script changes are needed.
