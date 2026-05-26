# examples/ — Example Projects Guide

These are real Unreal Engine project/plugin structures used to manually test UEPM and demonstrate correct layouts. They are **not** part of the automated test suite.

## Structure

```
examples/
├── plugins/
│   ├── example-plugin/         — publishable plugin (has [Plugin] section, no deps)
│   └── dependency-plugin/      — publishable plugin that depends on example-plugin
└── project/                    — sample UE project that consumes plugins via UEPM
```

## examples/plugins/example-plugin
A standalone plugin ready to publish to the npm registry.

- `ExamplePlugin.uplugin` — UE plugin descriptor
- `Config/UEPM.ini` — has `[Plugin]` section with name `@uepm/example-plugin@2.0.4`, engine range `>=5.7.0, <6.0.0`
- No `[Dependencies]` — leaf plugin

Demonstrates: what a correctly initialized plugin directory looks like after `uepm init`.

## examples/plugins/dependency-plugin
A plugin that depends on `example-plugin`, demonstrating transitive dependency resolution.

- `Config/UEPM.ini` — has `[Plugin]` + `[Dependencies]` with `"@uepm/example-plugin" = "^2.0.0"`
- `uepm.lock` — locked resolved versions
- `UEPMPlugins/` — installed dependency (example-plugin extracted here)
- `.gitignore` — excludes `UEPMPlugins/` when `CommitPlugins = false`

Demonstrates: transitive deps, lockfile generation, and the plugin-in-plugin layout.

## examples/project
A sample Unreal Engine project consuming UEPM-managed plugins.

- `Config/UEPM.ini` — `[Settings]` with `EngineVersion = "5.7"` + `[Dependencies]` pointing to `@uepm/dependency-plugin`
- `UEPMPlugins/` — installed plugins directory
- `Source/SampleProject/` — minimal UE project source

Demonstrates: project-context `uepm init`, `uepm install`, and plugin consumption from a UE project.

## Manual Testing Workflow

```bash
# From examples/plugins/example-plugin — test publish dry-run
cd examples/plugins/example-plugin
uepm publish --dry-run

# From examples/project — test install
cd examples/project
uepm install  # re-installs all locked deps

# From examples/plugins/dependency-plugin — test transitive install
cd examples/plugins/dependency-plugin
uepm install
```
