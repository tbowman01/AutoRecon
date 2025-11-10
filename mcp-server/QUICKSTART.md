# AutoRecon MCP Server - Quick Start Guide

Get up and running with the AutoRecon MCP server in minutes.

## Installation

### 1. Install Dependencies

```bash
pip install mcp
```

Or use the automated installer:

```bash
cd mcp-server
./install.sh
```

### 2. Configure Claude Desktop

Add to your Claude Desktop config file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "autorecon": {
      "command": "python",
      "args": ["/full/path/to/AutoRecon/mcp-server/autorecon_mcp_server.py"]
    }
  }
}
```

**Important**: Replace `/full/path/to/AutoRecon` with your actual path!

### 3. Restart Claude Desktop

Restart Claude Desktop to load the new server.

## Quick Test

Try these commands in Claude Desktop:

1. **List targets**:
   ```
   "List all AutoRecon targets"
   ```

2. **Start a scan**:
   ```
   "Start an AutoRecon scan on 192.168.1.1"
   ```

3. **Check status**:
   ```
   "What's the status of the scan for 192.168.1.1?"
   ```

4. **View results**:
   ```
   "Show me the discovered services on 192.168.1.1"
   ```

5. **Analyze**:
   ```
   "Analyze the scan results for 192.168.1.1"
   ```

## Common Usage Patterns

### Scanning Multiple Targets

```
"Start AutoRecon scans on 10.10.10.1, 10.10.10.2, and 10.10.10.3"
```

### Using Different Profiles

```
"Run a quick scan on 192.168.1.0/24"
```

### Searching Results

```
"Search for 'admin' in the scan results for 10.10.10.1"
```

### Reading Specific Files

```
"Show me the Nmap results for 10.10.10.1"
```

### Getting Recommendations

```
"What should I do next for 10.10.10.1?"
```

## Troubleshooting

### Server Not Showing Up

1. Check the config file path is correct
2. Verify the path to `autorecon_mcp_server.py` is absolute
3. Restart Claude Desktop
4. Check Claude Desktop's logs

### Scans Not Working

1. Test AutoRecon manually: `python autorecon.py --help`
2. Ensure all dependencies are installed
3. Check you have the necessary tools (nmap, etc.)

### Can't Find Results

1. Wait for scans to complete (can take 10-30 minutes)
2. Check the `results/` directory exists
3. Verify the target name matches exactly

## Tips

- **Be specific** with target names when asking about results
- **Wait patiently** for scans to complete before asking for results
- **Use analysis prompts** to get AI-powered insights
- **Search results** to quickly find specific information
- **Check manual commands** for next steps

## Next Steps

See the full [README.md](README.md) for:
- Complete feature documentation
- Advanced configuration options
- Development guide
- Security considerations

## Getting Help

- MCP SDK docs: https://modelcontextprotocol.io
- AutoRecon docs: See main README.md
- Issues: Check logs in Claude Desktop

## Examples in Action

### Example 1: Initial Reconnaissance

**You**: "Start an AutoRecon scan on 10.10.10.100"

**Claude**: *Uses start_scan tool*

**Claude**: "I've started an AutoRecon scan on 10.10.10.100 using the default profile. The scan is running in the background..."

---

**You**: "What's the status?"

**Claude**: *Uses get_scan_status tool*

**Claude**: "The scan for 10.10.10.100 is still running. Would you like me to check again in a few minutes?"

---

**You**: "Show me what services were found"

**Claude**: *Uses get_discovered_services tool*

**Claude**: "Here are the discovered services on 10.10.10.100:
- Port 22/tcp: SSH (OpenSSH 7.6p1)
- Port 80/tcp: HTTP (Apache 2.4.29)
- Port 3306/tcp: MySQL"

### Example 2: Deep Analysis

**You**: "Analyze the scan results for 10.10.10.100"

**Claude**: *Uses analyze_target prompt and reads multiple resources*

**Claude**: "Based on the scan results, here's my analysis:

**Attack Surface:**
- SSH service on port 22 (OpenSSH 7.6p1)
- HTTP service on port 80 (Apache 2.4.29)
- MySQL database on port 3306

**Security Concerns:**
- MySQL port is publicly accessible (unusual)
- Apache version may have known vulnerabilities
- Directory listing enabled on /uploads

**Recommendations:**
1. Attempt to enumerate MySQL users
2. Check for Apache CVEs for version 2.4.29
3. Investigate the /uploads directory for sensitive files
..."

### Example 3: Focused Investigation

**You**: "Search for 'login' in the results for 10.10.10.100"

**Claude**: *Uses search_results tool*

**Claude**: "Found 8 matches for 'login':
1. web_tcp_80_whatweb.txt - 'Login form detected'
2. web_tcp_80_nikto.txt - '/admin/login.php'
3. web_tcp_80_gobuster.txt - '/.../login' (Status: 200)
..."

## Support

For detailed documentation, see [README.md](README.md)
