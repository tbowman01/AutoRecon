# AutoRecon Rust Refactoring & WASM Implementation Plan

## Executive Summary

Refactor AutoRecon from Python to Rust with WebAssembly (WASM) compilation support, making it available via npm and pip packages. This will provide:

- **Performance**: Rust's zero-cost abstractions and memory safety
- **Portability**: WASM enables browser and embedded environments
- **Multi-platform**: Native binaries + WASM + Python/Node.js bindings
- **Type Safety**: Compile-time guarantees and better tooling

## Architecture Overview

### Workspace Structure

```
autorecon/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── autorecon-core/           # Core Rust library (platform-agnostic)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config/           # TOML configuration parser
│   │   │   ├── scanner/          # Scanning logic
│   │   │   ├── patterns/         # Pattern matching
│   │   │   ├── services/         # Service detection
│   │   │   ├── executor/         # Abstract command execution
│   │   │   └── output/           # Output management
│   │   └── Cargo.toml
│   ├── autorecon-cli/            # Native CLI binary
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── executor.rs       # Native process executor
│   │   └── Cargo.toml
│   ├── autorecon-wasm/           # WASM bindings
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── bindings.rs       # wasm-bindgen exports
│   │   ├── Cargo.toml
│   │   └── package.json          # npm package definition
│   ├── autorecon-py/             # Python bindings
│   │   ├── src/
│   │   │   ├── lib.rs            # PyO3 bindings
│   │   │   └── executor.rs       # Python callback executor
│   │   ├── Cargo.toml
│   │   └── pyproject.toml        # pip package definition
│   └── autorecon-node/           # Node.js native bindings (optional)
│       ├── src/
│       │   └── lib.rs            # napi-rs bindings
│       └── Cargo.toml
├── config/                       # Shared TOML configs
│   ├── port-scan-profiles.toml
│   ├── service-scans.toml
│   └── global-patterns.toml
├── tests/
├── examples/
└── docs/
```

## Core Components Breakdown

### 1. autorecon-core (Platform-Agnostic Library)

**Purpose**: Contains all core business logic, no I/O dependencies

**Key Modules**:

```rust
// config/mod.rs
pub struct PortScanProfile { ... }
pub struct ServiceScan { ... }
pub struct GlobalPatterns { ... }
pub struct Config {
    pub port_scan_profiles: HashMap<String, PortScanProfile>,
    pub service_scans: HashMap<String, ServiceScan>,
    pub global_patterns: Vec<Pattern>,
}

// scanner/mod.rs
pub struct Target {
    pub address: String,
    pub base_dir: PathBuf,
    pub scans: Vec<Scan>,
}

pub struct ScanContext {
    pub concurrent_targets: usize,
    pub concurrent_scans: usize,
    pub profile: String,
    pub verbosity: u8,
}

// executor/mod.rs (trait-based abstraction)
#[async_trait]
pub trait CommandExecutor {
    async fn execute(&self, cmd: &str) -> Result<CommandOutput>;
    async fn read_stream(&self, stdout: Stream, stderr: Stream)
        -> Result<Vec<String>>;
}

// services/mod.rs
pub struct ServiceDetector {
    patterns: Vec<Regex>,
}

impl ServiceDetector {
    pub fn detect(&self, output: &str) -> Vec<DetectedService> { ... }
}

// output/mod.rs (abstracted I/O)
#[async_trait]
pub trait OutputHandler {
    async fn write_log(&self, target: &Target, tag: &str, content: &str)
        -> Result<()>;
    async fn create_directory(&self, path: &Path) -> Result<()>;
}
```

**Dependencies**:
- `serde` + `serde_derive` - Serialization
- `toml` - Config parsing
- `regex` - Pattern matching
- `async-trait` - Async trait support
- `thiserror` - Error handling
- `chrono` - Timestamps

**Design Principles**:
- No direct I/O (filesystem, network, processes)
- All I/O through trait abstractions
- Can compile to WASM without modifications
- Pure business logic

### 2. autorecon-cli (Native Binary)

**Purpose**: Native CLI application for Linux/macOS/Windows

**Implementation**:

