import { promises as fs } from 'fs';
import * as path from 'path';
import { UPluginFile } from './types';
import {
  createFileNotFoundError,
  createPermissionDeniedError,
  createJSONParseError,
  createSchemaValidationError,
  UEPMError,
  ExitCode,
} from './errors';
import { validateUPluginSchema } from './validation';

/**
 * Plugin metadata extracted from .uplugin file
 */
export interface PluginMetadata {
  name: string;
  version: string;
  friendlyName?: string;
  description?: string;
  author?: string;
  homepage?: string;
  engineVersion?: string;
}

/**
 * Find a .uplugin file in the specified directory
 * @param directory - Directory to search for .uplugin file
 * @returns Path to the .uplugin file
 * @throws UEPMError if no .uplugin file is found or directory cannot be read
 */
export async function findPluginFile(directory: string): Promise<string> {
  try {
    const files = await fs.readdir(directory);
    const upluginFiles = files.filter(file => file.endsWith('.uplugin'));

    if (upluginFiles.length === 0) {
      throw createNoPluginFileError(directory);
    }

    if (upluginFiles.length > 1) {
      throw createMultiplePluginFilesError(directory, upluginFiles);
    }

    return path.join(directory, upluginFiles[0]);
  } catch (error) {
    // If it's already a UEPMError, rethrow it
    if (error instanceof UEPMError) {
      throw error;
    }
    
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
 * Read and parse a .uplugin file
 * @param filePath - Path to the .uplugin file
 * @returns Parsed UPluginFile object
 * @throws UEPMError if file cannot be read or parsed
 */
export async function readPlugin(filePath: string): Promise<UPluginFile> {
  try {
    const content = await fs.readFile(filePath, 'utf-8');
    
    let parsed: unknown;
    try {
      parsed = JSON.parse(content);
    } catch (parseError) {
      throw createJSONParseError(filePath, parseError as Error);
    }
    
    // Validate schema
    const validation = validateUPluginSchema(parsed);
    if (!validation.valid) {
      throw createSchemaValidationError(filePath, validation.missingFields);
    }
    
    return parsed as UPluginFile;
  } catch (error) {
    // If it's already a UEPMError, rethrow it
    if (error instanceof UEPMError) {
      throw error;
    }
    
    // Handle file system errors
    if (error && typeof error === 'object' && 'code' in error) {
      const fsError = error as NodeJS.ErrnoException;
      if (fsError.code === 'ENOENT') {
        throw createFileNotFoundError(filePath);
      }
      if (fsError.code === 'EACCES' || fsError.code === 'EPERM') {
        throw createPermissionDeniedError(filePath, 'read');
      }
    }
    
    throw error;
  }
}

/**
 * Extract metadata from a .uplugin file for package.json generation
 * @param filePath - Path to the .uplugin file
 * @returns PluginMetadata object with extracted information
 * @throws UEPMError if file cannot be read or parsed
 */
export async function extractPluginMetadata(filePath: string): Promise<PluginMetadata> {
  const plugin = await readPlugin(filePath);
  const pluginName = path.basename(filePath, '.uplugin');
  
  // Extract version - prefer VersionName over Version
  let version = '1.0.0'; // Default version
  if (plugin.VersionName) {
    version = plugin.VersionName;
  } else if (plugin.Version !== undefined) {
    version = plugin.Version.toString();
    // Pad to full semver: "2" → "2.0.0", "2.5" → "2.5.0"
    const parts = version.split('.');
    while (parts.length < 3) parts.push('0');
    version = parts.join('.');
  }
  
  const metadata: PluginMetadata = {
    name: pluginName,
    version,
    friendlyName: plugin.FriendlyName,
    description: plugin.Description,
    author: plugin.CreatedBy,
    homepage: plugin.DocsURL,
    engineVersion: plugin.EngineVersion,
  };
  
  return metadata;
}

/**
 * Write a .uplugin file, preserving formatting where possible
 * @param filePath - Path to the .uplugin file
 * @param plugin - UPluginFile object to write
 * @throws UEPMError if file cannot be written
 */
export async function writePlugin(
  filePath: string,
  plugin: UPluginFile
): Promise<void> {
  try {
    // Use 2-space indentation (Unreal default) with trailing newline
    const content = JSON.stringify(plugin, null, 2) + '\n';
    await fs.writeFile(filePath, content, 'utf-8');
  } catch (error) {
    // Handle file system errors
    if (error && typeof error === 'object' && 'code' in error) {
      const fsError = error as NodeJS.ErrnoException;
      if (fsError.code === 'EACCES' || fsError.code === 'EPERM') {
        throw createPermissionDeniedError(filePath, 'write');
      }
      if (fsError.code === 'ENOENT') {
        throw createFileNotFoundError(
          path.dirname(filePath),
          'Please check that the directory exists.'
        );
      }
    }
    
    throw error;
  }
}

/**
 * Create a UEPMError for no .uplugin file found
 */
function createNoPluginFileError(directory: string): UEPMError {
  return new UEPMError(
    'UPLUGIN_NOT_FOUND',
    `No .uplugin file found in directory: ${directory}`,
    ExitCode.FILE_NOT_FOUND,
    undefined,
    'Please run this command in your Unreal Engine plugin root directory.'
  );
}

/**
 * Create a UEPMError for multiple .uplugin files found
 */
function createMultiplePluginFilesError(directory: string, files: string[]): UEPMError {
  return new UEPMError(
    'MULTIPLE_UPLUGIN_FILES',
    `Multiple .uplugin files found in directory: ${directory}`,
    ExitCode.GENERAL_ERROR,
    `Found files: ${files.join(', ')}`,
    'Please specify which plugin to initialize or move other .uplugin files to a different directory.'
  );
}