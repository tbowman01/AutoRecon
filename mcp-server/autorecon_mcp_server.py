#!/usr/bin/env python3
"""
AutoRecon MCP Server

A Model Context Protocol (MCP) server that exposes AutoRecon's network reconnaissance
capabilities for integration with AI assistants and automation tools.

This server provides:
- Tools for initiating and managing scans
- Resources for retrieving scan results
- Prompts for scan analysis assistance
"""

import asyncio
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Optional
import glob
import re

# Add the mcp package to path
try:
    from mcp.server import Server
    from mcp.types import (
        Resource,
        Tool,
        TextContent,
        ImageContent,
        EmbeddedResource,
        Prompt,
        PromptMessage,
        GetPromptResult,
    )
    import mcp.server.stdio
except ImportError:
    print("Error: MCP SDK not installed. Install with: pip install mcp", file=sys.stderr)
    sys.exit(1)

# Get the AutoRecon directory
AUTORECON_DIR = Path(__file__).parent.parent.resolve()
AUTORECON_SCRIPT = AUTORECON_DIR / "autorecon.py"
DEFAULT_OUTPUT_DIR = AUTORECON_DIR / "results"

# Global tracking of running scans
running_scans = {}
scan_processes = {}

# Initialize the MCP server
app = Server("autorecon")


@app.list_resources()
async def list_resources() -> list[Resource]:
    """
    List available resources (scan results).

    Resources are dynamically discovered from the results directory.
    """
    resources = []

    if not DEFAULT_OUTPUT_DIR.exists():
        return resources

    # List all target directories
    for target_dir in DEFAULT_OUTPUT_DIR.iterdir():
        if target_dir.is_dir():
            target_name = target_dir.name

            # Add main summary resource
            resources.append(
                Resource(
                    uri=f"autorecon://results/{target_name}/summary",
                    name=f"Scan Summary: {target_name}",
                    mimeType="text/plain",
                    description=f"Summary of reconnaissance results for {target_name}",
                )
            )

            # Add scan files as resources
            scans_dir = target_dir / "scans"
            if scans_dir.exists():
                # Add command log
                commands_log = scans_dir / "_commands.log"
                if commands_log.exists():
                    resources.append(
                        Resource(
                            uri=f"autorecon://results/{target_name}/commands",
                            name=f"Commands Log: {target_name}",
                            mimeType="text/plain",
                            description=f"Log of all commands executed for {target_name}",
                        )
                    )

                # Add manual commands
                manual_commands = scans_dir / "_manual_commands.txt"
                if manual_commands.exists():
                    resources.append(
                        Resource(
                            uri=f"autorecon://results/{target_name}/manual",
                            name=f"Manual Commands: {target_name}",
                            mimeType="text/plain",
                            description=f"Suggested manual commands for {target_name}",
                        )
                    )

                # Add patterns log
                patterns_log = scans_dir / "_patterns.log"
                if patterns_log.exists():
                    resources.append(
                        Resource(
                            uri=f"autorecon://results/{target_name}/patterns",
                            name=f"Pattern Matches: {target_name}",
                            mimeType="text/plain",
                            description=f"Important findings and pattern matches for {target_name}",
                        )
                    )

    return resources


@app.read_resource()
async def read_resource(uri: str) -> str:
    """
    Read a specific resource by URI.
    """
    # Parse the URI
    if not uri.startswith("autorecon://results/"):
        raise ValueError(f"Unknown resource URI: {uri}")

    parts = uri.replace("autorecon://results/", "").split("/")
    if len(parts) != 2:
        raise ValueError(f"Invalid resource URI format: {uri}")

    target_name, resource_type = parts
    target_dir = DEFAULT_OUTPUT_DIR / target_name

    if not target_dir.exists():
        raise ValueError(f"Target directory not found: {target_name}")

    # Handle different resource types
    if resource_type == "summary":
        return await generate_summary(target_dir)

    scans_dir = target_dir / "scans"
    if not scans_dir.exists():
        raise ValueError(f"Scans directory not found for {target_name}")

    if resource_type == "commands":
        file_path = scans_dir / "_commands.log"
    elif resource_type == "manual":
        file_path = scans_dir / "_manual_commands.txt"
    elif resource_type == "patterns":
        file_path = scans_dir / "_patterns.log"
    else:
        raise ValueError(f"Unknown resource type: {resource_type}")

    if not file_path.exists():
        raise ValueError(f"Resource file not found: {file_path}")

    with open(file_path, "r", encoding="utf-8", errors="replace") as f:
        return f.read()


