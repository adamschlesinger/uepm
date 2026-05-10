import * as path from 'path';
import { promises as fs } from 'fs';
import { InitContext, PackageJson } from './types';
import { UEPMError, formatErrorMessage } from './errors';
import { extractPluginMetadata, PluginMetadata } from './uplugin-manager';
import { generatePluginPackageJsonWithDevConfig, mergePluginPackageJson, validatePluginPackageJson } from './plugin-package-json-generator';
import * as packageJsonManager from './package-json-manager';

export interface InitOptions {
  projectDir?: string;
  force?: boolean;
  pluginName?: string;
  engineVersion?: string;
}

export interface InitResult {
  success: boolean;
  message: string;
  alreadyInitialized?: boolean;
  context?: 'project' | 'plugin';
  filesCreated: string[];
  filesModified: string[];
}

export interface InitializationStrategy {
  initialize(context: InitContext, options: InitOptions): Promise<InitResult>;
}

/**
 * Plugin initialization strategy for setting up NPM distribution
 */
export class PluginInitializationStrategy implements InitializationStrategy {
  
  async initialize(context: InitContext, options: InitOptions = {}): Promise<InitResult> {
    if (context.type !== 'plugin') {
      throw new Error('PluginInitializationStrategy can only handle plugin contexts');
    }

    const directory = context.directory;
    const upluginPath = context.primaryFile;
    const pluginName = context.pluginName;
    
    const filesCreated: string[] = [];
    const filesModified: string[] = [];

    try {
      // Step 1: Extract metadata from .uplugin file
      const metadata = await extractPluginMetadata(upluginPath);
      
      // Step 2: Generate plugin-specific package.json configuration with development features
      const pluginPackageJson = await generatePluginPackageJsonWithDevConfig(metadata, upluginPath, {
        force: options.force,
        engineVersion: options.engineVersion
      });

      // Step 3: Handle existing package.json or create new one
      const packageJsonExists = await packageJsonManager.exists(directory);
      
      if (!packageJsonExists) {
        // Create new package.json
        await packageJsonManager.write(directory, pluginPackageJson);
        filesCreated.push('package.json');
        
        return {
          success: true,
          message: this.generateSuccessMessage(pluginName, 'created', filesCreated, filesModified),
          context: 'plugin',
          filesCreated,
          filesModified
        };
      } else {
        // Handle existing package.json
        const existingPackageJson = await packageJsonManager.read(directory);
        
        // Check if already initialized (unless force flag is set)
        const isAlreadyInitialized = this.isPluginPackageJsonInitialized(existingPackageJson, pluginName);
        
        if (isAlreadyInitialized && !options.force) {
          // Validate existing configuration
          const validation = validatePluginPackageJson(existingPackageJson, pluginName);
          
          if (validation.valid) {
            return {
              success: true,
              message: `Plugin "${pluginName}" is already initialized for NPM distribution.`,
              alreadyInitialized: true,
              context: 'plugin',
              filesCreated,
              filesModified
            };
          } else {
            // Configuration exists but has issues
            const issuesText = validation.issues.join('\n- ');
            return {
              success: false,
              message: `Plugin package.json has configuration issues:\n- ${issuesText}\n\nUse --force flag to reinitialize.`,
              context: 'plugin',
              filesCreated,
              filesModified
            };
          }
        }

        // Merge plugin configuration with existing package.json
        const mergedPackageJson = mergePluginPackageJson(
          existingPackageJson, 
          pluginPackageJson, 
          options.force || false
        );

        // Validate the merged configuration
        const validation = validatePluginPackageJson(mergedPackageJson, pluginName);
        if (!validation.valid) {
          const issuesText = validation.issues.join('\n- ');
          return {
            success: false,
            message: `Failed to create valid plugin configuration:\n- ${issuesText}`,
            context: 'plugin',
            filesCreated,
            filesModified
          };
        }

        // Write the updated package.json
        await packageJsonManager.write(directory, mergedPackageJson);
        filesModified.push('package.json');

        const actionTaken = isAlreadyInitialized ? 'reinitialized' : 'updated';
        return {
          success: true,
          message: this.generateSuccessMessage(pluginName, actionTaken, filesCreated, filesModified),
          context: 'plugin',
          filesCreated,
          filesModified
        };
      }

    } catch (error) {
      // Handle specific error types
      if (error instanceof UEPMError) {
        return {
          success: false,
          message: formatErrorMessage(error.toErrorMessage()),
          context: 'plugin',
          filesCreated,
          filesModified
        };
      }
      
      // Handle file system errors
      if (error && typeof error === 'object' && 'code' in error) {
        const fsError = error as NodeJS.ErrnoException;
        if (fsError.code === 'ENOENT') {
          return {
            success: false,
            message: `File not found: ${fsError.path || 'unknown file'}. Please check that all required files exist.`,
            context: 'plugin',
            filesCreated,
            filesModified
          };
        }
        if (fsError.code === 'EACCES' || fsError.code === 'EPERM') {
          return {
            success: false,
            message: `Permission denied accessing: ${fsError.path || 'unknown file'}. Please check file permissions.`,
            context: 'plugin',
            filesCreated,
            filesModified
          };
        }
      }
      
      // Generic error handling
      const message = error instanceof Error ? error.message : String(error);
      return {
        success: false,
        message: `Unexpected error during plugin initialization: ${message}`,
        context: 'plugin',
        filesCreated,
        filesModified
      };
    }
  }

  /**
   * Check if a package.json is already initialized for plugin distribution
   */
  private isPluginPackageJsonInitialized(packageJson: PackageJson, expectedPluginName: string): boolean {
    // Check for plugin-specific indicators
    return !!(
      packageJson.main?.endsWith('.uplugin') &&
      packageJson.unreal?.pluginName === expectedPluginName &&
      packageJson.unreal?.engineVersion &&
      packageJson.files?.some(file => file.endsWith('.uplugin')) &&
      packageJson.keywords?.some(keyword => 
        keyword.includes('unreal') || keyword.includes('ue4') || keyword.includes('ue5')
      )
    );
  }

  /**
   * Generate success message with next steps
   */
  private generateSuccessMessage(
    pluginName: string, 
    action: string, 
    filesCreated: string[], 
    filesModified: string[]
  ): string {
    let message = `Successfully ${action} plugin "${pluginName}" for NPM distribution!\n\n`;
    
    // List files that were changed
    if (filesCreated.length > 0) {
      message += `Files created:\n${filesCreated.map(f => `- ${f}`).join('\n')}\n\n`;
    }
    
    if (filesModified.length > 0) {
      message += `Files modified:\n${filesModified.map(f => `- ${f}`).join('\n')}\n\n`;
    }
    
    // Provide next steps guidance
    message += `Next steps:\n`;
    message += `1. Set your package name and scope in package.json (e.g. "@your-org/${pluginName.toLowerCase()}")\n`;
    message += `2. Review the engine version compatibility range in the "unreal" section\n`;
    message += `3. Check the .gitignore file for appropriate exclusions\n`;
    message += `4. Run 'npm test' to validate your plugin structure\n`;
    message += `5. Use 'npm run build' to build your plugin (if it has source modules)\n`;
    message += `6. Publish to NPM with 'npm publish' when ready\n`;
    
    return message;
  }
}