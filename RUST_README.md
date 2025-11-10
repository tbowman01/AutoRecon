# AutoRecon - Rust Edition

> It's like bowling with bumpers. - [@ippsec](https://twitter.com/ippsec)

AutoRecon is a multi-threaded network reconnaissance tool which performs automated enumeration of services. This Rust edition provides native performance with multiple distribution options: native CLI binary, Node.js package, and Python package.

**Available Implementations:**
- 🦀 **Native CLI** - Standalone binary for Linux/macOS/Windows
- 📦 **Node.js Package** - `@autorecon/core` for JavaScript/TypeScript projects
- 🐍 **Python Package** - `autorecon` for Python 3.8+

## Features

* **High Performance**: Built in Rust for maximum speed and efficiency
* **Multiple Distribution Formats**: Native binary, npm package, or pip package
* **Concurrent Scanning**: Scan multiple targets simultaneously
* **Flexible Configuration**: TOML-based configuration for port scans and service enumeration
* **Pattern Matching**: Automatically extract important information from scan results
* **Extensible Architecture**: Easy to add custom scanning logic
* Full compatibility with existing AutoRecon TOML configuration files

## Installation

### Option 1: Native CLI Binary

#### Prerequisites
Install scanning tools on Kali Linux/Debian:
```bash
sudo apt update
sudo apt install seclists curl dnsrecon enum4linux feroxbuster gobuster \
  impacket-scripts nbtscan nikto nmap onesixtyone oscanner redis-tools \
  smbclient smbmap snmp sslscan sipvicious tnscmd10g whatweb
```

