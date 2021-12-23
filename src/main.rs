use libfct4::{fs_operations, fct_archive::FctArchive};
use std::path::PathBuf;

fn show_help(program_name: &String) {
    println!(
        "FCT File Container is an archival software used to pack files\n\
        Modes:\n\
        a - Append to archive. Usage: {0} a <path to archive> <paths to files or directories>\n\
        c - Create archive. Usage: {0} c <chunk size (max: 65535)> <path to new archive> <paths to files or directories>\n\
        e - Extract from archive. Usage: {0} e <path to archive> <output directory> <file indices (if none, all is extracted)>\n\
        h - Show help. Usage: {0} h\n\
        l - List archive contents Usage: {0} l <path to archive> <file indices (if none, all is shown)>",
        //v - Can be added to all file modes for verbose output", 
        program_name
    )
}

fn main() {
    // print current directory
    println!("Current directory: {}", std::env::current_dir().unwrap().display());
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        show_help(&args[0]);
        return;
    }
    match args[1].as_str() {
        "a" | "append" => {
            let archive_path: PathBuf = PathBuf::from(args.get(2).expect("No archive path specified"));
            
            if args.len() < 4 {
                println!("No files or directories specified");
                return;
            }
            let mut paths: Vec<PathBuf> = Vec::new();
            for path in &args[3..] {
                let path: PathBuf = match std::fs::canonicalize(PathBuf::from(&path)) {
                    Ok(p) => p,
                    Err(_) => {
                        println!("Path is invalid");
                        return;
                    }
                };

                if path.is_dir() {
                    let mut expanded_paths: Vec<PathBuf> = fs_operations::expand_directory(&path);
                    paths.append(&mut expanded_paths);
                } else {
                    paths.push(path);
                }
            }
            let mut archive = match FctArchive::open(&archive_path) {
                Ok(archive) => archive,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            };
            let failed_files = archive.add_files(&paths);
            if failed_files.len() > 0 {
                println!("Failed to add files:");
                for file in failed_files {
                    println!("{}", file.display());
                }
            }
            else {
                println!("All files have successfully been added to the archive");
            }
            return;
        }
        "c" | "create" => {
            // get next argument and parse to u16
            let chunk_size: u16 = match args.get(2).expect("No chunk size specified!").parse::<u16>() {
                Ok(n) => n,
                Err(_) => {
                    println!("Chunk size must be a number between 0 and 65535");
                    return;
                }
            };
            // get next argument and parse to PathBuf
            let archive_path: PathBuf = PathBuf::from(args.get(3).expect("No archive path specified"));

            if args.len() < 5 {
                println!("No files or directories specified");
                return;
            }
            let mut paths: Vec<PathBuf> = Vec::new();
            for path in &args[4..] {
                let path: PathBuf = match std::fs::canonicalize(PathBuf::from(&path)) {
                    Ok(p) => p,
                    Err(_) => {
                        println!("Path is invalid");
                        return;
                    }
                };

                if path.is_dir() {
                    let mut expanded_paths: Vec<PathBuf> = fs_operations::expand_directory(&path);
                    paths.append(&mut expanded_paths);
                } else {
                    paths.push(path);
                }
            }

            // create archive
            let mut archive =  match FctArchive::create_new(&archive_path, chunk_size) {
                Ok(created_archive) => {
                    println!("Archive created");
                    created_archive
                },
                Err(e) => {
                    println!("{}", e);
                    return;
                },
            };

            let failed_files = archive.add_files(&paths);
            if failed_files.len() > 0 {
                for failed_file in failed_files {
                    println!("Failed to add file: {}", failed_file.display());
                }
            }
            else {
                println!("All files have successfully been added to the archive")
            }
        }
        "e" | "extract" => {
            let archive_path: PathBuf = PathBuf::from(&args[2]);
            let output_folder = PathBuf::from(&args[3]);
            let mut file_indices: Vec<u32> = Vec::new();
            if args.len() > 4 {
                for file_index in &args[4..] {
                    match file_index.parse::<u32>() {
                        Ok(n) => {
                            if n == 0 {
                                println!("File indices must start from 1");
                                return;
                            }
                            file_indices.push(n - 1);
                        },
                        Err(_) => {
                            println!("File indices must be numbers");
                            return;
                        }
                    }
                }
            }

            let mut archive = match FctArchive::open(&archive_path) {
                Ok(opened_archive) => {
                    println!("Archive opened");
                    opened_archive
                },
                Err(e) => {
                    println!("{}", e);
                    return;
                },
            };

            let failed_files = archive.extract_files(&output_folder, &mut file_indices);
            if failed_files.len() > 0 {
                for failed_file in failed_files {
                    println!("Failed to extract file: {}", failed_file.display());
                }
            }
            else {
                println!("All files have successfully been extracted from the archive")
            }
            return;
        }
        "h" | "help" => {
            show_help(&args[0]);
        }
        "l" | "list" => {
            // unimplemented!("List mode is not implemented yet");
            // get next argument and parse to PathBuf
            if args.len() < 3 {
                println!("No archive path specified");
                return;
            }
            let archive_path: PathBuf = PathBuf::from(&args[2]);
            let mut archive = match FctArchive::open(&archive_path) {
                Ok(opened_archive) => {
                    println!("Archive opened");
                    opened_archive
                },
                Err(e) => {
                    println!("{}", e);
                    return;
                },
            };
            archive.list_files();
        }
        "r" | "remove" => {
            let archive_path: PathBuf = PathBuf::from(&args[2]);
            let mut archive = match FctArchive::open(&archive_path) {
                Ok(opened_archive) => {
                    println!("Archive opened");
                    opened_archive
                },
                Err(e) => {
                    println!("Failed to open archive: {}", e);
                    return;
                },
            };
            let mut file_indices: Vec<u32> = Vec::new();
            for file_index in &args[3..] {
                let file_index: u32 = match file_index.parse::<u32>() {
                    Ok(n) => {
                        if n == 0 {
                            println!("File indices must start with 1");
                            return;
                        }
                        n - 1
                    },
                    Err(_) => {
                        println!("File index must be a number between 0 and 65535");
                        return;
                    }
                };
                file_indices.push(file_index);
            }
            match archive.remove_files(&file_indices) {
                Ok(()) => println!("All files have successfully been removed from the archive"),
                Err(e) => println!("Failed to remove files: {}", e)
            }
            return;
        }
        &_ => show_help(&args[0])
    }
}
