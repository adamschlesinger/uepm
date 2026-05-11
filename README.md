# UEPM — Unreal Engine Package Manager

A standalone CLI for managing Unreal Engine plugins via the npm registry. No Node.js required.

## Install

**macOS / Linux**
```sh
curl -fsSL https://github.com/bad-planning/uepm/releases/latest/download/install.sh | sh
```

**Windows (PowerShell)**
```powershell
irm https://github.com/bad-planning/uepm/releases/latest/download/install.ps1 | iex
```

Or grab a binary directly from [Releases](https://github.com/bad-planning/uepm/releases).

## Quick Start

```sh
cd YourUnrealProject    # directory containing a .uproject file
uepm init               # creates uepm.ini, UEPMPlugins/, modifies .uproject
uepm install @acme/cool-plugin
```

## Commands

| Command | Description |
|---|---|
| `uepm init [--yes]` | Initialize a project. Detects VCS, prompts for install mode. `--yes` accepts defaults. |
| `uepm install [@scope/pkg[@ver] ...]` | Install plugins. No args installs everything in `uepm.ini`. |
| `uepm uninstall @scope/pkg` | Remove a plugin and update `uepm.ini`. |
| `uepm update [@scope/pkg]` | Update one or all plugins to latest compatible versions. |
| `uepm list` | Show installed plugins and engine compatibility status. |

### Install modes

Chosen during `uepm init`, stored in `uepm.ini`. UEPM detects your VCS and suggests the right default.

| Mode | Behavior | When it's the default |
|---|---|---|
| `symlink` | Symlinks in `UEPMPlugins/` pointing into a cache | Git repo |
| `copy` | Real files copied into `UEPMPlugins/` | Perforce or Windows |
| `none` | UEPM manages `uepm.ini`/`uepm.lock` only | Manual preference |

`copy` mode produces real files that can be checked into Perforce.

## Project files

**`uepm.ini`** — human-edited, check this in:
```ini
[settings]
engine_version = 5.4
install_mode = symlink

[plugins]
@acme/cool-plugin = ^1.2.0
@acme/base-utils = ^2.0.0
```

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

Plugins are standard npm packages with UEPM metadata. Publish with any npm-compatible registry.

**Minimum `package.json`:**
```json
{
  "name": "@your-scope/plugin-name",
  "version": "1.0.0",
  "main": "YourPlugin.uplugin",
  "unreal": {
    "engineVersion": ">=5.0.0 <6.0.0",
    "pluginName": "YourPlugin"
  },
  "files": ["YourPlugin.uplugin", "Source/**/*", "Content/**/*", "Resources/**/*"],
  "keywords": ["unreal", "unreal-engine", "plugin", "uepm"]
}
```

```sh
npm publish --access public
```

Plugin authoring tooling (`uepm publish`, `uepm new`) is planned for Phase 2.

## Plugin dependencies

A plugin can declare its own UEPM dependencies in a `uepm.ini` at its package root. UEPM reads this after extraction and installs those deps recursively, with conflict detection.

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
