"""Input validation and sanitization for LinGet.

Provides validation functions for package names, search queries, and file paths
to ensure safe operation and prevent injection attacks.
"""

import os
import re
from pathlib import Path
from typing import Optional, Tuple, List

from . import logger


# Validation constants
MAX_PACKAGE_NAME_LENGTH = 128
MAX_QUERY_LENGTH = 256
MAX_FILE_PATH_LENGTH = 4096

# Package name pattern: alphanumeric, hyphens, dots, plus signs (common in package names)
# Block shell metacharacters and path traversal sequences
PACKAGE_NAME_PATTERN = re.compile(r"^[a-zA-Z0-9][a-zA-Z0-9._+\-]*$")

# Blocked characters that could be used for injection
BLOCKED_CHARS = set(";|&`$(){}[]<>!\\\"'*?\n\r\t")

# Path traversal patterns
PATH_TRAVERSAL_PATTERNS = ["..", "~", "//"]

# Maximum concurrent tasks to prevent resource exhaustion
MAX_CONCURRENT_TASKS = 50

# Default timeout for subprocess calls (seconds)
DEFAULT_SUBPROCESS_TIMEOUT = 300  # 5 minutes


class ValidationError(Exception):
    """Exception raised for validation errors."""

    def __init__(self, message: str, field: str = ""):
        self.message = message
        self.field = field
        super().__init__(message)


def validate_package_name(name: str) -> Tuple[bool, str]:
    """Validate a package name.

    Args:
        name: Package name to validate

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not name:
        return False, "Package name cannot be empty"

    if len(name) > MAX_PACKAGE_NAME_LENGTH:
        return (
            False,
            f"Package name too long (max {MAX_PACKAGE_NAME_LENGTH} characters)",
        )

    # Check for blocked characters
    for char in name:
        if char in BLOCKED_CHARS:
            return False, f"Package name contains invalid character: '{char}'"

    # Check against pattern
    if not PACKAGE_NAME_PATTERN.match(name):
        return (
            False,
            "Package name contains invalid characters (allowed: alphanumeric, hyphens, dots, plus signs)",
        )

    return True, ""


def sanitize_search_query(query: str) -> str:
    """Sanitize a search query by removing dangerous characters.

    Args:
        query: Raw search query

    Returns:
        Sanitized query string
    """
    if not query:
        return ""

    # Limit length
    if len(query) > MAX_QUERY_LENGTH:
        query = query[:MAX_QUERY_LENGTH]
        logger.warning(f"Search query truncated to {MAX_QUERY_LENGTH} characters")

    # Remove blocked characters
    sanitized = "".join(c for c in query if c not in BLOCKED_CHARS)

    # Remove path traversal patterns
    for pattern in PATH_TRAVERSAL_PATTERNS:
        sanitized = sanitized.replace(pattern, "")

    # Normalize whitespace
    sanitized = " ".join(sanitized.split())

    return sanitized.strip()


def validate_search_query(query: str) -> Tuple[bool, str]:
    """Validate a search query.

    Args:
        query: Search query to validate

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not query:
        return False, "Search query cannot be empty"

    if len(query) > MAX_QUERY_LENGTH:
        return False, f"Search query too long (max {MAX_QUERY_LENGTH} characters)"

    sanitized = sanitize_search_query(query)

    if not sanitized:
        return False, "Search query contains only invalid characters"

    # Check for suspicious patterns
    suspicious = ["$(", "${", "`", "||", "&&", ";"]
    for pattern in suspicious:
        if pattern in query:
            return False, f"Search query contains suspicious pattern: {pattern}"

    return True, ""


def validate_file_path(
    path: str,
    must_exist: bool = False,
    allow_absolute: bool = True,
) -> Tuple[bool, str]:
    """Validate a file path for import/export operations.

    Args:
        path: File path to validate
        must_exist: Whether the file must already exist
        allow_absolute: Whether absolute paths are allowed

    Returns:
        Tuple of (is_valid, error_message)
    """
    if not path:
        return False, "File path cannot be empty"

    if len(path) > MAX_FILE_PATH_LENGTH:
        return False, f"File path too long (max {MAX_FILE_PATH_LENGTH} characters)"

    # Check for path traversal
    for pattern in PATH_TRAVERSAL_PATTERNS:
        if pattern in path:
            return False, f"Path contains invalid pattern: {pattern}"

    # Check for blocked characters
    for char in path:
        if char in BLOCKED_CHARS and char != "/":  # Allow forward slash for paths
            return False, f"Path contains invalid character: '{char}'"

    try:
        p = Path(path)

        # Check if absolute paths are allowed
        if p.is_absolute() and not allow_absolute:
            return False, "Absolute paths are not allowed"

        # Resolve the path
        resolved = p.resolve()

        # Check for path traversal after resolution
        home = Path.home().resolve()
        if not str(resolved).startswith(str(home)) and not allow_absolute:
            return False, "Path must be within home directory"

        if must_exist and not resolved.exists():
            return False, f"File does not exist: {path}"

        if must_exist and not resolved.is_file():
            return False, f"Path is not a file: {path}"

        return True, ""

    except (OSError, ValueError) as e:
        return False, f"Invalid path: {str(e)}"


def validate_bulk_operation_count(count: int, threshold: int = 5) -> Tuple[bool, str]:
    """Validate the number of packages for bulk operations.

    Args:
        count: Number of packages
        threshold: Threshold for warning

    Returns:
        Tuple of (is_valid, error_message)
    """
    if count <= 0:
        return False, "No packages selected for operation"

    if count > MAX_CONCURRENT_TASKS:
        return False, f"Too many packages selected (max {MAX_CONCURRENT_TASKS})"

    if count > threshold:
        return True, f"Large operation: {count} packages selected"

    return True, ""


def sanitize_package_list(packages: List[str]) -> List[str]:
    """Sanitize a list of package names, removing invalid entries.

    Args:
        packages: List of package names

    Returns:
        List of valid package names
    """
    valid = []
    for pkg in packages:
        is_valid, error = validate_package_name(pkg)
        if is_valid:
            valid.append(pkg)
        else:
            logger.warning(f"Skipping invalid package name '{pkg}': {error}")

    return valid


def validate_source_name(
    source: str, allowed_sources: Optional[List[str]] = None
) -> bool:
    """Validate a package source name.

    Args:
        source: Source name to validate
        allowed_sources: List of allowed sources (if None, uses default list)

    Returns:
        True if valid, False otherwise
    """
    if not source:
        return False

    default_sources = [
        "apt",
        "flatpak",
        "cargo",
        "npm",
        "pip",
        "snap",
        "aur",
        "dnf",
        "brew",
        "all",
        "favorites",
    ]

    allowed = allowed_sources or default_sources

    return source.lower() in [s.lower() for s in allowed]


def get_safe_filename(filename: str) -> str:
    """Convert a string to a safe filename.

    Args:
        filename: Original filename

    Returns:
        Sanitized filename
    """
    if not filename:
        return "unnamed"

    # Replace unsafe characters
    unsafe = '<>:"/\\|?*'
    for char in unsafe:
        filename = filename.replace(char, "_")

    # Limit length
    if len(filename) > 255:
        name, ext = os.path.splitext(filename)
        filename = name[: 255 - len(ext)] + ext

    # Ensure not empty after sanitization
    if not filename or filename == ".":
        filename = "unnamed"

    return filename
