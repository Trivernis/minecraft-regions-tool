use flate2::read::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Result};

#[derive(Clone, Debug)]
pub struct ByteArrayCache {
    inner: Vec<u8>,
    position: usize,
}

impl ByteArrayCache {
    /// Creates a new byte array cache
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            position: 0,
        }
    }

    /// Creates a new byte array cache with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    pub fn write<R: Read>(&mut self, reader: R) -> Result<()> {
        let mut encoder = ZlibEncoder::new(reader, Compression::default());
        let mut buffer = Vec::new();
        encoder.read_to_end(&mut buffer)?;
        self.inner.append(&mut buffer);

        Ok(())
    }
}

impl Read for ByteArrayCache {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read = (&self.inner[self.position..]).read(buf)?;
        self.position += read;

        Ok(read)
    }
}
