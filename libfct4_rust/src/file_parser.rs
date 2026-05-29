use std::io::Read;
use std::path::PathBuf;

use crate::error::FctError;
use crate::fs_operations;

#[derive(Default, Debug, Clone)]
pub struct FileParser {
    pub file_path: PathBuf,
    pub chunk_count: u32,
    pub last_chunk_size: u16,
    /// Absolute byte offset of this entry's data in the archive.
    /// Zero for entries not yet committed to an archive.
    pub data_offset: u64,
}

impl FileParser {
    /// Build a `FileParser` from a filesystem file (for archiving).
    pub fn from_file(
        file_path: &PathBuf,
        root_dir: &PathBuf,
        chunk_size: u16,
    ) -> Result<Self, FctError> {
        let file_info = std::fs::metadata(file_path)?;
        let mut parser = FileParser::default();
        parser.file_path = fs_operations::format_path(root_dir, file_path)?;
        parser.chunk_count = u32::try_from(file_info.len() / chunk_size as u64)
            .map_err(|_| FctError::FileTooLarge)?;
        parser.last_chunk_size = u16::try_from(file_info.len() % chunk_size as u64)
            .map_err(|_| FctError::FileTooLarge)?;
        Ok(parser)
    }

    /// Parse a `FileParser` from an archive byte stream.
    /// Returns `Err(FctError::InvalidHeader)` at clean EOF (end of entries).
    pub fn from_archive<R: Read>(file: &mut R) -> Result<Self, FctError> {
        const FIELD_LEN: usize = 8;
        let mut parser = FileParser::default();
        let mut buf = [0u8; FIELD_LEN];

        // Use read() on the first byte to distinguish clean EOF from a partial read.
        let n = file.read(&mut buf[..1])?;
        if n == 0 {
            return Err(FctError::InvalidHeader); // clean EOF = no more entries
        }
        file.read_exact(&mut buf[1..])?;

        parser.chunk_count = u32::from_le_bytes(buf[..4].try_into().unwrap());
        parser.last_chunk_size = u16::from_le_bytes(buf[4..6].try_into().unwrap());
        let path_len = u16::from_le_bytes(buf[6..8].try_into().unwrap()) as usize;

        let mut path_buf = vec![0u8; path_len];
        file.read_exact(&mut path_buf)?;
        parser.file_path = PathBuf::from(
            String::from_utf8(path_buf).map_err(|_| FctError::InvalidHeader)?,
        );

        Ok(parser)
    }

    pub fn generate_header(&self) -> Result<Vec<u8>, FctError> {
        let mut header = Vec::new();
        header.extend_from_slice(&self.chunk_count.to_le_bytes());
        header.extend_from_slice(&self.last_chunk_size.to_le_bytes());
        let path_bytes = self
            .file_path
            .to_str()
            .ok_or(FctError::InvalidPath)?
            .as_bytes();
        let path_len =
            u16::try_from(path_bytes.len()).map_err(|_| FctError::PathTooLong)?;
        header.extend_from_slice(&path_len.to_le_bytes());
        header.extend_from_slice(path_bytes);
        Ok(header)
    }

    pub fn get_header_size(&self) -> usize {
        8 + self
            .file_path
            .to_str()
            .unwrap_or("")
            .as_bytes()
            .len()
    }

    /// Total bytes this entry occupies in the archive (data only, excluding the entry header).
    pub fn stored_data_size(&self, chunk_size: u16) -> u64 {
        (self.chunk_count as u64 + if self.last_chunk_size > 0 { 1 } else { 0 })
            * chunk_size as u64
    }
}

impl std::fmt::Display for FileParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "File Name: \"{}\"; Chunk Count: {}; Last Chunk Size: {}",
            self.file_path.display(),
            self.chunk_count,
            self.last_chunk_size
        )
    }
}
