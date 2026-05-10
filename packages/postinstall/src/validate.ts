import { promises as fs } from 'fs';
import * as path from 'path';
import * as semver from 'semver';
import { UProjectFile, PackageJson } from '@uepm/core';

export interface PluginInfo {
  name: string;
  version: string;
  engineVersion?: string; // Semver range from package.json
  path: string;
}

export interface CompatibilityResult {
  compatible: boolean;
  reason?: string;
}

export interface ValidationResult {
  compatible: PluginInfo[];
  incompatible: PluginInfo[];
  warnings: string[];
}

/**
 * Extract engine version from .uproject file
 * @param projectFile - Parsed UProjectFile object
 * @returns Engine version string
 */
export function getEngineVersion(projectFile: UProjectFile): string {
  return projectFile.EngineAssociation;
}

/**
 * Find all installed plugins in node_modules
 * @param nodeModulesDir - Path to node_modules directory
 * @returns Array of PluginInfo objects
 */
export async function findInstalledPlugins(
  nodeModulesDir: string
): Promise<PluginInfo[]> {
  const plugins: PluginInfo[] = [];

  try {
    // Check if node_modules exists
    await fs.access(nodeModulesDir);
  } catch {
    // node_modules doesn't exist, return empty array
    return plugins;
  }

  // Read all entries in node_modules
  const entries = await fs.readdir(nodeModulesDir, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.isDirectory()) {
      // Handle scoped packages (e.g., @uepm/example-plugin)
      if (entry.name.startsWith('@')) {
        const scopePath = path.join(nodeModulesDir, entry.name);
        const scopedEntries = await fs.readdir(scopePath, {
          withFileTypes: true,
        });

        for (const scopedEntry of scopedEntries) {
          if (scopedEntry.isDirectory()) {
            const pluginPath = path.join(scopePath, scopedEntry.name);
            const pluginInfo = await extractPluginInfo(
              pluginPath,
              `${entry.name}/${scopedEntry.name}`
            );
            if (pluginInfo) {
              plugins.push(pluginInfo);
            }
          }
        }
      } else {
        // Handle non-scoped packages
        const pluginPath = path.join(nodeModulesDir, entry.name);
        const pluginInfo = await extractPluginInfo(pluginPath, entry.name);
        if (pluginInfo) {
          plugins.push(pluginInfo);
        }
      }
    }
  }

  return plugins;
}

/**
 * Extract plugin info from a package directory
 * @param pluginPath - Path to the plugin directory
 * @param packageName - Name of the package
 * @returns PluginInfo object or null if not a valid plugin
 */
async function extractPluginInfo(
  pluginPath: string,
  packageName: string
): Promise<PluginInfo | null> {
  const packageJsonPath = path.join(pluginPath, 'package.json');

  try {
    const packageJsonContent = await fs.readFile(packageJsonPath, 'utf-8');
    const packageJson = JSON.parse(packageJsonContent) as PackageJson;

    // Check if this package has Unreal plugin metadata
    if (packageJson.unreal?.engineVersion) {
      return {
        name: packageName,
        version: packageJson.version,
        engineVersion: packageJson.unreal.engineVersion,
        path: pluginPath,
      };
    }

    // Check if there's a .uplugin file (alternative way to identify plugins)
    const files = await fs.readdir(pluginPath);
    const hasUplugin = files.some((file) => file.endsWith('.uplugin'));

    if (hasUplugin) {
      return {
        name: packageName,
        version: packageJson.version,
        engineVersion: packageJson.unreal?.engineVersion,
        path: pluginPath,
      };
    }
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code;
    if (code === 'ENOENT') {
      // No package.json — not a plugin directory
      return null;
    }
    // Unexpected error (malformed JSON, permission denied, etc.) — warn so the user knows
    console.warn(`⚠ Could not read package at ${pluginPath}: ${err}`);
    return null;
  }

  return null;
}

/**
 * Validate plugin compatibility with engine version
 * @param plugin - PluginInfo object
 * @param engineVersion - Engine version string
 * @returns CompatibilityResult
 */
export function validatePlugin(
  plugin: PluginInfo,
  engineVersion: string
): CompatibilityResult {
  // If plugin doesn't specify engine version, assume compatible
  if (!plugin.engineVersion) {
    return {
      compatible: true,
      reason: 'No engine version specified',
    };
  }

  // Clean the engine version (remove GUID format if present)
  const cleanEngineVersion = cleanVersionString(engineVersion);

  // If we can't parse the engine version, assume compatible but warn
  if (!semver.valid(cleanEngineVersion) && !semver.coerce(cleanEngineVersion)) {
    return {
      compatible: true,
      reason: `Cannot parse engine version: ${engineVersion}`,
    };
  }

  // Coerce the version to a valid semver if needed
  const coercedVersion = semver.coerce(cleanEngineVersion);
  if (!coercedVersion) {
    return {
      compatible: true,
      reason: `Cannot coerce engine version: ${engineVersion}`,
    };
  }

  // Check if engine version satisfies the plugin's requirement
  const satisfies = semver.satisfies(coercedVersion, plugin.engineVersion);

  if (satisfies) {
    return {
      compatible: true,
    };
  } else {
    return {
      compatible: false,
      reason: `Engine version ${engineVersion} does not satisfy ${plugin.engineVersion}`,
    };
  }
}

/**
 * Clean version string to make it semver-compatible
 * @param version - Version string (may be GUID or version number)
 * @returns Cleaned version string
 */
function cleanVersionString(version: string): string {
  // If it's a GUID (starts with { or is a UUID), extract version if possible
  if (version.startsWith('{') || version.match(/^[0-9a-f-]{36}$/i)) {
    // For GUID format, we can't determine the version
    // Return a placeholder that will be handled by the caller
    return '0.0.0';
  }

  // Remove any non-semver characters but keep dots and numbers
  return version.replace(/[^0-9.]/g, '');
}

/**
 * Format warning message for incompatible plugin
 * @param plugin - PluginInfo object
 * @param result - CompatibilityResult
 * @returns Formatted warning message
 */
export function formatWarning(
  plugin: PluginInfo,
  result: CompatibilityResult
): string {
  return `⚠️  Warning: Plugin "${plugin.name}" may be incompatible\n   Required: ${plugin.engineVersion}\n   Reason: ${result.reason}`;
}

/**
 * Main validation function
 * @param projectDir - Path to the project directory
 * @returns ValidationResult
 */
export async function validatePlugins(
  projectDir: string
): Promise<ValidationResult> {
  const result: ValidationResult = {
    compatible: [],
    incompatible: [],
    warnings: [],
  };

  try {
    // Read .uproject file
    const { readProject, findProjectFile } = await import('@uepm/core');
    const projectFilePath = await findProjectFile(projectDir);
    const projectFile = await readProject(projectFilePath);

    // Get engine version
    const engineVersion = getEngineVersion(projectFile);

    // Find installed plugins
    const nodeModulesPath = path.join(projectDir, 'node_modules');
    const plugins = await findInstalledPlugins(nodeModulesPath);

    // Validate each plugin
    for (const plugin of plugins) {
      const compatResult = validatePlugin(plugin, engineVersion);

      if (compatResult.compatible) {
        result.compatible.push(plugin);
      } else {
        result.incompatible.push(plugin);
        result.warnings.push(formatWarning(plugin, compatResult));
      }
    }
  } catch (error) {
    result.warnings.push(`Error during validation: ${error instanceof Error ? error.message : String(error)}`);
  }

  return result;
}