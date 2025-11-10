# autorecon-core

Platform-agnostic core library for AutoRecon network reconnaissance.

## Overview

`autorecon-core` provides the foundational functionality for AutoRecon, designed to be platform-independent and usable across different execution environments (native, Node.js, Python, WASM, etc.).

## Features

- **Configuration Parsing**: Load and parse TOML configuration files
- **Service Detection**: Regex-based pattern matching for service identification
- **Scan Orchestration**: Async scanning logic with configurable concurrency
- **Trait-Based Architecture**: Platform abstraction via `CommandExecutor` and `OutputHandler` traits
- **Pattern Matching**: Extract important information from command output
- **Error Handling**: Comprehensive error types with context

## Architecture

### Core Traits

```rust
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: &str) -> Result<CommandOutput>;
    async fn execute_with_timeout(&self, command: &str, timeout_secs: u64) -> Result<CommandOutput>;
    async fn is_tool_available(&self, tool: &str) -> bool;
}

#[async_trait]
pub trait OutputHandler: Send + Sync {
    async fn create_directory(&self, path: &Path) -> Result<()>;
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn append_file(&self, path: &Path, content: &str) -> Result<()>;
    async fn read_file(&self, path: &Path) -> Result<String>;
    async fn file_exists(&self, path: &Path) -> bool;
    async fn directory_exists(&self, path: &Path) -> bool;
}
```

### Main Components

- **`Config`**: TOML configuration parser
- **`Scanner`**: Main scanning orchestrator
- **`ServiceDetector`**: Service detection from port scan results
- **`PatternMatcher`**: Extract patterns from command output
- **`ScanContext`**: Configuration and runtime context

## Usage

```rust
use autorecon_core::{Config, Scanner, ScanContext};
use std::path::PathBuf;

// Load configuration
let config = Config::load_from_dir(&PathBuf::from("./config"))?;

// Create scan context
let context = ScanContext {
    concurrent_targets: 5,
    concurrent_scans: 10,
    profile: "default".to_string(),
    output_dir: PathBuf::from("./results"),
    nmap_extra: "-vv --reason -Pn".to_string(),
    heartbeat_interval: 60,
    verbosity: 1,
};

// Create scanner with your executor and output handler implementations
let scanner = Scanner::new(
    config,
    your_executor,
    your_output_handler,
    context,
);

// Scan a target
let result = scanner.scan_target("10.10.10.1").await?;
```

## Configuration

The library expects TOML configuration files in the following structure:

```
config/
├── port-scan-profiles.toml  # Port scanning configurations
├── service-scans.toml        # Service enumeration scans
└── global-patterns.toml      # Global pattern matching rules
```

### Port Scan Profiles

```toml
[default]
    [default.nmap-quick]
        [default.nmap-quick.service-detection]
        command = 'nmap -sV -sC {address}'
        pattern = '^(?P<port>\d+)/(?P<protocol>(tcp|udp)).*open.*(?P<service>\S+)'
```

### Service Scans

```toml
[http]
service-names = ['^http']

    [[http.scan]]
    name = 'nikto'
    command = 'nikto -h {scheme}://{address}:{port}'
```

## Testing

```bash
cargo test --package autorecon-core

# With output
cargo test --package autorecon-core -- --nocapture

# Specific test
cargo test --package autorecon-core config::tests::test_parse_port_scan_profiles
```

**Test Coverage**: 18/18 tests passing ✅

## API Documentation

Generate and view API documentation:

```bash
cargo doc --package autorecon-core --open
```

## Error Handling

The crate uses a custom `AutoReconError` type:

```rust
pub enum AutoReconError {
    Config(String),      // Configuration errors
    Execution(String),   // Command execution errors
    Io(String),          // File I/O errors
    TomlParse(String),   // TOML parsing errors
    Pattern(String),     // Pattern matching errors
}
```

## Dependencies

- `tokio`: Async runtime
- `toml`: TOML parsing
- `regex`: Pattern matching
- `async-trait`: Async trait support
- `thiserror`: Error handling
- `serde`: Serialization

## License

GPL-3.0