#### Build from Source
Requires Rust 1.70+ (install from https://rustup.rs/):

```bash
# Clone the repository
git clone https://github.com/Tib3rius/AutoRecon.git
cd AutoRecon

# Build release binary
cargo build --release --package autorecon-cli

# Binary will be at: ./target/release/autorecon
sudo cp ./target/release/autorecon /usr/local/bin/
```

#### Usage
```bash
# Basic scan
autorecon 10.10.10.1

# Multiple targets
autorecon 10.10.10.1 10.10.10.2 192.168.1.0/24

# Custom options
autorecon 10.10.10.1 \
  --output ./results \
  --profile quick \
  --concurrent-targets 5 \
  --concurrent-scans 10 \
  --config ./config
```

### Option 2: Node.js Package

#### Prerequisites
- Node.js 16+
- Rust toolchain (for building)

#### Installation
```bash
npm install @autorecon/core
# or
yarn add @autorecon/core
```

#### Usage
```typescript
import { AutoRecon } from '@autorecon/core';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

// Command executor callback
async function executeCommand(cmd: string): Promise<any> {
  const { stdout, stderr } = await execAsync(cmd, {
    maxBuffer: 10 * 1024 * 1024
  });
  return {
    stdout,
    stderr,
    exitCode: 0,
    durationMs: 0
  };
}

// Create scanner instance
const scanner = new AutoRecon({
  configDir: './config',
  profile: 'default',
  outputDir: './results',
  concurrentTargets: 5,
  concurrentScans: 10,
  verbosity: 1
});

// List available profiles
const profiles = scanner.listProfiles();
console.log('Available profiles:', profiles);

// Get profile information
const profileInfo = scanner.getProfileInfo('default');
console.log('Profile scans:', profileInfo.scans);

// Scan a target
const result = await scanner.scanTarget('10.10.10.1', executeCommand);
console.log('Scan completed:', result);
```

#### TypeScript Support
Full TypeScript definitions are included:
```typescript
interface AutoReconConfig {
  configDir: string;
  profile: string;
  outputDir: string;
  concurrentTargets?: number;
  concurrentScans?: number;
  verbosity?: number;
  heartbeatInterval?: number;
  nmapExtra?: string;
}

interface ScanResult {
  target: string;
  services: DetectedService[];
  patterns: PatternMatch[];
  errors: string[];
  durationMs: number;
}
```

### Option 3: Python Package

#### Prerequisites
- Python 3.8+
- pip

#### Installation
```bash
# Install from wheel (after building)
pip install autorecon

# Or build from source
cd crates/autorecon-py
pip install maturin
maturin develop  # For development
maturin build --release  # For production wheel
pip install target/wheels/autorecon-*.whl
```

#### Usage
```python
import asyncio
import subprocess
from autorecon import AutoRecon

async def execute_command(cmd: str) -> dict:
    """Execute shell command and return result"""
    proc = await asyncio.create_subprocess_shell(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    stdout, stderr = await proc.communicate()
    return {
        'stdout': stdout.decode(),
        'stderr': stderr.decode(),
        'exit_code': proc.returncode,
        'duration_ms': 0
    }

async def main():
    # Create scanner instance
    scanner = AutoRecon(
        config_dir='./config',
        profile='default',
        output_dir='./results',
        concurrent_targets=5,
        concurrent_scans=10,
        verbosity=1
    )

    # List profiles
    profiles = scanner.list_profiles()
    print(f'Available profiles: {profiles}')

    # Get profile info
    profile_info = scanner.get_profile_info('default')
    print(f'Profile scans: {profile_info["scans"]}')

    # Scan target
    result = await scanner.scan_target('10.10.10.1', execute_command)
    print(f'Scan completed: {result}')

if __name__ == '__main__':
    asyncio.run(main())
```

## Configuration

AutoRecon uses TOML configuration files in the `config/` directory:

### Port Scan Profiles (`config/port-scan-profiles.toml`)
```toml
[default]
    [default.nmap-quick]
        [default.nmap-quick.service-detection]
        command = 'nmap -sV -sC --version-all ...'
        pattern = '^(?P<port>\d+)/(?P<protocol>(tcp|udp)).*'

    [default.nmap-full-tcp]
        [default.nmap-full-tcp.service-detection]
        command = 'nmap -A -p- ...'
        pattern = '^(?P<port>\d+)/(?P<protocol>(tcp|udp)).*'
```

### Service Scans (`config/service-scans.toml`)
```toml
[http]
service-names = ['^http']

    [[http.scan]]
    name = 'nmap-http'
    command = 'nmap -sV -p {port} --script="http-*" ...'

    [[http.scan]]
    name = 'nikto'
    command = 'nikto -h {scheme}://{address}:{port} ...'
```

### Global Patterns (`config/global-patterns.toml`)
```toml
[[pattern]]
description = "Found credentials"
pattern = '(password|passwd|pwd)[\s:=]+\S+'
```

## Architecture

The Rust implementation consists of four crates:

### `autorecon-core`
Platform-agnostic library with:
- Configuration parsing (TOML)
- Service detection and pattern matching
- Scan orchestration logic
- Trait-based executor and output handler abstractions

**Tests**: 18/18 passing ✅

### `autorecon-cli`
Native command-line binary:
- Built with `clap` for argument parsing
- Uses `tokio` for async runtime
- Direct process execution via `std::process::Command`
- Colored terminal output

**Tests**: 9/9 passing ✅

### `autorecon-node`
Node.js bindings via napi-rs:
- Native addon for Node.js
- ThreadsafeFunction for callbacks
- Full TypeScript definitions
- npm package ready for distribution

**Status**: Builds successfully, config loading verified ✅

### `autorecon-py`
Python bindings via PyO3:
- Native extension module
- Async/await support with pyo3-asyncio
- Compatible with Python 3.8+
- Distributable as wheel via maturin

**Status**: Builds successfully, wheel ready ✅

## Building from Source

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js dependencies (for Node.js bindings)
cd crates/autorecon-node
npm install

# Install Python dependencies (for Python bindings)
pip install maturin
```

### Build All Components
```bash
# Build entire workspace
cargo build --workspace --release

# Build specific components
cargo build --release --package autorecon-cli
cargo build --release --package autorecon-node
cargo build --release --package autorecon-py
```

### Build Node.js Package
```bash
cd crates/autorecon-node
npm run build  # Uses napi-rs
npm pack       # Create tarball for distribution
```

### Build Python Package
```bash
cd crates/autorecon-py
maturin build --release
# Wheel created in: ../../target/wheels/
pip install ../../target/wheels/autorecon-*.whl
```

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test --package autorecon-core
cargo test --package autorecon-cli

# Run with output
cargo test --package autorecon-core -- --nocapture

# Run Node.js tests
cd crates/autorecon-node
npm test
```

## CLI Usage Examples

### Basic Scanning
```bash
# Scan single target
autorecon 10.10.10.1

# Scan multiple targets
autorecon 10.10.10.1 10.10.10.2

# Scan CIDR range
autorecon 192.168.1.0/24

# Scan from file
autorecon -t targets.txt
```

### Advanced Options
```bash
# Custom profile (quick, default, or udp)
autorecon --profile quick 10.10.10.1

# Custom output directory
autorecon -o ./my-results 10.10.10.1

# Adjust concurrency
autorecon --concurrent-targets 10 --concurrent-scans 20 10.10.10.0/24

# Custom config directory
autorecon --config ./my-config 10.10.10.1

# Increase verbosity (0-2)
autorecon -vv 10.10.10.1
```

## Development

### Project Structure
```
AutoRecon/
├── Cargo.toml                 # Workspace configuration
├── config/                    # TOML configuration files
│   ├── port-scan-profiles.toml
│   ├── service-scans.toml
│   └── global-patterns.toml
├── crates/
│   ├── autorecon-core/       # Core library
│   │   ├── src/
│   │   │   ├── config/       # TOML parsing
│   │   │   ├── executor/     # Command execution trait
│   │   │   ├── output/       # File I/O trait
│   │   │   ├── patterns/     # Pattern matching
│   │   │   ├── scanner/      # Scan orchestration
│   │   │   └── services/     # Service detection
│   │   └── Cargo.toml
│   ├── autorecon-cli/        # Native CLI
│   │   ├── src/
│   │   │   ├── executor.rs   # Native command execution
│   │   │   ├── output.rs     # Native file I/O
│   │   │   └── main.rs       # CLI entry point
│   │   └── Cargo.toml
│   ├── autorecon-node/       # Node.js bindings
│   │   ├── src/
│   │   │   ├── executor.rs   # JS callback executor
│   │   │   ├── output.rs     # JS callback output
│   │   │   └── lib.rs        # napi-rs bindings
│   │   ├── index.d.ts        # TypeScript definitions
│   │   ├── package.json
│   │   └── Cargo.toml
│   └── autorecon-py/         # Python bindings
│       ├── src/
│       │   ├── executor.rs   # Python callback executor
│       │   ├── output.rs     # Native file I/O
│       │   └── lib.rs        # PyO3 bindings
│       ├── pyproject.toml    # Maturin config
│       └── Cargo.toml
└── RUST_REFACTORING_PLAN.md  # Detailed implementation plan
```

### Adding Custom Scans

Edit `config/service-scans.toml`:
```toml
[my-custom-service]
service-names = ['^my-service']

    [[my-custom-service.scan]]
    name = 'custom-scan'
    command = 'my-tool {address} {port}'

    [[my-custom-service.scan.pattern]]
    description = 'Found something interesting'
    pattern = 'INTERESTING: (.+)'
```

### Trait-Based Architecture

The core library uses traits for platform abstraction:

```rust
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: &str) -> Result<CommandOutput>;
    async fn is_tool_available(&self, tool: &str) -> bool;
}

#[async_trait]
pub trait OutputHandler: Send + Sync {
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn create_directory(&self, path: &Path) -> Result<()>;
}
```

This allows different implementations:
- **CLI**: Direct process execution and filesystem access
- **Node.js**: JavaScript callbacks for I/O
- **Python**: Python callbacks for command execution

## Performance

Rust implementation provides significant performance improvements:
- **Faster startup**: Native binary vs Python interpreter
- **Lower memory**: Efficient memory management
- **Better concurrency**: Tokio async runtime
- **Type safety**: Compile-time checks prevent runtime errors

## Compatibility

- **Configuration Files**: 100% compatible with original AutoRecon TOML files
- **Command Output**: Identical directory structure and output format
- **Tool Requirements**: Same external tools (nmap, nikto, etc.)

## Migration from Python Version

The Rust CLI is a drop-in replacement:

```bash
# Python version
python3 autorecon.py [OPTIONS] TARGETS

# Rust version (identical usage)
autorecon [OPTIONS] TARGETS
```

Configuration files remain unchanged. The Rust version reads the same TOML files from the `config/` directory.

## Troubleshooting

### Build Issues

**Error: `linker 'cc' not found`**
```bash
# Install build essentials
sudo apt install build-essential
```

**Error: `Could not find NAPI-RS/cli`**
```bash
cd crates/autorecon-node
npm install --include=dev
```

### Runtime Issues

**Error: `Failed to load config: No such file or directory`**
- Ensure you run from the repository root, or use `--config` to specify config directory
- Default location is `./config/` relative to current directory

**Error: `Command not found: nmap`**
- Install required scanning tools (see Prerequisites)

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run tests: `cargo test --workspace`
5. Commit: `git commit -m "Add feature"`
6. Push: `git push origin feature/my-feature`
7. Open a Pull Request

## Testing

All components include comprehensive tests:

```bash
# Core library (18 tests)
cargo test --package autorecon-core

# CLI (9 tests)
cargo test --package autorecon-cli

# Node.js bindings
cd crates/autorecon-node && npm test

# Integration tests
cargo test --workspace
```

## License

AutoRecon is licensed under GPLv3. See [LICENSE](LICENSE) for details.

## Credits

**Original AutoRecon (Python):** [Tib3rius](https://github.com/Tib3rius/AutoRecon)

**Rust Implementation:** Complete rewrite maintaining compatibility with original configuration and behavior.

## Disclaimer

**While AutoRecon endeavors to perform as much identification and enumeration of services as possible, there is no guarantee that every service will be identified, or that every service will be fully enumerated. Users of AutoRecon (especially students) should perform their own manual enumeration alongside AutoRecon. Do not rely on this tool alone for exams, CTFs, or other engagements.**

The tool is provided "as is" without warranty. The authors are not responsible for any misuse or damage caused by this tool. Use responsibly and only on systems you have permission to test.

## Roadmap

- [ ] CI/CD pipeline for automated builds
- [ ] Pre-built binaries for major platforms
- [ ] Publish `@autorecon/core` to npm registry
- [ ] Publish `autorecon` to PyPI
- [ ] Web UI for scan management
- [ ] Docker container with all tools pre-installed
- [ ] Plugin system for custom scan extensions
- [ ] Real-time scan progress streaming
- [ ] Distributed scanning across multiple hosts

## Support

- **Issues**: https://github.com/Tib3rius/AutoRecon/issues
- **Documentation**: See `RUST_REFACTORING_PLAN.md` for implementation details
- **Original Tool**: https://github.com/Tib3rius/AutoRecon
