import { promises as fs } from 'fs';
import * as path from 'path';
import { InitContext, ContextDetectionResult } from './types';
import {
  createFileNotFoundError,
  createPermissionDeniedError,
} from './errors';

/**
 * Detect the initialization context (project vs plugin) in a directory
 * @param directory - Directory to scan for context files
 * @returns Context detection result with context information or errors
 */
export async function detectContext(directory: string): Promise<ContextDetectionResult> {
  try {
    const files = await fs.readdir(directory);
    const uprojectFiles = files.filter(file => file.endsWith('.uproject'));
    const upluginFiles = files.filter(file => file.endsWith('.uplugin'));

    // Handle no Unreal files found
    if (uprojectFiles.length === 0 && upluginFiles.length === 0) {
      return {
        success: false,
        error: `No Unreal Engine project (.uproject) or plugin (.uplugin) files found in ${directory}. Please run this command in a directory containing an Unreal Engine project or plugin.`
      };
    }

    // Handle both project and plugin files (prioritize project with warning)
    if (uprojectFiles.length > 0 && upluginFiles.length > 0) {
      const warnings = [`Both .uproject and .uplugin files found. Prioritizing project initialization. Plugin file ${upluginFiles[0]} will be ignored.`];

      // Handle multiple project files
      let projectFile = uprojectFiles[0];
      if (uprojectFiles.length > 1) {
        uprojectFiles.sort();
        projectFile = uprojectFiles[0];
        warnings.push(`Multiple .uproject files found. Using: ${projectFile}`);
      }

      return {
        success: true,
        context: {
          type: 'project',
          primaryFile: path.join(directory, projectFile),
          directory
        },
        warnings
      };
    }

    // Handle project-only context
    if (uprojectFiles.length > 0) {
      let projectFile = uprojectFiles[0];
      const warnings: string[] = [];

      if (uprojectFiles.length > 1) {
        uprojectFiles.sort();
        projectFile = uprojectFiles[0];
        warnings.push(`Multiple .uproject files found. Using: ${projectFile}`);
      }

      return {
        success: true,
        context: {
          type: 'project',
          primaryFile: path.join(directory, projectFile),
          directory
        },
        ...(warnings.length > 0 ? { warnings } : {})
      };
    }

    // Handle plugin-only context
    if (upluginFiles.length === 1) {
      const pluginFile = upluginFiles[0];
      const pluginName = path.basename(pluginFile, '.uplugin');

      return {
        success: true,
        context: {
          type: 'plugin',
          primaryFile: path.join(directory, pluginFile),
          directory,
          pluginName
        }
      };
    }

    // Handle multiple plugin files (only when no project files exist)
    if (upluginFiles.length > 1) {
      return {
        success: false,
        error: `Multiple .uplugin files found in ${directory}: ${upluginFiles.join(', ')}. Please specify which plugin to initialize or run the command in a directory with only one plugin file.`
      };
    }

    // This should never be reached, but handle it gracefully
    return {
      success: false,
      error: `Unexpected error during context detection in ${directory}`
    };

  } catch (error) {
    // Handle file system errors
    if (error && typeof error === 'object' && 'code' in error) {
      const fsError = error as NodeJS.ErrnoException;
      if (fsError.code === 'ENOENT') {
        throw createFileNotFoundError(directory, 'Please check that the directory exists.');
      }
      if (fsError.code === 'EACCES' || fsError.code === 'EPERM') {
        throw createPermissionDeniedError(directory, 'read');
      }
    }
    
    throw error;
  }
}

/**
 * Check if a directory contains a project context
 * @param directory - Directory to check
 * @returns True if directory contains .uproject files
 */
export async function hasProjectContext(directory: string): Promise<boolean> {
  try {
    const files = await fs.readdir(directory);
    return files.some(file => file.endsWith('.uproject'));
  } catch {
    return false;
  }
}

/**
 * Check if a directory contains a plugin context
 * @param directory - Directory to check
 * @returns True if directory contains .uplugin files
 */
export async function hasPluginContext(directory: string): Promise<boolean> {
  try {
    const files = await fs.readdir(directory);
    return files.some(file => file.endsWith('.uplugin'));
  } catch {
    return false;
  }
}