async def generate_summary(target_dir: Path) -> str:
    """
    Generate a summary of scan results for a target.
    """
    target_name = target_dir.name
    summary = [f"# Scan Summary for {target_name}\n"]

    scans_dir = target_dir / "scans"
    if not scans_dir.exists():
        return "No scan results found."

    # Check for patterns (important findings)
    patterns_log = scans_dir / "_patterns.log"
    if patterns_log.exists():
        with open(patterns_log, "r", encoding="utf-8", errors="replace") as f:
            patterns = f.read().strip()
            if patterns:
                summary.append("\n## Important Findings\n")
                summary.append(patterns)

    # List discovered services
    summary.append("\n## Scan Files\n")
    scan_files = [f for f in scans_dir.iterdir() if f.is_file() and not f.name.startswith("_")]
    if scan_files:
        for scan_file in sorted(scan_files)[:20]:  # Limit to first 20
            summary.append(f"- {scan_file.name}")
        if len(scan_files) > 20:
            summary.append(f"- ... and {len(scan_files) - 20} more files")
    else:
        summary.append("No scan files found yet.")

    # Check manual commands
    manual_commands = scans_dir / "_manual_commands.txt"
    if manual_commands.exists():
        with open(manual_commands, "r", encoding="utf-8", errors="replace") as f:
            commands = f.read().strip()
            if commands:
                summary.append("\n## Suggested Manual Commands\n")
                summary.append(commands)

    return "\n".join(summary)


@app.list_tools()
async def list_tools() -> list[Tool]:
    """
    List available tools for interacting with AutoRecon.
    """
    return [
        Tool(
            name="start_scan",
            description="Start an AutoRecon scan on one or more targets. Supports IP addresses, CIDR ranges, and hostnames.",
            inputSchema={
                "type": "object",
                "properties": {
                    "targets": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of targets to scan (IPs, CIDR ranges, or hostnames)",
                    },
                    "profile": {
                        "type": "string",
                        "enum": ["default", "quick", "full-tcp"],
                        "default": "default",
                        "description": "Scan profile to use (default, quick, or full-tcp)",
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Custom output directory (optional, defaults to ./results)",
                    },
                    "concurrent_targets": {
                        "type": "integer",
                        "default": 5,
                        "description": "Number of targets to scan concurrently",
                    },
                    "concurrent_scans": {
                        "type": "integer",
                        "default": 10,
                        "description": "Number of scans to run concurrently per target",
                    },
                },
                "required": ["targets"],
            },
        ),
        Tool(
            name="list_targets",
            description="List all scanned targets and their status",
            inputSchema={
                "type": "object",
                "properties": {},
            },
        ),
        Tool(
            name="get_scan_status",
            description="Get the status of a running or completed scan for a specific target",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target name/IP to check status for",
                    },
                },
                "required": ["target"],
            },
        ),
        Tool(
            name="list_scan_files",
            description="List all scan output files for a target",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target name/IP to list files for",
                    },
                    "filter": {
                        "type": "string",
                        "description": "Optional filter pattern (e.g., '*.txt', 'nmap*')",
                    },
                },
                "required": ["target"],
            },
        ),
        Tool(
            name="read_scan_file",
            description="Read the contents of a specific scan output file",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target name/IP",
                    },
                    "filename": {
                        "type": "string",
                        "description": "Name of the file to read",
                    },
                },
                "required": ["target", "filename"],
            },
        ),
        Tool(
            name="get_discovered_services",
            description="Get a list of discovered services and open ports for a target",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target name/IP",
                    },
                },
                "required": ["target"],
            },
        ),
        Tool(
            name="search_results",
            description="Search across all scan results for a target using a keyword or regex pattern",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target name/IP to search",
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (keyword or regex pattern)",
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "default": False,
                        "description": "Whether the search should be case-sensitive",
                    },
                },
                "required": ["target", "query"],
            },
        ),
    ]


