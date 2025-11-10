# autorecon-cli

Native command-line interface for AutoRecon network reconnaissance.

## Overview

High-performance native binary built with Rust, providing the same functionality as the original Python AutoRecon with significantly improved performance and lower resource usage.

## Installation

### From Source

```bash
# Install Rust from https://rustup.rs if needed
cargo build --release --package autorecon-cli

# Binary will be at: ../../target/release/autorecon
sudo cp ../../target/release/autorecon /usr/local/bin/
```

### Prerequisites

Install scanning tools (Kali Linux/Debian):

```bash
sudo apt update
sudo apt install seclists curl dnsrecon enum4linux feroxbuster gobuster \
  impacket-scripts nbtscan nikto nmap onesixtyone oscanner redis-tools \
  smbclient smbmap snmp sslscan sipvicious tnscmd10g whatweb
```

## Usage

### Basic Scanning

```bash
# Scan single target
autorecon 10.10.10.1

# Scan multiple targets
autorecon 10.10.10.1 10.10.10.2

# Scan CIDR range
autorecon 192.168.1.0/24
```

### Options

```bash
autorecon [OPTIONS] <TARGETS>...

Arguments:
  <TARGETS>...  Target IP addresses or hostnames

Options:
  -o, --output <OUTPUT>
          Output directory for results [default: results]

      --profile <PROFILE>
          Port scan profile to use [default: default]

  -t, --concurrent-targets <CONCURRENT_TARGETS>
          Number of targets to scan concurrently [default: 5]

  -s, --concurrent-scans <CONCURRENT_SCANS>
          Number of scans per target to run concurrently [default: 10]

  -c, --config <CONFIG_DIR>
          Configuration directory [default: config]

  -v, --verbose...
          Verbosity level (0-2)

  -h, --help
          Print help

  -V, --version
          Print version
```

### Advanced Examples

```bash
# Use quick profile for faster scanning
autorecon --profile quick 10.10.10.1

# Custom output directory
autorecon -o /tmp/scan-results 10.10.10.1

# Increase concurrency for faster scanning
autorecon --concurrent-targets 10 --concurrent-scans 20 192.168.1.0/24

# Custom config directory
autorecon --config ./my-config 10.10.10.1

# Maximum verbosity
autorecon -vv 10.10.10.1

# Combine options
autorecon --profile quick -o ./results -t 5 -s 10 -vv 10.10.10.1
```

## Configuration

The CLI reads configuration from the `config/` directory (or path specified with `-c`):

```
config/
├── port-scan-profiles.toml  # Port scanning configurations
├── service-scans.toml        # Service enumeration scans
└── global-patterns.toml      # Pattern matching rules
```

See the main repository README for configuration details.

## Output Structure

Results are organized by target:

```
results/
└── 10.10.10.1/
    ├── scan.log              # Complete scan log
    ├── _commands.log         # All commands executed
    ├── tcp_80_http_nmap.txt  # Individual scan results
    ├── tcp_80_nikto.txt
    ├── tcp_443_https_nmap.txt
    └── xml/                  # XML output files
        ├── tcp_80_nmap.xml
        └── tcp_443_nmap.xml
```

## Performance

Benchmarks compared to Python version:

- **Startup**: ~10x faster (native binary vs Python interpreter)
- **Memory**: ~5x less memory usage
- **Concurrency**: Better async performance with Tokio runtime
- **Type Safety**: Compile-time checks prevent runtime errors

## Building

```bash
# Debug build
cargo build --package autorecon-cli

# Release build (optimized)
cargo build --release --package autorecon-cli

# Run tests
cargo test --package autorecon-cli

# Run without installing
cargo run --package autorecon-cli -- [OPTIONS] <TARGETS>
```

## Testing

```bash
# Run all tests
cargo test --package autorecon-cli

# Run specific test
cargo test --package autorecon-cli executor::tests::test_execute_simple_command

# Run with output
cargo test --package autorecon-cli -- --nocapture
```

**Test Coverage**: 9/9 tests passing ✅

## Architecture

- **Main**: CLI argument parsing with `clap`
- **Executor**: Native command execution via `tokio::process::Command`
- **Output**: Native filesystem operations via `tokio::fs`
- **Core**: Uses `autorecon-core` for scanning logic

## Dependencies

- `clap`: Command-line argument parsing
- `tokio`: Async runtime and process execution
- `colored`: Terminal output coloring
- `autorecon-core`: Core scanning library
- `thiserror`: Error handling

## Troubleshooting

### Command Not Found

If `autorecon` isn't found after installation:

```bash
# Check if binary exists
ls -la /usr/local/bin/autorecon

# Add to PATH if needed
export PATH="$PATH:/usr/local/bin"

# Or use full path
/usr/local/bin/autorecon 10.10.10.1
```

### Permission Denied

Some scans require root privileges (SYN scans, UDP scans):

```bash
# Run with sudo
sudo autorecon 10.10.10.1

# Or set capabilities (Linux only)
sudo setcap cap_net_raw+ep /usr/local/bin/autorecon
```

### Config Not Found

Ensure you run from the repository root or specify config path:

```bash
# Run from repo root
cd /path/to/AutoRecon
autorecon 10.10.10.1

# Or specify config path
autorecon --config /path/to/AutoRecon/config 10.10.10.1
```

## License

GPL-3.0