```rust
// main.rs
use autorecon_core::*;
use tokio;
use clap::Parser;

struct NativeExecutor {
    semaphore: Arc<Semaphore>,
}

#[async_trait]
impl CommandExecutor for NativeExecutor {
    async fn execute(&self, cmd: &str) -> Result<CommandOutput> {
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .await?;
        Ok(output)
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = Config::load(&args.config_dir)?;
    let executor = NativeExecutor::new(args.concurrent_scans);
    let scanner = Scanner::new(config, executor);
    scanner.run(args.targets).await?;
}
```

**Dependencies**:
- `tokio` - Async runtime
- `clap` - CLI argument parsing
- `colored` - Terminal colors
- `autorecon-core`

### 3. autorecon-wasm (WebAssembly Bindings)

**Purpose**: WASM module for browser and Node.js environments

**Architecture**:
- Core logic compiled to WASM
- Host environment provides I/O via JavaScript callbacks
- Async support via `wasm-bindgen-futures`

**Implementation**:

```rust
// lib.rs
use wasm_bindgen::prelude::*;
use autorecon_core::*;

#[wasm_bindgen]
pub struct AutoReconWasm {
    config: Config,
    executor: WasmExecutor,
}

#[wasm_bindgen]
impl AutoReconWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(config_toml: &str) -> Result<AutoReconWasm, JsValue> {
        let config = Config::from_str(config_toml)?;
        Ok(AutoReconWasm {
            config,
            executor: WasmExecutor::new(),
        })
    }

    pub async fn scan_target(&self, address: &str, callbacks: JsCallbacks)
        -> Result<JsValue, JsValue> {
        // Scanning logic using callbacks for I/O
    }
}

// Executor that calls back to JavaScript
struct WasmExecutor {
    execute_callback: js_sys::Function,
    write_callback: js_sys::Function,
}
```

**JavaScript API** (TypeScript definitions):

```typescript
// autorecon.d.ts
export interface ScanCallbacks {
  executeCommand(cmd: string): Promise<CommandOutput>;
  writeLog(target: string, tag: string, content: string): Promise<void>;
  createDirectory(path: string): Promise<void>;
}

export class AutoReconWasm {
  constructor(configToml: string);
  scanTarget(address: string, callbacks: ScanCallbacks): Promise<ScanResult>;
  parseServiceOutput(output: string): DetectedService[];
}
```

**npm Package Structure**:

```json
{
  "name": "@autorecon/core",
  "version": "1.0.0",
  "type": "module",
  "main": "dist/autorecon.js",
  "types": "dist/autorecon.d.ts",
  "files": ["dist/", "autorecon_bg.wasm"],
  "scripts": {
    "build": "wasm-pack build --target bundler"
  },
  "dependencies": {}
}
```

**Usage Example** (Node.js):

```javascript
import { AutoReconWasm } from '@autorecon/core';
import { exec } from 'child_process';
import { promisify } from 'util';
import fs from 'fs/promises';

const execAsync = promisify(exec);

const autorecon = new AutoReconWasm(configToml);

const callbacks = {
  async executeCommand(cmd) {
    const { stdout, stderr } = await execAsync(cmd);
    return { stdout, stderr, exitCode: 0 };
  },
  async writeLog(target, tag, content) {
    await fs.appendFile(`results/${target}/scans/${tag}.log`, content);
  },
  async createDirectory(path) {
    await fs.mkdir(path, { recursive: true });
  }
};

await autorecon.scanTarget('192.168.1.1', callbacks);
```

**Dependencies**:
- `wasm-bindgen` - JS/Rust interop
- `wasm-bindgen-futures` - Async support
- `js-sys` - JavaScript types
- `web-sys` - Web APIs (optional)
- `console_error_panic_hook` - Better error messages
- `autorecon-core`

### 4. autorecon-py (Python Bindings)

**Purpose**: Native Python extension module

**Implementation**:

```rust
// lib.rs
use pyo3::prelude::*;
use autorecon_core::*;

#[pyclass]
struct AutoRecon {
    config: Config,
    executor: PyExecutor,
}

#[pymethods]
impl AutoRecon {
    #[new]
    fn new(config_path: &str) -> PyResult<Self> {
        let config = Config::load(config_path)?;
        Ok(AutoRecon {
            config,
            executor: PyExecutor::new(),
        })
    }

    fn scan_target<'py>(
        &self,
        py: Python<'py>,
        address: &str,
        execute_callback: PyObject,
    ) -> PyResult<&'py PyAny> {
        pyo3_asyncio::tokio::future_into_py(py, async move {
            // Scanning logic
            Ok(())
        })
    }
}

#[pymodule]
fn autorecon(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AutoRecon>()?;
    Ok(())
}
```