@app.call_tool()
async def call_tool(name: str, arguments: Any) -> list[TextContent]:
    """
    Handle tool calls.
    """
    try:
        if name == "start_scan":
            result = await handle_start_scan(arguments)
        elif name == "list_targets":
            result = await handle_list_targets()
        elif name == "get_scan_status":
            result = await handle_get_scan_status(arguments["target"])
        elif name == "list_scan_files":
            result = await handle_list_scan_files(
                arguments["target"],
                arguments.get("filter")
            )
        elif name == "read_scan_file":
            result = await handle_read_scan_file(
                arguments["target"],
                arguments["filename"]
            )
        elif name == "get_discovered_services":
            result = await handle_get_discovered_services(arguments["target"])
        elif name == "search_results":
            result = await handle_search_results(
                arguments["target"],
                arguments["query"],
                arguments.get("case_sensitive", False)
            )
        else:
            raise ValueError(f"Unknown tool: {name}")

        return [TextContent(type="text", text=result)]

    except Exception as e:
        return [TextContent(type="text", text=f"Error: {str(e)}")]


async def handle_start_scan(args: dict) -> str:
    """
    Start an AutoRecon scan.
    """
    targets = args["targets"]
    profile = args.get("profile", "default")
    output_dir = args.get("output_dir", str(DEFAULT_OUTPUT_DIR))
    concurrent_targets = args.get("concurrent_targets", 5)
    concurrent_scans = args.get("concurrent_scans", 10)

    # Build the command
    cmd = [
        sys.executable,
        str(AUTORECON_SCRIPT),
        *targets,
        "--profile", profile,
        "-o", output_dir,
        "-ct", str(concurrent_targets),
        "-cs", str(concurrent_scans),
    ]

    # Start the scan in the background
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(AUTORECON_DIR),
        text=True,
    )

    # Track the scan
    scan_id = f"{'-'.join(targets)}_{process.pid}"
    running_scans[scan_id] = {
        "targets": targets,
        "profile": profile,
        "output_dir": output_dir,
        "status": "running",
        "pid": process.pid,
    }
    scan_processes[scan_id] = process

    return json.dumps({
        "success": True,
        "message": f"Scan started for targets: {', '.join(targets)}",
        "scan_id": scan_id,
        "pid": process.pid,
        "profile": profile,
        "output_dir": output_dir,
    }, indent=2)


async def handle_list_targets() -> str:
    """
    List all scanned targets.
    """
    if not DEFAULT_OUTPUT_DIR.exists():
        return json.dumps({
            "targets": [],
            "message": "No results directory found. No scans have been run yet."
        }, indent=2)

    targets = []
    for target_dir in DEFAULT_OUTPUT_DIR.iterdir():
        if target_dir.is_dir():
            target_info = {
                "name": target_dir.name,
                "path": str(target_dir),
                "has_results": (target_dir / "scans").exists(),
            }

            # Check if scan is currently running
            target_info["status"] = "completed"
            for scan_id, scan_info in running_scans.items():
                if target_dir.name in scan_info["targets"]:
                    if scan_id in scan_processes:
                        if scan_processes[scan_id].poll() is None:
                            target_info["status"] = "running"
                            target_info["pid"] = scan_info["pid"]
                            break

            targets.append(target_info)

    return json.dumps({
        "targets": targets,
        "count": len(targets),
    }, indent=2)


