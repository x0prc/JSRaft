use memmap2::Mmap;
use std::fs::File;
use std::io;
use std::path::Path;

/// Read a UTF-8 source file using a read-only memory map.
pub fn read_text_mmap(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if !metadata.is_file() {
        return std::fs::read_to_string(path);
    }

    if metadata.len() == 0 {
        return Ok(String::new());
    }

    // The mapping is read-only and immediately copied into an owned String,
    // so callers do not hold a live view of the underlying file.
    let mmap = match unsafe { Mmap::map(&file) } {
        Ok(mmap) => mmap,
        Err(_) => return std::fs::read_to_string(path),
    };
    let source = std::str::from_utf8(&mmap)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    Ok(source.to_owned())
}
