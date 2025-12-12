use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use colored::Colorize;
use log::{debug, error, info, warn};
use env_logger;
use tempfile;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, short, help = "Clean up all tust temporary directories")]
    clean: bool,
    
    #[arg(trailing_var_arg = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    
    let args = Args::parse();
    
    // Handle --clean flag
    if args.clean {
        info!("Starting cleanup of temporary directories");
        match clean_temporary_directories() {
            Ok(()) => {
                info!("Cleanup completed successfully");
                println!("{}", "Cleanup completed successfully".green());
            }
            Err(e) => {
                error!("Failed to clean temporary directories: {}", e);
                eprintln!("{}", format!("Error: Failed to clean temporary directories: {}", e).red());
                std::process::exit(1);
            }
        }
        return;
    }
    
    if args.command.is_empty() {
        error!("No command provided");
        eprintln!("{}", "Error: No command provided".red());
        std::process::exit(1);
    }
    
    info!("Executing command: {:?}", args.command);
    
    // Get current directory
    let current_dir = match std::env::current_dir() {
        Ok(dir) => {
            info!("Current directory: {}", dir.display());
            dir
        }
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            eprintln!("{}", format!("Error: Failed to get current directory: {}", e).red());
            std::process::exit(1);
        }
    };
    
    // Create temporary directory with prefix for easy identification
    let temp_dir = match tempfile::Builder::new()
        .prefix("tust-")
        .tempdir() {
        Ok(dir) => {
            let temp_path = dir.path();
            info!("Created temporary directory: {}", temp_path.display());
            dir
        }
        Err(e) => {
            error!("Failed to create temporary directory: {}", e);
            eprintln!("{}", format!("Error: Failed to create temporary directory: {}", e).red());
            std::process::exit(1);
        }
    };
    let temp_path = temp_dir.path();
    
    info!("Copying current directory contents to temporary directory");
    println!("{}", "Testing command in temporary directory...".yellow());
    
    // Copy current directory contents to temporary directory
    if let Err(e) = copy_directory(&current_dir, temp_path) {
        error!("Failed to copy directory contents: {}", e);
        eprintln!("{}", format!("Error: Failed to copy directory contents: {}", e).red());
        std::process::exit(1);
    }
    
    // Run the command in the temporary directory
    info!("Running command in temporary directory: {:?}", args.command);
    let status = match Command::new(&args.command[0])
        .args(&args.command[1..])
        .current_dir(temp_path)
        .status() {
        Ok(status) => status,
        Err(e) => {
            error!("Failed to execute command: {}", e);
            eprintln!("{}", format!("Error: Failed to execute command: {}", e).red());
            std::process::exit(1);
        }
    };
    
    if !status.success() {
        let exit_code = status.code().unwrap_or(-1);
        error!("Command failed with exit code: {}", exit_code);
        eprintln!("{}", format!("Command failed with exit code: {}", exit_code).red());
        std::process::exit(exit_code);
    }
    
    info!("Command executed successfully");
    
    // Compare directories to find changes
    info!("Comparing directories to find changes");
    let changes = match compare_directories(&current_dir, temp_path) {
        Ok(changes) => {
            info!("Found {} changes", changes.len());
            changes
        }
        Err(e) => {
            error!("Failed to compare directories: {}", e);
            eprintln!("{}", format!("Error: Failed to compare directories: {}", e).red());
            std::process::exit(1);
        }
    };
    
    if changes.is_empty() {
        info!("No changes would be made");
        println!("{}", "No changes would be made".green());
        return;
    }
    
    // Display changes to user
    info!("Displaying {} changes to user", changes.len());
    println!("{}", "\nChanges that would be made:".blue().bold());
    for change in &changes {
        match change {
            Change::Create(path) => {
                debug!("Would create: {}", path.display());
                println!("  {}{}", "+ ".green(), path.display());
            }
            Change::Modify(path) => {
                debug!("Would modify: {}", path.display());
                println!("  {}{}", "~ ".yellow(), path.display());
            }
            Change::Delete(path) => {
                debug!("Would delete: {}", path.display());
                println!("  {}{}", "- ".red(), path.display());
            }
        }
    }
    
    // Ask for user confirmation
    info!("Asking user for confirmation");
    println!("\n{}", "Would you like to apply these changes? (y/n)".yellow());
    
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut input) {
        error!("Failed to read input: {}", e);
        eprintln!("{}", format!("Error: Failed to read input: {}", e).red());
        std::process::exit(1);
    }
    
    if input.trim().to_lowercase() != "y" {
        info!("User aborted the operation");
        println!("{}", "Aborted".red());
        return;
    }
    
    info!("User confirmed, applying {} changes", changes.len());
    
    // Apply changes to original directory
    if let Err(e) = apply_changes(&current_dir, temp_path, &changes) {
        error!("Failed to apply changes: {}", e);
        eprintln!("{}", format!("Error: Failed to apply changes: {}", e).red());
        std::process::exit(1);
    }
    
    info!("Changes applied successfully");
    println!("{}", "Changes applied successfully".green());
}