async def handle_get_scan_status(target: str) -> str:
    """
    Get the status of a scan for a specific target.
    """
    # Check running scans
    for scan_id, scan_info in running_scans.items():
        if target in scan_info["targets"]:
            if scan_id in scan_processes:
                process = scan_processes[scan_id]
                if process.poll() is None:
                    return json.dumps({
                        "target": target,
                        "status": "running",
                        "pid": scan_info["pid"],
                        "scan_id": scan_id,
                    }, indent=2)
                else:
                    return json.dumps({
                        "target": target,
                        "status": "completed",
                        "exit_code": process.returncode,
                        "scan_id": scan_id,
                    }, indent=2)

    # Check if target has results
    target_dir = DEFAULT_OUTPUT_DIR / target
    if target_dir.exists():
        return json.dumps({
            "target": target,
            "status": "completed",
            "has_results": (target_dir / "scans").exists(),
        }, indent=2)

    return json.dumps({
        "target": target,
        "status": "not_found",
        "message": "No scan found for this target",
    }, indent=2)


async def handle_list_scan_files(target: str, filter_pattern: Optional[str] = None) -> str:
    """
    List scan files for a target.
    """
    target_dir = DEFAULT_OUTPUT_DIR / target / "scans"

    if not target_dir.exists():
        return json.dumps({
            "error": f"No scan directory found for target: {target}",
            "files": []
        }, indent=2)

    files = []
    for file_path in target_dir.rglob("*"):
        if file_path.is_file():
            rel_path = file_path.relative_to(target_dir)

            # Apply filter if provided
            if filter_pattern:
                if not file_path.match(filter_pattern):
                    continue

            files.append({
                "name": file_path.name,
                "path": str(rel_path),
                "size": file_path.stat().st_size,
                "modified": file_path.stat().st_mtime,
            })

    # Sort by modification time (newest first)
    files.sort(key=lambda x: x["modified"], reverse=True)

    return json.dumps({
        "target": target,
        "files": files,
        "count": len(files),
    }, indent=2)


