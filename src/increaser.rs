use anyhow::Context;
use anyhow::Result;
use colored::ColoredString;
use colored::Colorize;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use semver::Version;

use cratup_search::{VersionMatch, get_colored_dir_path_and_matches, get_colored_pkg_deps};
use cratup_tree_sitter::{PackageAndDeps, VersionUpdate};

/// The Increaser struct now includes the current directory along with version update info.
pub struct Increaser {
    dir_path: PathBuf,
    current_version: String,
    next_version: String,
    package_name: Option<String>,
    package_dirs: Vec<(PathBuf, PackageAndDeps)>,
}

//update_dirs_and_packages
impl Increaser {
    /// Walks through the current directory (dir_path), finds all Cargo.toml files,
    /// updates their content by applying the version change, writes the updated content back,
    /// and returns a vector containing each file's path along with its package/dependency info.
    pub fn update_dirs_and_packages(&self) -> Result<Vec<(PathBuf, PackageAndDeps)>> {
        // Create the VersionUpdate using Increaser's version info.
        let version_update = VersionUpdate {
            package_name: self.package_name.as_deref(),
            current_version: &self.current_version,
            new_version: &self.next_version,
        };

        let results = Vec::new();
        for entry in WalkDir::new(&self.dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && entry.file_name() == "Cargo.toml" {
                let file_path = entry.path().to_path_buf();
                // Read the file contents.
                let content = fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read file {:?}", file_path))?;

                // Destructure to capture both values.
                let updated_source = version_update.update_all_pkg_and_deps(&content);

                // Write the updated content back to the file.
                fs::write(&file_path, updated_source)
                    .with_context(|| format!("Failed to write file {:?}", file_path))?;
            }
        }

        Ok(results)
    }

    /// Print version matches using the red color for current version matches.
    pub fn print_current_version_matches(&self) -> Result<Vec<VersionMatch>> {
        // Construct the VersionUpdate for current versions.
        let version_update = VersionUpdate {
            package_name: self.package_name.as_deref(),
            current_version: &self.current_version,
            // Here both current and new versions are the same, since we are highlighting the current version.
            new_version: &self.current_version,
        };

        self.print_version_matches(&version_update, |s| s.red())
    }

    /// Print version matches using the green color for next version matches.
pub fn print_next_version_matches(&self) -> Result<Vec<VersionMatch>> {
    // Create a new instance identical to `self` by cloning the required fields.
    let updated_increaser = Increaser::new(
        self.dir_path.clone(),
        self.next_version.clone(),
        "0.0.0".to_string(),
        self.package_name.clone(),
    )?;

    // Construct the VersionUpdate for the next version matches.
    let version_update = VersionUpdate {
        package_name: updated_increaser.package_name.as_deref(),
        current_version: &updated_increaser.next_version,
        new_version: &updated_increaser.next_version,
    };

    // Use the new instance to print the version matches with green coloring.
    updated_increaser.print_version_matches(&version_update, |s| s.green())
}

}

impl Increaser {
    pub fn new(
        dir_path: PathBuf,
        current_version: String,
        next_version: String,
        package_name: Option<String>,
    ) -> Result<Self> {
        // Parse versions using semver.
        let current_ver = Version::parse(&current_version)
            .expect("Failed to parse the current version as a valid semver string");
        let new_ver = Version::parse(&next_version)
            .expect("Failed to parse the new version as a valid semver string");

        // Check for equality.
        if current_ver == new_ver {
            eprintln!(
                "Error: the new version ({}) is the same as the current version ({}). Exiting.",
                next_version, current_version
            );
            std::process::exit(1);
        }

        let version_update = VersionUpdate {
            package_name: package_name.as_deref(),
            current_version: &current_version,
            new_version: &next_version,
        };

        // Load directories and their package/dependency information.
        let package_dirs = load_dirs_and_packages(&dir_path, &version_update)?;

        // Count total package/dependency elements across all directories.
        let total_count: usize = package_dirs.iter()
            .map(|(_, pkg_and_deps)| pkg_and_deps.count())
            .sum();

        if total_count == 0 {
            std::process::exit(1);
        }

        Ok(Self {
            dir_path,
            current_version,
            next_version,
            package_name,
            package_dirs,
        })
    }
}

/// Walks through the given directory, finds all Cargo.toml files,
/// reads their content, and returns a vector of tuples containing the file's path and its package/dependency info.
fn load_dirs_and_packages(
    dir_path: &Path,
    version_update: &VersionUpdate,
) -> Result<Vec<(PathBuf, PackageAndDeps)>> {
    let entries = WalkDir::new(dir_path)
        .into_iter()
        // Only keep successful directory entries.
        .filter_map(Result::ok)
        // Filter for files named "Cargo.toml".
        .filter(|entry| {
            entry.file_type().is_file() && entry.file_name() == "Cargo.toml"
        })
        // Map each entry to a Result containing an Option.
        .map(|entry| {
            let file_path = entry.path().to_path_buf();
            fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read file {:?}", file_path))
                .map(|content| {
                    version_update
                        .filtered_pkg_and_deps(&content)
                        .map(|pkg_deps| (file_path, pkg_deps))
                })
        })
        // Collect into a Result containing a vector of Option values.
        .collect::<Result<Vec<Option<(PathBuf, PackageAndDeps)>>, _>>()?
        // Filter out `None` values.
        .into_iter()
        .flatten()
        // Now filter out any paths that contain "target/release" or "target/debug".
        .filter(|(file_path, _)| {
            let path_str = file_path.to_string_lossy();
            !(path_str.contains("target/release") || path_str.contains("target/debug"))
        })
        .collect();

    Ok(entries)
}

//print_version_matches
impl Increaser {
    /// Generic method to print version matches with a specified color function
    /// and a provided version update.
    fn print_version_matches<F>(
        &self,
        _version_update: &VersionUpdate,
        color_version: F,
    ) -> Result<Vec<VersionMatch>>
    where
        F: Fn(&str) -> ColoredString + Copy,
    {
        // Using iterator combinators to process package_dirs.
        let version_matches: Vec<VersionMatch> = self.package_dirs.iter()
            .filter_map(|(file_path, pkg_deps)| {
                debug!("Found package info in file {:?}:", file_path);
                if let Some(ref pkg) = pkg_deps.package {
                    debug!("{:?}\n", pkg);
                } else {
                    debug!("No package info available.\n");
                }
                debug!("Dependencies:");
                for dep in &pkg_deps.dependencies {
                    debug!("{:?}", dep.name_pair);
                }

                // Create a new VersionMatch using the constructor.
                let version_match = VersionMatch::new(file_path.clone(), pkg_deps.clone());

                // Skip printing and adding if there are no matches.
                if version_match.matches == 0 {
                    return None;
                }

                // Use the provided color function to colorize output.
                let colored_dir_path = get_colored_dir_path_and_matches(&version_match, &self.dir_path);
                let colored_pkg_deps = get_colored_pkg_deps(&version_match.pkg_deps, color_version);
                println!("{}", colored_dir_path);
                println!("{}", colored_pkg_deps);

                Some(version_match)
            })
            .collect();

        Ok(version_matches)
    }
}

