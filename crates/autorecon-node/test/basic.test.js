#!/usr/bin/env node

const { AutoRecon } = require('../index.js');
const { exec } = require('child_process');
const { promisify } = require('util');

const execAsync = promisify(exec);

async function executeCommand(cmd) {
  const start = Date.now();
  try {
    const { stdout, stderr } = await execAsync(cmd, {
      maxBuffer: 10 * 1024 * 1024
    });
    return {
      stdout,
      stderr,
      exitCode: 0,
      durationMs: Date.now() - start
    };
  } catch (error) {
    return {
      stdout: error.stdout || '',
      stderr: error.stderr || '',
      exitCode: error.code || 1,
      durationMs: Date.now() - start
    };
  }
}

async function main() {
  console.log('AutoRecon Node.js Bindings - Basic Test');
  console.log('==========================================\n');

  try {
    // Create scanner instance
    console.log('1. Creating AutoRecon instance...');
    const scanner = new AutoRecon({
      configDir: './config',
      profile: 'default',
      outputDir: './test-results',
      concurrentTargets: 1,
      concurrentScans: 5,
      verbosity: 2
    });
    console.log('   ✓ Instance created\n');

    // List profiles
    console.log('2. Listing scan profiles...');
    const profiles = scanner.listProfiles();
    console.log(`   ✓ Found ${profiles.length} profiles: ${profiles.join(', ')}\n`);

    // Get profile info
    console.log('3. Getting profile information...');
    const profileInfo = scanner.getProfileInfo('default');
    if (profileInfo) {
      console.log(`   ✓ Profile: ${profileInfo.name}`);
      console.log(`   ✓ Scan count: ${profileInfo.scanCount}`);
      console.log(`   ✓ Scans: ${profileInfo.scans.join(', ')}\n`);
    }

    // Note: checkTool method not yet implemented in Node.js bindings
    // console.log('4. Checking tool availability...');

    console.log('\n✓ All basic tests passed!');
    console.log('\nNote: Actual scanning requires security tools (nmap, etc.)');
    console.log('To perform a real scan, use:');
    console.log('  const result = await scanner.scanTarget("127.0.0.1", executeCommand);');

  } catch (error) {
    console.error('\n✗ Test failed:', error.message);
    process.exit(1);
  }
}

main();
