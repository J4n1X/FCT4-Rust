use libfct4::fct_archive::FctArchive;
use std::path::Path;

use super::{fmt_size, resolve_targets};

pub fn run(
    archive_path: &Path,
    filters: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = FctArchive::open(&archive_path.to_path_buf())?;
    let headers = archive.get_headers().clone();
    if headers.is_empty() {
        println!("Archive is empty.");
        return Ok(());
    }
    let indices = resolve_targets(&filters, &headers);
    if indices.is_empty() {
        println!("No matching entries.");
        return Ok(());
    }
    println!("{:<6} {:<60} {}", "Index", "Path", "Size");
    println!("{}", "-".repeat(80));
    for &i in &indices {
        let h = &headers[i as usize];
        let size =
            h.chunk_count as u64 * archive.chunk_size as u64 + h.last_chunk_size as u64;
        println!(
            "{:<6} {:<60} {}",
            i + 1,
            h.file_path.display(),
            fmt_size(size)
        );
    }
    Ok(())
}
