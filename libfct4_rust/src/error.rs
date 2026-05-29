use std::fmt;
use std::io;

#[derive(Debug)]
pub enum FctError {
    Io(io::Error),
    InvalidHeader,
    InvalidChunkSize,
    FileTooLarge,
    PathTooLong,
    InvalidPath,
    EntryNotFound,
    Custom(&'static str),
}

impl fmt::Display for FctError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FctError::Io(e) => write!(f, "I/O error: {}", e),
            FctError::InvalidHeader => write!(f, "Invalid or corrupt archive header"),
            FctError::InvalidChunkSize => write!(f, "Chunk size exceeds maximum (65535)"),
            FctError::FileTooLarge => write!(f, "File is too large for this archive format"),
            FctError::PathTooLong => write!(f, "File path is too long (max 65535 bytes)"),
            FctError::InvalidPath => write!(f, "Could not compute relative path"),
            FctError::EntryNotFound => write!(f, "Archive entry not found"),
            FctError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for FctError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FctError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for FctError {
    fn from(e: io::Error) -> Self {
        FctError::Io(e)
    }
}