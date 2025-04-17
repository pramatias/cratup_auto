// use anyhow::Result;
use colored::{ColoredString, Colorize};
use log::debug;
// use std::fs;
use std::path::{Path, PathBuf};
// use walkdir::WalkDir;

use crate::file_parts::build_directory_display;
use cratup_tree_sitter::PackageAndDeps;

#[derive(Debug)]
pub struct VersionMatch {
    pub file_path: String,
    pub matches: usize,
    pub pkg_deps: PackageAndDeps,
}

impl VersionMatch {
    /// Constructs a new VersionMatch by extracting a String from a PathBuf and counting
    /// the package and dependency elements.
    pub fn new(file_path: PathBuf, pkg_deps: PackageAndDeps) -> Self {
        // Immediately extract the displayable string from the PathBuf.
        let file_path_str = file_path
            .to_str()
            .expect("Invalid Unicode in file_path")
            .to_owned();

        debug!("Creating new VersionMatch for file: {:?}", file_path_str);
        debug!(
            "Initial package/deps state - has package: {}, dependencies count: {}",
            pkg_deps.package.is_some(),
            pkg_deps.dependencies.len()
        );

        // Count 1 if a package exists, plus the number of dependencies.
        let mut count = 0;
        if pkg_deps.package.is_some() {
            count += 1;
            debug!("Package exists - incrementing count to {}", count);
        } else {
            debug!("No package found in this match");
        }

        count += pkg_deps.dependencies.len();
        debug!(
            "Added {} dependencies - final match count: {}",
            pkg_deps.dependencies.len(),
            count
        );

        let version_match = VersionMatch {
            file_path: file_path_str,
            matches: count,
            pkg_deps,
        };

        debug!("Created VersionMatch: {:?}", version_match);
        version_match
    }
}

/// Returns a colored string representing the package info and its dependencies.
/// The function takes a reference to a PackageAndDeps and a closure for coloring the version.
/// Returns a colored string representing the package info and its dependencies.
/// The function takes a reference to a PackageAndDeps and a closure for coloring the version.
pub fn get_colored_pkg_deps<F>(pkg_deps: &PackageAndDeps, color_version: F) -> String
where
    F: Fn(&str) -> ColoredString,
{
    debug!(
        "Starting get_colored_pkg_deps for package: {:?}",
        pkg_deps.package.as_ref().map(|p| &p.name)
    );

    // Process dependencies.
        debug!("Processing {} dependencies", pkg_deps.dependencies.len());
        let deps_display = pkg_deps
            .dependencies
            .iter()
            .map(|dep| {
                debug!("Processing dependency: {}", dep.name);
                // Format each dependency's name pair with the provided color for the version.
                let formatted_name_pair =
                    format_pair_with_version(&dep.name_pair, &dep.version, |s| color_version(s));
                format!("\t{}: {}", dep.name.yellow(), formatted_name_pair)
            })
            .collect::<Vec<String>>()
            .join("\n");

    // If package info is available, include it in the output.
    if let Some(pkg) = &pkg_deps.package {
        debug!("Package info available, processing package: {}", pkg.name);
        let formatted_name_pair =
            format_pair_with_version(&pkg.name_pair, &pkg.version, |s| color_version(s));
        debug!("Formatted name pair: {:?}", formatted_name_pair);

        // Use `format_pair_with_version` to format pkg.version_pair, coloring the version.
        let formatted_version_pair =
            format_pair_with_version(&pkg.version_pair, &pkg.version, |s| color_version(s));
        debug!("Formatted version pair: {:?}", formatted_version_pair);

        let pkg_display = format!(
            "{}: {} {}",
            pkg.name.purple(),
            formatted_name_pair,
            formatted_version_pair
        );
        debug!("Final package display string: {}", pkg_display);

        format!("\t{}\n{}", pkg_display, deps_display)
    } else {
        debug!("No package info available, only displaying dependencies");
        // Otherwise, just return the dependencies display.
        deps_display
    }
}

/// Helper function: given a `pair` string, it finds the first occurrence of `version`
/// and splits the string into a prefix and suffix. It then colors the `version` using the provided
/// `color_fn` and returns the concatenated result.
fn format_pair_with_version<F>(pair: &str, version: &str, color_fn: F) -> String
where
    F: Fn(&str) -> ColoredString,
{
    debug!("Formatting pair: '{}' with version: '{}'", pair, version);

    if let Some((prefix, suffix)) = pair.split_once(version) {
        debug!(
            "Version found in pair. Prefix: '{}', Suffix: '{}'",
            prefix, suffix
        );
        let colored_version = color_fn(version);
        debug!("Colored version: {:?}", colored_version);
        let result = format!("{}{}{}", prefix, colored_version, suffix);
        debug!("Formatted result: '{}'", result);
        result
    } else {
        debug!(
            "Version '{}' not found in pair '{}', returning original string",
            version, pair
        );
        // if the version is not found in the name pair, return the original string
        pair.to_string()
    }
}

pub fn get_colored_dir_path_and_matches(version_match: &VersionMatch, current_dir: &Path) -> String {
    debug!("Starting to build match info for version update");
    debug!("VersionMatch details: {:?}", version_match);
    debug!("Current directory: {:?}", current_dir);

    // Here, file_path is already a String that was derived from the PathBuf.
    let current_dir_str = current_dir
        .to_str()
        .expect("Invalid Unicode in current_dir");

    // Use the file_path string directly.
    let display_path = build_directory_display(&version_match.file_path, current_dir_str);
    debug!("Built display path: '{}'", display_path);

    // Build the matches info using another helper function.
    let matches_info = build_matches_info(version_match.matches);
    debug!("Built matches info: '{}'", matches_info);

    // Append the matches info to the display path.
    let final_display = format!("{} {}", display_path, matches_info);
    debug!("Final formatted output: '{}'", final_display);
    debug!("Finished building match information");

    final_display
}

pub fn get_colored_dir_path(file_path: &Path, root_dir: &Path) -> String {
    debug!("Starting to build colored directory display for package directory");
    debug!("Search directory: {:?}", file_path);
    debug!("Package directory: {:?}", root_dir);

    // Convert both paths to string slices.
    let file_path_str = file_path
        .to_str()
        .expect("Invalid Unicode in search directory");
    let root_dir_str = root_dir
        .to_str()
        .expect("Invalid Unicode in package directory");

    // Use the existing helper to build the relative display path.
    let colored_display = build_directory_display( file_path_str, root_dir_str);

    // Apply coloring formatting. For instance, here we color the path in green.
    debug!("Final colored display: '{}'", colored_display);

    colored_display
}

fn build_matches_info(matches: usize) -> String {
    debug!("Building matches info for count: {}", matches);

    let colored_matches = matches.to_string().green();
    debug!("Formatted colored matches: {:?}", colored_matches);

    let result = format!("({} matches)", colored_matches);
    debug!("Final matches info string: '{}'", result);

    result
}
