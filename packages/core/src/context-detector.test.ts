import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fc from 'fast-check';
import { detectContext, hasProjectContext, hasPluginContext } from './context-detector';
import { directoryConfigArbitrary } from './test-generators';
import { promises as fs } from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('Context Detector', () => {
  let tempDir: string;

  beforeEach(async () => {
    // Create a temporary directory for each test
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-context-test-'));
  });

  afterEach(async () => {
    // Clean up temporary directory
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  describe('Unit Tests', () => {
    it('should detect project context with single .uproject file', async () => {
      const uprojectPath = path.join(tempDir, 'TestProject.uproject');
      await fs.writeFile(uprojectPath, JSON.stringify({ EngineAssociation: '5.3' }));

      const result = await detectContext(tempDir);

      expect(result.success).toBe(true);
      if (!result.success) return;
      expect(result.context.type).toBe('project');
      expect(result.context.primaryFile).toBe(uprojectPath);
      expect(result.context.directory).toBe(tempDir);
    });

    it('should detect plugin context with single .uplugin file', async () => {
      const upluginPath = path.join(tempDir, 'TestPlugin.uplugin');
      await fs.writeFile(upluginPath, JSON.stringify({ FileVersion: 3 }));

      const result = await detectContext(tempDir);

      expect(result.success).toBe(true);
      if (!result.success) return;
      expect(result.context.type).toBe('plugin');
      expect(result.context.primaryFile).toBe(upluginPath);
      expect(result.context.directory).toBe(tempDir);
      expect(result.context.pluginName).toBe('TestPlugin');
    });

    it('should prioritize project when both .uproject and .uplugin exist', async () => {
      const uprojectPath = path.join(tempDir, 'TestProject.uproject');
      const upluginPath = path.join(tempDir, 'TestPlugin.uplugin');
      await fs.writeFile(uprojectPath, JSON.stringify({ EngineAssociation: '5.3' }));
      await fs.writeFile(upluginPath, JSON.stringify({ FileVersion: 3 }));

      const result = await detectContext(tempDir);

      expect(result.success).toBe(true);
      if (!result.success) return;
      expect(result.context.type).toBe('project');
      expect(result.context.primaryFile).toBe(uprojectPath);
      expect(result.warnings).toBeDefined();
      expect(result.warnings![0]).toContain('Both .uproject and .uplugin files found');
    });

    it('should return error when no Unreal files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'package.json'), '{}');

      const result = await detectContext(tempDir);

      expect(result.success).toBe(false);
      if (result.success) return;
      expect(result.error).toContain('No Unreal Engine project (.uproject) or plugin (.uplugin) files found');
    });

    it('should return error when multiple .uplugin files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'Plugin1.uplugin'), JSON.stringify({ FileVersion: 3 }));
      await fs.writeFile(path.join(tempDir, 'Plugin2.uplugin'), JSON.stringify({ FileVersion: 3 }));

      const result = await detectContext(tempDir);

      expect(result.success).toBe(false);
      if (result.success) return;
      expect(result.error).toContain('Multiple .uplugin files found');
    });

    it('should handle multiple .uproject files with warning', async () => {
      await fs.writeFile(path.join(tempDir, 'Project1.uproject'), JSON.stringify({ EngineAssociation: '5.3' }));
      await fs.writeFile(path.join(tempDir, 'Project2.uproject'), JSON.stringify({ EngineAssociation: '5.3' }));

      const result = await detectContext(tempDir);

      expect(result.success).toBe(true);
      if (!result.success) return;
      expect(result.context.type).toBe('project');
      expect(result.warnings).toBeDefined();
      expect(result.warnings![0]).toContain('Multiple .uproject files found');
    });
  });

  describe('Property-Based Tests', () => {
    /**
     * Feature: init-plugin-support, Property 8: Context detection accuracy
     * Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6
     * 
     * For any directory configuration, the init command should correctly identify 
     * whether it contains project files, plugin files, both, or neither
     */
    it('Property 8: Context detection accuracy', async () => {
      await fc.assert(
        fc.asyncProperty(
          directoryConfigArbitrary(),
          async (dirConfig) => {
            // Create a unique temp directory for this iteration
            const testDir = await fs.mkdtemp(path.join(os.tmpdir(), 'uepm-prop8-test-'));
            
            try {
              // Create files according to the configuration
              for (const uprojectFile of dirConfig.uprojectFiles) {
                const filePath = path.join(testDir, uprojectFile);
                await fs.writeFile(filePath, JSON.stringify({ EngineAssociation: '5.3' }));
              }
              
              for (const upluginFile of dirConfig.upluginFiles) {
                const filePath = path.join(testDir, upluginFile);
                await fs.writeFile(filePath, JSON.stringify({ FileVersion: 3 }));
              }
              
              for (const otherFile of dirConfig.otherFiles) {
                const filePath = path.join(testDir, otherFile);
                await fs.writeFile(filePath, 'content');
              }
              
              // Run context detection
              const result = await detectContext(testDir);
              
              // Verify behavior based on file configuration
              if (dirConfig.uprojectFiles.length === 0 && dirConfig.upluginFiles.length === 0) {
                // No Unreal files - should return error
                expect(result.success).toBe(false);
                if (!result.success) {
                  expect(result.error).toContain('No Unreal Engine project (.uproject) or plugin (.uplugin) files found');
                }

              } else if (dirConfig.upluginFiles.length > 1 && dirConfig.uprojectFiles.length === 0) {
                // Multiple plugins only - should return error
                expect(result.success).toBe(false);
                if (!result.success) {
                  expect(result.error).toContain('Multiple .uplugin files found');
                }

              } else if (dirConfig.uprojectFiles.length > 0) {
                // Has project files - should prioritize project
                expect(result.success).toBe(true);
                if (!result.success) return;
                expect(result.context.type).toBe('project');
                expect(result.context.directory).toBe(testDir);
                expect(result.context.primaryFile).toContain('.uproject');

                // Should warn about multiple projects if applicable
                if (dirConfig.uprojectFiles.length > 1) {
                  expect(result.warnings).toBeDefined();
                  expect(result.warnings!.some(w => w.includes('Multiple .uproject files found'))).toBe(true);
                }

                // Should warn about ignored plugin if both exist
                if (dirConfig.upluginFiles.length > 0) {
                  expect(result.warnings).toBeDefined();
                  expect(result.warnings!.some(w => w.includes('Both .uproject and .uplugin files found'))).toBe(true);
                }

              } else if (dirConfig.upluginFiles.length === 1) {
                // Single plugin only - should detect plugin
                expect(result.success).toBe(true);
                if (!result.success) return;
                expect(result.context.type).toBe('plugin');
                expect(result.context.directory).toBe(testDir);
                expect(result.context.primaryFile).toContain('.uplugin');
                expect(result.context.pluginName).toBeDefined();

                // Plugin name should match filename without extension
                const expectedPluginName = path.basename(dirConfig.upluginFiles[0], '.uplugin');
                expect(result.context.pluginName).toBe(expectedPluginName);
              }
              
            } finally {
              // Clean up this iteration's temp directory
              await fs.rm(testDir, { recursive: true, force: true });
            }
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('Helper Functions', () => {
    it('hasProjectContext should return true when .uproject files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'Test.uproject'), '{}');
      
      const result = await hasProjectContext(tempDir);
      
      expect(result).toBe(true);
    });

    it('hasProjectContext should return false when no .uproject files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'Test.uplugin'), '{}');
      
      const result = await hasProjectContext(tempDir);
      
      expect(result).toBe(false);
    });

    it('hasPluginContext should return true when .uplugin files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'Test.uplugin'), '{}');
      
      const result = await hasPluginContext(tempDir);
      
      expect(result).toBe(true);
    });

    it('hasPluginContext should return false when no .uplugin files exist', async () => {
      await fs.writeFile(path.join(tempDir, 'Test.uproject'), '{}');
      
      const result = await hasPluginContext(tempDir);
      
      expect(result).toBe(false);
    });
  });
});