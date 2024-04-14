//! # File Operations in Rust
//!
//! This module provides a set of utility functions for file operations.
//!
//! For a detailed explanation of the functions and their usage, see
//! [the detailed explanation](../explanations/file.md).
//!
//! ...
//! (rest of your code)
//! ...



use std::io::{BufRead,Read, Seek, SeekFrom, Write};
use std::fs::File;
use std::io;
use std::os::fd::{FromRawFd, RawFd};
use std::path::Path;
use memmap::{Mmap, MmapOptions};




pub fn read_all(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    let mut file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    if metadata.len() == 0 || buf.is_empty() {
        println!("File is empty or buffer is empty");
        return Ok(0); // Return early if the file is empty or the buffer is empty
    }
    file.seek(SeekFrom::Start(0))?; // Add this line to seek to the start of the file
    let mut pos = 0;

    while pos < buf.len() {
        match file.read(&mut buf[pos..]) {
            Ok(0) => break, // EOF
            Ok(n) => pos += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
            Err(e) => return Err(e),
        }
    }

    Ok(pos)
}


pub fn write_all(mut file: &File, buf: &[u8]) -> io::Result<usize> {
    file.write_all(buf)?;
    Ok(buf.len())
}


pub unsafe fn mmap_file(filename: &Path) -> Result<(Mmap, usize), std::io::Error> {
    let file = File::open(filename)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;

    if file_size > 0 {
        let mmap = MmapOptions::new().map(&file)?;
        Ok((mmap, file_size))
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "File is empty or does not exist."))
    }
}

pub fn buffer_for_each_line<F>(buf: &[u8], mut cb: F)
where
    F: FnMut(&str) -> bool,
{
    let mut pos = 0;
    let size = buf.len();

    while pos < size {
        let mut end = pos;
        while end < size && buf[end] != b'\n' {
            end += 1;
        }
        let mut len = end - pos;
        if end > pos && buf[end - 1] == b'\r' {
            len -= 1;
        }

        let line = &buf[pos..pos + len];
        pos = end + 1;

        if(cb(std::str::from_utf8(line).unwrap())) {
            break;
        }

    }

}

pub fn buffer_for_each_line_reverse<F>(buf: &[u8], mut cb: F)
    where
        F: FnMut(&str) -> bool,
{
    let mut end = buf.len();
    if end > 0 {
        end -= 1;
    }

    while end > 0 {
        let mut is_newline = false;
        if end > 1 && buf[end] == b'\n' && buf[end - 1] == b'\r' {
            end -= 2; // Exclude both '\n' and '\r'
            is_newline = true;
        } else if buf[end] == b'\n' {
            end -= 1; // Exclude '\n'
            is_newline = true;
        }

        let mut pos = end;
        while pos > 0 && buf[pos - 1] != b'\n' {
            pos -= 1;
        }

        let len = end - pos + 1;
        let line = &buf[pos..pos + len];
        if pos > 0 {
            end = pos - 1;
        } else {
            end = 0;
        }

        if cb(std::str::from_utf8(line).unwrap()) {
            break;
        }
    }
}

pub fn file_for_each_line<F>(filename: &str, mut cb: F) -> io::Result<()>
    where
        F: FnMut(&str) -> io::Result<()>,
{
    let file = File::open(&Path::new(filename))?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        cb(&line)?;
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{OpenOptions, remove_file};
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_read_all() {
        let mut file = OpenOptions::new().write(true).read(true).create(true).open("testfile.txt").unwrap();
        file.write_all(b"Hello, world!").unwrap();
        file.sync_all().unwrap(); // Ensure all writes are flushed to the file
        let fd = file.as_raw_fd();
        let mut buffer = [0u8; 13];
        let result = read_all(fd, &mut buffer);
        let bytes_read = result.unwrap();
        println!("Read {} bytes", bytes_read ); // Print the number of bytes read
        println!("Buffer contents: {:?}", &buffer); // Print the contents of the buffer
        assert_eq!(bytes_read, 13);
        assert_eq!(&buffer, b"Hello, world!");

        remove_file("testfile.txt").unwrap();
    }

    #[test]
    fn test_write_all() {
        let file = OpenOptions::new().write(true).read(true).create(true).open("testfile.txt").unwrap();
        let result = write_all(&file, b"Hello, world!");
        assert_eq!(result.unwrap(), 13);
    }

    #[test]
    fn test_mmap_file() {
        let result = unsafe { mmap_file(Path::new("testfile.txt")) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_buffer_for_each_line() {
        let buffer = b"Hello\nworld\n";
        let mut lines = Vec::new();
        buffer_for_each_line(buffer, |line| {
            lines.push(line.to_string());
            false
        });
        assert_eq!(lines, vec!["Hello", "world"]);
    }

    #[test]
    fn test_buffer_for_each_line_reverse() {
        let buffer = b"Hello\nworld\n";
        let mut lines = Vec::new();
        buffer_for_each_line_reverse(buffer, |line| {
            lines.push(line.to_string());
            false
        });
        assert_eq!(lines, vec!["world", "Hello"]);
    }

    #[test]
    fn test_file_for_each_line() {
        let result = file_for_each_line("testfile.txt", |line| {
            assert!(line.len() > 0);
            Ok(())
        });
        assert!(result.is_ok());
    }


    #[test]
    fn test_read_all_empty_file() {
        let path = Path::new("empty.txt");
        if !path.exists() {
            File::create(&path).expect("Failed to create file 'empty.txt'");
        }
        let file = File::open(&path).expect("Failed to open file 'empty.txt'");
        let fd = file.as_raw_fd();
        let mut buffer = [0u8; 10];
        let result = read_all(fd, &mut buffer);
        assert_eq!(result.unwrap(), 0);
        remove_file(path).unwrap()
    }

    #[test]
    fn test_write_all_empty_buffer() {
        let file = OpenOptions::new().write(true).read(true).create(true).open("testfile.txt").unwrap();
        let result = write_all(&file, b"");
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_mmap_file_non_existent() {
        let result = unsafe { mmap_file(Path::new("non_existent.txt")) };
        assert!(result.is_err());
    }

    #[test]
    fn test_buffer_for_each_line_empty() {
        let buffer = b"";
        let mut lines = Vec::new();
        buffer_for_each_line(buffer, |line| {
            lines.push(line.to_string());
            false
        });
        assert_eq!(lines, Vec::<String>::new());
    }

    #[test]
    fn test_buffer_for_each_line_reverse_empty() {
        let buffer = b"";
        let mut lines = Vec::new();
        buffer_for_each_line_reverse(buffer, |line| {
            lines.push(line.to_string());
            false
        });
        assert_eq!(lines, Vec::<String>::new());
    }

    #[test]
    fn test_file_for_each_line_non_existent() {
        let result = file_for_each_line("non_existent.txt", |line| {
            assert!(line.len() > 0);
            Ok(())
        });
        assert!(result.is_err());
    }
}