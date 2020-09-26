use crate::scan::ScanStatistics;
use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Result, Seek, SeekFrom, Write};

const BLOCK_SIZE: usize = 4096;

pub struct RegionFile {
    reader: BufReader<File>,
    locations: Locations,
    #[allow(dead_code)]
    timestamps: Timestamps,
}

impl RegionFile {
    pub fn new(reader: BufReader<File>) -> Result<Self> {
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

    /// Writes a corrected version of the region file back to the disk
    pub fn write(&self, writer: &mut BufWriter<File>) -> Result<()> {
        let location_bytes = self.locations.to_bytes();
        writer.write_all(&location_bytes.as_slice())?;

        writer.flush()
    }

    /// Returns the number of chunks in the file
    pub fn count_chunks(&self) -> usize {
        return self.locations.valid_entries().len();
    }

    /// Scans the chunk entries for possible errors
    pub fn scan_chunks(&mut self) -> Result<ScanStatistics> {
        let mut statistic = ScanStatistics::new();

        let entries = self.locations.valid_entries();
        let mut corrected_entries = Vec::new();
        statistic.total_chunks = entries.len() as u64;

        for (offset, sections) in &entries {
            self.reader
                .seek(SeekFrom::Start(*offset as u64 * BLOCK_SIZE as u64))?;
            match self.read_chunk() {
                Ok(chunk) => {
                    let chunk_sections = ((chunk.length + 4) as f64 / BLOCK_SIZE as f64).ceil();
                    if *sections != chunk_sections as u8 || chunk.length >= 1_048_576 {
                        statistic.invalid_length += 1;
                        corrected_entries.push((*offset, chunk_sections as u8));
                    } else {
                        corrected_entries.push((*offset, *sections));
                    }
                }
                Err(e) => {
                    println!("Failed to read chunk at {}: {}", offset, e);
                }
            }
        }
        self.locations.set_entries(corrected_entries);

        Ok(statistic)
    }

    /// Reads a chunk at the current location
    fn read_chunk(&mut self) -> Result<Chunk> {
        let mut length_raw = [0u8; 4];
        self.reader.read_exact(&mut length_raw)?;
        let length = BigEndian::read_u32(&length_raw);
        let compression_type = self.reader.read_u8()?;

        if length > 0 {
            self.reader.seek(SeekFrom::Current((length - 1) as i64))?;
        } else {
            self.reader.seek(SeekFrom::Current((length) as i64))?;
        }

        Ok(Chunk {
            length,
            compression_type,
        })
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
            let offset_raw = [0u8, bytes[i], bytes[i + 1], bytes[i + 2]];
            let offset = BigEndian::read_u32(&offset_raw);
            let count = bytes[i + 3];
            locations.push((offset, count));
        }

        Self { inner: locations }
    }

    /// Returns the byte representation of the locations table
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for (offset, sections) in &self.inner {
            let mut offset_raw = [0u8; 4];
            BigEndian::write_u32(&mut offset_raw, *offset);
            bytes.append(&mut offset_raw[1..4].to_vec());
            bytes.push(*sections);
        }

        bytes
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

    /// Returns chunk entry list
    pub fn valid_entries(&self) -> Vec<(u32, u8)> {
        self.inner
            .iter()
            .filter_map(|e| if (*e).0 >= 2 { Some(*e) } else { None })
            .collect()
    }

    /// Replaces the entry list with a new one
    pub fn set_entries(&mut self, entries: Vec<(u32, u8)>) {
        self.inner = entries;
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

#[derive(Debug)]
pub struct Chunk {
    pub length: u32,
    pub compression_type: u8,
}
