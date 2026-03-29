"""Settings persistence for LinGet."""

import json
import os

SETTINGS_PATH = os.path.expanduser("~/.config/linget/settings.json")

DEFAULT_SETTINGS = {
    "theme": "monokai",
    "auto_refresh": True,
    "refresh_interval": 600,
    "confirm_destructive": True,
    "default_source": "all",
    "offline_mode": False,
}


def load_settings() -> dict:
    """Load settings from JSON file.

    Returns merged settings with defaults for any missing keys.
    Handles missing files and corrupt JSON gracefully.
    """
    if not os.path.exists(SETTINGS_PATH):
        return DEFAULT_SETTINGS.copy()

    try:
        with open(SETTINGS_PATH, "r") as f:
            user_settings = json.load(f)

        # Merge with defaults to handle missing keys (migration)
        merged = DEFAULT_SETTINGS.copy()
        merged.update(user_settings)
        return merged
    except (json.JSONDecodeError, IOError, TypeError):
        return DEFAULT_SETTINGS.copy()


def save_settings(settings: dict) -> None:
    """Save settings to JSON file.

    Creates the config directory if it doesn't exist.
    """
    os.makedirs(os.path.dirname(SETTINGS_PATH), exist_ok=True)
    with open(SETTINGS_PATH, "w") as f:
        json.dump(settings, f, indent=2)
