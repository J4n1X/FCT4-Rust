pub mod add;
pub mod create;
pub mod extract;
pub mod list;
pub mod remove;

use libfct4::file_parser::FileParser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Expand a list of file/directory paths into individual canonicalized file paths.
/// Directories are walked recursively. Invalid paths are skipped with a warning.
pub fn expand_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for path in paths {
        match std::fs::canonicalize(&path) {
            Ok(canonical) if canonical.is_dir() => {
                result.extend(libfct4::fs_operations::expand_directory(&canonical));
            }
            Ok(canonical) => result.push(canonical),
            Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
        }
    }
    result
}

/// Convert target strings into sorted unique 0-based indices into `headers`.
///
/// Resolution order for each target:
/// 1. Parse as a 1-based integer index.
/// 2. Exact `file_path` match or full-prefix match (folder path given in full).
/// 3. Component-sequence match: the target's path components appear as a contiguous
///    run anywhere inside the stored path's components. This lets users type a bare
///    folder name (e.g. `sub`) and match entries like `project/src/sub/file.rs`.
///
/// An empty `targets` slice returns all indices.
pub fn resolve_targets(targets: &[String], headers: &[FileParser]) -> Vec<u32> {
    if targets.is_empty() {
        return (0..headers.len() as u32).collect();
    }
    let mut set = HashSet::new();
    for target in targets {
        if let Ok(n) = target.parse::<u32>() {
            if n > 0 && n as usize <= headers.len() {
                set.insert(n - 1);
            }
            continue;
        }
        let target_path = Path::new(target);
        let target_comps: Vec<_> = target_path.components().collect();
        let tn = target_comps.len();
        for (i, h) in headers.iter().enumerate() {
            // Fast path: exact or full-prefix match.
            if h.file_path == target_path || h.file_path.starts_with(target_path) {
                set.insert(i as u32);
                continue;
            }
            // Component-sequence match: find the target run anywhere in the stored path.
            let stored_comps: Vec<_> = h.file_path.components().collect();
            let sn = stored_comps.len();
            if tn <= sn {
                'window: for start in 0..=(sn - tn) {
                    for (j, tc) in target_comps.iter().enumerate() {
                        if stored_comps[start + j] != *tc {
                            continue 'window;
                        }
                    }
                    set.insert(i as u32);
                    break;
                }
            }
        }
    }
    let mut v: Vec<u32> = set.into_iter().collect();
    v.sort_unstable();
    v
}

/// Format a byte count as a human-readable string (B / KiB / MiB / GiB).
pub fn fmt_size(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.1} GiB", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.1} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else if bytes >= 1 << 10 {
        format!("{:.1} KiB", bytes as f64 / (1u64 << 10) as f64)
    } else {
        format!("{} B", bytes)
    }
}
