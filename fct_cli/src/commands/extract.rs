use libfct4::fct_archive::FctArchive;
use std::path::Path;

use super::resolve_targets;

pub fn run(
    archive_path: &Path,
    output: &Path,
    targets: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = FctArchive::open(&archive_path.to_path_buf())?;
    let headers = archive.get_headers().clone();
    if headers.is_empty() {
        println!("Archive is empty.");
        return Ok(());
    }
    let mut indices = resolve_targets(&targets, &headers);
    if indices.is_empty() {
        println!("No matching entries.");
        return Ok(());
    }
    let total = indices.len();
    let failed = archive.extract_files(&output.to_path_buf(), &mut indices);
    if failed.is_empty() {
        println!("All {} file(s) extracted successfully.", total);
    } else {
        eprintln!("{} file(s) failed to extract:", failed.len());
        for f in &failed {
            eprintln!("  {}", f.display());
        }
    }
    Ok(())
}
