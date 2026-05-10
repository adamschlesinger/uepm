import * as path from 'path';
import { promises as fs } from 'fs';
import { PackageJson } from './types';
import { PluginMetadata } from './uplugin-manager';

/**
 * Plugin-specific package.json configuration
 */
export interface PluginPackageJsonOptions {
  force?: boolean;
  engineVersion?: string;
}

/**
 * Check if a plugin has source code modules by examining directory structure
 * @param pluginDirectory - Directory containing the plugin
 * @returns Promise<boolean> indicating if source modules exist
 */
async function hasSourceModules(pluginDirectory: string): Promise<boolean> {
  try {
    const sourceDir = path.join(pluginDirectory, 'Source');
    const stats = await fs.stat(sourceDir);
    return stats.isDirectory();
  } catch {
    return false;
  }
}

/**
 * Generate gitignore patterns appropriate for Unreal Engine plugins
 * @returns Array of gitignore patterns
 */
function generatePluginGitignorePatterns(): string[] {
  return [
    '# Unreal Engine plugin build artifacts',
    'Binaries/',
    'Intermediate/',
    'DerivedDataCache/',
    '*.tmp',
    '*.log',
    '',
    '# Node.js dependencies',
    'node_modules/',
    'npm-debug.log*',
    'yarn-debug.log*',
    'yarn-error.log*',
    '',
    '# IDE files',
    '.vscode/',
    '.idea/',
    '*.swp',
    '*.swo',
    '*~',
    '',
    '# OS generated files',
    '.DS_Store',
    '.DS_Store?',
    '._*',
    '.Spotlight-V100',
    '.Trashes',
    'ehthumbs.db',
    'Thumbs.db'
  ];
}

/**
 * Create or update .gitignore file with plugin-appropriate patterns
 * @param pluginDirectory - Directory containing the plugin
 */
async function createPluginGitignore(pluginDirectory: string): Promise<void> {
  const gitignorePath = path.join(pluginDirectory, '.gitignore');
  const patterns = generatePluginGitignorePatterns();

  try {
    const existingContent = await fs.readFile(gitignorePath, 'utf-8');
    const existingLines = new Set(existingContent.split('\n').map(line => line.trim()));

    // Determine which real (non-blank, non-comment) patterns are missing
    const missingPatterns = patterns.filter(
      pattern => pattern !== '' && !pattern.startsWith('#') && !existingLines.has(pattern.trim())
    );

    if (missingPatterns.length > 0) {
      const updatedContent = existingContent.trimEnd() + '\n\n' + missingPatterns.join('\n') + '\n';
      await fs.writeFile(gitignorePath, updatedContent);
    }
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && (error as NodeJS.ErrnoException).code === 'ENOENT') {
      await fs.writeFile(gitignorePath, patterns.join('\n') + '\n');
    } else {
      throw error;
    }
  }
}

/**
 * Generate build scripts based on plugin structure
 * @param pluginDirectory - Directory containing the plugin
 * @param pluginName - Name of the plugin
 * @returns Object containing build scripts
 */
async function generateBuildScripts(pluginDirectory: string, pluginName: string): Promise<Record<string, string>> {
  const hasSource = await hasSourceModules(pluginDirectory);
  
  if (hasSource) {
    // Plugin has source modules - provide meaningful build scripts
    return {
      'build': 'echo "Building plugin with source modules..." && npm run test',
      'clean': 'echo "Cleaning plugin build artifacts..." && rm -rf Binaries/ Intermediate/',
      'prebuild': 'npm run clean',
      'postbuild': 'echo "Plugin build completed successfully"'
    };
  } else {
    // Plugin is content-only - provide basic scripts
    return {
      'build': 'echo "Content-only plugin - no build required"',
      'clean': 'echo "Cleaning temporary files..." && find . -name "*.tmp" -delete || true'
    };
  }
}

/**
 * Generate NPM publish configuration that excludes unnecessary files
 * @param pluginDirectory - Directory containing the plugin
 * @param upluginFileName - Name of the .uplugin file
 * @returns Array of file patterns to include in NPM package
 */
async function generatePublishFiles(pluginDirectory: string, upluginFileName: string): Promise<string[]> {
  const baseFiles = [
    upluginFileName,
    'README.md',
    'LICENSE*',
    'CHANGELOG*'
  ];
  
  // Check which directories exist and include them conditionally
  const conditionalDirs = ['Source', 'Content', 'Resources', 'Config'];
  const existingDirs: string[] = [];
  
  for (const dir of conditionalDirs) {
    try {
      const dirPath = path.join(pluginDirectory, dir);
      const stats = await fs.stat(dirPath);
      if (stats.isDirectory()) {
        existingDirs.push(`${dir}/**/*`);
      }
    } catch {
      // Directory doesn't exist, skip it
    }
  }
  
  return [...baseFiles, ...existingDirs];
}

