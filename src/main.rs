use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use colored::Colorize;
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
    let args = Args::parse();
    
    // Handle --clean flag
    if args.clean {
        clean_temporary_directories().expect("Failed to clean temporary directories");
        println!("{}", "Cleanup completed successfully".green());
        return;
    }
    
    if args.command.is_empty() {
        eprintln!("{}", "Error: No command provided".red());
        std::process::exit(1);
    }
    
    // Get current directory
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    
    // Create temporary directory with prefix for easy identification
    let temp_dir = tempfile::Builder::new()
        .prefix("tust-")
        .tempdir()
        .expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();
    
    println!("{}", "Testing command in temporary directory...".yellow());
    
    // Copy current directory contents to temporary directory
    copy_directory(&current_dir, temp_path).expect("Failed to copy directory contents");
    
    // Run the command in the temporary directory
    let status = Command::new(&args.command[0])
        .args(&args.command[1..])
        .current_dir(temp_path)
        .status()
        .expect("Failed to execute command");
    
    if !status.success() {
        eprintln!("{}", format!("Command failed with exit code: {}", status.code().unwrap_or(-1)).red());
        std::process::exit(status.code().unwrap_or(1));
    }
    
    // Compare directories to find changes
    let changes = compare_directories(&current_dir, temp_path)
        .expect("Failed to compare directories");
    
    if changes.is_empty() {
        println!("{}", "No changes would be made".green());
        return;
    }
    
    // Display changes to user
    println!("{}", "\nChanges that would be made:".blue().bold());
    for change in &changes {
        match change {
            Change::Create(path) => println!("  {}{}", "+".green(), path.display()),
            Change::Modify(path) => println!("  {}{}", "~".yellow(), path.display()),
            Change::Delete(path) => println!("  {}{}", "-".red(), path.display()),
        }
    }
    
    // Ask for user confirmation
    println!("\n{}", "Would you like to apply these changes? (y/n)".yellow());
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read input");
    
    if input.trim().to_lowercase() != "y" {
        println!("{}", "Aborted".red());
        return;
    }
    
    // Apply changes to original directory
    apply_changes(&current_dir, temp_path, &changes)
        .expect("Failed to apply changes");
    
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
                        // Delete the directory and its contents
                        fs::remove_dir_all(&entry_path)?;
                        cleaned_count += 1;
                        println!("  {}{}", "-".red(), entry_path.display());
                    }
                }
            }
        }
    }
    
    println!("{}", format!("Cleaned up {} temporary directories", cleaned_count).blue());
    Ok(())
}
