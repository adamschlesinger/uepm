# UEPM - Unreal Engine Plugin Manager

A modern plugin management system that brings the NPM ecosystem to Unreal Engine plugin development. UEPM allows you to distribute, install, and manage Unreal Engine plugins using familiar NPM workflows.

## Features

- **🚀 One-command setup**: Initialize any Unreal project with `npx @uepm/init`
- **📦 NPM distribution**: Publish and install plugins using standard NPM commands
- **🔍 Automatic validation**: Engine compatibility checking on every install
- **🔗 Dependency management**: Plugins can depend on other plugins with automatic resolution
- **🩹 Patch support**: Full compatibility with [patch-package](https://github.com/ds300/patch-package) for modifications to managed plugins
- **⚡ Lightweight**: Only download what you need - init runs once via npx

## Quick Start

### 1. Initialize your Unreal project (one-time setup)

Navigate to your Unreal Engine project directory and run:

```bash
cd YourUnrealProject
npx @uepm/init
```

This will:
- Add `node_modules` to your project's plugin search paths
- Create or update `package.json` with UEPM configuration
- Install validation hooks for compatibility checking

### 2. Install plugins from NPM

```bash
npm install @uepm/example-plugin
```

### 3. Open your project in Unreal Engine

Plugins from `node_modules` will be automatically discovered and loaded!

## Try the Complete Example

For a working demonstration:

```bash
cd samples/project
npm install
# Open SampleProject.uproject in Unreal Engine
```

The sample project includes example plugins and demonstrates the complete workflow.

## CLI Commands

### `npx @uepm/init`

Initialize an Unreal Engine project **or plugin** for NPM support. The command automatically detects context from the files in the current directory.

**Usage:**
```bash
npx @uepm/init [options]
```

**Options:**
- `-f, --force` - Force reinitialization even if already initialized
- `-d, --project-dir <path>` - Project directory (defaults to current directory)
- `-h, --help` - Display help information
- `-V, --version` - Display version number

#### Project initialization

Run in a directory containing a `.uproject` file:

```bash
cd YourUnrealProject
npx @uepm/init
```

This will:
- Add `UEPMPlugins` to your project's `AdditionalPluginDirectories`
- Create or update `package.json` with the `uepm-postinstall` hook

#### Plugin initialization

Run in a directory containing a `.uplugin` file:

```bash
cd YourPlugin
npx @uepm/init
```

This will:
- Read metadata from your `.uplugin` file (name, version, description, author)
- Create or update `package.json` configured for NPM distribution
- Set the `unreal.engineVersion` compatibility range
- Include a `files` array covering `Source/`, `Content/`, `Resources/`, and `Config/`
- Add build scripts if a `Source/` directory is detected
- Create or update `.gitignore` with Unreal Engine plugin patterns

**Examples:**
```bash
# Initialize current directory (project or plugin, auto-detected)
npx @uepm/init

# Force reinitialize
npx @uepm/init --force

# Initialize a specific directory
npx @uepm/init --project-dir ./MyPlugin
```

After plugin initialization, review the generated `package.json` and then publish:

```bash
# Review and adjust the generated configuration
cat package.json

# Publish to NPM
npm publish
```

### `uepm-postinstall`

Validation hook that runs automatically after `npm install`. This command is typically not run manually.

**Usage:**
```bash
uepm-postinstall [project-directory]
```

This command:
- Sets up plugin symlinks in the `UEPMPlugins` directory
- Validates plugin compatibility with your engine version
- Displays warnings for incompatible plugins

## Plugin Package Structure

To create UEPM-compatible plugins, your `package.json` must include specific metadata:

### Required Fields

```json
{
  "name": "@your-scope/plugin-name",
  "version": "1.0.0",
  "description": "Your plugin description",
  "main": "YourPlugin.uplugin",
  "unreal": {
    "engineVersion": ">=5.0.0 <6.0.0",
    "pluginName": "YourPlugin"
  },
  "files": [
    "YourPlugin.uplugin",
    "Source/**/*",
    "Resources/**/*",
    "Content/**/*"
  ],
  "keywords": ["unreal", "unreal-engine", "plugin", "uepm"],
  "license": "MIT"
}
```

### Engine Version Compatibility

Use semantic versioning ranges to specify engine compatibility:

| Range | Description | Example |
|-------|-------------|---------|
| `">=5.0.0 <6.0.0"` | UE 5.x only | Most common for UE5 plugins |
| `"^5.3.0"` | UE 5.3+ but not 6.0 | Compatible with 5.3, 5.4, etc. |
| `"~5.3.0"` | UE 5.3.x only | Patch versions of 5.3 only |
| `">=4.27.0"` | UE 4.27 and later | Cross-generation compatibility |
| `"5.3.0"` | Exact version | Only UE 5.3.0 (not recommended) |

### Plugin Dependencies

Declare dependencies on other UEPM plugins:

```json
{
  "dependencies": {
    "@uepm/utility-plugin": "^1.0.0",
    "@company/base-plugin": "~2.1.0"
  }
}
```

Also declare them in your `.uplugin` file:

```json
{
  "Plugins": [
    {
      "Name": "UtilityPlugin",
      "Enabled": true
    }
  ]
}
```

## Patch-Package Integration

UEPM is fully compatible with [patch-package](https://github.com/ds300/patch-package) for persisting modifications to plugin source code.

### Making Changes

1. **Modify plugin source** in `node_modules`:
   ```bash
   # Edit the plugin files
   code node_modules/@uepm/example-plugin/Source/ExamplePlugin/Private/ExamplePlugin.cpp
   ```

2. **Create a patch**:
   ```bash
   npx patch-package @uepm/example-plugin
   ```

3. **Commit the patch file**:
   ```bash
   git add patches/
   git commit -m "Patch example plugin for custom behavior"
   ```

### Automatic Application

Patches are automatically applied during `npm install` thanks to the postinstall script that UEPM sets up:

```json
{
  "scripts": {
    "postinstall": "patch-package && uepm-postinstall"
  }
}
```

### Best Practices

- **Keep patches minimal** - only change what's necessary
- **Document your changes** - add comments explaining modifications
- **Test thoroughly** - ensure patches work across different environments
- **Version carefully** - patches are tied to specific plugin versions

## Troubleshooting

### Common Issues

#### "No .uproject file found"

**Problem**: Running `npx @uepm/init` in wrong directory.

**Solution**: Navigate to your Unreal Engine project root (where the `.uproject` file is located).

```bash
cd path/to/your/UnrealProject
npx @uepm/init
```

#### Plugin not loading in Unreal Engine

**Problem**: Plugin appears in `node_modules` but doesn't load in Unreal Engine.

**Solutions**:
1. **Check the Output Log** in Unreal Engine for error messages
2. **Verify plugin structure** - ensure valid `.uplugin` file exists
3. **Regenerate project files** - close Unreal Engine and regenerate
4. **Check engine compatibility** - run `npx uepm-postinstall` to see warnings

#### Engine compatibility warnings

**Problem**: Seeing warnings about plugin compatibility during `npm install`.

**Example warning**:
```
⚠️ Plugin @uepm/example-plugin@1.0.0 may be incompatible
   Required: >=5.0.0 <6.0.0, Found: 4.27.2
```

**Solutions**:
1. **Update Unreal Engine** to a compatible version
2. **Contact plugin author** for compatibility updates
3. **Use at your own risk** - plugin may still work despite warnings
4. **Fork and update** the plugin for your engine version

#### Build errors after installing plugins

**Problem**: C++ compilation errors after installing plugins.

**Solutions**:
1. **Clean and rebuild** your project
2. **Check for conflicting dependencies** between plugins
3. **Verify plugin source compatibility** with your engine version
4. **Review applied patches** - ensure they're still valid

#### NPM installation failures

**Problem**: `npm install` fails with network or permission errors.

**Solutions**:
```bash
# Clear NPM cache
npm cache clean --force

# Delete and reinstall
rm -rf node_modules package-lock.json
npm install

# Check NPM configuration
npm config list
```

#### Multiple .uproject files

**Problem**: Directory contains multiple `.uproject` files.

**Solution**: UEPM will use the first `.uproject` file found alphabetically. Move to a directory with only one project file, or specify the project directory:

```bash
npx @uepm/init --project-dir ./SpecificProject
```

### Getting Help

1. **Check the samples** - [`samples/`](./samples) contains working examples
2. **Review plugin structure** - examine `samples/plugins/` for reference
3. **Enable verbose logging** - check Unreal Engine's Output Log for details
4. **Test with sample project** - verify UEPM works with `samples/project/`

## Architecture

### System Components

UEPM consists of several focused packages:

#### Core Packages

- **[@uepm/init](./packages/init)** - One-time initialization tool (run via npx)
- **[@uepm/postinstall](./packages/postinstall)** - Postinstall hooks for setup and validation
- **[@uepm/core](./packages/core)** - Shared utilities for file management

#### Sample Packages

- **[@uepm/example-plugin](./samples/plugins/example-plugin)** - Basic plugin example
- **[@uepm/dependency-plugin](./samples/plugins/dependency-plugin)** - Plugin with dependencies

### How It Works

1. **Initialization**: `@uepm/init` modifies your `.uproject` file to include `node_modules` in plugin search paths
2. **Installation**: Standard `npm install` downloads plugins to `node_modules`
3. **Setup**: Postinstall hook creates symlinks in `UEPMPlugins/` directory for Unreal Engine
4. **Validation**: Automatic compatibility checking warns about engine version mismatches
5. **Loading**: Unreal Engine discovers and loads plugins from the configured directories

### Project Structure

```
your-unreal-project/
├── YourProject.uproject          # Modified by UEPM init
├── package.json                  # NPM configuration
├── UEPMPlugins/                  # Symlinks to plugins (for UE)
│   ├── example-plugin/           # → ../node_modules/@uepm/example-plugin
│   └── dependency-plugin/        # → ../node_modules/@uepm/dependency-plugin
├── node_modules/@uepm/           # Actual plugin files
│   ├── example-plugin/
│   └── dependency-plugin/
├── patches/                      # patch-package modifications
└── Source/                       # Your project source
```

## Contributing

We welcome contributions to UEPM! Here's how to get started:

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-org/uepm.git
   cd uepm
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Build all packages**:
   ```bash
   npm run build
   ```

4. **Run tests**:
   ```bash
   npm test
   ```

### Working on Packages

Each package can be developed independently:

```bash
# Work on the init package
cd packages/init
npm run build
npm test

# Work on the core utilities
cd packages/core
npm run build
npm test
```

### Testing Changes

1. **Unit tests**: Each package has comprehensive unit tests
2. **Integration tests**: Test the complete workflow with sample projects
3. **Property-based tests**: Verify correctness properties with random inputs

```bash
# Run all tests
npm test

# Run tests for specific package
cd packages/init && npm test

# Run tests in watch mode
cd packages/core && npm run test:watch
```

### Creating Example Plugins

To contribute example plugins:

1. **Follow the structure** in `samples/plugins/example-plugin/`
2. **Include comprehensive tests** for plugin structure and functionality
3. **Document the example** with clear README and comments
4. **Test with sample project** to ensure compatibility

### Submitting Changes

1. **Fork the repository** and create a feature branch
2. **Make your changes** with appropriate tests
3. **Ensure all tests pass** - run `npm test` from the root
4. **Update documentation** if needed
5. **Submit a pull request** with a clear description

### Code Style

- **TypeScript**: All code should be written in TypeScript
- **Testing**: Use Vitest for unit tests and fast-check for property-based tests
- **Documentation**: Include JSDoc comments for public APIs
- **Formatting**: Follow the existing code style (we may add Prettier in the future)

### Release Process

Releases are managed through NPM workspaces:

1. **Update versions** in affected packages
2. **Build all packages**: `npm run build`
3. **Run full test suite**: `npm test`
4. **Publish packages**: `npm publish --workspaces`

## Requirements

- **Node.js** >= 18.0.0
- **NPM** >= 7.0.0 (for workspaces support)
- **Unreal Engine** 5.0+ (4.27+ may work but is not officially supported)

## License

MIT - see [LICENSE](./LICENSE) for details.