/**
 * Generate a plugin-specific package.json configuration
 * @param metadata - Plugin metadata extracted from .uplugin file
 * @param upluginPath - Path to the .uplugin file
 * @param options - Additional configuration options
 * @returns PackageJson object configured for plugin distribution
 */
export function generatePluginPackageJson(
  metadata: PluginMetadata,
  upluginPath: string,
  options: PluginPackageJsonOptions = {}
): PackageJson {
  const upluginFileName = path.basename(upluginPath);
  const pluginDirectory = path.dirname(upluginPath);
  
  // Determine engine version compatibility
  const engineVersion = options.engineVersion ||
                       metadata.engineVersion ||
                       '>=5.0.0 <6.0.0'; // Default to UE5 compatibility
  
  // Generate package name from plugin name (convert to kebab-case for NPM)
  const packageName = metadata.name
    .replace(/([A-Z])/g, '-$1')
    .toLowerCase()
    .replace(/^-/, '');
  
  // Standard plugin files to include in NPM package (will be refined by generatePublishFiles)
  const pluginFiles = [
    upluginFileName,
    'Source/**/*',
    'Content/**/*',
    'Resources/**/*',
    'Config/**/*',
    'README.md',
    'LICENSE*',
    'CHANGELOG*'
  ];
  
  // Plugin-specific keywords for NPM discovery
  const pluginKeywords = [
    'unreal-engine',
    'ue5',
    'ue4',
    'plugin',
    'gamedev',
    'game-development'
  ];
  
  // Add category-specific keywords if available
  if (metadata.description) {
    // Extract potential keywords from description
    const descriptionKeywords = metadata.description
      .toLowerCase()
      .split(/\s+/)
      .filter(word => word.length > 3 && /^[a-z]+$/.test(word))
      .slice(0, 3); // Limit to 3 additional keywords
    
    pluginKeywords.push(...descriptionKeywords);
  }
  
  const packageJson: PackageJson = {
    name: packageName,
    version: metadata.version,
    description: metadata.description || metadata.friendlyName || `Unreal Engine plugin: ${metadata.name}`,
    main: upluginFileName,
    files: pluginFiles,
    keywords: pluginKeywords,
    scripts: {
      test: 'vitest --run',
      'test:watch': 'vitest'
    },
    unreal: {
      engineVersion: engineVersion,
      pluginName: metadata.name
    },
    engines: {
      node: '>=16.0.0'
    },
    license: 'MIT'
  };
  
  // Add optional metadata if available
  if (metadata.author) {
    packageJson.author = metadata.author;
  }
  
  if (metadata.homepage) {
    packageJson.homepage = metadata.homepage;
  }
  
  // Add development dependencies for testing
  packageJson.devDependencies = {
    'vitest': '^3.0.0',
    '@types/node': '^22.0.0'
  };
  
  // Add build scripts (placeholder - will be enhanced by generateBuildScripts)
  packageJson.scripts = {
    ...packageJson.scripts,
    'build': 'echo "Build script - implement based on your plugin structure"',
    'clean': 'echo "Clean script - implement based on your plugin structure"'
  };
  
  return packageJson;
}

/**
 * Enhanced plugin package.json generation with development configuration
 * @param metadata - Plugin metadata extracted from .uplugin file
 * @param upluginPath - Path to the .uplugin file
 * @param options - Additional configuration options
 * @returns Promise<PackageJson> object configured for plugin distribution with development features
 */
export async function generatePluginPackageJsonWithDevConfig(
  metadata: PluginMetadata,
  upluginPath: string,
  options: PluginPackageJsonOptions = {}
): Promise<PackageJson> {
  const upluginFileName = path.basename(upluginPath);
  const pluginDirectory = path.dirname(upluginPath);
  
  // Generate base package.json
  const basePackageJson = generatePluginPackageJson(metadata, upluginPath, options);
  
  // Generate development-specific configurations
  const [buildScripts, publishFiles] = await Promise.all([
    generateBuildScripts(pluginDirectory, metadata.name),
    generatePublishFiles(pluginDirectory, upluginFileName)
  ]);
  
  // Create or update .gitignore file
  try {
    await createPluginGitignore(pluginDirectory);
  } catch (error) {
    // Log warning but don't fail the entire process
    console.warn(`Warning: Could not create/update .gitignore: ${error}`);
  }
  
  // Merge build scripts with existing scripts
  const enhancedPackageJson: PackageJson = {
    ...basePackageJson,
    files: publishFiles,
    scripts: {
      ...basePackageJson.scripts,
      ...buildScripts
    }
  };
  
  return enhancedPackageJson;
}

