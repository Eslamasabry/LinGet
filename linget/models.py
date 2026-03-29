"""Data models for LinGet."""

from datetime import datetime
from enum import Enum


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
