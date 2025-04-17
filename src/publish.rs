use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

pub fn find_publishable_dirs(current_dir: &Path) -> Vec<PathBuf> {
    debug!(
        "Starting search for publishable directories in: {:?}",
        current_dir
    );
    let mut publishable_dirs = Vec::new();
    let mut total_dirs_scanned = 0;
    let mut skipped_non_dir = 0;

    // Walk through the given directory recursively.
    for entry in WalkDir::new(current_dir).into_iter().filter_map(|e| e.ok()) {
        // Only consider directories.
        if entry.file_type().is_dir() {
            total_dirs_scanned += 1;
            let dir_path = entry.clone().into_path();

            let cargo_toml = dir_path.join("Cargo.toml");

            debug!("Checking directory: {:?}", dir_path);

            if cargo_toml.exists() {
                debug!("Found Cargo.toml in directory: {:?}", dir_path);
                publishable_dirs.push(entry.into_path());
                debug!("Added to publishable list: {:?}", dir_path);
            } else {
                debug!("Directory does not contain Cargo.toml: {:?}", dir_path);
            }
        } else {
            skipped_non_dir += 1;
            debug!("Skipping non-directory entry: {:?}", entry.path());
        }
    }

    debug!(
        "Directory scan completed. Stats:
        - Total directories scanned: {}
        - Non-directory entries skipped: {}
        - Publishable directories found: {}",
        total_dirs_scanned,
        skipped_non_dir,
        publishable_dirs.len()
    );

    publishable_dirs
}

#[derive(Debug)]
pub enum PublishState {
    Published(String),
    Unpublished(String),
}

/// Iterates over the vector in a nested loop. Only directories that are still unpublished
/// will have the publish command executed. If the publish command succeeds, the state is updated.
pub fn publish_modules(dirs: &[PathBuf]) -> Result<Vec<PublishState>> {
    debug!("Starting module publication for {} directories", dirs.len());
    debug!("Input directories: {:?}", dirs);

    // Convert incoming directories into a vector of PublishState
    let mut publish_states: Vec<PublishState> = dirs
        .iter()
        .map(|p| {
            let dir = p.to_string_lossy().into_owned();
            debug!("Initializing Unpublished state for directory: {}", dir);
            PublishState::Unpublished(dir)
        })
        .collect();

    // Compute total iterations as the square of the length.
    let total_iterations = (publish_states.len() * publish_states.len()) as u64;
    debug!(
        "Setting up progress bar for {} total iterations",
        total_iterations
    );

    let pb = ProgressBar::new(total_iterations);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .expect("Failed to set progress bar template"),
    );

    // Outer loop iterates as many times as there are entries
    for outer_iter in 0..publish_states.len() {
        debug!("Starting outer iteration {}", outer_iter + 1);
        let mut progress = false;

        let num_modules = publish_states.len();

        // Inner loop: Attempt to publish unpublished modules
        for (idx, state) in publish_states.iter_mut().enumerate() {
            debug!("Processing module {} of {}", idx + 1, num_modules);

            // Borrow state immutably to check its variant and clone the directory string
            if let PublishState::Unpublished(dir) = state {
                let dir_clone = dir.clone(); // Now work with a full owned copy
                debug!("Attempting to publish directory: {}", dir_clone);

                match publish_module(&dir_clone, "publish") {
                    Ok(_) => {
                        debug!("Successfully published directory: {}", dir_clone);
                        // Now we can safely update *state since no borrow is active.
                        *state = PublishState::Published(dir_clone.clone());
                        debug!("Updated state to Published for directory: {}", dir_clone);

                        progress = true;
                    }
                    Err(e) => {
                        debug!("Publish failed for {}: {:?}", dir_clone, e);
                    }
                }
            } else {
                debug!("Module already published, skipping");
            }

            pb.inc(1);
            debug!("Progress bar incremented");
        }

        if !progress {
            debug!(
                "No progress made in iteration {}, breaking loop",
                outer_iter + 1
            );
            break;
        } else {
            debug!("Progress made in iteration {}, continuing", outer_iter + 1);
        }
    }

    pb.finish_with_message("All publish commands completed.");
    debug!("Publication process completed");
    debug!("Final states: {:?}", publish_states);

    Ok(publish_states)
}

pub fn print_modules(publish_states: &[PublishState]) {
    debug!("Starting to print module publication status");
    debug!("Total modules to print: {}", publish_states.len());

    println!("Published modules:");
    debug!("Printing published modules section");

    // Track published count for debugging
    let mut published_count = 0;
    for state in publish_states {
        if let PublishState::Published(module) = state {
            debug!("Printing published module: {}", module);
            println!("{}", module.green());
            published_count += 1;
        }
    }
    debug!("Printed {} published modules", published_count);

    println!("\nUnpublished modules:");
    debug!("Printing unpublished modules section");

    // Track unpublished count for debugging
    let mut unpublished_count = 0;
    for state in publish_states {
        if let PublishState::Unpublished(module) = state {
            debug!("Printing unpublished module: {}", module);
            println!("{}", module.red());
            unpublished_count += 1;
        }
    }
    debug!("Printed {} unpublished modules", unpublished_count);

    debug!(
        "Finished printing modules ({} published, {} unpublished)",
        published_count, unpublished_count
    );
}

fn publish_module(dir: &str, command: &str) -> Result<()> {
    debug!("Attempting to publish module in directory: {}", dir);
    debug!("Using cargo command: {}", command);

    let mut cmd = Command::new("cargo");
    cmd.arg(command)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    debug!("Constructed command: {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => {
            debug!("Command executed successfully, status: {}", s);
            s
        }
        Err(e) => {
            debug!("Command execution failed: {:?}", e);
            return Err(e.into());
        }
    };

    if status.success() {
        debug!("Publish succeeded for directory: {}", dir);
        Ok(())
    } else {
        debug!(
            "Publish failed for directory: {}, exit status: {:?}",
            dir,
            status.code()
        );
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Publish failed for {} with status {:?}", dir, status.code()),
        )
        .into())
    }
}
