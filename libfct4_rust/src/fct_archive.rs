use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom, BufWriter, BufReader};
use bufreaderwriter::BufReaderWriter;
use std::path::PathBuf;
use crate::file_parser::FileParser;
use crate::error::*;

//const DEFAULT_CHUNK_SIZE: u16 = 256;
const MAX_CHUNK_SIZE: u16 = 65535;
const ARCHIVE_HEADER_SIZE: usize = 5;
const ARCHIVE_HEADER_MAGIC: &str = "FCT";

pub struct FctArchive {
    pub chunk_size: u16,
    pub archive_file: BufReaderWriter<File>,
    pub archive_path: PathBuf, 
    headers: Vec<FileParser>,
    headers_stale: bool
}

#[allow(dead_code)]
impl FctArchive {
    pub fn create_new(archive_path: &PathBuf, chunk_size: u16) -> Result<Self, &'static str>{
        if chunk_size > MAX_CHUNK_SIZE {
            return Err("Chunk size is too big");
        }
        let chunk_size = chunk_size;
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(archive_path) {
            Ok(file) => {
                let mut archive_file = BufReaderWriter::new_writer(file);
                //let mut archive_file = file;
                archive_file.write(ARCHIVE_HEADER_MAGIC.as_bytes()).expect("Failed to write archive header");
                archive_file.write(&chunk_size.to_le_bytes()).expect("Failed to write chunk size");
                Ok(FctArchive {
                    chunk_size: chunk_size,
                    archive_file: archive_file,
                    archive_path: archive_path.to_path_buf(),
                    headers: Vec::new(),
                    headers_stale: false
                })
            },
            Err(_) => {
                println!("Error creating archive: Could not create file.");
                //let error_message = format!("{}", e);
                return Err("Error creating archive: Could not create file.");
            }
        }
    }

    pub fn open(archive_path: &PathBuf) -> Result<Self, &'static str>{
        let mut file_header_buffer = [0u8; ARCHIVE_HEADER_SIZE];
        match OpenOptions::new()
            .read(true)
            .write(true)
            .open(archive_path)
        {
            Ok(file) => {
                let mut archive_file = BufReaderWriter::new_reader(file);
                //let mut archive_file = file;
                archive_file.read_exact(&mut file_header_buffer).expect("Could not read archive header.");
                if &file_header_buffer[..3] != ARCHIVE_HEADER_MAGIC.as_bytes() {
                    println!("Invalid archive header");
                    return Err("Invalid archive header");
                }
                let chunk_size = u16::from_le_bytes(file_header_buffer[3..].try_into().expect("Invalid chunk size read!"));
                let mut archive = FctArchive {
                    chunk_size: chunk_size,
                    archive_file: archive_file,
                    archive_path: archive_path.to_path_buf(),
                    headers: Vec::new(),
                    headers_stale: true
                };
                archive.get_headers();

                Ok(archive)
            },
            Err(e) => {
                println!("Error opening archive: {}", e);
                return Err("Error opening archive file");
            }
        }
    }

    fn seek_to_start(&mut self) {
        self.archive_file.seek(SeekFrom::Start(ARCHIVE_HEADER_SIZE as u64))
            .expect("Could not seek to start of archive");
    }

    // seek over file while reading the header
    fn seek_file(&mut self) -> Option<FileParser> {

        let parsed_file = match FileParser::from_archive(&mut self.archive_file){
            Ok(file) => file,
            Err(_) => return None
        };
        // TODO: fast seeking
        // seek chunks
        match self.seek_data(&parsed_file) {
            Ok(_) => {
                return Some(parsed_file);
            },
            Err(_) => {
                return None;
            }
        }
    }

    fn seek_data(&mut self, file_parser: &FileParser) -> Result<(), &'static str> {
        // seek chunks
        let byte_count: i64 = (if file_parser.last_chunk_size > 0 {1} else {0} + file_parser.chunk_count as i64) * self.chunk_size as i64;
        if byte_count < 0 {
            println!("Negative byte count, probably overflow, attempting slow seek");
            for _ in 0..(file_parser.chunk_count + 1) {
                match self.archive_file.seek(SeekFrom::Current(self.chunk_size as i64)){
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error seeking over file: {}", e);
                        return Err("Error seeking over file");
                    }
                }
            }
            Ok(())
        }
        else {
            match self.archive_file.seek(SeekFrom::Current(byte_count)){
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error seeking over file: {}", e);
                    Err("Error seeking over file")
                }
            }
        }
    }

    fn seek_to_entry(&mut self, entry_index: u32) -> Result<(), &'static str> {
        let mut current_index = 0;
        self.seek_to_start();
        // limit by length of vector for safety
        for index in 0..self.headers.len() {
            if current_index == entry_index {
                return Ok(());
            }
            for _ in 0..(self.headers[index].chunk_count + 1) {
                unwrap_or_return_error!(self.archive_file.seek(SeekFrom::Current(self.chunk_size as i64)), "Error seeking over file");
            }
            current_index += 1;
        }
        Err("Could not find entry")
    }

    // TODO: Implement with large buffers to avoid overhead
    // writes file data to the archive
    fn write_file_to_archive<Reader: Read + Seek>(&mut self, file: &mut Reader, header: &FileParser) -> Result<(), &'static str>{
        let mut file_buffer = Vec::with_capacity(self.chunk_size as usize);
        for _ in 0..header.chunk_count {
            file_buffer.clear();
            unwrap_or_return_error!(
                std::io::Read::by_ref(file).take(self.chunk_size as u64).read_to_end(&mut file_buffer), 
                "Could not read file"
            );
            unwrap_or_return_error!(self.archive_file.write(&file_buffer),"Could not write to archive");
        }
        if header.last_chunk_size > 0 {
            file_buffer.clear();
            unwrap_or_return_error!(
                std::io::Read::by_ref(file).take(self.chunk_size as u64).read_to_end(&mut file_buffer), 
                "Could not read file"
            );
            file_buffer.resize(self.chunk_size as usize, 0);
            unwrap_or_return_error!(self.archive_file.write(&file_buffer),"Could not write to archive");
        }
        Ok(())
    }

    // TODO: Implement with large buffers to avoid overhead
    fn write_file_from_archive<Writer: Write + Seek>(&mut self, file: &mut Writer, header: &FileParser, fill: bool) -> Result<(), &'static str>{
        let mut file_buffer = Vec::with_capacity(self.chunk_size as usize);
        for _ in 0..header.chunk_count {
            file_buffer.clear();
            unwrap_or_return_error!(
                std::io::Read::by_ref(&mut self.archive_file)
                    .take(self.chunk_size as u64)
                    .read_to_end(&mut file_buffer), 
                "Could not read file"
            );
            unwrap_or_return_error!(
                file.write(&file_buffer), 
                "Error extracting file: Could not write file"
            );
        }
        if header.last_chunk_size > 0 {
            file_buffer.clear();
            unwrap_or_return_error!(
                std::io::Read::by_ref(&mut self.archive_file)
                    .take(self.chunk_size as u64)
                    .read_to_end(&mut file_buffer),
                "Could not read file"
            );
            if fill {
                file_buffer.resize(self.chunk_size as usize, 0);
            }
            else {
                file_buffer.resize(header.last_chunk_size as usize, 0);
            }
            match file.write(&file_buffer) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("Error extracting file: {}", e);
                    Err("Error extracting file")
                }
            }
        }
        else {
            Ok(())
        }
    }

    pub fn get_headers(&mut self) -> &Vec<FileParser> {
        if !self.headers_stale {
            return &self.headers;
        }
        self.headers.clear();
        self.seek_to_start();
        loop {
            match self.seek_file() {
                Some(file) => {
                    self.headers.push(file);
                },
                None => {
                    break;
                }
            }
        }
        self.headers_stale = false;
        return &self.headers;
    }        

    
    pub fn add_file(&mut self, file_path: &PathBuf) -> Result<(), &'static str>{
        self.archive_file.seek(SeekFrom::End(0)).expect("Could not seek to end of archive");
        
        let mut file = match File::open(file_path){
            Ok(f) => BufReader::new(f),
            Err(_) => {
                return Err("Error adding file: Could not open file");
            }
        };
        let current_dir = std::env::current_dir().unwrap();
        let parser = unwrap_or_return_error!(
            FileParser::from_file(
                &file_path,
                &current_dir,
                self.chunk_size
            ),
            "Error adding file: Could not create file parser"
        );
        println!("Adding file: {}", parser.file_path.display());
        unwrap_or_return_error!(
            self.archive_file.write(&parser.generate_header().unwrap()),
            "Could not write file header"
        );
        self.write_file_to_archive(&mut file, &parser)
    }

    // Add files and return list of failed files
    pub fn add_files(&mut self, file_paths: &Vec<PathBuf>) -> Vec<PathBuf>{
        let mut failed_files: Vec<PathBuf> = Vec::new();
        for file_path in file_paths {
            match self.add_file(&file_path) {
                Ok(_) => {},
                Err(e) => {
                    println!("Error adding file: {}", e);
                    failed_files.push(file_path.clone());
                }
            };
        }
        failed_files
    }

    // This function probably isn't needed
    pub fn extract_file(&mut self, output_folder: PathBuf, index: u32, output_path: &PathBuf) -> Result<(), &'static str>{
        self.seek_to_start();
        if !output_folder.exists() {
            unwrap_or_return_error!(
                std::fs::create_dir_all(&output_folder),
                "Error extracting file: Could not create output folder"
            );
        }

        for _ in 0..index {
            match self.seek_file(){
                Some(_) => {},
                None => return Err("Could not seek to file")
            }
        }

        let mut header = unwrap_or_return_error!(
            FileParser::from_archive(&mut self.archive_file),
            "Could not parse file header"
        );
        println!("Extracting file: {}", header.file_path.display());

        let file_path = output_path.join(&header.file_path);
        if file_path.exists() {
            println!("File already exists, skipping");
            return Ok(());
        }

        let file = match OpenOptions::new()
                .write(true)
                .create(true)
                .open(file_path) {
            Ok(f) => BufReaderWriter::new_writer(f),
            Err(_) => {
                return Err("Error extracting file: Could not create file");
            }
        };
        // prepend header file path 
        header.file_path = output_folder.join(&header.file_path);
        self.write_file_from_archive(&mut BufReaderWriter::new_writer(file), &header, false)  
    }

    // this is more sophisticated than adding files because of optimisations
    pub fn extract_files(&mut self, output_folder: &PathBuf ,indices: &mut Vec<u32>) -> Vec<PathBuf>{
        self.seek_to_start();
        if self.headers_stale {
            self.get_headers();
        }
        if !output_folder.exists() {
            match std::fs::create_dir_all(output_folder) {
                Ok(_) => {},
                Err(e) => {
                    println!("Error extracting files: Could not create output folder: {}", e);
                    // fill vector with the paths of all indices
                    let output_vector = indices.iter().map(|i| {
                        self.headers[*i as usize].file_path.clone()
                    }).collect();
                    return output_vector;
                }
            }
        }
        let mut failed_files: Vec<PathBuf> = Vec::new();
        if indices.len() == 0 {
            for i in 0..self.headers.len() {
                indices.push(i as u32);
            }
        }

        indices.sort();
        let mut prev_directory: PathBuf = self.headers[0].file_path.parent().unwrap().to_path_buf();
        for i in 0..self.headers.len() {
            if !indices.contains(&(i as u32)) {
                self.seek_file();
            }
            else{
                let mut header = self.headers[i as usize].clone();
                let orig_header_size: i64 = header.get_header_size() as i64;
                header.file_path = output_folder.join(&header.file_path);
                let cur_directory = header.file_path.parent().unwrap();
                if cur_directory != prev_directory {
                    match std::fs::create_dir_all(header.file_path.parent().unwrap()) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Error extracting files: Could not create output folder: {}", e);
                            failed_files.push(header.file_path.clone());
                            continue;
                        }
                    }
                    prev_directory = cur_directory.to_path_buf();
                }

                println!("Extracting file: {}", header.file_path.display());

                // seek over header
                self.archive_file.seek(SeekFrom::Current(orig_header_size)).expect("Could not seek to file");
                // write out file
                let mut out_file = match OpenOptions::new()
                .write(true)
                .create(true)
                .open(&header.file_path) {
                    Ok(f) => BufWriter::new(f),
                    Err(_) => {
                        //println!("Error opening file for extracting: {}", e);
                        failed_files.push(self.headers[i as usize].file_path.clone());
                        continue;
                    }
                };
                match self.write_file_from_archive(&mut out_file, &header, false) {
                    Ok(_) => {},
                    Err(_) => {
                        //println!("Error extracting file data: {}", e);
                        failed_files.push(PathBuf::from(&header.file_path));
                    }
                }
            }
        }
        failed_files
    }

    pub fn list_files(&mut self) {
        if self.headers_stale {
            self.get_headers();
        }
        if self.headers.len() == 0 {
            println!("No files in archive");
            return;
        }
        for index in 0..self.headers.len() {
            println!(
                "{}: {} {}", 
                index + 1,
                self.headers[index].file_path.display(),
                self.headers[index].chunk_count as u64 
                    * self.chunk_size as u64 
                    + self.headers[index].last_chunk_size as u64
            );
        }
    }

    // remove files by moving non-matched items to a new archive. Returns the new archive
    pub fn remove_files(&mut self, file_indices: &Vec<u32>) -> Result<(), &'static str>{
        if self.headers.len() == 0 {
            return Err("No files in archive");
        }

        let mut tmp_archive = unwrap_or_return_error!(
            FctArchive::create_new(&self.archive_path.with_extension("tmp"), self.chunk_size),
            "Error removing files: Could not create temporary archive"
        );
        self.seek_to_start();
        tmp_archive.seek_to_start();

        let mut index = 0;
        loop {
            let header = match FileParser::from_archive(&mut self.archive_file) {
                Ok(h) => h,
                Err(_) => break
            };

            if !file_indices.contains(&(index as u32)) {
                // write header to tmp archive
                unwrap_or_return_error!(
                    tmp_archive.archive_file.write(&header.generate_header().unwrap()),
                    "Could not write file header to the temporary archive"
                );
                // write file to tmp archive
                unwrap_or_return_error!(
                    self.write_file_from_archive(&mut tmp_archive.archive_file, &header, true),
                    "Could not write data to the temporary archive"
                );
            }
            else {
                println!("Removing file: {}", header.file_path.display());
                unwrap_or_return_error!(
                    self.seek_data(&header),
                    "Could not seek over file data in the original archive"
                );
            }
            index += 1;
        }
        std::fs::remove_file(self.archive_path.as_path()).expect("Error removing files: Could not remove old archive");
        std::fs::rename(
            tmp_archive.archive_path.as_path(),
            self.archive_path.as_path()
        ).expect("Error removing files: Could not rename temporary archive");
        
        // replace old self
        self.archive_path = tmp_archive.archive_path;
        self.archive_file = tmp_archive.archive_file;
        self.headers = tmp_archive.headers;
        self.headers_stale = true;
        Ok(())
    }
}