"""Package cache management for LinGet.

Provides caching functionality to improve startup performance by storing
package lists with timestamps and loading them immediately while refreshing
in the background.
"""

import json
import os
from datetime import datetime, timedelta
from typing import List, Optional

from linget.models import Package, PackageStatus

CACHE_DIR = os.path.expanduser("~/.cache/linget")
CACHE_FILE = os.path.join(CACHE_DIR, "packages.json")
DEFAULT_MAX_AGE_SECONDS = 3600  # 1 hour


def _ensure_cache_dir():
    """Ensure the cache directory exists."""
    os.makedirs(CACHE_DIR, exist_ok=True)


def load_cached_packages() -> Optional[List[Package]]:
    """Load packages from cache if available.

    Returns:
        List of Package objects if cache exists and is readable, None otherwise.
    """
    if not os.path.exists(CACHE_FILE):
        return None

    try:
        with open(CACHE_FILE, "r") as f:
            data = json.load(f)

        packages = []
        for pkg_data in data.get("packages", []):
            # Parse status from string
            status_str = pkg_data.get("status", "installed")
            try:
                status = PackageStatus(status_str)
            except ValueError:
                status = PackageStatus.INSTALLED

            packages.append(
                Package(
                    name=pkg_data.get("name", ""),
                    version=pkg_data.get("version", ""),
                    source=pkg_data.get("source", ""),
                    status=status,
                    size=pkg_data.get("size", ""),
                    desc=pkg_data.get("description", ""),
                )
            )

        return packages
    except (json.JSONDecodeError, KeyError, TypeError, OSError):
        return None


def save_cached_packages(packages: List[Package]) -> bool:
    """Save packages to cache with current timestamp.

    Args:
        packages: List of Package objects to cache.

    Returns:
        True if save was successful, False otherwise.
    """
    _ensure_cache_dir()

    try:
        data = {
            "timestamp": datetime.now().isoformat(),
            "packages": [
                {
                    "name": pkg.name,
                    "version": pkg.version,
                    "source": pkg.source,
                    "status": pkg.status.value,
                    "size": pkg.size,
                    "description": pkg.description,
                }
                for pkg in packages
            ],
        }

        # Write to temp file then rename for atomicity
        temp_file = CACHE_FILE + ".tmp"
        with open(temp_file, "w") as f:
            json.dump(data, f, indent=2)

        os.replace(temp_file, CACHE_FILE)
        return True
    except (OSError, TypeError):
        return False


def is_cache_valid(max_age: int = DEFAULT_MAX_AGE_SECONDS) -> bool:
    """Check if cache exists and is within the valid age.

    Args:
        max_age: Maximum age in seconds (default 3600 = 1 hour).

    Returns:
        True if cache exists and is not older than max_age.
    """
    if not os.path.exists(CACHE_FILE):
        return False

    try:
        with open(CACHE_FILE, "r") as f:
            data = json.load(f)

        timestamp_str = data.get("timestamp")
        if not timestamp_str:
            return False

        cache_time = datetime.fromisoformat(timestamp_str)
        age = datetime.now() - cache_time

        return age.total_seconds() <= max_age
    except (json.JSONDecodeError, KeyError, TypeError, ValueError, OSError):
        return False


def get_cache_timestamp() -> Optional[datetime]:
    """Get the timestamp of the cached data.

    Returns:
        datetime when cache was created, or None if cache doesn't exist.
    """
    if not os.path.exists(CACHE_FILE):
        return None

    try:
        with open(CACHE_FILE, "r") as f:
            data = json.load(f)

        timestamp_str = data.get("timestamp")
        if not timestamp_str:
            return None

        return datetime.fromisoformat(timestamp_str)
    except (json.JSONDecodeError, KeyError, TypeError, ValueError, OSError):
        return None


def clear_cache() -> bool:
    """Clear the package cache.

    Returns:
        True if cache was cleared or didn't exist, False on error.
    """
    try:
        if os.path.exists(CACHE_FILE):
            os.remove(CACHE_FILE)
        return True
    except OSError:
        return False


def get_cache_age_text() -> str:
    """Get human-readable text describing cache age.

    Returns:
        String like "2 hours ago", "just now", "Never", etc.
    """
    timestamp = get_cache_timestamp()
    if not timestamp:
        return "Never"

    age = datetime.now() - timestamp

    if age.total_seconds() < 60:
        return "just now"
    elif age.total_seconds() < 3600:
        minutes = int(age.total_seconds() / 60)
        return f"{minutes} minute{'s' if minutes != 1 else ''} ago"
    elif age.total_seconds() < 86400:
        hours = int(age.total_seconds() / 3600)
        return f"{hours} hour{'s' if hours != 1 else ''} ago"
    else:
        days = int(age.total_seconds() / 86400)
        return f"{days} day{'s' if days != 1 else ''} ago"


def should_use_cache(max_age: int = DEFAULT_MAX_AGE_SECONDS) -> bool:
    """Check if we should use the cache (exists and is valid).

    Convenience function combining existence and validity checks.

    Args:
        max_age: Maximum age in seconds (default 3600 = 1 hour).

    Returns:
        True if cache should be used for immediate display.
    """
    return is_cache_valid(max_age) or os.path.exists(CACHE_FILE)
