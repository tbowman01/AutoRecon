#!/bin/bash
# AutoRecon MCP Server Installation Script

set -e

echo "AutoRecon MCP Server Installation"
echo "=================================="
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
AUTORECON_DIR="$(dirname "$SCRIPT_DIR")"

echo "AutoRecon directory: $AUTORECON_DIR"
echo ""

# Check Python version
echo "Checking Python version..."
PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
echo "Found Python $PYTHON_VERSION"
echo ""

# Install dependencies
echo "Installing dependencies..."
pip install -r "$SCRIPT_DIR/requirements.txt"
echo ""

# Make server executable
echo "Making server executable..."
chmod +x "$SCRIPT_DIR/autorecon_mcp_server.py"
echo ""

# Detect OS and set config path
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    CONFIG_DIR="$HOME/Library/Application Support/Claude"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    CONFIG_DIR="$HOME/.config/Claude"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    CONFIG_DIR="$APPDATA/Claude"
else
    CONFIG_DIR=""
fi

echo "Installation complete!"
echo ""
echo "Next steps:"
echo "==========="
echo ""

if [ -n "$CONFIG_DIR" ]; then
    CONFIG_FILE="$CONFIG_DIR/claude_desktop_config.json"

    echo "1. Add the following to your Claude Desktop configuration:"
    echo "   Config file: $CONFIG_FILE"
    echo ""
    echo '   {'
    echo '     "mcpServers": {'
    echo '       "autorecon": {'
    echo '         "command": "python3",'
    echo "         \"args\": [\"$SCRIPT_DIR/autorecon_mcp_server.py\"]"
    echo '       }'
    echo '     }'
    echo '   }'
    echo ""

    if [ -f "$CONFIG_FILE" ]; then
        echo "   NOTE: Config file already exists. Merge the configuration carefully."
    else
        echo "   NOTE: Config file doesn't exist yet. You can create it with the above content."
    fi
else
    echo "1. Add this server to your Claude Desktop configuration."
    echo "   See README.md for platform-specific instructions."
fi

echo ""
echo "2. Restart Claude Desktop to load the server."
echo ""
echo "3. Test by asking Claude: 'List available AutoRecon targets'"
echo ""
echo "For more information, see: $SCRIPT_DIR/README.md"
