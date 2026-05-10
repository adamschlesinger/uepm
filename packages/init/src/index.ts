import * as path from 'path';
import {
  findProjectFile,
  readProject,
  writeProject,
  addPluginDirectory,
  hasPluginDirectory,
  UEPMError,
  detectContext,
  PluginInitializationStrategy,
  InitOptions as CoreInitOptions,
  InitResult as CoreInitResult,
} from '@uepm/core';
import * as packageJsonManager from '@uepm/core';

// Export command-related classes
export { CommandRegistry, Command } from './command-registry';
export { InitCommand } from './init-command';

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
  filesCreated?: string[];
  filesModified?: string[];
}

/**
 * Initialize an Unreal Engine project or plugin for NPM support
 * @param options - Initialization options
 * @returns Result of the initialization
 */
export async function init(options: InitOptions = {}): Promise<InitResult> {
  const projectDir = options.projectDir || process.cwd();

  try {
    // Step 1: Detect context (project vs plugin)
    const contextResult = await detectContext(projectDir);

    if (!contextResult.success) {
      return {
        success: false,
        message: contextResult.error,
      };
    }

    const context = contextResult.context;

    // Step 2: Handle warnings if any
    let warningMessage = '';
    if (contextResult.warnings && contextResult.warnings.length > 0) {
      warningMessage = contextResult.warnings.join('\n') + '\n\n';
    }

    // Step 3: Choose strategy based on context
    if (context.type === 'plugin') {
      // Use plugin initialization strategy
      const pluginStrategy = new PluginInitializationStrategy();
      const coreOptions: CoreInitOptions = {
        projectDir: options.projectDir,
        force: options.force,
        pluginName: options.pluginName,
        engineVersion: options.engineVersion,
      };
      
      const result = await pluginStrategy.initialize(context, coreOptions);
      
      return {
        success: result.success,
        message: warningMessage + result.message,
        alreadyInitialized: result.alreadyInitialized,
        context: result.context,
        filesCreated: result.filesCreated,
        filesModified: result.filesModified,
      };
    } else {
      // Use project initialization strategy (backward compatibility)
      return await initializeProject(projectDir, options, warningMessage);
    }

  } catch (error) {
    // Handle UEPMErrors by converting to error result
    if (error instanceof UEPMError) {
      return {
        success: false,
        message: error.message + (error.suggestion ? `\n${error.suggestion}` : ''),
      };
    }
    
    // For other errors, wrap them in a generic error response
    const message = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      message: `Unexpected error during initialization: ${message}`,
    };
  }
}

/**
 * Project initialization strategy (maintains backward compatibility)
 * @param projectDir - Project directory
 * @param options - Initialization options
 * @param warningPrefix - Any warning messages to prepend
 * @returns Result of the initialization
 */
async function initializeProject(
  projectDir: string, 
  options: InitOptions, 
  warningPrefix: string = ''
): Promise<InitResult> {
  // Step 1: Find the .uproject file
  const uprojectPath = await findProjectFile(projectDir);

  // Step 2: Read and modify the .uproject file
  const project = await readProject(uprojectPath);

  // Check if already initialized (unless force flag is set)
  const alreadyHasUEPMPlugins = hasPluginDirectory(project, 'UEPMPlugins');
  if (alreadyHasUEPMPlugins && !options.force) {
    return {
      success: true,
      message: warningPrefix + 'Project is already initialized for NPM plugin support.',
      alreadyInitialized: true,
      context: 'project',
      filesCreated: [],
      filesModified: [],
    };
  }

  // Add UEPMPlugins to AdditionalPluginDirectories
  const modifiedProject = addPluginDirectory(project, 'UEPMPlugins');
  await writeProject(uprojectPath, modifiedProject);

  const filesCreated: string[] = [];
  const filesModified: string[] = ['*.uproject'];

  // Step 3: Create or update package.json
  const packageJsonExists = await packageJsonManager.exists(projectDir);
  const projectName = path.basename(uprojectPath, '.uproject').toLowerCase();

  if (!packageJsonExists) {
    // Create new package.json
    await packageJsonManager.create(projectDir, projectName);
    filesCreated.push('package.json');
  } else {
    // Update existing package.json
    let packageJson = await packageJsonManager.read(projectDir);
    
    // Add postinstall script
    packageJson = packageJsonManager.addPostinstallScript(packageJson);
    
    // Ensure @uepm/postinstall is in devDependencies
    packageJson = packageJsonManager.ensurePostinstallDependency(packageJson);
    
    await packageJsonManager.write(projectDir, packageJson);
    filesModified.push('package.json');
  }

  // Note: Plugin setup will be handled by the postinstall hook

  // Success!
  const actionTaken = alreadyHasUEPMPlugins ? 'reinitialized' : 'initialized';
  let message = warningPrefix + `Successfully ${actionTaken} project for NPM plugin support!\n\n`;
  
  // List files that were changed
  if (filesCreated.length > 0) {
    message += `Files created:\n${filesCreated.map(f => `- ${f}`).join('\n')}\n\n`;
  }
  
  if (filesModified.length > 0) {
    message += `Files modified:\n${filesModified.map(f => `- ${f}`).join('\n')}\n\n`;
  }
  
  message += `Next steps:\n1. Run 'npm install' to install dependencies\n2. Install Unreal Engine plugins via NPM (e.g., 'npm install @uepm/example-plugin')\n3. Open your project in Unreal Engine`;
  
  return {
    success: true,
    message,
    context: 'project',
    filesCreated,
    filesModified,
  };
}