**Python API**:

```python
# autorecon/__init__.py
from .autorecon import AutoRecon, ScanResult, DetectedService

__all__ = ['AutoRecon', 'ScanResult', 'DetectedService']
```

**Usage Example**:

```python
import asyncio
import subprocess
from autorecon import AutoRecon

async def execute_command(cmd: str) -> dict:
    proc = await asyncio.create_subprocess_shell(
        cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )
    stdout, stderr = await proc.communicate()
    return {
        'stdout': stdout.decode(),
        'stderr': stderr.decode(),
        'exit_code': proc.returncode
    }

async def main():
    scanner = AutoRecon('/path/to/config')
    result = await scanner.scan_target('192.168.1.1', execute_command)
    print(result)

asyncio.run(main())
```

**pip Package Setup** (using maturin):

```toml
# pyproject.toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "autorecon"
version = "1.0.0"
description = "Network reconnaissance automation tool"
requires-python = ">=3.8"
dependencies = []

[tool.maturin]
features = ["pyo3/extension-module"]
```

**Dependencies**:
- `pyo3` - Python bindings
- `pyo3-asyncio` - Async support
- `tokio` - Runtime
- `autorecon-core`

## Technical Considerations

### WASM Limitations & Solutions

**Challenges**:
1. **No process spawning**: WASM can't execute system commands
2. **No filesystem access**: Sandboxed environment
3. **Limited threading**: No threads in WASM (only Web Workers)

**Solutions**:
1. **Callback-based architecture**: Host environment executes commands
2. **Abstract I/O layer**: All I/O through trait abstractions
3. **Single-threaded async**: Use `wasm-bindgen-futures` for async without threads

### Async Runtime Strategy

**Native (CLI/Python)**:
- Use `tokio` runtime
- Full multi-threading support
- Direct process spawning

**WASM**:
- Use `wasm-bindgen-futures`
- Single-threaded cooperative multitasking
- Callbacks to JavaScript event loop

### Configuration Loading

**Native**: Load from filesystem
**WASM**: Pass as string from JavaScript
**Python**: Load from filesystem or string

### Output Management

**Native**: Direct filesystem writes
**WASM**: Callback to JavaScript for writes
**Python**: Callback to Python or direct writes

## Migration Strategy

### Phase 1: Core Library (Weeks 1-3)

1. Set up Cargo workspace
2. Implement configuration parser
   - Port TOML parsing logic
   - Create Rust structs for all config types
   - Add validation
3. Implement pattern matching
   - Service detection patterns
   - Global patterns
   - Output highlighting
4. Create abstract executor traits
5. Build service detection engine
6. Implement scan orchestration logic
7. Unit tests for core components

### Phase 2: Native CLI (Weeks 4-5)

1. Implement `NativeExecutor` with tokio
2. Port CLI argument parsing
3. Implement native I/O handlers
4. Add colored output
5. Create directory structure management
6. Integration tests with real tools
7. Cross-platform testing (Linux/macOS/Windows)

### Phase 3: WASM Bindings (Weeks 6-7)

1. Set up `wasm-bindgen` infrastructure
2. Implement `WasmExecutor` with callbacks
3. Create JavaScript/TypeScript API
4. Build npm package
5. Write Node.js examples
6. Add browser-compatible build
7. Documentation and examples

### Phase 4: Python Bindings (Weeks 8-9)

1. Set up `PyO3` infrastructure
2. Implement Python async integration
3. Create `PyExecutor` with callbacks
4. Set up maturin for building
5. Create pip package
6. Write Python examples
7. Documentation

### Phase 5: Testing & CI/CD (Week 10)

1. Comprehensive integration tests
2. Set up GitHub Actions
   - Rust tests
   - WASM build and test
   - Python wheels for multiple platforms
   - npm package build
3. Benchmarking suite
4. Documentation generation
5. Release automation

### Phase 6: Publishing (Week 11)