async def handle_read_scan_file(target: str, filename: str) -> str:
    """
    Read a specific scan file.
    """
    # Support both relative and full paths
    target_dir = DEFAULT_OUTPUT_DIR / target / "scans"

    if not target_dir.exists():
        return f"Error: No scan directory found for target: {target}"

    # Try to find the file
    file_path = target_dir / filename

    if not file_path.exists():
        # Try to find it recursively
        matches = list(target_dir.rglob(filename))
        if matches:
            file_path = matches[0]
        else:
            return f"Error: File not found: {filename}"

    try:
        with open(file_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        return f"# {filename}\n\n{content}"

    except Exception as e:
        return f"Error reading file: {str(e)}"


async def handle_get_discovered_services(target: str) -> str:
    """
    Extract discovered services from Nmap scans.
    """
    target_dir = DEFAULT_OUTPUT_DIR / target / "scans"

    if not target_dir.exists():
        return json.dumps({
            "error": f"No scan directory found for target: {target}",
            "services": []
        }, indent=2)

    services = []

    # Look for nmap service scan results
    nmap_files = list(target_dir.glob("*_nmap*.txt"))

    for nmap_file in nmap_files:
        try:
            with open(nmap_file, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()

                # Parse open ports (looking for lines like "80/tcp open http")
                port_pattern = r"(\d+)/(tcp|udp)\s+open\s+(\S+)(?:\s+(.+))?"
                matches = re.findall(port_pattern, content)

                for match in matches:
                    port, protocol, service, version = match
                    service_info = {
                        "port": int(port),
                        "protocol": protocol,
                        "service": service,
                        "version": version.strip() if version else "",
                    }
                    if service_info not in services:
                        services.append(service_info)

        except Exception as e:
            continue

    # Sort by port number
    services.sort(key=lambda x: x["port"])

    return json.dumps({
        "target": target,
        "services": services,
        "count": len(services),
    }, indent=2)


async def handle_search_results(target: str, query: str, case_sensitive: bool = False) -> str:
    """
    Search across scan results for a target.
    """
    target_dir = DEFAULT_OUTPUT_DIR / target / "scans"

    if not target_dir.exists():
        return json.dumps({
            "error": f"No scan directory found for target: {target}",
            "matches": []
        }, indent=2)

    matches = []
    flags = 0 if case_sensitive else re.IGNORECASE

    try:
        pattern = re.compile(query, flags)
    except re.error as e:
        return f"Error: Invalid regex pattern: {str(e)}"

    # Search through all text files
    for file_path in target_dir.rglob("*"):
        if file_path.is_file() and file_path.suffix in [".txt", ".log", ".xml", ""]:
            try:
                with open(file_path, "r", encoding="utf-8", errors="replace") as f:
                    for line_num, line in enumerate(f, 1):
                        if pattern.search(line):
                            matches.append({
                                "file": str(file_path.relative_to(target_dir)),
                                "line": line_num,
                                "content": line.strip()[:200],  # Truncate long lines
                            })
            except Exception:
                continue

    return json.dumps({
        "target": target,
        "query": query,
        "matches": matches[:100],  # Limit to first 100 matches
        "total_matches": len(matches),
    }, indent=2)


@app.list_prompts()
async def list_prompts() -> list[Prompt]:
    """
    List available prompts for scan analysis.
    """
    return [
        Prompt(
            name="analyze_target",
            description="Analyze scan results for a target and provide a security assessment",
            arguments=[
                {
                    "name": "target",
                    "description": "Target name/IP to analyze",
                    "required": True,
                }
            ],
        ),
        Prompt(
            name="suggest_next_steps",
            description="Suggest next steps for further enumeration or exploitation",
            arguments=[
                {
                    "name": "target",
                    "description": "Target name/IP to analyze",
                    "required": True,
                }
            ],
        ),
        Prompt(
            name="find_vulnerabilities",
            description="Identify potential vulnerabilities from scan results",
            arguments=[
                {
                    "name": "target",
                    "description": "Target name/IP to analyze",
                    "required": True,
                }
            ],
        ),
    ]


@app.get_prompt()
async def get_prompt(name: str, arguments: dict[str, str]) -> GetPromptResult:
    """
    Generate a prompt for scan analysis.
    """
    target = arguments.get("target")

    if not target:
        raise ValueError("Target is required")

    # Get scan data
    target_dir = DEFAULT_OUTPUT_DIR / target
    if not target_dir.exists():
        raise ValueError(f"No results found for target: {target}")

    # Read relevant files
    summary = await generate_summary(target_dir)

    if name == "analyze_target":
        prompt_text = f"""Please analyze the following reconnaissance scan results for {target} and provide a comprehensive security assessment:

{summary}

Provide:
1. Overview of discovered services and attack surface
2. Potential security concerns
3. Notable findings or misconfigurations
4. Risk assessment
"""

    elif name == "suggest_next_steps":
        prompt_text = f"""Based on these reconnaissance scan results for {target}, suggest the next steps for further enumeration:

{summary}

Provide:
1. Prioritized list of services to investigate further
2. Specific enumeration techniques for each service
3. Tools and commands to run
4. What to look for in the results
"""

    elif name == "find_vulnerabilities":
        prompt_text = f"""Analyze these scan results for {target} and identify potential vulnerabilities:

{summary}

Provide:
1. Known vulnerabilities for discovered services/versions
2. Common misconfigurations to check for
3. Potential attack vectors
4. Recommended exploitation techniques (if applicable)
"""

    else:
        raise ValueError(f"Unknown prompt: {name}")

    return GetPromptResult(
        description=f"Analysis prompt for {target}",
        messages=[
            PromptMessage(
                role="user",
                content=TextContent(type="text", text=prompt_text),
            )
        ],
    )


async def main():
    """
    Main entry point for the MCP server.
    """
    # Ensure results directory exists
    DEFAULT_OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Run the server
    async with mcp.server.stdio.stdio_server() as (read_stream, write_stream):
        await app.run(
            read_stream,
            write_stream,
            app.create_initialization_options(),
        )


if __name__ == "__main__":
    asyncio.run(main())
