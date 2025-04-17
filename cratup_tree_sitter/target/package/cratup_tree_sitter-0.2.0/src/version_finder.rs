// use semver::Version;
use log::debug;

// use thiserror::Error;

use crate::tree_traversal::{PackageAndDeps, PackageAndDepsNodes, TomlParser};

#[derive(Debug)]
pub struct VersionUpdate<'a> {
    pub package_name: Option<&'a str>,
    pub current_version: &'a str,
    pub new_version: &'a str,
}

//update_all_pkg_and_deps
impl<'a> VersionUpdate<'a> {
    pub fn update_all_pkg_and_deps(&self, source: &str) -> String {
        debug!(
            "Starting version update from '{}' to '{}'",
            self.current_version, self.new_version
        );
        let mut updated_source = source.to_owned();
        let mut iteration = 1;

        loop {
            debug!("\n--- Iteration {} ---", iteration);
            iteration += 1;

            // Run one update pass. If an update was applied, use the new source; otherwise, break.
            match self.update_pass(&updated_source) {
                Some(new_source) => updated_source = new_source,
                None => {
                    debug!("No more versions to update in this iteration");
                    break;
                }
            }
        }

        debug!("\nVersion update completed");
        updated_source
    }
}

//update_pass
impl<'a> VersionUpdate<'a> {
    /// Performs one update pass over the package and its dependencies.
    /// Returns Some(updated_source) if an update was applied, otherwise None.
    pub fn update_pass(&self, source: &str) -> Option<String> {
        let mut updated_source = source.to_owned();

        let version_finder = match TomlParser::new(&updated_source) {
            Ok(vf) => {
                debug!("Successfully initialized TomlParser");
                vf
            }
            Err(e) => {
                debug!("Error initializing TomlParser: {:?}", e);
                return None;
            }
        };

        // Prepare the new version string wrapped in double quotes.
        let new_version_quoted = format!("\"{}\"", self.new_version);

        // Find the package and dependencies.
        if let Some(pkg_and_deps) = version_finder.find_package_and_deps() {
            // Filter the package and dependency info using the update criteria.
            let filtered = self.filter_package_and_deps(pkg_and_deps);

            // Update the package if available.
            if let Some((pkg_node, pkg_info)) = filtered.package {
                debug!(
                    "Updating package {} from version {} to {}",
                    pkg_info.name, pkg_info.version, new_version_quoted
                );
                updated_source = version_finder.edit_node(pkg_node, &new_version_quoted);
                return Some(updated_source);
            }

            // Otherwise, update the first matching dependency.
            debug!("Checking {} dependencies...", filtered.dependencies.len());
            if let Some((dep_node, dep_info)) = filtered.dependencies.iter().next() {
                debug!(
                    "Updating dependency '{}' from version '{}' to '{}'",
                    dep_info.name, dep_info.version, new_version_quoted
                );
                updated_source = version_finder.edit_node(*dep_node, &new_version_quoted);
                return Some(updated_source);
            }
        } else {
            debug!("No package/dependency information found");
        }

        // No update was made in this pass.
        None
    }
}

// filtered_pkg_and_deps
impl<'a> VersionUpdate<'a> {
    pub fn filtered_pkg_and_deps(&self, source: &str) -> Option<PackageAndDeps> {
        debug!("Starting filtered_pkg_and_deps update: {:?}", self);

        // Initialize the TomlParser with the provided source.
        let version_finder = match TomlParser::new(source) {
            Ok(vf) => {
                debug!("Successfully initialized TomlParser");
                vf
            }
            Err(e) => {
                debug!("Error initializing TomlParser: {:?}", e);
                return None;
            }
        };

        // Attempt to find package and dependency information.
        debug!("Looking for package and dependency information...");
        if let Some(pkg_and_deps) = version_finder.find_package_and_deps() {
            // Filter the package and dependency information based on the update criteria.
            debug!("Filtering package and dependencies for display");
            let filtered = self.filter_package_and_deps(pkg_and_deps);
            debug!("Filtered result: {:?}", filtered);
            return Some(filtered.into());
        } else {
            debug!("No package/dependency information found");
        }

        debug!("Returning None - no valid package/dependency information found");
        None
    }
}

/// This function filters the given `PackageAndDepsNodes` according to the package name in `update`.
/// - If the package info's name does not match, it sets `package` to `None`.
/// - The dependencies HashMap is filtered so that only those whose `name` matches `package_name` are kept.
// filter_package_and_deps
impl<'a> VersionUpdate<'a> {
    pub fn filter_package_and_deps(
        &self,
        pkg_and_deps: PackageAndDepsNodes<'a>,
    ) -> PackageAndDepsNodes<'a> {
        // Filter package: Only include if a package name is provided and both name and version match.
        let filtered_package = match (pkg_and_deps.package, self.package_name) {
            (Some((pkg_node, pkg_info)), Some(pkg_name))
                if pkg_info.name == pkg_name && pkg_info.version == self.current_version =>
            {
                Some((pkg_node, pkg_info))
            }
            (Some((pkg_node, pkg_info)), None) if pkg_info.version == self.current_version => {
                Some((pkg_node, pkg_info))
            }
            _ => None,
        };

        // Filter dependencies:
        // If package name is provided: both name and version must match.
        // If not provided: only the version must match.
        let filtered_dependencies = pkg_and_deps
            .dependencies
            .into_iter()
            .filter(|(_, dep_info)| match self.package_name {
                Some(pkg_name) => {
                    dep_info.name == pkg_name && dep_info.version == self.current_version
                }
                None => dep_info.version == self.current_version,
            })
            .collect();

        PackageAndDepsNodes {
            package: filtered_package,
            dependencies: filtered_dependencies,
        }
    }
}
