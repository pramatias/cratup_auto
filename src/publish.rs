use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

/// Returns true if `path` contains a segment “target” immediately followed by
/// “debug” or “release” (e.g. “…/target/debug/…”)—i.e. something we want to exclude.
fn is_excluded_target_dir(path: &Path) -> bool {
    // Walk the components in pairs
    let mut comps = path.components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .peekable();

    while let Some(curr) = comps.next() {
        if curr == "target" {
                    return true
        }
    }
    false
}

/// Find all subdirectories containing a Cargo.toml, excluding any under
/// target/debug or target/release.
pub fn find_publishable_dirs(current_dir: &Path) -> Vec<PathBuf> {
    debug!("Starting search for publishable directories in: {:?}", current_dir);

    // WalkDir + iterator chain does all the work:
    let publishable_dirs: Vec<PathBuf> = WalkDir::new(current_dir)
        .into_iter()
        // skip broken entries
        .filter_map(Result::ok)
        // only directories
        .filter(|e| e.file_type().is_dir())
        // get their PathBuf
        .map(|e| e.into_path())
        // has Cargo.toml?
        .filter(|dir| dir.join("Cargo.toml").exists())
        // not under target/debug or target/release?
        .filter(|dir| {
            let excluded = is_excluded_target_dir(dir);
            if excluded {
                debug!("Excluding target dir: {:?}", dir);
            }
            !excluded
        })
        .inspect(|dir| debug!("Will publish: {:?}", dir))
        .collect();

    debug!(
        "Directory scan completed. \
         Publishable directories found: {}",
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
