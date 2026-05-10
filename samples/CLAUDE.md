# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Structure

```
samples/
├── plugins/
│   ├── example-plugin/      # Minimal reference plugin (@uepm/example-plugin)
│   └── dependency-plugin/   # Plugin that depends on example-plugin (@uepm/dependency-plugin)
├── project/                 # Sample UE 5.7 project with UEPM configured
│   ├── SampleProject.uproject  (AdditionalPluginDirectories includes "UEPMPlugins")
│   ├── package.json            (postinstall: "uepm-postinstall")
│   └── UEPMPlugins/            (symlinks to node_modules — relative paths)
└── tests/                   # Structural validation tests for the sample project
```

## Running Tests

```bash
# From repo root
npm test   # includes samples/tests

# From samples/tests directly
cd samples/tests && npm test

# Single sample plugin's own tests
cd samples/plugins/example-plugin && npm test
cd samples/plugins/dependency-plugin && npm test
```

The `samples/project/` directory itself has no test runner — all structural assertions live in `samples/tests/`.

## Sample Project

`SampleProject.uproject` uses engine `5.7` and has `UEPMPlugins` in `AdditionalPluginDirectories`. The `samples/tests/src/sample-project-structure.test.ts` validates this structure. If the engine version in the `.uproject` changes, update the assertion in that test file.

## Sample Plugins

Both plugins follow the required UEPM plugin `package.json` shape:
- `"main"` pointing to the `.uplugin` file
- `"unreal.engineVersion"` as a semver range
- `"unreal.pluginName"` matching the `.uplugin` filename stem
- `"keywords"` including `"unreal"` and `"uepm"`

When creating new sample plugins, mirror this structure exactly — it's the reference implementation that developers copy.

## Symlinks in `samples/project/UEPMPlugins/`

These symlinks must be relative (e.g. `../node_modules/@uepm/example-plugin`), not absolute. If they were ever regenerated as absolute paths, recreate them with:
```bash
cd samples/project/UEPMPlugins
ln -sfn ../node_modules/@uepm/example-plugin example-plugin
ln -sfn ../node_modules/@uepm/dependency-plugin dependency-plugin
```
