use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use sha2::{Sha256, Digest};
use glob::glob;

/// Checks if a path exists
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Calculates the SHA-256 hash of a file's contents
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = hasher.finalize();
    
    Ok(format!("{:x}", hash))
}

/// Gets the relative path from a base path
pub fn get_relative_path(base: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(base)
        .with_context(|| {
            format!(
                "Failed to get relative path. Base: {}, Path: {}", 
                base.display(), 
                path.display()
            )
        })
        .map(|p| p.to_path_buf())
}

/// Finds files matching a pattern in a directory
pub fn find_files(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let glob_pattern = dir.join(pattern).to_string_lossy().to_string();
    
    let paths = glob(&glob_pattern)
        .with_context(|| format!("Failed to read glob pattern: {}", glob_pattern))?
        .filter_map(Result::ok)
        .collect();
        
    Ok(paths)
}

/// Reads a file's contents as a string
pub fn read_file_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

/// Creates a directory and any parent directories
pub fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))
}

/// Writes a string to a file, creating the file if it doesn't exist
pub fn write_string_to_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write to file: {}", path.display()))
}

/// Copies a file from source to destination
pub fn copy_file(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        create_dir_all(parent)?;
    }
    
    fs::copy(source, dest)
        .with_context(|| {
            format!(
                "Failed to copy file from {} to {}", 
                source.display(), 
                dest.display()
            )
        })?;
        
    Ok(())
}

/// Removes a file
pub fn remove_file(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("Failed to remove file: {}", path.display()))
} 