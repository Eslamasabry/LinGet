"""Example plugin for LinGet - demonstrates the plugin API.

This is a mock plugin showing the structure and expected behavior.
It can be used as a template for creating real package backend plugins.

To install:
    Copy this file to ~/.config/linget/plugins/myplugin_plugin.py
    Modify the class name and implementation for your backend
"""

# Import the base class from the plugin system
from linget.plugin import PackageBackendPlugin
from linget.models import Package, PackageStatus


class ExampleBackendPlugin(PackageBackendPlugin):
    """Example plugin implementation showing the expected structure.

    This is a mock implementation that demonstrates how to create
    a custom package backend plugin for LinGet.

    In a real plugin, you would:
    1. Change the class name to something descriptive
    2. Set the 'name' attribute to a unique identifier
    3. Implement all abstract methods with real backend commands
    4. Handle errors gracefully with try/except blocks
    """

    # Plugin metadata - REQUIRED
    name = "example"  # Unique identifier used as source name
    api_version = "1.0"  # Must match the plugin API version
    description = "Example plugin demonstrating the LinGet plugin API"

    def can_execute(self, pkg: Package) -> bool:
        """Check if this plugin handles the given package.

        This method is called to determine if this plugin should handle
        install/update/remove operations for a package.

        Args:
            pkg: The package to check

        Returns:
            True if this plugin should handle the package

        Example:
            # Handle packages with source matching our name
            return pkg.source == self.name

            # Or handle packages with specific prefix
            return pkg.name.startswith("example-")
        """
        # Example: handle packages where source matches our plugin name
        return pkg.source == self.name

    def fetch_installed(self) -> list[Package]:
        """Fetch list of installed packages from this backend.

        This method is called during refresh to populate the package list.
        It should return all packages currently installed by this backend.

        Returns:
            List of Package objects with status=INSTALLED

        Example:
            # Parse output from backend list command
            packages = []
            result = run_backend_command(["list", "--installed"])
            for line in result.stdout.splitlines():
                name, version = parse_line(line)
                packages.append(Package(
                    name=name,
                    version=version,
                    source=self.name,
                    status=PackageStatus.INSTALLED,
                    desc=f"Package from {self.name}"
                ))
            return packages
        """
        # Example: return mock data (replace with real backend query)
        return [
            Package(
                name="example-package",
                version="1.0.0",
                source=self.name,
                status=PackageStatus.INSTALLED,
                desc="An example package",
            )
        ]

    def install(self, pkg: Package) -> bool:
        """Install the given package.

        This method is called when the user requests to install a package
        that this plugin handles.

        Args:
            pkg: Package to install (name and source are key fields)

        Returns:
            True if installation succeeded, False on failure

        Example:
            result = run_backend_command(["install", pkg.name])
            return result.returncode == 0
        """
        # Example: simulate success (replace with real install command)
        print(f"[ExamplePlugin] Would install: {pkg.name}")
        return True

    def remove(self, pkg: Package) -> bool:
        """Remove/uninstall the given package.

        This method is called when the user requests to remove a package
        that this plugin handles.

        Args:
            pkg: Package to remove

        Returns:
            True if removal succeeded, False on failure

        Example:
            result = run_backend_command(["uninstall", pkg.name])
            return result.returncode == 0
        """
        # Example: simulate success (replace with real remove command)
        print(f"[ExamplePlugin] Would remove: {pkg.name}")
        return True

    def update(self, pkg: Package) -> bool:
        """Update the given package to the latest version.

        This method is called when the user requests to update a package
        that this plugin handles.

        Args:
            pkg: Package to update

        Returns:
            True if update succeeded, False on failure

        Example:
            result = run_backend_command(["upgrade", pkg.name])
            return result.returncode == 0
        """
        # Example: simulate success (replace with real update command)
        print(f"[ExamplePlugin] Would update: {pkg.name}")
        return True

    def search(self, query: str) -> list[Package]:
        """Search for packages matching the query.

        This method is called when the user searches for new packages.
        It should return packages that are NOT already installed.

        Args:
            query: Search string entered by user

        Returns:
            List of Package objects with status=NOT_INSTALLED

        Example:
            packages = []
            result = run_backend_command(["search", query])
            for line in result.stdout.splitlines():
                name, version, desc = parse_search_result(line)
                packages.append(Package(
                    name=name,
                    version=version,
                    source=self.name,
                    status=PackageStatus.NOT_INSTALLED,
                    desc=desc
                ))
            return packages
        """
        # Example: return mock search results
        if "example" in query.lower():
            return [
                Package(
                    name="example-tool",
                    version="2.0.0",
                    source=self.name,
                    status=PackageStatus.NOT_INSTALLED,
                    desc="An example tool found by search",
                ),
                Package(
                    name="example-lib",
                    version="1.5.0",
                    source=self.name,
                    status=PackageStatus.NOT_INSTALLED,
                    desc="An example library",
                ),
            ]
        return []


# Plugin registration
# The plugin system will automatically find and instantiate this class
# when the file is loaded from ~/.config/linget/plugins/
