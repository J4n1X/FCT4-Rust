use libfct4::fct_archive::FctArchive;
use std::path::{Path, PathBuf};

use super::expand_paths;

pub fn run(
    archive_path: &Path,
    files: Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expanded = expand_paths(files);
    if expanded.is_empty() {
        return Err("No valid files to append.".into());
    }
    let mut archive = FctArchive::open(&archive_path.to_path_buf())?;
    let failed = archive.add_files(&expanded);
    if failed.is_empty() {
        println!("All {} file(s) appended successfully.", expanded.len());
    } else {
        eprintln!("{} file(s) failed to append:", failed.len());
        for f in &failed {
            eprintln!("  {}", f.display());
        }
    }
    Ok(())
}
