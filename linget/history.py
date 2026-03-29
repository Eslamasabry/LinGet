"""Task history persistence for LinGet."""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional


# Default history file location
HISTORY_FILE = Path.home() / ".config" / "linget" / "task_history.json"


def ensure_history_dir():
    """Ensure the history directory exists."""
    HISTORY_FILE.parent.mkdir(parents=True, exist_ok=True)


def load_task_history(limit: int = 100) -> List[Dict[str, Any]]:
    """Load task history from disk.

    Args:
        limit: Maximum number of tasks to load (most recent first)

    Returns:
        List of task dictionaries
    """
    if not HISTORY_FILE.exists():
        return []

    try:
        with open(HISTORY_FILE, "r") as f:
            history = json.load(f)
        # Return most recent tasks first
        return history[-limit:]
    except Exception as e:
        print(f"Error loading task history: {e}", flush=True)
        return []


def save_task(
    package_name: str,
    package_source: str,
    action: str,
    status: str,
    error_type: str = "none",
    error_message: str = "",
    timestamp: Optional[str] = None,
):
    """Save a task to history.

    Args:
        package_name: Name of the package
        package_source: Source (apt, flatpak, etc.)
        action: Action performed (install, update, remove)
        status: Final status (done, error, cancelled)
        error_type: Type of error if failed
        error_message: Error message if failed
        timestamp: ISO format timestamp (defaults to now)
    """
    ensure_history_dir()

    task_record = {
        "timestamp": timestamp or datetime.now().isoformat(),
        "package": package_name,
        "source": package_source,
        "action": action,
        "status": status,
        "error_type": error_type,
        "error_message": error_message,
    }

    history = []
    if HISTORY_FILE.exists():
        try:
            with open(HISTORY_FILE, "r") as f:
                history = json.load(f)
        except Exception:
            pass

    history.append(task_record)

    # Keep only last 500 tasks to prevent file bloat
    if len(history) > 500:
        history = history[-500:]

    try:
        with open(HISTORY_FILE, "w") as f:
            json.dump(history, f, indent=2)
    except Exception as e:
        print(f"Error saving task history: {e}", flush=True)


def clear_task_history():
    """Clear all task history."""
    if HISTORY_FILE.exists():
        try:
            HISTORY_FILE.unlink()
        except Exception as e:
            print(f"Error clearing task history: {e}", flush=True)


def get_task_stats(days: int = 7) -> Dict[str, Any]:
    """Get task statistics for the last N days.

    Returns:
        Dict with counts by status, action, source
    """
    history = load_task_history(limit=1000)

    from datetime import timedelta

    cutoff = datetime.now() - timedelta(days=days)

    recent_tasks = [
        t for t in history if datetime.fromisoformat(t["timestamp"]) > cutoff
    ]

    stats = {
        "total": len(recent_tasks),
        "successful": sum(1 for t in recent_tasks if t["status"] == "done"),
        "failed": sum(1 for t in recent_tasks if t["status"] == "error"),
        "cancelled": sum(1 for t in recent_tasks if t["status"] == "cancelled"),
        "by_action": {},
        "by_source": {},
    }

    for task in recent_tasks:
        action = task["action"]
        source = task["source"]
        stats["by_action"][action] = stats["by_action"].get(action, 0) + 1
        stats["by_source"][source] = stats["by_source"].get(source, 0) + 1

    return stats
