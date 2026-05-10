import { promises as fs } from 'fs';
import * as path from 'path';
import { PackageJson } from '@uepm/core';

/**
 * Set up UEPM plugin symlinks in UEPMPlugins directory
 * This creates a dedicated folder for Unreal Engine plugins and symlinks them from node_modules
 */
export async function setupPlugins(projectDir: string): Promise<void> {
  const packageJsonPath = path.join(projectDir, 'package.json');
  
  try {
    // Read package.json to find dependencies
    const packageJsonContent = await fs.readFile(packageJsonPath, 'utf-8');
    const packageJson: PackageJson = JSON.parse(packageJsonContent);
    
    const nodeModulesDir = path.join(projectDir, 'node_modules');
    const uepmPluginsDir = path.join(projectDir, 'UEPMPlugins');
    
    // Ensure UEPMPlugins directory exists
    await fs.mkdir(uepmPluginsDir, { recursive: true });
    
    // Process all dependencies to find UEPM plugins
    const allDeps = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    };
    
    let pluginsLinked = 0;
    
    for (const [packageName, version] of Object.entries(allDeps || {})) {
      // Handle file: dependencies (for local development)
      if (typeof version === 'string' && version.startsWith('file:')) {
        const filePath = version.replace('file:', '');
        const absolutePath = path.resolve(projectDir, filePath);
        
        // Create symlink in node_modules for NPM workspace compatibility
        const nodeModulesLinkPath = path.join(nodeModulesDir, packageName);
        
        try {
          // Remove existing symlink/directory if it exists
          try {
            await fs.unlink(nodeModulesLinkPath);
          } catch {
            // Ignore if doesn't exist
          }
          
          // Create symlink in node_modules (relative path)
          const relativePathForNodeModules = path.relative(path.dirname(nodeModulesLinkPath), absolutePath);
          await fs.symlink(relativePathForNodeModules, nodeModulesLinkPath, 'dir');
          console.log(`✓ Linked ${packageName} -> ${filePath} (node_modules)`);
        } catch (error) {
          console.warn(`⚠ Failed to link ${packageName} in node_modules: ${error}`);
        }
      }
      
      // Handle UEPM plugins (both file: and NPM dependencies)
      const isUEPMPlugin = packageName.startsWith('@uepm/') && 
                          !packageName.includes('core') && 
                          !packageName.includes('init') && 
                          !packageName.includes('validate') &&
                          !packageName.includes('postinstall');
      
      if (isUEPMPlugin) {
        let sourcePath: string;
        
        if (typeof version === 'string' && version.startsWith('file:')) {
          // Local file dependency
          const filePath = version.replace('file:', '');
          sourcePath = path.resolve(projectDir, filePath);
        } else {
          // NPM dependency
          sourcePath = path.join(nodeModulesDir, packageName);
        }
        
        // Check if source exists
        try {
          await fs.access(sourcePath);
        } catch {
          console.warn(`⚠ Plugin source not found: ${sourcePath}`);
          continue;
        }
        
        // Create symlink in UEPMPlugins directory
        const pluginName = packageName.replace('@uepm/', '');
        const uepmLinkPath = path.join(uepmPluginsDir, pluginName);
        
        try {
          // Remove existing symlink/directory if it exists
          try {
            await fs.unlink(uepmLinkPath);
          } catch {
            // Ignore if doesn't exist
          }
          
          // Create symlink in UEPMPlugins (relative path)
          const relativeSourcePath = path.relative(path.dirname(uepmLinkPath), sourcePath);
          await fs.symlink(relativeSourcePath, uepmLinkPath, 'dir');
          console.log(`✓ Linked ${packageName} -> UEPMPlugins/${pluginName}`);
          pluginsLinked++;
        } catch (error) {
          console.warn(`⚠ Failed to link ${packageName} in UEPMPlugins: ${error}`);
        }
      }
    }
    
    if (pluginsLinked > 0) {
      console.log(`✓ Successfully linked ${pluginsLinked} UEPM plugin(s) to UEPMPlugins directory`);
    }
    
  } catch (error) {
    console.warn(`⚠ Failed to setup UEPM plugins: ${error}`);
  }
}