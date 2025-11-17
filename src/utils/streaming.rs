//! Streaming utilities for large files
//!
//! This module provides utilities for processing large files efficiently without
//! loading them entirely into memory. The main component is `LimitedReader`, which
//! enforces memory limits to prevent out-of-memory errors.
//!
//! # Examples
//!
//! ```no_run
//! use omniparse::utils::streaming::{LimitedReader, read_with_limit};
//! use std::fs::File;
//! use std::io::Read;
//!
//! // Using LimitedReader directly
//! let file = File::open("large_file.txt")?;
//! let mut limited = LimitedReader::new(file, 10 * 1024 * 1024); // 10 MB limit
//!
//! let mut buffer = Vec::new();
//! limited.read_to_end(&mut buffer)?;
//! println!("Read {} bytes", limited.bytes_read());
//!
//! // Using the convenience function
//! let file = File::open("another_file.txt")?;
//! let data = read_with_limit(file, 5 * 1024 * 1024)?; // 5 MB limit
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io::{self, Read, BufReader};

/// Default memory limit for streaming operations (100 MB)
pub const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024;

/// Default buffer size for streaming operations (64 KB)
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

/// A reader that enforces memory limits while reading
///
/// This reader wraps another reader and tracks the number of bytes read.
/// If the read count exceeds the specified limit, further reads will fail
/// with an error. This prevents out-of-memory errors when processing
/// unexpectedly large files.
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::LimitedReader;
/// use std::io::{Cursor, Read};
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
/// let mut limited = LimitedReader::new(cursor, 100);
///
/// let mut buffer = Vec::new();
/// limited.read_to_end(&mut buffer).unwrap();
///
/// assert_eq!(buffer, data);
/// assert_eq!(limited.bytes_read(), data.len());
/// ```
pub struct LimitedReader<R: Read> {
    inner: R,
    limit: usize,
    read: usize,
}

impl<R: Read> LimitedReader<R> {
    /// Create a new LimitedReader with the specified memory limit
    ///
    /// # Arguments
    ///
    /// * `inner` - The underlying reader
    /// * `limit` - Maximum number of bytes to read
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::utils::streaming::LimitedReader;
    /// use std::io::Cursor;
    ///
    /// let data = b"Hello";
    /// let cursor = Cursor::new(data);
    /// let limited = LimitedReader::new(cursor, 1024);
    ///
    /// assert_eq!(limited.limit(), 1024);
    /// ```
    pub fn new(inner: R, limit: usize) -> Self {
        Self {
            inner,
            limit,
            read: 0,
        }
    }

    /// Create a new LimitedReader with the default memory limit
    ///
    /// Uses `DEFAULT_MEMORY_LIMIT` (100 MB) as the limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::utils::streaming::{LimitedReader, DEFAULT_MEMORY_LIMIT};
    /// use std::io::Cursor;
    ///
    /// let data = b"Hello";
    /// let cursor = Cursor::new(data);
    /// let limited = LimitedReader::with_default_limit(cursor);
    ///
    /// assert_eq!(limited.limit(), DEFAULT_MEMORY_LIMIT);
    /// ```
    pub fn with_default_limit(inner: R) -> Self {
        Self::new(inner, DEFAULT_MEMORY_LIMIT)
    }

    /// Get the number of bytes read so far
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::utils::streaming::LimitedReader;
    /// use std::io::{Cursor, Read};
    ///
    /// let data = b"Hello";
    /// let cursor = Cursor::new(data);
    /// let mut limited = LimitedReader::new(cursor, 100);
    ///
    /// let mut buf = [0u8; 3];
    /// limited.read(&mut buf).unwrap();
    ///
    /// assert_eq!(limited.bytes_read(), 3);
    /// ```
    pub fn bytes_read(&self) -> usize {
        self.read
    }

    /// Get the memory limit
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::utils::streaming::LimitedReader;
    /// use std::io::Cursor;
    ///
    /// let cursor = Cursor::new(b"data");
    /// let limited = LimitedReader::new(cursor, 1024);
    ///
    /// assert_eq!(limited.limit(), 1024);
    /// ```
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Check if the limit has been reached
    ///
    /// # Examples
    ///
    /// ```
    /// use omniparse::utils::streaming::LimitedReader;
    /// use std::io::{Cursor, Read};
    ///
    /// let data = vec![0u8; 200];
    /// let cursor = Cursor::new(data);
    /// let mut limited = LimitedReader::new(cursor, 100);
    ///
    /// let mut buf = Vec::new();
    /// let _ = limited.read_to_end(&mut buf);
    ///
    /// assert!(limited.is_limit_reached());
    /// ```
    pub fn is_limit_reached(&self) -> bool {
        self.read >= self.limit
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Check if we've already hit the limit
        if self.is_limit_reached() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Memory limit of {} bytes exceeded", self.limit),
            ));
        }

        // Calculate how many bytes we can still read
        let remaining = self.limit - self.read;
        let max_read = buf.len().min(remaining);

        // Read up to the limit
        let n = self.inner.read(&mut buf[..max_read])?;
        self.read += n;

        Ok(n)
    }
}

