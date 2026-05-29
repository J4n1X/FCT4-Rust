use pathdiff;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::error::FctError;

#[allow(dead_code)]
fn get_folders(file_indexes: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for path in file_indexes {
        let parent = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        if !folders.contains(&parent) {
            folders.push(parent);
        }
    }
    folders.sort_by(|a, b| {
        if a.to_str().unwrap().len() > b.to_str().unwrap().len() {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Less
        }
    });
    folders
}

#[allow(dead_code)]
pub fn create_directories(file_indexes: &[PathBuf]) {
    get_folders(file_indexes).iter().for_each(|folder| {
        match fs::create_dir(folder) {
            Ok(_) => {}
            Err(e) => println!("Error creating folder: {}", e),
        }
    });
}

#[allow(dead_code)]
pub fn expand_directory(path: &PathBuf) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }
    files
}

// default function for all OSes
#[cfg(not(target_os = "windows"))]
pub fn format_path<'a, P: ?Sized>(
    root_dir: &'a P,
    file_path: &'a P,
) -> Result<PathBuf, FctError>
where
    P: AsRef<Path>,
    &'a Path: From<&'a P>,
{
    match pathdiff::diff_paths(file_path.into(), root_dir.into()) {
        Some(path) => Ok(path),
        None => Err(FctError::InvalidPath),
    }
}

// special function for windows
#[cfg(target_os = "windows")]
pub fn format_path<'a, P: ?Sized>(
    root_dir: &'a P,
    file_path: &'a P,
) -> Result<PathBuf, FctError>
where
    P: AsRef<Path>,
    &'a Path: From<&'a P>,
{
    let root_dir_canonical = fs::canonicalize(root_dir)?;
    match pathdiff::diff_paths(file_path.into(), &root_dir_canonical) {
        Some(path) => Ok(path),
        None => Err(FctError::InvalidPath),
    }
}