#[derive(Debug)]
enum Change {
    Create(PathBuf),
    Modify(PathBuf),
    Delete(PathBuf),
}

fn copy_directory(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        
        if entry_path.is_dir() {
            copy_directory(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path)?;
        }
    }
    
    Ok(())
}

fn compare_directories(
    original: &Path,
    modified: &Path,
) -> std::io::Result<Vec<Change>> {
    let mut changes = Vec::new();
    
    // Get all files in both directories
    let mut original_files = HashSet::new();
    collect_files(original, Path::new(""), &mut original_files)?;
    
    let mut modified_files = HashSet::new();
    collect_files(modified, Path::new(""), &mut modified_files)?;
    
    // Find new files
    for file in &modified_files {
        if !original_files.contains(file) {
            changes.push(Change::Create(file.clone()));
        }
    }
    
    // Find deleted files
    for file in &original_files {
        if !modified_files.contains(file) {
            changes.push(Change::Delete(file.clone()));
        }
    }
    
    // Find modified files
    for file in original_files.intersection(&modified_files) {
        let original_path = original.join(file);
        let modified_path = modified.join(file);
        
        if fs::metadata(&original_path)?.len() != fs::metadata(&modified_path)?.len() {
            changes.push(Change::Modify(file.clone()));
            continue;
        }
        
        let original_content = fs::read(&original_path)?;
        let modified_content = fs::read(&modified_path)?;
        
        if original_content != modified_content {
            changes.push(Change::Modify(file.clone()));
        }
    }
    
    Ok(changes)
}

fn collect_files(base: &Path, prefix: &Path, files: &mut HashSet<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        let current_path = prefix.join(entry_name);
        
        if entry_path.is_dir() {
            // Recursively collect files from subdirectory, preserving the path prefix
            collect_files(&entry_path, &current_path, files)?;
        } else {
            files.insert(current_path);
        }
    }
    
    Ok(())
}

fn apply_changes(
    original: &Path,
    modified: &Path,
    changes: &[Change],
) -> std::io::Result<()> {
    for change in changes {
        match change {
            Change::Create(path) => {
                let original_path = original.join(path);
                let modified_path = modified.join(path);
                
                if let Some(parent) = original_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                fs::copy(modified_path, original_path)?;
            }
            Change::Modify(path) => {
                let original_path = original.join(path);
                let modified_path = modified.join(path);
                
                fs::copy(modified_path, original_path)?;
            }
            Change::Delete(path) => {
                let original_path = original.join(path);
                fs::remove_file(original_path)?;
            }
        }
    }
    
    Ok(())
}

/// Clean up all temporary directories created by tust
fn clean_temporary_directories() -> std::io::Result<()> {
    // Get the system temporary directory
    let temp_dir = std::env::temp_dir();
    debug!("Scanning temporary directory: {}", temp_dir.display());
    let mut cleaned_count = 0;
    
    // Iterate through all entries in the temporary directory
    for entry in fs::read_dir(temp_dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        
        // Check if it's a directory with the tust- prefix
        if entry_path.is_dir() {
            if let Some(dir_name) = entry_path.file_name() {
                if let Some(dir_name_str) = dir_name.to_str() {
                    if dir_name_str.starts_with("tust-") {
                        debug!("Found tust temporary directory: {}", entry_path.display());
                        // Delete the directory and its contents
                        match fs::remove_dir_all(&entry_path) {
                            Ok(()) => {
                                cleaned_count += 1;
                                info!("Deleted temporary directory: {}", entry_path.display());
                                println!("  {}{}", "-".red(), entry_path.display());
                            }
                            Err(e) => {
                                warn!("Failed to delete temporary directory {}: {}", entry_path.display(), e);
                                eprintln!("  {}{}: {}", "!".yellow(), entry_path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }
    
    info!("Cleaned up {} temporary directories", cleaned_count);
    println!("{}", format!("Cleaned up {} temporary directories", cleaned_count).blue());
    Ok(())
}
