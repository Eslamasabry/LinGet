"""Plugin system for LinGet - Base classes and registry.

This module provides the foundation for custom package backends via plugins.
Plugins are loaded from ~/.config/linget/plugins/ and must follow the
PackageBackendPlugin interface.
"""

import importlib.util
import os
from abc import ABC, abstractmethod
from typing import List, Optional
from .models import Package, PackageStatus


API_VERSION = "1.0"

PLUGINS_DIR = os.path.expanduser("~/.config/linget/plugins")


class PackageBackendPlugin(ABC):
    """Base class for custom package backend plugins.

    Plugins must inherit from this class and implement all abstract methods.
    The plugin system supports API versioning for backward compatibility.

    Attributes:
        name: Unique identifier for the plugin (used as source name)
        api_version: The API version this plugin was built for
        description: Human-readable description of the plugin
    """

    name: str = ""
    api_version: str = "1.0"
    description: str = ""

    @abstractmethod
    def can_execute(self, pkg: Package) -> bool:
        """Check if this plugin can handle the given package.

        Args:
            pkg: The package to check

        Returns:
            True if this plugin can install/update/remove this package
        """
        pass

    @abstractmethod
    def fetch_installed(self) -> List[Package]:
        """Fetch list of currently installed packages.

        Returns:
            List of Package objects representing installed packages
        """
        pass

    @abstractmethod
    def install(self, pkg: Package) -> bool:
        """Install the given package.

        Args:
            pkg: Package to install

        Returns:
            True if installation succeeded, False otherwise
        """
        pass

    @abstractmethod
    def remove(self, pkg: Package) -> bool:
        """Remove/uninstall the given package.

        Args:
            pkg: Package to remove

        Returns:
            True if removal succeeded, False otherwise
        """
        pass

    @abstractmethod
    def update(self, pkg: Package) -> bool:
        """Update the given package to latest version.

        Args:
            pkg: Package to update

        Returns:
            True if update succeeded, False otherwise
        """
        pass

    @abstractmethod
    def search(self, query: str) -> List[Package]:
        """Search for packages matching the query.

        Args:
            query: Search string

        Returns:
            List of Package objects found (not installed)
        """
        pass

    def check_api_version(self) -> bool:
        """Verify plugin API version compatibility.

        Returns:
            True if plugin API version is compatible
        """
        return self.api_version == API_VERSION


class PluginRegistry:
    """Registry for managing loaded plugins.

    This class handles plugin discovery, loading, and lookup.
    It's used by the main application to route tasks to the appropriate plugin.
    """

    def __init__(self):
        self._plugins: dict[str, PackageBackendPlugin] = {}
        self._load_errors: List[str] = []

    def register(self, plugin: PackageBackendPlugin) -> bool:
        """Register a plugin instance.

        Args:
            plugin: Plugin instance to register

        Returns:
            True if registration succeeded
        """
        if not plugin.name:
            self._load_errors.append("Plugin missing name attribute")
            return False

        if not plugin.check_api_version():
            self._load_errors.append(
                f"Plugin '{plugin.name}' API version {plugin.api_version} "
                f"!= required {API_VERSION}"
            )
            return False

        self._plugins[plugin.name] = plugin
        return True

    def unregister(self, name: str) -> bool:
        """Remove a plugin from the registry.

        Args:
            name: Plugin name to remove

        Returns:
            True if plugin was found and removed
        """
        if name in self._plugins:
            del self._plugins[name]
            return True
        return False

    def get(self, name: str) -> Optional[PackageBackendPlugin]:
        """Get a plugin by name.

        Args:
            name: Plugin identifier

        Returns:
            Plugin instance or None if not found
        """
        return self._plugins.get(name)

    def get_for_package(self, pkg: Package) -> Optional[PackageBackendPlugin]:
        """Find plugin that can handle the given package.

        Args:
            pkg: Package to find handler for

        Returns:
            Compatible plugin or None
        """
        for plugin in self._plugins.values():
            if plugin.can_execute(pkg):
                return plugin
        return None

    def get_all_installed(self) -> List[Package]:
        """Fetch installed packages from all plugins.

        Returns:
            Combined list of packages from all registered plugins
        """
        all_packages = []
        for plugin in self._plugins.values():
            try:
                packages = plugin.fetch_installed()
                all_packages.extend(packages)
            except Exception as e:
                self._load_errors.append(f"Plugin '{plugin.name}' fetch error: {e}")
        return all_packages

    def search_all(self, query: str) -> List[Package]:
        """Search for packages across all plugins.

        Args:
            query: Search string

        Returns:
            Combined search results from all plugins
        """
        results = []
        for plugin in self._plugins.values():
            try:
                found = plugin.search(query)
                results.extend(found)
            except Exception as e:
                self._load_errors.append(f"Plugin '{plugin.name}' search error: {e}")
        return results

    @property
    def plugin_names(self) -> List[str]:
        """Return list of registered plugin names."""
        return list(self._plugins.keys())

    @property
    def plugins(self) -> List[PackageBackendPlugin]:
        """Return list of registered plugin instances."""
        return list(self._plugins.values())

    @property
    def load_errors(self) -> List[str]:
        """Return list of errors encountered during loading."""
        return self._load_errors.copy()

    def clear_errors(self):
        """Clear the error log."""
        self._load_errors.clear()