/// Read all bytes from a reader with a memory limit
///
/// This is a convenience function that creates a `LimitedReader` and reads
/// all data into a vector.
///
/// # Arguments
///
/// * `reader` - The reader to read from
/// * `limit` - Maximum number of bytes to read
///
/// # Errors
///
/// Returns an error if the limit is exceeded or if reading fails.
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::read_with_limit;
/// use std::io::Cursor;
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
///
/// let result = read_with_limit(cursor, 100).unwrap();
/// assert_eq!(result, data);
/// ```
pub fn read_with_limit<R: Read>(reader: R, limit: usize) -> io::Result<Vec<u8>> {
    let mut limited = LimitedReader::new(reader, limit);
    let mut buffer = Vec::new();
    limited.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Read all bytes from a reader with the default memory limit
///
/// Uses `DEFAULT_MEMORY_LIMIT` (100 MB) as the limit.
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::read_with_default_limit;
/// use std::io::Cursor;
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
///
/// let result = read_with_default_limit(cursor).unwrap();
/// assert_eq!(result, data);
/// ```
pub fn read_with_default_limit<R: Read>(reader: R) -> io::Result<Vec<u8>> {
    read_with_limit(reader, DEFAULT_MEMORY_LIMIT)
}

/// Create a buffered reader with the default buffer size
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::buffered_reader;
/// use std::io::Cursor;
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
/// let buffered = buffered_reader(cursor);
/// ```
pub fn buffered_reader<R: Read>(reader: R) -> BufReader<R> {
    BufReader::with_capacity(DEFAULT_BUFFER_SIZE, reader)
}

/// Read chunks from a reader and process them with a callback
///
/// This function reads data in chunks and calls the provided callback for each chunk.
/// This is useful for processing large files without loading them entirely into memory.
///
/// # Arguments
///
/// * `reader` - The reader to read from
/// * `chunk_size` - Size of each chunk in bytes
/// * `callback` - Function to call for each chunk
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::read_chunks;
/// use std::io::Cursor;
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
///
/// let mut collected = Vec::new();
/// read_chunks(cursor, 5, |chunk| {
///     collected.extend_from_slice(chunk);
///     Ok(())
/// }).unwrap();
///
/// assert_eq!(collected, data);
/// ```
pub fn read_chunks<R, F>(mut reader: R, chunk_size: usize, mut callback: F) -> io::Result<()>
where
    R: Read,
    F: FnMut(&[u8]) -> io::Result<()>,
{
    let mut buffer = vec![0u8; chunk_size];
    
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        
        callback(&buffer[..n])?;
    }
    
    Ok(())
}

/// Read chunks from a reader with a memory limit
///
/// This combines chunked reading with memory limits for safe processing
/// of large files.
///
/// # Arguments
///
/// * `reader` - The reader to read from
/// * `chunk_size` - Size of each chunk in bytes
/// * `limit` - Maximum total bytes to read
/// * `callback` - Function to call for each chunk
///
/// # Examples
///
/// ```
/// use omniparse::utils::streaming::read_chunks_limited;
/// use std::io::Cursor;
///
/// let data = b"Hello, World!";
/// let cursor = Cursor::new(data);
///
/// let mut collected = Vec::new();
/// read_chunks_limited(cursor, 5, 100, |chunk| {
///     collected.extend_from_slice(chunk);
///     Ok(())
/// }).unwrap();
///
/// assert_eq!(collected, data);
/// ```
pub fn read_chunks_limited<R, F>(
    reader: R,
    chunk_size: usize,
    limit: usize,
    callback: F,
) -> io::Result<()>
where
    R: Read,
    F: FnMut(&[u8]) -> io::Result<()>,
{
    let limited = LimitedReader::new(reader, limit);
    read_chunks(limited, chunk_size, callback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_limited_reader_within_limit() {
        let data = b"Hello, World!";
        let cursor = Cursor::new(data);
        let mut limited = LimitedReader::new(cursor, 100);
        
        let mut buffer = Vec::new();
        limited.read_to_end(&mut buffer).unwrap();
        
        assert_eq!(buffer, data);
        assert_eq!(limited.bytes_read(), data.len());
        assert!(!limited.is_limit_reached());
    }

    #[test]
    fn test_limited_reader_exceeds_limit() {
        let data = vec![0u8; 1000];
        let cursor = Cursor::new(data);
        let mut limited = LimitedReader::new(cursor, 100);
        
        let mut buffer = Vec::new();
        let result = limited.read_to_end(&mut buffer);
        
        assert!(result.is_err());
        assert_eq!(limited.bytes_read(), 100);
        assert!(limited.is_limit_reached());
    }

    #[test]
    fn test_read_with_limit() {
        let data = b"Hello, World!";
        let cursor = Cursor::new(data);
        
        let result = read_with_limit(cursor, 100).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_read_chunks() {
        let data = b"Hello, World!";
        let cursor = Cursor::new(data);
        
        let mut collected = Vec::new();
        read_chunks(cursor, 5, |chunk| {
            collected.extend_from_slice(chunk);
            Ok(())
        }).unwrap();
        
        assert_eq!(collected, data);
    }
}
