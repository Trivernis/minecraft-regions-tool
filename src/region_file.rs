use byteorder::{BigEndian, ByteOrder};
use std::io::{Read, Result};

const BLOCK_SIZE: usize = 4096;

pub struct RegionFile {
    reader: Box<dyn Read>,
    locations: Locations,
    timestamps: Timestamps,
}

impl RegionFile {
    pub fn new(reader: Box<dyn Read>) -> Result<Self> {
        let mut locations_raw = [0u8; BLOCK_SIZE];
        let mut timestamps_raw = [0u8; BLOCK_SIZE];
        let mut reader = reader;
        reader.read_exact(&mut locations_raw)?;
        reader.read_exact(&mut timestamps_raw)?;

        Ok(Self {
            locations: Locations::from_bytes(&locations_raw),
            timestamps: Timestamps::from_bytes(&timestamps_raw),
            reader,
        })
    }

    /// Returns the number of chunks in the file
    pub fn count_chunks(&self) -> usize {
        let mut count = 0;
        for x in 0..32 {
            for z in 0..32 {
                if !(self.locations.get_chunk_offset(x, z) == Some(0)
                    && self.locations.get_chunk_sectors(x, z) == Some(0))
                {
                    count += 1;
                }
            }
        }

        return count;
    }
}

#[derive(Debug)]
pub struct Locations {
    inner: Vec<(u32, u8)>,
}

impl Locations {
    pub fn from_bytes(bytes: &[u8; BLOCK_SIZE]) -> Self {
        let mut locations = Vec::new();

        for i in (0..BLOCK_SIZE - 1).step_by(4) {
            let mut offset = BigEndian::read_u32(&bytes[i..i + 4]);
            offset = offset >> 1;
            let count = bytes[i + 3];
            locations.push((offset, count));
        }

        Self { inner: locations }
    }

    /// Returns the offset of a chunk
    pub fn get_chunk_offset(&self, x: usize, z: usize) -> Option<u32> {
        let index = x % 32 + (z % 32) * 32;
        self.inner.get(index).map(|e| (*e).0)
    }

    /// Returns the number of sectors for a chunk
    pub fn get_chunk_sectors(&self, x: usize, z: usize) -> Option<u8> {
        let index = x % 32 + (z % 32) * 32;
        self.inner.get(index).map(|e| (*e).1)
    }
}

#[derive(Debug)]
pub struct Timestamps {
    inner: Vec<u32>,
}

impl Timestamps {
    pub fn from_bytes(bytes: &[u8; BLOCK_SIZE]) -> Self {
        let mut timestamps = Vec::new();

        for i in (0..BLOCK_SIZE - 1).step_by(4) {
            timestamps.push(BigEndian::read_u32(&bytes[i..i + 4]))
        }

        Self { inner: timestamps }
    }
}