1. Publish to crates.io
2. Publish to npm
3. Publish to PyPI
4. Update documentation
5. Migration guide for existing users
6. Announcement and examples

## Performance Expectations

**Expected Improvements**:
- **Startup time**: 5-10x faster (Rust vs Python interpreter)
- **Memory usage**: 2-3x lower (no Python overhead)
- **Parsing**: 10-50x faster (native regex vs Python)
- **Concurrency**: Better async performance with tokio

**Benchmarking Plan**:
- Compare scan initialization time
- Measure pattern matching performance
- Test concurrent scan throughput
- Monitor memory usage under load

## Compatibility & Breaking Changes

### Config Files
- **No changes required**: Existing TOML configs work as-is
- Enhanced validation and error messages

### Command Line Interface
- **Mostly compatible**: Same arguments and flags
- Improved help text and error messages
- New flags for Rust-specific features

### Output Format
- **100% compatible**: Same directory structure
- Same log file formats
- Pattern matching output identical

### Migration Path
- Side-by-side installation possible
- Gradual migration recommended
- Compatibility mode for edge cases

## Distribution Channels

### Native Binaries
- **Releases**: GitHub Releases with prebuilt binaries
- **Package managers**:
  - Cargo: `cargo install autorecon-cli`
  - Homebrew: `brew install autorecon` (future)
  - apt/yum repositories (future)

### npm Package
- **Registry**: npmjs.com
- **Package**: `@autorecon/core`
- **Targets**: Node.js 16+, browsers with WASM support

### pip Package
- **Registry**: pypi.org
- **Package**: `autorecon`
- **Wheels**: Pre-built for Linux/macOS/Windows
- **Python**: 3.8+

## Documentation Plan

### Developer Documentation
- Architecture guide
- API reference (auto-generated with rustdoc)
- Contribution guidelines
- Building from source

### User Documentation
- Installation guides for all platforms
- Migration guide from Python version
- Configuration reference
- Usage examples
- Troubleshooting

### API Documentation
- Rust: docs.rs
- TypeScript: TypeDoc
- Python: Sphinx

## Success Criteria

### Functional Requirements
- [ ] All current features implemented
- [ ] Config files work without modification
- [ ] Output format identical
- [ ] Same external tool integrations

### Performance Requirements
- [ ] Faster startup than Python version
- [ ] Lower memory usage
- [ ] Equal or better concurrent performance

### Distribution Requirements
- [ ] Available on crates.io
- [ ] Available on npmjs.com
- [ ] Available on pypi.org
- [ ] Prebuilt binaries for major platforms

### Quality Requirements
- [ ] >80% code coverage
- [ ] No critical security vulnerabilities
- [ ] Passes all integration tests
- [ ] Documentation complete

## Risks & Mitigation

### Risk: WASM I/O Abstraction Complexity
**Mitigation**: Prototype early, use well-tested callback patterns

### Risk: External Tool Compatibility
**Mitigation**: Extensive testing with real tools, version detection

### Risk: Python Async Integration
**Mitigation**: Use proven pyo3-asyncio library, thorough testing

### Risk: Cross-platform Build Complexity
**Mitigation**: Use GitHub Actions matrix builds, automated testing

### Risk: Adoption Resistance
**Mitigation**: Maintain compatibility, provide migration guide, keep Python version

## Future Enhancements

### Built-in Scanning (Reduce External Dependencies)
- Native port scanner (replace nmap for basic scans)
- Built-in HTTP client (reduce curl/wget dependency)
- Native DNS resolver

### Cloud Integration
- AWS/Azure/GCP scanning
- Cloud-native deployment
- Distributed scanning

### Web UI
- Browser-based interface using WASM
- Real-time progress visualization
- Interactive reporting

### Advanced Features
- Machine learning for service detection
- Automated vulnerability correlation
- Integration with vulnerability databases

## Conclusion

This refactoring will modernize AutoRecon while maintaining its core strengths:
- **Performance**: Significant speed and efficiency gains
- **Portability**: Run anywhere WASM is supported
- **Maintainability**: Type safety and better tooling
- **Distribution**: Easy installation via cargo/npm/pip
- **Compatibility**: Existing configs and workflows preserved

The modular architecture ensures that each component (CLI, WASM, Python) can evolve independently while sharing the same robust core.
