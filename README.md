# UEPM — Unreal Engine Plugin Manager

A standalone CLI for managing Unreal Engine plugins via the npm registry. No Node.js required.

## Install

**macOS / Linux**
```sh
curl -fsSL https://github.com/adamschlesinger/uepm/releases/latest/download/install.sh | sh
```

**Windows (PowerShell)**
```powershell
irm https://github.com/adamschlesinger/uepm/releases/latest/download/install.ps1 | iex
```

Or grab a binary directly from [Releases](https://github.com/adamschlesinger/uepm/releases).

## Quick Start

**Plugin consumer:**
```sh
cd YourUnrealProject    # directory containing a .uproject file
uepm init               # creates Config/UEPM.ini, UEPMPlugins/, modifies .uproject
uepm install @acme/cool-plugin
```

**Plugin author:**
```sh
cd YourPlugin           # directory containing a .uplugin file
uepm init               # detects .uplugin, prompts for plugin metadata, writes [Plugin]
uepm publish            # builds .tgz, PUTs to registry — no npm required
```

## Commands

| Command | Description |
|---|---|
| `uepm init [--yes]` | **Project context** (`.uproject` present): creates `Config/UEPM.ini`, `UEPMPlugins/`, modifies `.uproject`. **Plugin context** (`.uplugin` present): prompts for publish metadata and writes `[Plugin]` section. |
| `uepm install [@scope/pkg[@ver] ...]` | Install plugins. No args installs everything in `Config/UEPM.ini`. |
| `uepm uninstall @scope/pkg` | Remove a plugin and update `Config/UEPM.ini`. |
| `uepm update [@scope/pkg]` | Update one or all plugins to latest compatible versions. |
| `uepm list` | Show installed plugins and engine compatibility status. |
| `uepm publish [--tag TAG] [--dry-run] [--yes] [--access public\|restricted]` | Publish this plugin to the registry. Reads `[Plugin]` from `Config/UEPM.ini`. Requires `UEPM_TOKEN`. |

## Project files

**`Config/UEPM.ini`** — human-edited, check this in:
```toml
[Settings]
EngineVersion = "5.4"
CommitPlugins = false

[Dependencies]
"@acme/cool-plugin" = "^1.2.0"
"@acme/base-utils" = "^2.0.0"
```

`CommitPlugins = false` means `UEPMPlugins/` is gitignored and restored by `uepm install` after clone.
Set to `true` to check the installed plugins into version control (recommended for Perforce).

**`uepm.lock`** — machine-generated, check this in:
```json
{
  "version": 1,
  "plugins": {
    "@acme/cool-plugin": {
      "resolved": "1.2.3",
      "tarball": "https://registry.npmjs.org/...",
      "sha512": "sha512-...",
      "dependencies": {}
    }
  }
}
```

## Publishing plugins

Run `uepm init` in your plugin directory to write a `[Plugin]` section:

```toml
# Config/UEPM.ini (inside the plugin source tree)
[Plugin]
Name        = "@your-scope/plugin-name"
Version     = "1.0.0"
Description = "Does cool things"
Author      = "Your Studio"
License     = "MIT"
EngineRange = ">=5.3.0, <6.0.0"
Main        = "YourPlugin.uplugin"

[Dependencies]
# transitive deps go here
```

Then publish — no Node.js or npm required:

```sh
export UEPM_TOKEN=<your-npm-token>
uepm publish                  # interactive confirm + upload
uepm publish --dry-run        # validate and list files without uploading
uepm publish --tag beta       # publish under a non-default dist-tag
```

`uepm publish` builds the `.tgz` tarball in memory, computes SHA-512 integrity,
and PUTs directly to the registry API. `package.json` is never written to disk.

## Plugin dependencies

A plugin can declare its own UEPM dependencies in a `Config/UEPM.ini` at its package root. UEPM reads this after extraction and installs those deps recursively, with conflict detection.

### Local development

Use `file:` paths to work against local plugin source without publishing:

```toml
[Dependencies]
"@acme/cool-plugin" = "file:../plugins/cool-plugin"
```

`file:` deps create a symlink in `UEPMPlugins/` pointing at the source directory — edits are visible immediately.

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `UEPM_REGISTRY` | `https://registry.npmjs.org` | Registry base URL |
| `UEPM_TOKEN` | — | Bearer token for private registries |
| `RUST_LOG` | — | Log level (`debug`, `info`, `warn`, `error`) |

## Contributing

```sh
cargo build
cargo test
cargo test registry      # run tests for a specific module
```

Modules: `manifest`, `lockfile`, `uproject`, `registry`, `installer`, `resolver`, `commands/`

## License

MIT — see [LICENSE](./LICENSE)
