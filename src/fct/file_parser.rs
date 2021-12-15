use std::fs::{self};
use std::path::PathBuf;
use std::io::Read;
use crate::fct::fs_operations;

#[derive(Default, Debug, Clone)]
pub struct FileParser {
    pub file_path: PathBuf,
    pub chunk_count: u32,
    pub last_chunk_size: u16
}

impl FileParser {
    pub fn from_file(file_path: &PathBuf, root_dir: &PathBuf, chunk_size: u16) -> Result<Self, &'static str> {
        match fs::metadata(&file_path){
            Ok(file_info) => {
                let mut parser = FileParser::default();
                if file_info.permissions().readonly() {
                    return Err("File is readonly");
                }
                parser.file_path = match fs_operations::format_path(root_dir, file_path) {
                    Ok(path) => path,
                    Err(_) => return Err("Could not format path")
                };
                // calculate chunk count and last chunk size
                let chunk_count_result = u32::try_from(file_info.len() / chunk_size as u64);

                let last_chunk_size_result =  u16::try_from(file_info.len() % chunk_size as u64);

                match chunk_count_result {
                    Ok(chunk_count) => parser.chunk_count = chunk_count,
                    Err(_) => return Err("File size is too big")
                }

                match last_chunk_size_result {
                    Ok(value) => parser.last_chunk_size = value,
                    Err(_) => return Err("Final Chunk is too big")
                }
                return Ok(parser);
            }
            Err(_) => return Err("File not found")
        }
    }

    pub fn from_archive<R: Read>(file: &mut R) -> Result<Self, &'static str> {
        const PROPERTY_FIELD_LEN: usize = 8;
        let mut parser = FileParser::default();
        let mut buffer = [0u8; PROPERTY_FIELD_LEN];

        let bytes_read = file.read(&mut buffer).expect("Failed to read file header data");
        if bytes_read == 0 {
            return Err("File is empty or EOF reached");
        }
        // with this check, t
        if bytes_read != PROPERTY_FIELD_LEN {
            return Err("File header is incomplete");
        }
        parser.chunk_count = u32::from_le_bytes(buffer[..4].try_into().unwrap());
        parser.last_chunk_size = u16::from_le_bytes(buffer[4..6].try_into().unwrap());
        let file_path_len = u16::from_le_bytes(buffer[6..8].try_into().unwrap()) as usize;
        
        // read file_path_len amount of bytes
        let mut file_path_buffer = vec![0u8; file_path_len];
        file.read_exact(&mut file_path_buffer).expect("Failed to read file path");
        parser.file_path = PathBuf::from(String::from_utf8(file_path_buffer).expect("Failed to convert file path to string"));

        //println!("{:?}", parser);

        return Ok(parser);
    }

    pub fn generate_header(&self) -> Result<Vec<u8>, &'static str> {
        let mut header = Vec::new();
        header.extend_from_slice(&self.chunk_count.to_le_bytes());
        header.extend_from_slice(&self.last_chunk_size.to_le_bytes());

        let file_path_bytes = self.file_path.to_str().unwrap().as_bytes();
        match u16::try_from(file_path_bytes.len()) {
            Ok(value) => header.extend_from_slice(&value.to_le_bytes()),
            Err(_) => return Err("File path is too big")
        }
        header.extend_from_slice(&file_path_bytes);
        return Ok(header);
    }

    pub fn get_header_size(&self) -> usize {
        return 8 + self.file_path.to_str().unwrap().as_bytes().len();
    }
}

impl std::fmt::Display for FileParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "File Name: \"{}\"; Chunk Count: {}; Last Chunk Size: {}", self.file_path.display(), self.chunk_count, self.last_chunk_size);
    }
}