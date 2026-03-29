"""Search functionality for finding packages across repositories."""

import asyncio
from typing import List
from .models import Package, PackageStatus


async def search_apt_repositories(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search APT repositories for packages matching query."""
    found = []

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    try:
        code, out = await run_cmd(["apt-cache", "search", "--names-only", query])
        if code == 0:
            for line in out.splitlines():
                if " - " in line:
                    parts = line.split(" - ", 1)
                    name = parts[0].strip()
                    desc = parts[1].strip() if len(parts) > 1 else ""

                    # Check if already installed
                    already_installed = any(
                        p.name == name and p.source == "apt" for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version="?",
                                source="apt",
                                status=PackageStatus.NOT_INSTALLED,
                                desc=desc,
                            )
                        )
    except Exception as e:
        print(f"APT search error: {e}", flush=True)

    return found


async def search_flatpak_remotes(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search Flatpak remotes for packages matching query."""
    found = []

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    try:
        code, out = await run_cmd(["flatpak", "search", query])
        if code == 0:
            for line in out.splitlines():
                parts = line.split("\t")
                if len(parts) >= 3:
                    name = parts[0]
                    desc = parts[2] if len(parts) > 2 else ""

                    already_installed = any(
                        p.name == name and p.source == "flatpak"
                        for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version="?",
                                source="flatpak",
                                status=PackageStatus.NOT_INSTALLED,
                                desc=desc,
                            )
                        )
    except Exception as e:
        print(f"Flatpak search error: {e}", flush=True)

    return found


async def search_new_packages(
    query: str, installed_packages: List[Package], source_filter: str = "all"
) -> List[Package]:
    """Search for new packages across all enabled repositories.

    Args:
        query: Search string
        installed_packages: Currently known packages to exclude
        source_filter: "all" or specific source like "apt", "flatpak"

    Returns:
        List of new packages found
    """
    found_packages = []

    # Search APT repositories
    if source_filter in ("all", "apt"):
        apt_packages = await search_apt_repositories(query, installed_packages)
        found_packages.extend(apt_packages)

    # Search Flatpak remotes
    if source_filter in ("all", "flatpak"):
        flatpak_packages = await search_flatpak_remotes(query, installed_packages)
        found_packages.extend(flatpak_packages)

    return found_packages
