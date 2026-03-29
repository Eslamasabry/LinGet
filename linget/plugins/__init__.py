"""Plugin discovery and management for LinGet.

This module provides the main entry point for plugin functionality.
Import from here to access the plugin system.

Example:
    from linget.plugins import get_plugin_registry, load_plugins

    # Load plugins at startup
    registry = get_plugin_registry()
    count = load_plugins(registry)
    print(f"Loaded {count} plugins")

    # Access loaded plugins
    for name in registry.plugin_names:
        print(f"Plugin: {name}")
"""

from ..plugin import (
    PackageBackendPlugin,
    PluginRegistry,
    API_VERSION,
    PLUGINS_DIR,
    load_plugins,
    reload_plugins,
    get_plugin_registry,
)

__all__ = [
    "PackageBackendPlugin",
    "PluginRegistry",
    "API_VERSION",
    "PLUGINS_DIR",
    "load_plugins",
    "reload_plugins",
    "get_plugin_registry",
]
