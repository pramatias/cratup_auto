use anyhow::Context;
use anyhow::Result;
// use anyhow::Error;

use colored::ColoredString;
use log::{debug, trace};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
// use std::process;
use strsim::levenshtein;
use walkdir::WalkDir;

use crate::string_format::{get_colored_pkg_deps, get_colored_dir_path};
use cratup_tree_sitter::{PackageAndDeps, PkgInfo, TomlParser};

/// The Search struct holds the current directory, the version to query, and optionally a package name.
/// It also includes a list of directories with package/dependency information.
pub struct Search {
    dir_path: PathBuf,
    version: Option<String>,
    package_name: Option<String>,
    pub pkg_deps_dirs: Vec<(PathBuf, PackageAndDeps)>,
}

impl Search {
    /// Creates a new Search instance and loads the directories with package and dependency information.
    /// No filtering is done at this stage.
    pub fn new(
        dir_path: PathBuf,
        version: Option<String>,
        package_name: Option<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let package_dirs = load_dirs_pkgs_deps(&dir_path)?;
        Ok(Self {
            dir_path,
            version,
            package_name,
            pkg_deps_dirs: package_dirs,
        })
    }

    /// The search method applies filtering by version and package name.
    /// It updates the pkg_deps_dirs field with the filtered results and returns a clone of it.
    pub fn search(&mut self) -> Result<(), Box<dyn Error>> {
        // Apply version filtering if specified.
        if let Some(ref ver) = self.version {
            self.pkg_deps_dirs = filter_by_version(self.pkg_deps_dirs.clone(), ver);
            debug!(
                "After filtering by version '{}', {} result(s) remain",
                ver,
                self.pkg_deps_dirs.len()
            );
        }

        // Create a clone of package name to avoid borrowing `self.package_name` immutably for too long.
        if let Some(pkg_name) = self.package_name.clone() {
            {
                // Use a narrower scope so that the immutable borrow from pkg_name ends quickly.
                self.filter_by_package_name(&pkg_name)?;
            }
            debug!(
                "After filtering by package name '{}', {} result(s) remain",
                pkg_name,
                self.pkg_deps_dirs.len()
            );
        }

        Ok(())
    }

