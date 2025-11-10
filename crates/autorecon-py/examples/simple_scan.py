#!/usr/bin/env python3
"""
Simple example of using AutoRecon Python bindings.

This example demonstrates basic usage including:
- Creating a scanner instance
- Listing profiles
- Getting profile information
- Scanning a target (requires security tools installed)
"""

import asyncio
import subprocess
from autorecon import AutoRecon


async def execute_command(cmd: str) -> dict:
    """
    Execute a shell command asynchronously.

    Args:
        cmd: Shell command to execute

    Returns:
        Dictionary with stdout, stderr, exit_code, and duration_ms
    """
    proc = await asyncio.create_subprocess_shell(
        cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )

    start = asyncio.get_event_loop().time()
    stdout, stderr = await proc.communicate()
    duration_ms = int((asyncio.get_event_loop().time() - start) * 1000)

    return {
        'stdout': stdout.decode(),
        'stderr': stderr.decode(),
        'exit_code': proc.returncode,
        'duration_ms': duration_ms
    }


async def main():
    print("AutoRecon Python Bindings - Simple Example")
    print("=" * 50)
    print()

    # Create scanner instance
    print("1. Creating AutoRecon scanner...")
    scanner = AutoRecon(
        config_dir='../../config',  # Adjust path as needed
        profile='default',
        output_dir='./results',
        concurrent_scans=5,
        verbosity=1
    )
    print("   ✓ Scanner created\n")

    # List available profiles
    print("2. Listing available scan profiles...")
    profiles = scanner.list_profiles()
    print(f"   ✓ Found {len(profiles)} profiles: {', '.join(profiles)}\n")

    # Get profile information
    print("3. Getting profile information...")
    for profile_name in profiles:
        info = scanner.get_profile_info(profile_name)
        if info:
            print(f"   • {info['name']}: {info['scan_count']} scans")
            if profile_name == 'default':
                print(f"     Scans: {', '.join(info['scans'])}")
    print()

    # Check if nmap is available
    print("4. Checking for required tools...")
    nmap_check = await execute_command('command -v nmap')
    if nmap_check['exit_code'] == 0:
        print("   ✓ nmap is installed\n")

        # Uncomment to scan localhost (requires nmap)
        # print("5. Scanning localhost...")
        # result = await scanner.scan_target('127.0.0.1', execute_command)
        #
        # if result.success:
        #     print(f"   ✓ Scan completed in {result.duration_ms}ms")
        #     print(f"   ✓ Found {len(result.services)} services:")
        #     for service in result.services:
        #         version = f" ({service.version})" if service.version else ""
        #         print(f"     - {service.protocol}/{service.port}: {service.service}{version}")
        # else:
        #     print(f"   ✗ Scan failed: {result.error}")
    else:
        print("   ⚠ nmap not found - skipping scan example")
        print("     Install nmap to test scanning functionality")

    print("\n" + "=" * 50)
    print("Example completed successfully!")


if __name__ == "__main__":
    asyncio.run(main())
