# FCT File Container Version 4

A Rust implementation of the FCT archive format. Two crates:

- `libfct4_cli/` — library crate (`libfct4`), published to crates.io
- `fct_gui/` — CLI frontend (`fct4_cli`)

## CLI Usage

All subcommands have a single-letter alias.

### Create a new archive

```
fct4_cli create -n <chunk-size> <archive> <file|dir>...
fct4_cli c      -n <chunk-size> <archive> <file|dir>...
```

`chunk-size` is the global chunk size in bytes (1–65535). Directories are expanded recursively. Paths are stored relative to the current working directory.

### Append files to an existing archive

```
fct4_cli append <archive> <file|dir>...
fct4_cli a      <archive> <file|dir>...
```

### List archive contents

```
fct4_cli list <archive> [filter...]
fct4_cli l    <archive> [filter...]
```

Without filters, all entries are listed. Filters can be:

- A 1-based index: `1`, `3`
- A path component or sequence: `src`, `src/lib` — matches any stored path that contains those components

Output columns: index, stored path, uncompressed size.

### Extract files

```
fct4_cli extract <archive> <output-dir> [target...]
fct4_cli e       <archive> <output-dir> [target...]
```

Without targets, all entries are extracted. Targets follow the same rules as list filters. Directories within the archive are extracted with their full stored path structure under `output-dir`.

### Remove entries

```
fct4_cli remove <archive> <target>...
fct4_cli r      <archive> <target>...
```

Targets follow the same rules as list filters. Specifying a folder component removes all entries under that folder.

### Examples

```sh
# Create an archive with 4 KiB chunks from a source tree
fct4_cli create -n 4096 project.fct src/

# List everything
fct4_cli list project.fct

# List only files under 'tests/'
fct4_cli list project.fct tests

# Extract a single file by index
fct4_cli extract project.fct ./out 3

# Extract all files under 'src/lib' to ./out
fct4_cli extract project.fct ./out src/lib

# Remove entry 2 and all entries under 'docs/'
fct4_cli remove project.fct 2 docs
```

## Format Specification

All multi-byte integer fields are little-endian.

### Archive Header (5 bytes)

| Field      | Size (bytes) | Value                  |
|------------|--------------|------------------------|
| Magic      | 3            | ASCII `FCT`            |
| Chunk size | 2            | `u16`, range 1–65535   |

The chunk size is a global property of the archive. All file data is padded to a multiple of this size.

### File Entries

Each file entry follows immediately after the previous one, starting directly after the archive header. Entries are read sequentially until EOF.

**Entry header**

| Field           | Size (bytes)  | Notes                                       |
|-----------------|---------------|---------------------------------------------|
| Chunk count     | 4             | `u32` — number of full chunks               |
| Last chunk size | 2             | `u16` — bytes used in the final chunk; 0 means the file ends on a full chunk boundary |
| Path length     | 2             | `u16` — byte length of the path string      |
| Path            | path length   | UTF-8, no null terminator, `/` as separator |

**Entry data**

Follows the header immediately. The stored byte count is always a multiple of the chunk size:

```
stored_bytes = (chunk_count + (1 if last_chunk_size > 0 else 0)) * chunk_size
```

The actual file size is:

```
file_size = chunk_count * chunk_size + last_chunk_size
```

Any padding bytes at the end of the final chunk are zeros.

**Limits**

| Property          | Maximum                                     |
|-------------------|---------------------------------------------|
| Chunk size        | 65535 bytes                                 |
| File size         | `(2^32 - 1) * chunk_size + chunk_size` bytes (roughly 255 TiB at max chunk size) |
| Path length       | 65535 bytes                                 |
| Entries per archive | unbounded (sequential scan)              |