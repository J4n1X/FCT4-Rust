use libfct4::fct_archive::FctArchive;
use std::path::{Path, PathBuf};

use super::expand_paths;

pub fn run(
    archive_path: &Path,
    chunk_size: u16,
    files: Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expanded = expand_paths(files);
    if expanded.is_empty() {
        return Err("No valid files to add.".into());
    }
    let mut archive = FctArchive::create_new(&archive_path.to_path_buf(), chunk_size)?;
    println!("Archive created: {}", archive_path.display());
    let failed = archive.add_files(&expanded);
    if failed.is_empty() {
        println!("All {} file(s) added successfully.", expanded.len());
    } else {
        eprintln!("{} file(s) failed to add:", failed.len());
        for f in &failed {
            eprintln!("  {}", f.display());
        }
    }
    Ok(())
}