def load_plugins(registry: PluginRegistry) -> int:
    """Discover and load plugins from the plugins directory.

    Scans ~/.config/linget/plugins/ for files matching *_plugin.py,
    imports them, and registers any PackageBackendPlugin instances found.

    Args:
        registry: PluginRegistry to populate

    Returns:
        Number of plugins successfully loaded
    """
    if not os.path.exists(PLUGINS_DIR):
        os.makedirs(PLUGINS_DIR, exist_ok=True)
        return 0

    loaded_count = 0

    for filename in os.listdir(PLUGINS_DIR):
        if not filename.endswith("_plugin.py"):
            continue

        plugin_path = os.path.join(PLUGINS_DIR, filename)
        module_name = filename[:-3]  # Remove .py

        try:
            spec = importlib.util.spec_from_file_location(module_name, plugin_path)
            if spec is None or spec.loader is None:
                registry._load_errors.append(f"Failed to load spec for {filename}")
                continue

            module = importlib.util.module_from_spec(spec)

            exec_module_with_isolation(spec.loader, module, registry, filename)
            loaded_count += 1

        except Exception as e:
            registry._load_errors.append(f"Failed to load {filename}: {e}")

    return loaded_count


def exec_module_with_isolation(
    loader, module, registry: PluginRegistry, filename: str
) -> None:
    """Execute a plugin module with exception isolation.

    This ensures that a single failing plugin doesn't crash the entire
    plugin loading process.

    Args:
        loader: The module spec loader
        module: The module to execute
        registry: Registry to register found plugins
        filename: Source filename for error reporting
    """
    try:
        loader.exec_module(module)

        # Look for plugin classes
        for attr_name in dir(module):
            attr = getattr(module, attr_name)
            if (
                isinstance(attr, type)
                and issubclass(attr, PackageBackendPlugin)
                and attr is not PackageBackendPlugin
            ):
                try:
                    instance = attr()
                    if registry.register(instance):
                        pass  # Successfully registered
                except Exception as e:
                    registry._load_errors.append(
                        f"Failed to instantiate plugin from {filename}: {e}"
                    )

    except Exception as e:
        registry._load_errors.append(f"Error executing {filename}: {e}")


# Global registry instance for the application
_global_registry: Optional[PluginRegistry] = None


def get_plugin_registry() -> PluginRegistry:
    """Get the global plugin registry, creating it if necessary.

    Returns:
        The global PluginRegistry instance
    """
    global _global_registry
    if _global_registry is None:
        _global_registry = PluginRegistry()
    return _global_registry


def reload_plugins() -> PluginRegistry:
    """Reload all plugins from disk.

    Returns:
        A fresh PluginRegistry with all plugins reloaded
    """
    global _global_registry
    _global_registry = PluginRegistry()
    load_plugins(_global_registry)
    return _global_registry
