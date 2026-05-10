import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fc from 'fast-check';
import { promises as fs } from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  generatePluginPackageJson,
  generatePluginPackageJsonWithDevConfig,
  mergePluginPackageJson,
  validatePluginPackageJson,
  PluginPackageJsonOptions
} from './plugin-package-json-generator';
import { 
  pluginMetadataArbitrary, 
  pluginNameArbitrary, 
  semverArbitrary,
  packageJsonArbitrary,
  pluginPackageJsonArbitrary
} from './test-generators';
import { PluginMetadata } from './uplugin-manager';
import { PackageJson } from './types';

describe('Plugin Package.json Generator - Property-Based Tests', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create a unique temporary directory for each test
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-plugin-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  /**
   * Feature: init-plugin-support, Property 1: Plugin package.json creation
   * Validates: Requirements 1.1
   * 
   * For any directory containing a valid uplugin file, when the init command is executed,
   * a package.json file should be created with proper plugin distribution configuration.
   */
  it('Property 1: creates valid package.json for plugin distribution', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify basic package.json structure
          expect(packageJson.name).toBeDefined();
          expect(packageJson.version).toBeDefined();
          expect(packageJson.description).toBeDefined();
          expect(packageJson.main).toBeDefined();
          expect(packageJson.files).toBeDefined();
          expect(packageJson.keywords).toBeDefined();
          expect(packageJson.scripts).toBeDefined();
          expect(packageJson.unreal).toBeDefined();
          
          // Verify plugin-specific configuration
          expect(packageJson.main).toBe(`${pluginName}.uplugin`);
          expect(packageJson.unreal?.pluginName).toBe(metadata.name);
          expect(packageJson.unreal?.engineVersion).toBeDefined();
          
          // Verify files array includes essential plugin files
          expect(packageJson.files).toContain(`${pluginName}.uplugin`);
          expect(packageJson.files).toContain('Source/**/*');
          expect(packageJson.files).toContain('Content/**/*');
          
          // Verify keywords include Unreal Engine related terms
          expect(packageJson.keywords).toContain('unreal-engine');
          expect(packageJson.keywords).toContain('plugin');
          
          // Verify test scripts are included
          expect(packageJson.scripts?.test).toBeDefined();
          expect(packageJson.scripts?.['test:watch']).toBeDefined();
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 2: Plugin name extraction consistency
   * Validates: Requirements 1.2
   * 
   * For any uplugin filename, the generated package.json should have a package name
   * that correctly corresponds to the uplugin filename.
   */
  it('Property 2: extracts plugin name consistently from uplugin filename', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify main field points to correct uplugin file
          expect(packageJson.main).toBe(`${pluginName}.uplugin`);
          
          // Verify unreal.pluginName matches the metadata name
          expect(packageJson.unreal?.pluginName).toBe(metadata.name);
          
          // Verify package name is derived from plugin name (kebab-case conversion)
          const expectedPackageName = metadata.name
            .replace(/([A-Z])/g, '-$1')
            .toLowerCase()
            .replace(/^-/, '');
          expect(packageJson.name).toBe(expectedPackageName);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 3: Version information extraction
   * Validates: Requirements 1.3
   * 
   * For any uplugin file containing version information, the generated package.json
   * should extract and use that version correctly.
   */
  it('Property 3: extracts version information correctly', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify version is extracted from metadata
          expect(packageJson.version).toBe(metadata.version);
          
          // Verify version is a valid semver string
          expect(packageJson.version).toMatch(/^\d+\.\d+\.\d+/);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 4: Metadata extraction completeness
   * Validates: Requirements 1.4
   * 
   * For any uplugin file with metadata fields, the generated package.json should
   * extract and include all available relevant metadata.
   */
  it('Property 4: extracts all available metadata completely', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify description is extracted (with fallback)
          if (metadata.description) {
            expect(packageJson.description).toBe(metadata.description);
          } else if (metadata.friendlyName) {
            expect(packageJson.description).toBe(metadata.friendlyName);
          } else {
            expect(packageJson.description).toBe(`Unreal Engine plugin: ${metadata.name}`);
          }
          
          // Verify author is extracted if available
          if (metadata.author) {
            expect(packageJson.author).toBe(metadata.author);
          }
          
          // Verify homepage is extracted if available
          if (metadata.homepage) {
            expect(packageJson.homepage).toBe(metadata.homepage);
          }
          
          // Verify engine version is used if available
          if (metadata.engineVersion) {
            expect(packageJson.unreal?.engineVersion).toBe(metadata.engineVersion);
          } else {
            expect(packageJson.unreal?.engineVersion).toBe('>=5.0.0 <6.0.0');
          }
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 5: Plugin files array configuration
   * Validates: Requirements 1.5, 3.5
   * 
   * For any plugin initialization, the generated package.json should include a files array
   * that covers all standard plugin directories.
   */
  it('Property 5: configures files array with standard plugin directories', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify files array is present and is an array
          expect(Array.isArray(packageJson.files)).toBe(true);
          expect(packageJson.files!.length).toBeGreaterThan(0);
          
          // Verify essential plugin files are included
          expect(packageJson.files).toContain(`${pluginName}.uplugin`);
          expect(packageJson.files).toContain('Source/**/*');
          expect(packageJson.files).toContain('Content/**/*');
          expect(packageJson.files).toContain('Resources/**/*');
          expect(packageJson.files).toContain('Config/**/*');
          
          // Verify documentation files are included
          expect(packageJson.files).toContain('README.md');
          expect(packageJson.files).toContain('LICENSE*');
          expect(packageJson.files).toContain('CHANGELOG*');
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 6: Main field correctness
   * Validates: Requirements 1.6
   * 
   * For any plugin initialization, the generated package.json main field should point
   * to the correct uplugin file.
   */
  it('Property 6: sets main field to correct uplugin file', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify main field points to the uplugin file
          expect(packageJson.main).toBe(`${pluginName}.uplugin`);
          expect(packageJson.main).toMatch(/\.uplugin$/);
          
          // Verify main field matches the basename of the provided path
          const expectedMain = path.basename(upluginPath);
          expect(packageJson.main).toBe(expectedMain);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 7: Plugin keywords inclusion
   * Validates: Requirements 1.7
   * 
   * For any plugin package.json generation, appropriate Unreal Engine plugin keywords
   * should be included.
   */
  it('Property 7: includes appropriate Unreal Engine plugin keywords', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify keywords array is present and is an array
          expect(Array.isArray(packageJson.keywords)).toBe(true);
          expect(packageJson.keywords!.length).toBeGreaterThan(0);
          
          // Verify essential Unreal Engine keywords are included
          expect(packageJson.keywords).toContain('unreal-engine');
          expect(packageJson.keywords).toContain('plugin');
          expect(packageJson.keywords).toContain('gamedev');
          expect(packageJson.keywords).toContain('game-development');
          
          // Verify UE version keywords are included
          const hasUEVersionKeyword = packageJson.keywords!.some(keyword => 
            keyword === 'ue4' || keyword === 'ue5'
          );
          expect(hasUEVersionKeyword).toBe(true);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 9: Unreal section configuration
   * Validates: Requirements 3.1, 3.2, 3.3
   * 
   * For any plugin initialization, the generated package.json should include a properly
   * configured unreal section with engineVersion and pluginName.
   */
  it('Property 9: configures unreal section with engine version and plugin name', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        fc.option(fc.string({ minLength: 1, maxLength: 20 })),
        async (metadata, pluginName, customEngineVersion) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          const options: PluginPackageJsonOptions = {};
          
          if (customEngineVersion) {
            options.engineVersion = customEngineVersion;
          }
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath, options);
          
          // Verify unreal section is present
          expect(packageJson.unreal).toBeDefined();
          expect(typeof packageJson.unreal).toBe('object');
          
          // Verify pluginName is set correctly
          expect(packageJson.unreal?.pluginName).toBe(metadata.name);
          
          // Verify engineVersion is set
          expect(packageJson.unreal?.engineVersion).toBeDefined();
          
          // Verify engineVersion priority: custom > metadata > default
          if (customEngineVersion) {
            expect(packageJson.unreal?.engineVersion).toBe(customEngineVersion);
          } else if (metadata.engineVersion) {
            expect(packageJson.unreal?.engineVersion).toBe(metadata.engineVersion);
          } else {
            expect(packageJson.unreal?.engineVersion).toBe('>=5.0.0 <6.0.0');
          }
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 10: Plugin development setup
   * Validates: Requirements 3.4, 7.1, 7.2
   * 
   * For any plugin initialization, the generated package.json should include test scripts
   * and appropriate development dependencies.
   */
  it('Property 10: sets up plugin development environment', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify test scripts are included
          expect(packageJson.scripts).toBeDefined();
          expect(packageJson.scripts?.test).toBeDefined();
          expect(packageJson.scripts?.['test:watch']).toBeDefined();
          
          // Verify test scripts use appropriate testing framework
          expect(packageJson.scripts?.test).toContain('vitest');
          expect(packageJson.scripts?.['test:watch']).toContain('vitest');
          
          // Verify development dependencies are included
          expect(packageJson.devDependencies).toBeDefined();
          expect(packageJson.devDependencies?.['vitest']).toBeDefined();
          expect(packageJson.devDependencies?.['@types/node']).toBeDefined();
          
          // Verify build and clean scripts are included
          expect(packageJson.scripts?.build).toBeDefined();
          expect(packageJson.scripts?.clean).toBeDefined();
          
          // Verify engines field specifies Node.js requirement
          expect(packageJson.engines).toBeDefined();
          expect(packageJson.engines?.node).toBeDefined();
          expect(packageJson.engines?.node).toMatch(/>=\d+/);
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe('Plugin Package.json Merging - Property-Based Tests', () => {
  /**
   * Feature: init-plugin-support, Property 11: Existing configuration preservation
   * Validates: Requirements 4.1, 4.2
   * 
   * For any existing package.json in plugin context, the init command should preserve
   * existing configuration while adding plugin-specific fields.
   */
  it('Property 11: preserves existing configuration while adding plugin fields', async () => {
    await fc.assert(
      fc.asyncProperty(
        packageJsonArbitrary(),
        pluginPackageJsonArbitrary(),
        async (existingPackageJson, pluginPackageJson) => {
          // Merge plugin configuration with existing package.json
          const merged = mergePluginPackageJson(existingPackageJson, pluginPackageJson, false);
          
          // Verify existing fields are preserved
          expect(merged.name).toBe(existingPackageJson.name);
          expect(merged.version).toBe(existingPackageJson.version);
          
          if (existingPackageJson.description) {
            expect(merged.description).toBe(existingPackageJson.description);
          }
          
          if (existingPackageJson.author) {
            expect(merged.author).toBe(existingPackageJson.author);
          }
          
          // Verify plugin-specific fields are added if not present
          if (!existingPackageJson.main) {
            expect(merged.main).toBe(pluginPackageJson.main);
          }
          
          if (!existingPackageJson.unreal) {
            expect(merged.unreal).toEqual(pluginPackageJson.unreal);
          }
          
          // Verify keywords are merged without duplicates
          if (pluginPackageJson.keywords) {
            for (const keyword of pluginPackageJson.keywords) {
              expect(merged.keywords).toContain(keyword);
            }
          }
          
          // Verify existing keywords are preserved
          if (existingPackageJson.keywords) {
            for (const keyword of existingPackageJson.keywords) {
              expect(merged.keywords).toContain(keyword);
            }
          }
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 12: Force flag behavior
   * Validates: Requirements 4.3
   * 
   * For any plugin context with existing configuration, the force flag should cause
   * plugin-specific fields to be overwritten.
   */
  it('Property 12: force flag overwrites plugin-specific fields', async () => {
    await fc.assert(
      fc.asyncProperty(
        packageJsonArbitrary(),
        pluginPackageJsonArbitrary(),
        async (existingPackageJson, pluginPackageJson) => {
          // Ensure existing package.json has some plugin-specific fields
          const existingWithPluginFields = {
            ...existingPackageJson,
            main: 'different.uplugin',
            unreal: {
              engineVersion: '4.27.0',
              pluginName: 'DifferentPlugin'
            }
          };
          
          // Merge with force flag
          const merged = mergePluginPackageJson(existingWithPluginFields, pluginPackageJson, true);
          
          // Verify plugin-specific fields are overwritten
          expect(merged.main).toBe(pluginPackageJson.main);
          expect(merged.unreal).toEqual(pluginPackageJson.unreal);
          
          // Verify non-plugin fields are still preserved
          expect(merged.name).toBe(existingWithPluginFields.name);
          expect(merged.version).toBe(existingWithPluginFields.version);
        }
      ),
      { numRuns: 100 }
    );
  });
});

describe('Plugin Development Configuration - Property-Based Tests', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create a unique temporary directory for each test
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-plugin-dev-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  /**
   * Feature: init-plugin-support, Property 18: Plugin development configuration
   * Validates: Requirements 7.3, 7.5
   * 
   * For any plugin initialization, appropriate development configurations should be set up
   * including gitignore patterns and publish configuration.
   */
  it('Property 18: sets up appropriate development configurations', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          const upluginPath = path.join(tempDir, `${pluginName}.uplugin`);
          
          // Generate plugin package.json
          const packageJson = generatePluginPackageJson(metadata, upluginPath);
          
          // Verify NPM publish configuration excludes unnecessary files
          expect(packageJson.files).toBeDefined();
          expect(Array.isArray(packageJson.files)).toBe(true);
          
          // Verify files array includes essential plugin files but excludes build artifacts
          expect(packageJson.files).toContain(`${pluginName}.uplugin`);
          expect(packageJson.files).toContain('Source/**/*');
          expect(packageJson.files).toContain('Content/**/*');
          expect(packageJson.files).toContain('Resources/**/*');
          expect(packageJson.files).toContain('Config/**/*');
          
          // Verify documentation files are included
          expect(packageJson.files).toContain('README.md');
          expect(packageJson.files).toContain('LICENSE*');
          expect(packageJson.files).toContain('CHANGELOG*');
          
          // Verify build artifacts and temporary files are NOT included
          const excludedPatterns = [
            'Binaries/**/*',
            'Intermediate/**/*',
            'node_modules/**/*',
            '.git/**/*',
            '*.tmp',
            '*.log'
          ];
          
          for (const pattern of excludedPatterns) {
            expect(packageJson.files).not.toContain(pattern);
          }
          
          // Verify package is set to public by default (not private)
          expect(packageJson.private).not.toBe(true);
          
          // Verify appropriate license is set
          expect(packageJson.license).toBeDefined();
          expect(typeof packageJson.license).toBe('string');
          
          // Verify engines field specifies Node.js requirement
          expect(packageJson.engines).toBeDefined();
          expect(packageJson.engines?.node).toBeDefined();
          expect(packageJson.engines?.node).toMatch(/>=\d+/);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 19: Conditional build script inclusion
   * Validates: Requirements 7.4
   *
   * `generatePluginPackageJsonWithDevConfig` (the async function used by
   * PluginInitializationStrategy) inspects the plugin directory for a `Source/`
   * subdirectory and generates different build scripts depending on whether source
   * modules exist:
   *
   *   - With Source/: `build`, `clean`, `prebuild`, `postbuild` are all present
   *     and contain source-module-specific commands.
   *   - Without Source/: only `build` and `clean` are present with content-only
   *     commands; `prebuild` and `postbuild` are absent.
   *
   * This property verifies that the distinction holds for arbitrary plugin metadata,
   * i.e. the conditional behaviour is driven solely by directory structure, not by
   * metadata values.
   */
  it('Property 19: generatePluginPackageJsonWithDevConfig includes source build scripts when Source/ exists, and omits them when it does not', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginMetadataArbitrary(),
        pluginNameArbitrary(),
        async (metadata, pluginName) => {
          // --- Case 1: plugin directory contains a Source/ subdirectory ---
          const withSourceDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-prop19-src-'));
          try {
            await fs.mkdir(path.join(withSourceDir, 'Source'));
            const upluginPath = path.join(withSourceDir, `${pluginName}.uplugin`);
            await fs.writeFile(upluginPath, JSON.stringify({ FileVersion: 3 }));

            const pkgWithSource = await generatePluginPackageJsonWithDevConfig(metadata, upluginPath);

            // Source-module scripts must all be present
            expect(pkgWithSource.scripts?.build).toBeDefined();
            expect(pkgWithSource.scripts?.clean).toBeDefined();
            expect(pkgWithSource.scripts?.prebuild).toBeDefined();
            expect(pkgWithSource.scripts?.postbuild).toBeDefined();

            // The build command must reflect source-module intent
            expect(pkgWithSource.scripts?.build).toContain('Building plugin with source modules');
            // The clean command must remove Unreal build artifact directories
            expect(pkgWithSource.scripts?.clean).toContain('Binaries/');
          } finally {
            await fs.rm(withSourceDir, { recursive: true, force: true });
          }

          // --- Case 2: plugin directory has no Source/ subdirectory ---
          const withoutSourceDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-prop19-nosrc-'));
          try {
            const upluginPath = path.join(withoutSourceDir, `${pluginName}.uplugin`);
            await fs.writeFile(upluginPath, JSON.stringify({ FileVersion: 3 }));

            const pkgWithoutSource = await generatePluginPackageJsonWithDevConfig(metadata, upluginPath);

            // Only the content-only scripts should be present
            expect(pkgWithoutSource.scripts?.build).toBeDefined();
            expect(pkgWithoutSource.scripts?.clean).toBeDefined();

            // Source-only lifecycle hooks must be absent
            expect(pkgWithoutSource.scripts?.prebuild).toBeUndefined();
            expect(pkgWithoutSource.scripts?.postbuild).toBeUndefined();

            // The build command must reflect content-only intent
            expect(pkgWithoutSource.scripts?.build).toContain('Content-only plugin');
          } finally {
            await fs.rm(withoutSourceDir, { recursive: true, force: true });
          }
        }
      ),
      { numRuns: 25 }
    );
  });
});

describe('Plugin Package.json Validation - Property-Based Tests', () => {
  /**
   * Feature: init-plugin-support, Property 13: Configuration validation
   * Validates: Requirements 4.4
   * 
   * For any existing package.json in plugin context, the init command should validate
   * that the main field points to the correct uplugin file.
   */
  it('Property 13: validates main field points to correct uplugin file', async () => {
    await fc.assert(
      fc.asyncProperty(
        pluginPackageJsonArbitrary(),
        pluginNameArbitrary(),
        async (packageJson, expectedPluginName) => {
          // Test with correct configuration
          const correctPackageJson = {
            ...packageJson,
            main: `${expectedPluginName}.uplugin`,
            unreal: {
              ...packageJson.unreal,
              pluginName: expectedPluginName
            }
          };
          
          const correctValidation = validatePluginPackageJson(correctPackageJson, expectedPluginName);
          expect(correctValidation.valid).toBe(true);
          expect(correctValidation.issues).toHaveLength(0);
          
          // Test with incorrect main field
          const incorrectMainPackageJson = {
            ...packageJson,
            main: 'wrong-file.js',
            unreal: {
              ...packageJson.unreal,
              pluginName: expectedPluginName
            }
          };
          
          const incorrectMainValidation = validatePluginPackageJson(incorrectMainPackageJson, expectedPluginName);
          expect(incorrectMainValidation.valid).toBe(false);
          expect(incorrectMainValidation.issues.some(issue => 
            issue.includes('Main field should point to the .uplugin file')
          )).toBe(true);
          
          // Test with incorrect plugin name
          const incorrectNamePackageJson = {
            ...packageJson,
            main: `${expectedPluginName}.uplugin`,
            unreal: {
              ...packageJson.unreal,
              pluginName: 'WrongPluginName'
            }
          };
          
          const incorrectNameValidation = validatePluginPackageJson(incorrectNamePackageJson, expectedPluginName);
          expect(incorrectNameValidation.valid).toBe(false);
          expect(incorrectNameValidation.issues.some(issue => 
            issue.includes('Plugin name mismatch')
          )).toBe(true);
        }
      ),
      { numRuns: 100 }
    );
  });

  /**
   * Feature: init-plugin-support, Property 14: Conflict resolution
   * Validates: Requirements 4.5
   * 
   * For any conflicting configuration in existing package.json, the init command should
   * display warnings and preserve existing values unless force flag is used.
   */
  it('Property 14: handles conflicting configuration appropriately', async () => {
    await fc.assert(
      fc.asyncProperty(
        packageJsonArbitrary(),
        pluginPackageJsonArbitrary(),
        async (existingPackageJson, pluginPackageJson) => {
          // Create conflicting configuration by ensuring both have different values for same fields
          const conflictingExisting = {
            ...existingPackageJson,
            main: 'existing-main.js',
            description: 'Existing description',
            author: 'Existing Author',
            unreal: {
              engineVersion: '4.27.0',
              pluginName: 'ExistingPlugin'
            }
          };
          
          // Merge without force flag - should preserve existing values
          const mergedWithoutForce = mergePluginPackageJson(conflictingExisting, pluginPackageJson, false);
          
          // Verify existing values are preserved when there are conflicts
          expect(mergedWithoutForce.main).toBe(conflictingExisting.main);
          expect(mergedWithoutForce.description).toBe(conflictingExisting.description);
          expect(mergedWithoutForce.author).toBe(conflictingExisting.author);
          expect(mergedWithoutForce.unreal).toEqual(conflictingExisting.unreal);
          
          // Merge with force flag - should overwrite with plugin values
          const mergedWithForce = mergePluginPackageJson(conflictingExisting, pluginPackageJson, true);
          
          // Verify plugin values overwrite existing values when force is used
          expect(mergedWithForce.main).toBe(pluginPackageJson.main);
          expect(mergedWithForce.unreal).toEqual(pluginPackageJson.unreal);
          
          // Non-plugin specific fields should still be preserved even with force
          expect(mergedWithForce.name).toBe(conflictingExisting.name);
          expect(mergedWithForce.version).toBe(conflictingExisting.version);
        }
      ),
      { numRuns: 100 }
    );
  });
});