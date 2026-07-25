"""Search functionality for finding packages across repositories."""

import asyncio
import shutil
from typing import List
from .models import Package, PackageStatus


SEARCH_TIMEOUT = 30


async def _run_cmd(cmd: List[str]) -> tuple[int, str]:
    proc = await asyncio.create_subprocess_exec(
        *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
    )
    stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=SEARCH_TIMEOUT)
    return proc.returncode, stdout.decode(errors="ignore")


async def search_apt_repositories(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search APT repositories for packages matching query."""
    found = []
    if not shutil.which("apt-cache"):
        return found

    try:
        code, out = await _run_cmd(["apt-cache", "search", "--names-only", query])
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
    if not shutil.which("flatpak"):
        return found

    try:
        code, out = await _run_cmd(["flatpak", "search", query])
        if code == 0:
            for line in out.splitlines():
                parts = line.split("\t")
                if len(parts) >= 3:
                    name = parts[2].strip()
                    desc = parts[1].strip() if len(parts) > 1 else ""

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
    if not shutil.which("snap"):
        return found

    try:
        code, out = await _run_cmd(["snap", "find", query])
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

    # Check for yay first, fallback to paru
    aur_helper = None
    for helper in ["yay", "paru"]:
        try:
            if shutil.which(helper):
                aur_helper = helper
                break
        except:
            continue

    if not aur_helper:
        return found

    try:
        code, out = await _run_cmd([aur_helper, "-Ss", query])
        if code == 0:
            current_pkg = None

            def emit_package(pkg_data):
                if not pkg_data:
                    return
                name, version, desc = pkg_data
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

            for line in out.splitlines():
                line = line.strip()
                if not line:
                    continue

                if line.startswith("aur/"):
                    emit_package(current_pkg)
                    parts = line.split()
                    if len(parts) >= 2:
                        name = parts[0].replace("aur/", "")
                        version = parts[1]
                        desc = " ".join(parts[2:]) if len(parts) > 2 else ""
                        current_pkg = (name, version, desc)
                elif current_pkg and not line.startswith("aur/"):
                    name, version, desc = current_pkg
                    desc = desc + " " + line if desc else line
                    current_pkg = (name, version, desc)
            emit_package(current_pkg)
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
    tasks = []

    if source_filter in ("all", "apt"):
        tasks.append(search_apt_repositories(query, installed_packages))
    if source_filter in ("all", "flatpak"):
        tasks.append(search_flatpak_remotes(query, installed_packages))
    if source_filter in ("all", "snap"):
        tasks.append(search_snap_store(query, installed_packages))
    if source_filter in ("all", "aur"):
        tasks.append(search_aur(query, installed_packages))
    if source_filter in ("all", "dnf"):
        tasks.append(search_dnf_repositories(query, installed_packages))
    if source_filter in ("all", "brew"):
        tasks.append(search_brew(query, installed_packages))

    found_packages: List[Package] = []
    results = await asyncio.gather(*tasks, return_exceptions=True)
    for result in results:
        if isinstance(result, list):
            found_packages.extend(result)

    return found_packages


async def search_dnf_repositories(
    query: str, installed_packages: List[Package]
) -> List[Package]:
    """Search DNF repositories for packages matching query."""
    found = []
    if not shutil.which("dnf"):
        return found

    try:
        code, out = await _run_cmd(["dnf", "search", query])
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

    try:
        # Search for formulae
        code, out = await _run_cmd(["brew", "search", "--formula", query])
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
        code, out = await _run_cmd(["brew", "search", "--cask", query])
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
