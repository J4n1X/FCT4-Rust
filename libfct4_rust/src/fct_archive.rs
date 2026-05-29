use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use bufreaderwriter::BufReaderWriter;

use crate::error::FctError;
use crate::file_parser::FileParser;

const MAX_CHUNK_SIZE: u16 = 65535;
const ARCHIVE_HEADER_SIZE: usize = 5;
const ARCHIVE_HEADER_MAGIC: &[u8; 3] = b"FCT";

/// I/O buffer size for bulk data transfers (1 MiB).
const IO_BUFFER_SIZE: usize = 1 << 20;

pub struct FctArchive {
    pub chunk_size: u16,
    pub archive_file: BufReaderWriter<File>,
    pub archive_path: PathBuf,
    headers: Vec<FileParser>,
    headers_stale: bool,
}

/// The main archive struct
impl FctArchive {
    /// Create a new archive at the given path with the specified chunk size
    pub fn create_new(archive_path: &PathBuf, chunk_size: u16) -> Result<Self, FctError> {
        if chunk_size > MAX_CHUNK_SIZE {
            return Err(FctError::InvalidChunkSize);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(archive_path)?;
        let mut archive_file = BufReaderWriter::new_writer(file);
        archive_file.write_all(ARCHIVE_HEADER_MAGIC)?;
        archive_file.write_all(&chunk_size.to_le_bytes())?;
        Ok(FctArchive {
            chunk_size,
            archive_file,
            archive_path: archive_path.to_path_buf(),
            headers: Vec::new(),
            headers_stale: false,
        })
    }

    /// Open an existing archive and read its chunk size from the header
    pub fn open(archive_path: &PathBuf) -> Result<Self, FctError> {
        let mut file_header_buffer = [0u8; ARCHIVE_HEADER_SIZE];
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(archive_path)?;
        let mut archive_file = BufReaderWriter::new_reader(file);
        archive_file.read_exact(&mut file_header_buffer)?;
        if &file_header_buffer[..3] != ARCHIVE_HEADER_MAGIC {
            return Err(FctError::InvalidHeader);
        }
        let chunk_size = u16::from_le_bytes(
            file_header_buffer[3..]
                .try_into()
                .map_err(|_| FctError::InvalidHeader)?,
        );
        let mut archive = FctArchive {
            chunk_size,
            archive_file,
            archive_path: archive_path.to_path_buf(),
            headers: Vec::new(),
            headers_stale: true,
        };
        archive.get_headers();
        Ok(archive)
    }

    /// Seek to the first file entry
    fn seek_to_start(&mut self) {
        self.archive_file
            .seek(SeekFrom::Start(ARCHIVE_HEADER_SIZE as u64))
            .expect("Could not seek to start of archive");
    }

    /// Copy exactly `n` bytes from the archive to `dest` using a 1 MiB buffer.
    fn copy_bytes_to<W: Write>(&mut self, dest: &mut W, n: u64) -> Result<(), FctError> {
        if n == 0 {
            return Ok(());
        }
        let buf_cap = (n as usize).min(IO_BUFFER_SIZE);
        let mut buf = vec![0u8; buf_cap];
        let mut remaining = n;
        while remaining > 0 {
            let to_read = (remaining as usize).min(buf_cap);
            self.archive_file.read_exact(&mut buf[..to_read])?;
            dest.write_all(&buf[..to_read])?;
            remaining -= to_read as u64;
        }
        Ok(())
    }

    /// Copy exactly `n` bytes from `src` into the archive using a 1 MiB buffer.
    fn copy_bytes_from<R: Read>(&mut self, src: &mut R, n: u64) -> Result<(), FctError> {
        if n == 0 {
            return Ok(());
        }
        let buf_cap = (n as usize).min(IO_BUFFER_SIZE);
        let mut buf = vec![0u8; buf_cap];
        let mut remaining = n;
        while remaining > 0 {
            let to_read = (remaining as usize).min(buf_cap);
            src.read_exact(&mut buf[..to_read])?;
            self.archive_file.write_all(&buf[..to_read])?;
            remaining -= to_read as u64;
        }
        Ok(())
    }

    /// Write a file's data into the archive, zero-padding the last chunk.
    fn write_file_to_archive<R: Read>(
        &mut self,
        src: &mut R,
        header: &FileParser,
    ) -> Result<(), FctError> {
        // Bulk-copy all full chunks in one shot — no per-chunk loop.
        let full_bytes = header.chunk_count as u64 * self.chunk_size as u64;
        self.copy_bytes_from(src, full_bytes)?;
        // Write the last partial chunk, zero-padded to chunk_size.
        if header.last_chunk_size > 0 {
            let mut last_buf = vec![0u8; self.chunk_size as usize];
            src.read_exact(&mut last_buf[..header.last_chunk_size as usize])?;
            self.archive_file.write_all(&last_buf)?;
        }
        Ok(())
    }

    /// Extract a file's data from the archive, trimming the last chunk to its actual size.
    fn write_file_from_archive<W: Write>(
        &mut self,
        dest: &mut W,
        header: &FileParser,
    ) -> Result<(), FctError> {
        // Bulk-copy all full chunks in one shot — no per-chunk loop.
        let full_bytes = header.chunk_count as u64 * self.chunk_size as u64;
        self.copy_bytes_to(dest, full_bytes)?;
        // Write the last chunk trimmed to the actual file size.
        if header.last_chunk_size > 0 {
            let mut last_buf = vec![0u8; self.chunk_size as usize];
            self.archive_file.read_exact(&mut last_buf)?;
            dest.write_all(&last_buf[..header.last_chunk_size as usize])?;
        }
        Ok(())
    }

    /// Read and cache all file entry headers, recording each entry's data offset.
    /// No-op when the cache is fresh.
    pub fn get_headers(&mut self) -> &Vec<FileParser> {
        if !self.headers_stale {
            return &self.headers;
        }
        self.headers.clear();
        self.seek_to_start();
        loop {
            let mut parser = match FileParser::from_archive(&mut self.archive_file) {
                Ok(p) => p,
                Err(_) => break,
            };
            // Record the absolute byte offset where this entry's data begins.
            parser.data_offset = self
                .archive_file
                .seek(SeekFrom::Current(0))
                .expect("Could not query archive position");
            let data_len = parser.stored_data_size(self.chunk_size);
            if self
                .archive_file
                .seek(SeekFrom::Current(data_len as i64))
                .is_err()
            {
                self.headers.push(parser);
                break;
            }
            self.headers.push(parser);
        }
        self.headers_stale = false;
        &self.headers
    }

    /// Append a single file to the archive.
    pub fn add_file(&mut self, file_path: &PathBuf) -> Result<(), FctError> {
        self.archive_file.seek(SeekFrom::End(0))?;
        let src_file = File::open(file_path)?;
        let mut src = BufReader::with_capacity(IO_BUFFER_SIZE, src_file);
        let current_dir = std::env::current_dir()?;
        let parser = FileParser::from_file(file_path, &current_dir, self.chunk_size)?;
        println!("Adding file: {}", parser.file_path.display());
        self.archive_file.write_all(&parser.generate_header()?)?;
        self.headers_stale = true;
        self.write_file_to_archive(&mut src, &parser)
    }

    /// Add multiple files; returns paths of any that failed.
    pub fn add_files(&mut self, file_paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut failed: Vec<PathBuf> = Vec::new();
        for path in file_paths {
            if let Err(e) = self.add_file(path) {
                println!("Error adding {}: {}", path.display(), e);
                failed.push(path.clone());
            }
        }
        self.headers_stale = true;
        failed
    }

    /// Extract a single file by index into `output_path`.
    pub fn extract_file(
        &mut self,
        output_folder: PathBuf,
        index: u32,
        output_path: &PathBuf,
    ) -> Result<(), FctError> {
        if self.headers_stale {
            self.get_headers();
        }
        let header = self
            .headers
            .get(index as usize)
            .ok_or(FctError::EntryNotFound)?
            .clone();
        if !output_folder.exists() {
            std::fs::create_dir_all(&output_folder)?;
        }
        let out_path = output_path.join(&header.file_path);
        if out_path.exists() {
            println!("File already exists, skipping: {}", out_path.display());
            return Ok(());
        }
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        println!("Extracting file: {}", out_path.display());
        let out_file = OpenOptions::new().write(true).create(true).open(&out_path)?;
        let mut dest = BufWriter::with_capacity(IO_BUFFER_SIZE, out_file);
        // Jump directly to this entry's data using the precomputed offset.
        self.archive_file.seek(SeekFrom::Start(header.data_offset))?;
        self.write_file_from_archive(&mut dest, &header)
    }

    /// Extract files by index list; extracts all if `indices` is empty.
    pub fn extract_files(
        &mut self,
        output_folder: &PathBuf,
        indices: &mut Vec<u32>,
    ) -> Vec<PathBuf> {
        if self.headers_stale {
            self.get_headers();
        }
        if !output_folder.exists() {
            if let Err(e) = std::fs::create_dir_all(output_folder) {
                println!("Could not create output folder: {}", e);
                return self.headers.iter().map(|h| h.file_path.clone()).collect();
            }
        }
        // Build a HashSet for O(1) membership tests; empty = extract all.
        let target_set: HashSet<u32> = if indices.is_empty() {
            (0..self.headers.len() as u32).collect()
        } else {
            indices.iter().copied().collect()
        };

        let mut failed: Vec<PathBuf> = Vec::new();
        for i in 0..self.headers.len() {
            if !target_set.contains(&(i as u32)) {
                continue;
            }
            let header = self.headers[i].clone();
            let out_path = output_folder.join(&header.file_path);
            if let Some(parent) = out_path.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        println!("Could not create {}: {}", parent.display(), e);
                        failed.push(header.file_path.clone());
                        continue;
                    }
                }
            }
            println!("Extracting file: {}", out_path.display());
            let out_file = match OpenOptions::new().write(true).create(true).open(&out_path) {
                Ok(f) => f,
                Err(e) => {
                    println!("Could not create {}: {}", out_path.display(), e);
                    failed.push(header.file_path.clone());
                    continue;
                }
            };
            let mut dest = BufWriter::with_capacity(IO_BUFFER_SIZE, out_file);
            // Jump directly to this entry's data using the precomputed offset.
            self.archive_file
                .seek(SeekFrom::Start(header.data_offset))
                .expect("Could not seek to file data");
            if let Err(e) = self.write_file_from_archive(&mut dest, &header) {
                println!("Error extracting {}: {}", out_path.display(), e);
                failed.push(header.file_path.clone());
            }
        }
        failed
    }

    pub fn list_files(&mut self) {
        if self.headers_stale {
            self.get_headers();
        }
        if self.headers.is_empty() {
            println!("No files in archive");
            return;
        }
        for (i, header) in self.headers.iter().enumerate() {
            println!(
                "{}: {} {}",
                i + 1,
                header.file_path.display(),
                header.chunk_count as u64 * self.chunk_size as u64
                    + header.last_chunk_size as u64
            );
        }
    }

    /// Remove files by index, writing survivors to a temp archive then replacing the original.
    pub fn remove_files(&mut self, file_indices: &[u32]) -> Result<(), FctError> {
        if self.headers_stale {
            self.get_headers();
        }
        if self.headers.is_empty() {
            return Err(FctError::Custom("No files in archive"));
        }
        let tmp_path = self.archive_path.with_extension("tmp");
        let mut tmp = FctArchive::create_new(&tmp_path, self.chunk_size)?;
        let headers = self.headers.clone();
        for (index, header) in headers.iter().enumerate() {
            if file_indices.contains(&(index as u32)) {
                println!("Removing file: {}", header.file_path.display());
            } else {
                // Seek to this entry's data and copy header + raw data verbatim to tmp.
                self.archive_file
                    .seek(SeekFrom::Start(header.data_offset))?;
                tmp.archive_file.write_all(&header.generate_header()?)?;
                let data_size = header.stored_data_size(self.chunk_size);
                if data_size > 0 {
                    let buf_cap = (data_size as usize).min(IO_BUFFER_SIZE);
                    let mut buf = vec![0u8; buf_cap];
                    let mut remaining = data_size;
                    while remaining > 0 {
                        let to_read = (remaining as usize).min(buf_cap);
                        self.archive_file.read_exact(&mut buf[..to_read])?;
                        tmp.archive_file.write_all(&buf[..to_read])?;
                        remaining -= to_read as u64;
                    }
                }
            }
        }
        std::fs::remove_file(&self.archive_path)?;
        std::fs::rename(&tmp_path, &self.archive_path)?;
        // The inode from tmp is now at self.archive_path; take its file handle.
        // self.archive_path is already correct (unchanged).
        self.archive_file = tmp.archive_file;
        self.headers_stale = true;
        self.get_headers();
        Ok(())
    }
}