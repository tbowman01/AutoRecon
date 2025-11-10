# @autorecon/core

Native Node.js bindings for AutoRecon - Network reconnaissance automation tool

## Features

- **Native Performance**: Built with Rust and napi-rs for maximum speed
- **Zero-Copy**: Efficient data transfer between JavaScript and Rust
- **Async/Await**: Full async support with native Promises
- **TypeScript**: Complete TypeScript type definitions
- **Cross-Platform**: Supports Linux, macOS, Windows (x64, ARM64, musl)

## Installation

```bash
npm install @autorecon/core
```

## Quick Start

```typescript
import { AutoRecon } from '@autorecon/core';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

// Create scanner instance
const scanner = new AutoRecon({
  configDir: './config',
  profile: 'default',
  outputDir: './results',
  concurrentTargets: 5,
  concurrentScans: 10
});

// Define command execution callback
async function executeCommand(cmd) {
  const start = Date.now();
  try {
    const { stdout, stderr } = await execAsync(cmd);
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

// Scan a target
const result = await scanner.scanTarget('192.168.1.1', executeCommand);

if (result.success) {
  console.log(`✓ Scan completed in ${result.durationMs}ms`);
  console.log(`  Found ${result.services.length} services:`);

  result.services.forEach(svc => {
    console.log(`  - ${svc.protocol}/${svc.port}: ${svc.service}${svc.version ? ' (' + svc.version + ')' : ''}`);
  });
} else {
  console.error(`✗ Scan failed: ${result.error}`);
}
```

## API Reference

### Class: `AutoRecon`

Main scanner class for network reconnaissance.

#### Constructor

```typescript
new AutoRecon(config: AutoReconConfig)
```

**Parameters:**
- `config.configDir` (string): Path to configuration directory
- `config.profile` (string): Scan profile name ('default', 'quick', 'udp')
- `config.outputDir` (string): Output directory for results
- `config.concurrentTargets` (number, optional): Max concurrent targets (default: 5)
- `config.concurrentScans` (number, optional): Max scans per target (default: 10)
- `config.verbosity` (number, optional): Verbosity level 0-2 (default: 1)
- `config.nmapExtra` (string, optional): Additional nmap arguments

#### Methods

##### `scanTarget(address, executeCallback)`

Scan a single target.

```typescript
scanTarget(address: string, executeCallback: ExecuteCallback): Promise<ScanResult>
```

**Parameters:**
- `address`: Target IP address or hostname
- `executeCallback`: Function to execute shell commands

**Returns:** `Promise<ScanResult>`

```typescript
interface ScanResult {
  address: string;
  services: DetectedService[];
  baseDir: string;
  durationMs: number;
  success: boolean;
  error?: string;
}
```

##### `listProfiles()`

Get list of available scan profiles.

```typescript
listProfiles(): string[]
```

**Returns:** Array of profile names

##### `getProfileInfo(profileName)`

Get information about a specific profile.

```typescript
getProfileInfo(profileName: string): ProfileInfo | null
```

**Returns:** Profile information or null if not found

```typescript
interface ProfileInfo {
  name: string;
  scanCount: number;
  scans: string[];
}
```

##### `checkTool(toolName, checkCallback)`

Check if a security tool is available.

```typescript
checkTool(toolName: string, checkCallback: CheckToolCallback): Promise<boolean>
```

## Command Execution Callback

The `executeCallback` function must have this signature:

```typescript
type ExecuteCallback = (command: string) => Promise<CommandOutput>

interface CommandOutput {
  stdout: string;
  stderr: string;
  exitCode: number;
  durationMs: number;
}
```

### Example with child_process

```typescript
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

async function executeCommand(cmd) {
  const start = Date.now();
  try {
    const { stdout, stderr } = await execAsync(cmd, {
      maxBuffer: 10 * 1024 * 1024 // 10MB buffer
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
```

### Example with better timeout handling

```typescript
import { spawn } from 'child_process';

async function executeCommandWithTimeout(cmd, timeoutMs = 3600000) {
  const start = Date.now();

  return new Promise((resolve) => {
    const child = spawn('sh', ['-c', cmd]);

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => stdout += data.toString());
    child.stderr.on('data', (data) => stderr += data.toString());

    const timeout = setTimeout(() => {
      child.kill('SIGTERM');
    }, timeoutMs);

    child.on('close', (code) => {
      clearTimeout(timeout);
      resolve({
        stdout,
        stderr,
        exitCode: code || 0,
        durationMs: Date.now() - start
      });
    });
  });
}
```

## Configuration

AutoRecon requires a configuration directory with TOML files:

```
config/
├── port-scan-profiles.toml
├── service-scans.toml
└── global-patterns.toml
```

These files define:
- Port scanning strategies
- Service-specific enumeration commands
- Pattern matching for vulnerability detection

See the [AutoRecon documentation](https://github.com/Tib3rius/AutoRecon) for configuration details.

## Examples

### Scan multiple targets concurrently

```typescript
const targets = ['192.168.1.1', '192.168.1.2', '192.168.1.3'];

const results = await Promise.all(
  targets.map(target => scanner.scanTarget(target, executeCommand))
);

results.forEach(result => {
  if (result.success) {
    console.log(`✓ ${result.address}: ${result.services.length} services`);
  } else {
    console.log(`✗ ${result.address}: ${result.error}`);
  }
});
```

### Check for required tools

```typescript
const requiredTools = ['nmap', 'nikto', 'gobuster'];

for (const tool of requiredTools) {
  const available = await scanner.checkTool(tool, async (name) => {
    try {
      await execAsync(`which ${name}`);
      return true;
    } catch {
      return false;
    }
  });

  if (!available) {
    console.warn(`⚠ Tool not found: ${tool}`);
  }
}
```

### List and select profiles

```typescript
const profiles = scanner.listProfiles();
console.log('Available profiles:', profiles);

const profileInfo = scanner.getProfileInfo('quick');
console.log(`\nProfile: ${profileInfo.name}`);
console.log(`Scans: ${profileInfo.scans.join(', ')}`);
```

## Performance

Native bindings provide significant performance benefits:

- **Startup**: 5-10x faster than Python
- **Memory**: 2-3x lower usage
- **Parsing**: 10-50x faster regex/pattern matching
- **Concurrency**: Better async performance

## Platform Support

Pre-built binaries available for:

- **Linux**: x64 (glibc, musl), ARM64 (glibc, musl)
- **macOS**: x64, ARM64 (Apple Silicon)
- **Windows**: x64, ARM64

## License

GPL-3.0 - Same as AutoRecon

## Contributing

See [CONTRIBUTING.md](https://github.com/Tib3rius/AutoRecon/blob/master/CONTRIBUTING.md)

## Credits

- Original AutoRecon by [Tib3rius](https://github.com/Tib3rius)
- Rust refactoring with native bindings
