import { setupPlugins } from './setup';
import { validatePlugins } from './validate';

export { setupPlugins, validatePlugins };
export * from './setup';
export * from './validate';

/**
 * Run both plugin setup and validation
 * This is the main function called by the postinstall hook
 */
export async function runPostinstall(projectDir: string = process.cwd()): Promise<void> {
  console.log('🔧 Running UEPM postinstall...');
  
  try {
    // Step 1: Set up plugin symlinks
    console.log('📦 Setting up plugin symlinks...');
    await setupPlugins(projectDir);
    
    // Step 2: Validate plugin compatibility
    console.log('🔍 Validating plugin compatibility...');
    const result = await validatePlugins(projectDir);
    
    // Report validation results
    if (result.compatible.length > 0) {
      console.log(`✅ ${result.compatible.length} compatible plugin(s):`);
      for (const plugin of result.compatible) {
        const versionInfo = plugin.engineVersion ? ` (requires ${plugin.engineVersion})` : '';
        console.log(`   ✓ ${plugin.name}@${plugin.version}${versionInfo}`);
      }
    }
    
    if (result.incompatible.length > 0) {
      console.log(`⚠️  ${result.incompatible.length} potentially incompatible plugin(s):`);
    }

    if (result.warnings.length > 0) {
      for (const warning of result.warnings) {
        console.warn(`   ${warning}`);
      }
    }
    
    if (result.compatible.length === 0 && result.incompatible.length === 0) {
      console.log('ℹ️  No UEPM plugins found');
    }
    
    console.log('✅ UEPM postinstall complete!');
    
  } catch (error) {
    console.error('❌ UEPM postinstall failed:', error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}