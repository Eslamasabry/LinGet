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


async def search_snap_store(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search Snap store for packages matching query."""
    found = []

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    try:
        code, out = await run_cmd(["snap", "find", query])
        if code == 0:
            for line in out.splitlines()[1:]:  # Skip header line
                parts = line.split()
                if len(parts) >= 1:
                    name = parts[0]
                    version = parts[1] if len(parts) > 1 else "?"
                    publisher = parts[2] if len(parts) > 2 else "?"
                    desc = " ".join(parts[3:]) if len(parts) > 3 else ""

                    already_installed = any(
                        p.name == name and p.source == "snap"
                        for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version=version,
                                source="snap",
                                status=PackageStatus.NOT_INSTALLED,
                                desc=f"{desc} (by {publisher})",
                            )
                        )
    except Exception as e:
        print(f"Snap search error: {e}", flush=True)

    return found


async def search_aur(query: str, installed_packages: List[Package]) -> List[Package]:
    """Search AUR for packages matching query using yay or paru."""
    found = []

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    # Check for yay first, fallback to paru
    aur_helper = None
    for helper in ["yay", "paru"]:
        try:
            proc = await asyncio.create_subprocess_exec(
                "which",
                helper,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await proc.communicate()
            if proc.returncode == 0:
                aur_helper = helper
                break
        except:
            continue

    if not aur_helper:
        return found

    try:
        code, out = await run_cmd([aur_helper, "-Ss", query])
        if code == 0:
            current_pkg = None
            for line in out.splitlines():
                line = line.strip()
                if not line:
                    continue

                # AUR search format: "aur/package-name version ..."
                if line.startswith("aur/"):
                    parts = line.split()
                    if len(parts) >= 2:
                        name = parts[0].replace("aur/", "")
                        version = parts[1]
                        # Description is on the same line or next lines
                        desc = " ".join(parts[2:]) if len(parts) > 2 else ""
                        current_pkg = (name, version, desc)
                elif current_pkg and not line.startswith("aur/"):
                    # This is a continuation of description
                    name, version, desc = current_pkg
                    desc = desc + " " + line if desc else line
                    current_pkg = (name, version, desc)

                # Check if we've completed this entry
                if current_pkg and (line.startswith("aur/") or not line.strip()):
                    name, version, desc = current_pkg
                    already_installed = any(
                        p.name == name and p.source == "aur" for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version=version,
                                source="aur",
                                status=PackageStatus.NOT_INSTALLED,
                                desc=desc or "AUR Package",
                            )
                        )
                    current_pkg = None

            # Don't forget the last package
            if current_pkg:
                name, version, desc = current_pkg
                already_installed = any(
                    p.name == name and p.source == "aur" for p in installed_packages
                )
                if not already_installed:
                    found.append(
                        Package(
                            name=name,
                            version=version,
                            source="aur",
                            status=PackageStatus.NOT_INSTALLED,
                            desc=desc or "AUR Package",
                        )
                    )
    except Exception as e:
        print(f"AUR search error: {e}", flush=True)

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

    # Search Snap store
    if source_filter in ("all", "snap"):
        snap_packages = await search_snap_store(query, installed_packages)
        found_packages.extend(snap_packages)

    # Search AUR
    if source_filter in ("all", "aur"):
        aur_packages = await search_aur(query, installed_packages)
        found_packages.extend(aur_packages)

    # Search DNF repositories
    if source_filter in ("all", "dnf"):
        dnf_packages = await search_dnf_repositories(query, installed_packages)
        found_packages.extend(dnf_packages)

    # Step 43: Search Homebrew (macOS only)
    if source_filter in ("all", "brew"):
        brew_packages = await search_brew(query, installed_packages)
        found_packages.extend(brew_packages)

    return found_packages


async def search_dnf_repositories(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search DNF repositories for packages matching query."""
    found = []

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    try:
        # First check if dnf is available
        proc = await asyncio.create_subprocess_exec(
            "which",
            "dnf",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        await proc.communicate()
        if proc.returncode != 0:
            return found  # dnf not available

        code, out = await run_cmd(["dnf", "search", query])
        if code == 0:
            for line in out.splitlines():
                # Skip headers and metadata lines
                if not line or line.startswith(" ") or line.startswith("Last metadata"):
                    continue
                # Parse format: name.arch : description
                if " : " in line:
                    parts = line.split(" : ", 1)
                    name_arch = parts[0].strip()
                    desc = parts[1].strip() if len(parts) > 1 else ""
                    # Extract name from "name.arch" format
                    if "." in name_arch:
                        name = name_arch.rsplit(".", 1)[0]
                    else:
                        name = name_arch

                    # Check if already installed
                    already_installed = any(
                        p.name == name and p.source == "dnf" for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version="?",
                                source="dnf",
                                status=PackageStatus.NOT_INSTALLED,
                                desc=desc,
                            )
                        )
    except Exception as e:
        print(f"DNF search error: {e}", flush=True)

    return found


async def search_brew(query: str, installed_packages: List[Package]) -> List[Package]:
    """Search Homebrew formulae and casks matching query."""
    found = []
    import sys

    # Only search on macOS
    if sys.platform != "darwin":
        return found

    # Check if brew is available
    import shutil

    if not shutil.which("brew"):
        return found

    async def run_cmd(cmd):
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, _ = await proc.communicate()
        return proc.returncode, stdout.decode(errors="ignore")

    try:
        # Search for formulae
        code, out = await run_cmd(["brew", "search", "--formula", query])
        if code == 0:
            for line in out.splitlines():
                name = line.strip()
                if name and not name.startswith("==>"):
                    already_installed = any(
                        p.name == name and p.source == "brew"
                        for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version="?",
                                source="brew",
                                status=PackageStatus.NOT_INSTALLED,
                                desc="Homebrew Formula",
                            )
                        )

        # Search for casks
        code, out = await run_cmd(["brew", "search", "--cask", query])
        if code == 0:
            for line in out.splitlines():
                name = line.strip()
                if name and not name.startswith("==>"):
                    already_installed = any(
                        p.name == name and p.source == "brew"
                        for p in installed_packages
                    )
                    if not already_installed:
                        found.append(
                            Package(
                                name=name,
                                version="?",
                                source="brew",
                                status=PackageStatus.NOT_INSTALLED,
                                desc="Homebrew Cask",
                            )
                        )
    except Exception as e:
        print(f"Homebrew search error: {e}", flush=True)

    return found
