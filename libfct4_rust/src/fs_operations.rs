//use relative_path::RelativePath;
use pathdiff;
use std::path::{Path,PathBuf};
use std::fs::{self};
use walkdir::WalkDir;

// create folders for a list of path buffers
#[allow(dead_code)]
fn get_folders(file_indexes: &Vec<PathBuf>) ->  Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for path in file_indexes {
        // FIXME: This probably wont work with files in the root directory.
        // Not that that should happen on linux
        let parent = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        if !folders.contains(&parent) {
            folders.push(parent);
        }
    }
    // TODO: rewrite this
    folders.sort_by(|a, b| if a.to_str().unwrap().len() > b.to_str().unwrap().len() { 
            std::cmp::Ordering::Greater
        } 
        else {  
            std::cmp::Ordering::Less
        });
    println!("{:?}", folders);
    return folders;
}

#[allow(dead_code)]
pub fn create_directories(file_indexes: &Vec<PathBuf>){
    get_folders(file_indexes).iter().for_each(|folder| {
        match fs::create_dir(folder) {
            Ok(_) => return,
            Err(e) => println!("Error creating folder: {}", e)
        }
    });
}

#[allow(dead_code)]
pub fn expand_directory(path: &PathBuf) -> Vec<PathBuf> {
    // get all files in directory recursively
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&path) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }
    return files;
}

// default function for all OSes
#[cfg(not(target_os = "windows"))]
pub fn format_path<'a, P: ?Sized>(root_dir: &'a P, file_path: &'a P) -> Result<PathBuf, &'static str> where P : AsRef<Path>, &'a Path: From<&'a P>  {
    match pathdiff::diff_paths(file_path.into(), root_dir.into()) {
        Some(path) => Ok(path),
        None => Err("Could not get relative path")
    }
}

// special function for windows
#[cfg(target_os = "windows")]
pub fn format_path<'a, P: ?Sized>(root_dir: &'a P, file_path: &'a P) -> Result<PathBuf, &'static str> where P : AsRef<Path>, &'a Path: From<&'a P>  {
    // if you're using std::fs::current_dir() you need canonicalize it first
    let root_dir_canonical = match fs::canonicalize(root_dir) {
        Ok(p) => p,
        Err(_) => return Err("Could not get canonical path for root directory")
    };
    match pathdiff::diff_paths(file_path.into(), &root_dir_canonical) {
        Some(path) => Ok(path),
        None => Err("Could not get relative path")
    }
}