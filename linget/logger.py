"""Centralized logging configuration for LinGet.

Provides consistent logging across the application with file and console output,
automatic log rotation, and proper formatting for production use.
"""

import logging
import logging.handlers
import os
import sys
import traceback
from pathlib import Path
from typing import Optional


# Log directory and file paths
LOG_DIR = Path.home() / ".local" / "share" / "linget"
LOG_FILE = LOG_DIR / "linget.log"
MAX_LOG_SIZE = 5 * 1024 * 1024  # 5 MB
MAX_BACKUP_COUNT = 3

# Log format strings
CONSOLE_FORMAT = "%(levelname)s: %(message)s"
FILE_FORMAT = (
    "%(asctime)s - %(name)s - %(levelname)s - %(filename)s:%(lineno)d - %(message)s"
)

# Global logger instance
_logger: Optional[logging.Logger] = None


def ensure_log_dir() -> None:
    """Ensure the log directory exists."""
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def setup_logging(
    level: int = logging.INFO,
    log_to_file: bool = True,
    log_to_console: bool = True,
) -> logging.Logger:
    """Set up logging for LinGet.

    Args:
        level: Minimum log level (DEBUG, INFO, WARNING, ERROR)
        log_to_file: Whether to log to file
        log_to_console: Whether to log to console

    Returns:
        Configured logger instance
    """
    global _logger

    if _logger is not None:
        return _logger

    logger = logging.getLogger("linget")
    logger.setLevel(level)
    logger.handlers = []  # Clear any existing handlers

    # File handler with rotation
    if log_to_file:
        ensure_log_dir()
        file_handler = logging.handlers.RotatingFileHandler(
            LOG_FILE,
            maxBytes=MAX_LOG_SIZE,
            backupCount=MAX_BACKUP_COUNT,
            encoding="utf-8",
        )
        file_handler.setLevel(logging.DEBUG)  # Log everything to file
        file_formatter = logging.Formatter(FILE_FORMAT)
        file_handler.setFormatter(file_formatter)
        logger.addHandler(file_handler)

    # Console handler
    if log_to_console:
        console_handler = logging.StreamHandler(sys.stdout)
        console_handler.setLevel(level)
        console_formatter = logging.Formatter(CONSOLE_FORMAT)
        console_handler.setFormatter(console_formatter)
        logger.addHandler(console_handler)

    _logger = logger
    return logger


def get_logger() -> logging.Logger:
    """Get the global logger instance.

    If logging hasn't been set up, initializes with defaults.

    Returns:
        Logger instance
    """
    global _logger
    if _logger is None:
        return setup_logging()
    return _logger


def log_exception(
    exc: Exception,
    context: str = "",
    level: int = logging.ERROR,
) -> None:
    """Log an exception with full traceback and context.

    Args:
        exc: The exception to log
        context: Additional context about where/why the error occurred
        level: Log level for the exception
    """
    logger = get_logger()

    if context:
        message = f"{context}: {str(exc)}"
    else:
        message = str(exc)

    tb_str = traceback.format_exc()

    logger.log(level, f"{message}\nTraceback:\n{tb_str}")


def log_subprocess_error(
    cmd: list,
    return_code: int,
    stderr: Optional[str] = None,
    context: str = "",
) -> None:
    """Log a subprocess error with command details.

    Args:
        cmd: Command that was executed (as list)
        return_code: Return code from the process
        stderr: Stderr output if available
        context: Additional context about the operation
    """
    logger = get_logger()

    cmd_str = " ".join(str(c) for c in cmd)
    message = f"Subprocess failed (exit {return_code}): {cmd_str}"

    if context:
        message = f"{context}: {message}"

    if stderr:
        message += f"\nStderr: {stderr[:500]}"  # Limit stderr length

    logger.error(message)


def debug(msg: str, *args, **kwargs) -> None:
    """Log a debug message."""
    get_logger().debug(msg, *args, **kwargs)


def info(msg: str, *args, **kwargs) -> None:
    """Log an info message."""
    get_logger().info(msg, *args, **kwargs)


def warning(msg: str, *args, **kwargs) -> None:
    """Log a warning message."""
    get_logger().warning(msg, *args, **kwargs)


def error(msg: str, *args, **kwargs) -> None:
    """Log an error message."""
    get_logger().error(msg, *args, **kwargs)


def critical(msg: str, *args, **kwargs) -> None:
    """Log a critical message."""
    get_logger().critical(msg, *args, **kwargs)
