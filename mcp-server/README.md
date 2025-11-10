# AutoRecon MCP Server

A Model Context Protocol (MCP) server that exposes AutoRecon's network reconnaissance capabilities for integration with AI assistants like Claude.

## Overview

This MCP server allows AI assistants to:
- Start and manage AutoRecon scans programmatically
- Access scan results through structured resources
- Search and analyze reconnaissance data
- Get intelligent recommendations for next steps

## Features

### Tools (Actions)

1. **start_scan** - Initiate reconnaissance scans
   - Support for multiple targets (IPs, CIDR ranges, hostnames)
   - Configurable scan profiles (default, quick, full-tcp)
   - Concurrent scanning options

2. **list_targets** - View all scanned targets and their status

3. **get_scan_status** - Check if a scan is running or completed

4. **list_scan_files** - Browse all scan output files for a target
   - Optional filtering by pattern

5. **read_scan_file** - Read specific scan output files

6. **get_discovered_services** - Extract open ports and services
   - Parses Nmap results automatically

7. **search_results** - Search across all scan outputs
   - Regex support
   - Case-sensitive/insensitive options

### Resources

Dynamic resources for each scanned target:
- **Summary** - Overview of scan results and findings
- **Commands Log** - All executed commands
- **Manual Commands** - Suggested manual commands to run
- **Pattern Matches** - Important findings flagged by AutoRecon

### Prompts

Pre-built prompts for AI-assisted analysis:
- **analyze_target** - Comprehensive security assessment
- **suggest_next_steps** - Prioritized enumeration recommendations
- **find_vulnerabilities** - Vulnerability identification

## Installation

### Prerequisites

- Python 3.8 or higher
- AutoRecon installed and working
- MCP SDK

### Setup

1. Install the required dependencies:

```bash
pip install -r requirements.txt
```

Or install the MCP SDK directly:

```bash
pip install mcp
```

2. Make the server executable:

```bash
chmod +x autorecon_mcp_server.py
```

3. Test the server:

```bash
python autorecon_mcp_server.py
```

## Configuration

### Claude Desktop

To use this MCP server with Claude Desktop, add it to your Claude configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "autorecon": {
      "command": "python",
      "args": ["/path/to/AutoRecon/mcp-server/autorecon_mcp_server.py"]
    }
  }
}
```

Replace `/path/to/AutoRecon` with the actual path to your AutoRecon installation.

### Other MCP Clients

For other MCP clients, use the stdio transport and point to the server script:

```bash
python /path/to/AutoRecon/mcp-server/autorecon_mcp_server.py
```

## Usage Examples

### Starting a Scan

```
"Start an AutoRecon scan on 10.10.10.1 using the default profile"
```

The AI assistant will use the `start_scan` tool:
```json
{
  "targets": ["10.10.10.1"],
  "profile": "default"
}
```

### Viewing Results

```
"Show me what services were discovered on 10.10.10.1"
```

The AI assistant will use the `get_discovered_services` tool to extract ports and services.

### Analyzing Scans

```
"Analyze the scan results for 10.10.10.1"
```

The AI assistant will use the `analyze_target` prompt to provide a comprehensive security assessment.

### Searching Results

```
"Search for 'admin' in the scan results for 10.10.10.1"
```

The AI assistant will use the `search_results` tool to find all occurrences.

### Reading Specific Files

```
"Show me the Nmap scan results for port 80 on 10.10.10.1"
```

The AI assistant will:
1. Use `list_scan_files` to find relevant files
2. Use `read_scan_file` to display the content

## Architecture

The server is implemented as a single Python file using the MCP SDK:

- **Async I/O** - Non-blocking operations for better performance
- **Dynamic Resources** - Scan results are discovered automatically
- **Process Management** - Tracks running scans via subprocess
- **Error Handling** - Graceful error messages for all operations

## Directory Structure

The server expects AutoRecon's standard directory structure:

```
results/
├── 10.10.10.1/
│   ├── exploit/
│   ├── loot/
│   ├── report/
│   └── scans/
│       ├── _commands.log
│       ├── _manual_commands.txt
│       ├── _patterns.log
│       └── [scan output files]
└── 10.10.10.2/
    └── ...
```

## Security Considerations

- **Local Only** - This server is designed for local use only
- **No Authentication** - The server does not implement authentication
- **Process Isolation** - Scans run as separate subprocesses
- **File System Access** - Limited to the AutoRecon results directory

⚠️ **Warning**: Do not expose this server to untrusted networks or users. It provides direct access to reconnaissance tools and scan results.

## Troubleshooting

### Server Not Starting

- Ensure MCP SDK is installed: `pip install mcp`
- Check Python version: `python --version` (requires 3.8+)
- Verify AutoRecon path in the configuration

### Scans Not Running

- Ensure AutoRecon works from command line
- Check that `autorecon.py` is executable
- Verify all AutoRecon dependencies are installed

### No Results Showing

- Wait for scans to complete (can take several minutes)
- Check the `results/` directory exists
- Verify scan completed without errors

### Connection Issues

- Restart Claude Desktop after configuration changes
- Check the configuration file syntax (valid JSON)
- Review Claude Desktop logs for errors

## Development

### Testing Locally

You can test the server using the MCP inspector:

```bash
npx @modelcontextprotocol/inspector python autorecon_mcp_server.py
```

This will open a web interface to test tools, resources, and prompts.

### Adding New Features

The server is modular and easy to extend:

1. **Add Tools** - Define in `list_tools()` and implement in `call_tool()`
2. **Add Resources** - Update `list_resources()` and `read_resource()`
3. **Add Prompts** - Define in `list_prompts()` and implement in `get_prompt()`

## Contributing

Contributions are welcome! Please ensure:
- Code follows the existing style
- All features are documented
- Error handling is comprehensive

## License

This MCP server follows the same license as AutoRecon (GPL v3).

## Support

For issues specific to the MCP server:
- Check the troubleshooting section above
- Review MCP SDK documentation: https://modelcontextprotocol.io

For AutoRecon issues:
- See the main AutoRecon README
- Check AutoRecon documentation

## Changelog

### Version 1.0.0 (Initial Release)
- Complete MCP server implementation
- 7 tools for scan management and analysis
- Dynamic resource discovery
- 3 analysis prompts
- Full Claude Desktop integration
