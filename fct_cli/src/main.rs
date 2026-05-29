use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(name = "fct4_rust", about = "FCT File Container v4 archiver")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new archive
    #[command(alias = "c")]
    Create {
        /// Chunk size in bytes (1-65535)
        #[arg(short = 'n', long)]
        chunk_size: u16,
        /// Path to create the archive at
        archive: PathBuf,
        /// Files or directories to add
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Append files or directories to an existing archive
    #[command(alias = "a")]
    Append {
        /// Path to the archive
        archive: PathBuf,
        /// Files or directories to append
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Extract files from an archive
    #[command(alias = "e")]
    Extract {
        /// Path to the archive
        archive: PathBuf,
        /// Output directory
        output: PathBuf,
        /// Targets: 1-based indices, file paths, or folder paths (empty = all)
        targets: Vec<String>,
    },
    /// List archive contents
    #[command(alias = "l")]
    List {
        /// Path to the archive
        archive: PathBuf,
        /// Filters: 1-based indices or folder paths (empty = all)
        filters: Vec<String>,
    },
    /// Remove entries from an archive
    #[command(alias = "r")]
    Remove {
        /// Path to the archive
        archive: PathBuf,
        /// Targets: 1-based indices, file paths, or folder paths
        #[arg(required = true)]
        targets: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Create {
            chunk_size,
            archive,
            files,
        } => commands::create::run(&archive, chunk_size, files),
        Commands::Append { archive, files } => commands::add::run(&archive, files),
        Commands::Extract {
            archive,
            output,
            targets,
        } => commands::extract::run(&archive, &output, targets),
        Commands::List { archive, filters } => commands::list::run(&archive, filters),
        Commands::Remove { archive, targets } => commands::remove::run(&archive, targets),
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
