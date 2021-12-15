use relative_path::RelativePath;
use std::path::PathBuf;
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

pub fn format_path(root_dir: &PathBuf, file_path: &PathBuf) -> Result<PathBuf, &'static str> {
    let return_path = PathBuf::from(
        RelativePath::new(root_dir.to_str().unwrap())
        .relative(RelativePath::new(file_path.to_str().unwrap()))
        .as_str()
    );
    Ok(return_path)
}