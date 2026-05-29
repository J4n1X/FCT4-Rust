use libfct4::fct_archive::FctArchive;
use std::path::Path;

use super::resolve_targets;

pub fn run(
    archive_path: &Path,
    targets: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = FctArchive::open(&archive_path.to_path_buf())?;
    let headers = archive.get_headers().clone();
    if headers.is_empty() {
        return Err("Archive is empty.".into());
    }
    let indices = resolve_targets(&targets, &headers);
    if indices.is_empty() {
        println!("No matching entries to remove.");
        return Ok(());
    }
    archive.remove_files(&indices)?;
    println!("{} entry/entries removed.", indices.len());
    Ok(())
}
