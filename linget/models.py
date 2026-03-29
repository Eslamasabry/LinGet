"""Data models for LinGet."""

import json
import os
from datetime import datetime
from enum import Enum


# Favorites persistence
FAVORITES_PATH = os.path.expanduser("~/.config/linget/favorites.json")


def load_favorites():
    """Load favorites from JSON file. Returns set of row_keys."""
    if not os.path.exists(FAVORITES_PATH):
        return set()
    try:
        with open(FAVORITES_PATH, "r") as f:
            data = json.load(f)
            return set(f"{item['source']}-{item['name']}" for item in data)
    except (json.JSONDecodeError, KeyError, TypeError):
        return set()


def save_favorites(favorites_set):
    """Save favorites set to JSON file as list of {source, name} objects."""
    os.makedirs(os.path.dirname(FAVORITES_PATH), exist_ok=True)
    data = []
    for row_key in sorted(favorites_set):
        parts = row_key.split("-", 1)
        if len(parts) == 2:
            data.append({"source": parts[0], "name": parts[1]})
    with open(FAVORITES_PATH, "w") as f:
        json.dump(data, f, indent=2)


def is_favorite(pkg, favorites_set):
    """Check if a package is in the favorites set."""
    return pkg.row_key in favorites_set


class PackageStatus(Enum):
    INSTALLED = "installed"
    UPDATE = "update"
    NOT_INSTALLED = "available"


class ErrorType(Enum):
    """Classify task failures for better user feedback and retry strategies."""

    NONE = "none"
    AUTH_CANCELLED = "auth_cancelled"
    NETWORK = "network"
    NOT_FOUND = "not_found"
    CONFLICT = "conflict"
    LOCKED = "locked"
    DISK_FULL = "disk_full"
    PERMISSION = "permission"
    TIMEOUT = "timeout"
    UNKNOWN = "unknown"


class Package:
    """Represents a package from any source."""

    def __init__(self, name, version, source, status, size="", desc=""):
        self.name = name
        self.version = version
        self.source = source
        self.status = status
        self.size = size
        self.description = desc

    @property
    def row_key(self) -> str:
        """Unique key for DataTable row."""
        return f"{self.source}-{self.name}"


class Task:
    """Represents a package operation task."""

    def __init__(self, package: Package, action: str):
        self.id = (
            f"{action}-{package.source}-{package.name}-{datetime.now().timestamp():.0f}"
        )
        self.package = package
        self.action = action
        self.progress = 0.0
        self.status = "queued"
        self.error_type = ErrorType.NONE
        self.error_message = ""