    /// The fuzzy_search method is used as a fallback when the normal search yields no results.
    /// It uses similarity scoring (with a given threshold) to search for similar package names.
    pub fn fuzzy_search(&self) -> Result<Vec<(PathBuf, PackageAndDeps)>, Box<dyn std::error::Error>> {
        if let Some(ref pkg_name) = self.package_name {
            debug!(
                "Performing fuzzy search for package: '{}'",
                pkg_name
            );
            if let Some((path, pkg_and_deps)) = find_closest_package(&self.dir_path, pkg_name)? {
                Ok(vec![(path, pkg_and_deps)])
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    }

    /// The `display` method iterates through the package/dependency directories,
    /// formats the package and dependency information using `get_colored_pkg_deps`,
    /// and prints the results.
    pub fn display<F>(&self, color_version: F)
    where
        F: Fn(&str) -> ColoredString,
    {
        for (pkg_dir, pkg_deps) in &self.pkg_deps_dirs {
            // Call the new function with the search directory and the current package directory.
            let colored_path = get_colored_dir_path(pkg_dir, &self.dir_path);
            let formatted = get_colored_pkg_deps(pkg_deps, &color_version);
            println!("{}\n{}", colored_path, formatted);
        }
    }
}

/// Searches for the first package with a name similar to `package_name` based on
/// the Levenshtein distance. Returns the package directory and its package/dependency
/// info if the distance is within the given threshold.
/// Finds the package with the smallest Levenshtein distance from the provided package name.
/// Returns the closest package (if any) regardless of the distance.
fn find_closest_package(
    dir_path: &PathBuf,
    package_name: &str,
) -> Result<Option<(PathBuf, PackageAndDeps)>, Box<dyn std::error::Error>> {
    debug!(
        "Searching for the closest match to package '{}' in directory {:?}",
        package_name, dir_path
    );

    // Load directories containing only package information.
    debug!("Loading package directories from {:?}", dir_path);
    let pkg_dirs = load_dirs_pkgs(&dir_path)?;
    debug!("Found {} potential package directories", pkg_dirs.len());

    // Find the package with the minimum Levenshtein distance.
    debug!("Calculating Levenshtein distances for all potential matches");
    let candidate = pkg_dirs
        .into_iter()
        .map(|(path, pkg_info)| {
            let distance = levenshtein(&pkg_info.name, package_name);
            trace!(
                "Package '{}' has a Levenshtein distance of {}",
                pkg_info.name, distance
            );
            (distance, path, pkg_info)
        })
        .min_by_key(|(distance, _, _)| *distance);

    if let Some((distance, path, pkg_info)) = &candidate {
        debug!(
            "Found best match: '{}' at path {:?} with distance {}",
            pkg_info.name, path, distance
        );
        let pkg_and_deps = PackageAndDeps {
            package: Some(pkg_info.clone()),
            dependencies: Vec::new(),
        };
        Ok(Some((path.clone(), pkg_and_deps)))
    } else {
        debug!("No matching package found");
        Ok(None)
    }
}

// Filters package directories by exact package name (or dependency name) match.
// If no exact matches exist, the caller (in `new`) will perform the fallback similarity search.
impl Search {
    /// Filters the internal package directories by exact package or dependency name match.
    /// The method updates the `pkg_deps_dirs` field in place.
    fn filter_by_package_name(&mut self, pkg_name: &str) -> Result<(), Box<dyn Error>> {
        debug!("Filtering packages by name: {}", pkg_name);
        debug!("Total packages to check: {}", self.pkg_deps_dirs.len());

        self.pkg_deps_dirs = self
            .pkg_deps_dirs
            .clone()
            .into_iter()
            .map(|(path, pkg_and_deps)| {
                let filtered_pkg_and_deps = filter_package_and_deps(pkg_and_deps, pkg_name);
                (path, filtered_pkg_and_deps)
            })
            .filter(|(_, pkg_and_deps)| {
                pkg_and_deps.package.is_some() || !pkg_and_deps.dependencies.is_empty()
            })
            .collect();

        debug!("Found {} matching packages", self.pkg_deps_dirs.len());
        Ok(())
    }
}

fn filter_package_and_deps(mut pkg_and_deps: PackageAndDeps, pkg_name: &str) -> PackageAndDeps {
    let strip_quotes = |s: &str| s.replace("\"", "");

    if let Some(pkg) = &mut pkg_and_deps.package {
        if strip_quotes(&pkg.name) != pkg_name {
            debug!(
                "Package '{}' does not match '{}', setting package to None",
                pkg.name, pkg_name
            );
            pkg_and_deps.package = None;
        }
    }

    pkg_and_deps
        .dependencies
        .retain(|dep| strip_quotes(&dep.name) == pkg_name);
    debug!(
        "Filtered dependencies for package: {:?}",
        pkg_and_deps.package.as_ref().map(|p| &p.name)
    );

    pkg_and_deps
}

 fn filter_by_version(
     package_dirs: Vec<(PathBuf, PackageAndDeps)>,
     version: &str,
 ) -> Vec<(PathBuf, PackageAndDeps)> {
     debug!("Filtering packages by version: {}", version);
     debug!("Total packages to check: {}", package_dirs.len());

     package_dirs
         .into_iter()
         .filter_map(|(path, mut pkg_and_deps)| {
             // check if the package itself matches
             let pkg_matches = pkg_and_deps
                 .package
                 .as_ref()
                 .map_or(false, |pkg| {
                     let m = pkg.version == version;
                     debug!(
                         "Package '{}' version '{}' {} match target '{}'",
                         pkg.name,
                         pkg.version,
                         if m { "does" } else { "does not" },
                         version
                     );
                     m
                 });

             // check if any dependency matches
             let deps_matches = pkg_and_deps
                 .dependencies
                 .iter()
                 .any(|dep| dep.version == version);

             let overall_match = pkg_matches || deps_matches;
             debug!(
                 "Package at {:?} {} match version criteria",
                 path,
                 if overall_match { "does" } else { "does not" }
             );

             if !overall_match {
                return None;
             }

             // prune out any deps that aren’t exactly this version
             let before = pkg_and_deps.dependencies.len();
             pkg_and_deps
                 .dependencies
                 .retain(|dep| dep.version == version);
             let after = pkg_and_deps.dependencies.len();
             debug!(
                 "Pruned dependencies: {} → {} entries (version '{}')",
                 before, after, version
             );

            // if the package itself didn’t match, drop it
            if !pkg_matches {
                pkg_and_deps.package = None;
                debug!("Package field set to None because version != '{}'", version);
            }

            Some((path, pkg_and_deps))
         })
         .collect()
 }

/// Loads directories and their package/dependency information.
/// This method walks the directory recursively and collects package information from Cargo.toml files.
fn load_dirs_pkgs_deps(dir_path: &Path) -> Result<Vec<(PathBuf, PackageAndDeps)>> {
    debug!(
        "Starting directory scan for Cargo.toml files in: {:?}",
        dir_path
    );

    // Create an iterator over all Cargo.toml files in the directory.
    let cargo_toml_entries = WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "Cargo.toml");

    // Process each file using iterator combinators.
    let results: Vec<(PathBuf, PackageAndDeps)> = cargo_toml_entries
        .map(|entry| -> Result<Option<(PathBuf, PackageAndDeps)>> {
            let file_path = entry.path().to_path_buf();
            debug!("Found Cargo.toml at: {:?}", file_path);

            // Read the file content.
            let content = fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read file {:?}", file_path))?;
            debug!("Successfully read file ({} bytes)", content.len());

            debug!("Parsing TOML content...");
            let toml_parser = TomlParser::new(&content)
                .with_context(|| format!("Failed to parse TOML in {:?}", file_path))?;
            debug!("TOML parsed successfully");

            // Look for package and dependencies in the TOML.
            debug!("Looking for package and dependencies in TOML...");
            if let Some(pkg_deps_nodes) = toml_parser.find_package_and_deps() {
                debug!("Found package/dependencies section in TOML");

                let package = pkg_deps_nodes.package.map(|(_node, pkg_info)| {
                    debug!("Found package: {}", pkg_info.name);
                    pkg_info
                });

                let dependencies: Vec<_> = pkg_deps_nodes
                    .dependencies
                    .into_iter()
                    .map(|(_node, deps_info)| {
                        debug!(
                            "Found dependency: {} = {}",
                            deps_info.name, deps_info.version
                        );
                        deps_info
                    })
                    .collect();

                let pkg_and_deps = PackageAndDeps {
                    package,
                    dependencies,
                };
                debug!(
                    "Processed package with {} dependencies",
                    pkg_and_deps.dependencies.len()
                );

                Ok(Some((file_path, pkg_and_deps)))
            } else {
                debug!("No package/dependencies section found in this TOML file");
                Ok(None)
            }
        })
        // Collect all the results, propagating any errors.
        .collect::<Result<Vec<_>, _>>()?
        // Flatten out files where no package/dependencies section was found.
        .into_iter()
        .flatten()
        // Now filter out any paths that contain "target/release" or "target/debug".
        .filter(|(file_path, _)| {
            let path_str = file_path.to_string_lossy();
            !(path_str.contains("/target/") || path_str.contains("/target/"))
        })
        .collect();

    debug!(
        "Directory scan completed. Processed {} files with Cargo.toml, found {} with packages",
        results.len(),
        results.len()
    );

    debug!("Total packages found: {}", results.len());
    Ok(results)
}

fn load_dirs_pkgs(dir_path: &Path) -> Result<Vec<(PathBuf, PkgInfo)>> {
    debug!("Starting package discovery in directory: {:?}", dir_path);

    // Process each entry that is a file named "Cargo.toml", using iterators.
    let intermediate: Result<Vec<_>, anyhow::Error> = WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|entry| entry.ok()) // Skip erroneous entries.
        // Filter out entries whose full path contains "target/release" or "target/debug".
        .filter(|entry| {
            let path_str = entry.path().to_string_lossy();
            !(path_str.contains("target/release") || path_str.contains("target/debug"))
        })
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "Cargo.toml")
        .enumerate() // Keep track of file order.
        .map(|(i, entry)| -> Result<Option<(PathBuf, PkgInfo)>> {
            let file_path = entry.path().to_path_buf();
            debug!("[{}] Processing Cargo.toml at: {:?}", i + 1, file_path);

            // Read file content.
            let content = fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read file {:?}", file_path))?;
            debug!("  Read {} bytes from file", content.len());

            // Parse the content using the TomlParser.
            debug!("  Parsing TOML content...");
            let toml_parser = TomlParser::new(&content)
                .with_context(|| format!("Failed to parse TOML in {:?}", file_path))?;
            debug!("  TOML parsed successfully");

            // Extract the package info.
            debug!("  Searching for package information...");
            if let Some(pkg_info) = toml_parser.find_package() {
                debug!("  Found package: {} v{}", pkg_info.name, pkg_info.version);
                debug!("    Package path: {:?}", file_path);
                Ok(Some((file_path, pkg_info)))
            } else {
                debug!("  No package information found in this file");
                Ok(None)
            }
        })
        .collect();

    // Flatten the vector of Option<(PathBuf, PkgInfo)> into a vector of (PathBuf, PkgInfo)
    let packages: Vec<(PathBuf, PkgInfo)> = intermediate?.into_iter().flatten().collect();

    debug!("Package discovery completed");
    debug!("  Packages found: {}", packages.len());
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    use cratup_tree_sitter::DepsInfo;
    #[test]
    fn test_filter_package_and_deps_match() {
        let pkg_and_deps = PackageAndDeps {
            package: Some(PkgInfo {
                name: "\"test-package\"".to_string(),
                version: "1.0.0".to_string(),
                name_pair: "test-package".to_string(),
                version_pair: "1.0.0".to_string(),
            }),
            dependencies: vec![
                DepsInfo {
                    name: "\"test-package\"".to_string(),
                    version: "1.0.0".to_string(),
                    name_pair: "test-package".to_string(),
                    version_pair: "1.0.0".to_string(),
                },
                DepsInfo {
                    name: "\"other-package\"".to_string(),
                    version: "2.0.0".to_string(),
                    name_pair: "other-package".to_string(),
                    version_pair: "2.0.0".to_string(),
                },
            ],
        };

        let filtered = filter_package_and_deps(pkg_and_deps.clone(), "test-package");
        assert_eq!(
            filtered.package.as_ref().unwrap().name,
            pkg_and_deps.package.as_ref().unwrap().name
        );
        assert_eq!(filtered.dependencies.len(), 1);
    }

    #[test]
    fn test_filter_package_and_deps_no_match() {
        let pkg_and_deps = PackageAndDeps {
            package: Some(PkgInfo {
                name: "\"test-package\"".to_string(),
                version: "1.0.0".to_string(),
                name_pair: "test-package".to_string(),
                version_pair: "1.0.0".to_string(),
            }),
            dependencies: vec![
                DepsInfo {
                    name: "\"test-package\"".to_string(),
                    version: "1.0.0".to_string(),
                    name_pair: "test-package".to_string(),
                    version_pair: "1.0.0".to_string(),
                },
                DepsInfo {
                    name: "\"other-package\"".to_string(),
                    version: "2.0.0".to_string(),
                    name_pair: "other-package".to_string(),
                    version_pair: "2.0.0".to_string(),
                },
            ],
        };

        let filtered = filter_package_and_deps(pkg_and_deps.clone(), "non-existent-package");
        assert!(filtered.package.is_none());
        assert!(filtered.dependencies.is_empty());
    }

    #[test]
    fn test_filter_package_and_deps_empty_dependencies() {
        let pkg_and_deps = PackageAndDeps {
            package: Some(PkgInfo {
                name: "\"test-package\"".to_string(),
                version: "1.0.0".to_string(),
                name_pair: "test-package".to_string(),
                version_pair: "1.0.0".to_string(),
            }),
            dependencies: vec![],
        };

        let filtered = filter_package_and_deps(pkg_and_deps.clone(), "test-package");
        assert_eq!(
            filtered.package.as_ref().unwrap().name,
            pkg_and_deps.package.as_ref().unwrap().name
        );
        assert!(filtered.dependencies.is_empty());
    }

    #[test]
    fn test_filter_package_and_deps_no_package() {
        let pkg_and_deps = PackageAndDeps {
            package: None,
            dependencies: vec![DepsInfo {
                name: "\"test-package\"".to_string(),
                version: "1.0.0".to_string(),
                name_pair: "test-package".to_string(),
                version_pair: "1.0.0".to_string(),
            }],
        };

        let filtered = filter_package_and_deps(pkg_and_deps.clone(), "test-package");
        assert!(filtered.package.is_none());
        assert_eq!(filtered.dependencies.len(), 1);
    }
}
