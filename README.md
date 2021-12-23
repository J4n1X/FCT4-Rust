# FCT File Container Version 4
This is an implementation of the FCT File Container Version 4 in Rust. 

## Specification

### Archive Header

The archive header contains the file magic "FCT" and the chunk size used throughout the archive, stored in 2 bytes.

### File Entry Header

The File Entry Header contains metadata about an archived file.

| Field            | Size (in bytes)  |
|------------------|------------------|
| Chunk Count      | 4                |
| Last Chunk Size  | 2                | 
| File Name Length | 2                | 
| File Name        | File Name Length |

### Storing of File Data

Files are stored directly after a File Entry Header and are aligned in size to the global chunk size.