/**
 * Merge plugin-specific configuration with existing package.json
 * @param existingPackageJson - Existing package.json content
 * @param pluginPackageJson - Plugin-specific package.json configuration
 * @param force - Whether to overwrite existing plugin-specific fields
 * @returns Merged PackageJson object
 */
export function mergePluginPackageJson(
  existingPackageJson: PackageJson,
  pluginPackageJson: PackageJson,
  force: boolean = false
): PackageJson {
  const merged = { ...existingPackageJson };
  
  // Always update these plugin-specific fields if force is true
  const pluginSpecificFields = ['main', 'unreal', 'files'];
  
  for (const field of pluginSpecificFields) {
    if (force || !merged[field]) {
      merged[field] = pluginPackageJson[field];
    }
  }
  
  // Merge keywords, avoiding duplicates
  if (pluginPackageJson.keywords) {
    const existingKeywords = merged.keywords || [];
    const newKeywords = pluginPackageJson.keywords.filter(
      keyword => !existingKeywords.includes(keyword)
    );
    merged.keywords = [...existingKeywords, ...newKeywords];
  }
  
  // Merge scripts, preserving existing ones unless force is true
  if (pluginPackageJson.scripts) {
    merged.scripts = merged.scripts || {};
    for (const [scriptName, scriptValue] of Object.entries(pluginPackageJson.scripts)) {
      if (force || !merged.scripts[scriptName]) {
        merged.scripts[scriptName] = scriptValue;
      }
    }
  }
  
  // Merge devDependencies, preserving existing ones
  if (pluginPackageJson.devDependencies) {
    merged.devDependencies = merged.devDependencies || {};
    for (const [depName, depVersion] of Object.entries(pluginPackageJson.devDependencies)) {
      if (!merged.devDependencies[depName]) {
        merged.devDependencies[depName] = depVersion;
      }
    }
  }
  
  // Update metadata fields if not present or if force is true
  const metadataFields = ['description', 'author', 'homepage', 'license'];
  for (const field of metadataFields) {
    if ((force || !merged[field]) && pluginPackageJson[field]) {
      merged[field] = pluginPackageJson[field];
    }
  }
  
  return merged;
}

/**
 * Validate that a package.json is properly configured for plugin distribution
 * @param packageJson - Package.json to validate
 * @param expectedPluginName - Expected plugin name from .uplugin file
 * @returns Validation result with any issues found
 */
export function validatePluginPackageJson(
  packageJson: PackageJson,
  expectedPluginName: string
): { valid: boolean; issues: string[] } {
  const issues: string[] = [];
  
  // Check main field points to .uplugin file
  if (!packageJson.main || !packageJson.main.endsWith('.uplugin')) {
    issues.push('Main field should point to the .uplugin file');
  }
  
  // Check unreal section is present and configured
  if (!packageJson.unreal) {
    issues.push('Missing unreal section in package.json');
  } else {
    if (!packageJson.unreal.pluginName) {
      issues.push('Missing pluginName in unreal section');
    } else if (packageJson.unreal.pluginName !== expectedPluginName) {
      issues.push(`Plugin name mismatch: expected ${expectedPluginName}, got ${packageJson.unreal.pluginName}`);
    }
    
    if (!packageJson.unreal.engineVersion) {
      issues.push('Missing engineVersion in unreal section');
    }
  }
  
  // Check files array includes essential plugin files
  if (!packageJson.files || !Array.isArray(packageJson.files)) {
    issues.push('Missing or invalid files array');
  } else {
    const hasUpluginFile = packageJson.files.some(file => file.endsWith('.uplugin'));
    if (!hasUpluginFile) {
      issues.push('Files array should include the .uplugin file');
    }
  }
  
  // Check for appropriate keywords
  if (!packageJson.keywords || !Array.isArray(packageJson.keywords)) {
    issues.push('Missing keywords array');
  } else {
    const hasUnrealKeywords = packageJson.keywords.some(keyword => 
      keyword.includes('unreal') || keyword.includes('ue4') || keyword.includes('ue5')
    );
    if (!hasUnrealKeywords) {
      issues.push('Should include Unreal Engine related keywords');
    }
  }
  
  return {
    valid: issues.length === 0,
    issues
  };
}