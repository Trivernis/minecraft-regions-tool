use crate::chunk::{Chunk, ChunkScanError};
use crate::scan::ScanOptions;
use crate::scan::ScanStatistics;
use byteorder::{BigEndian, ByteOrder, WriteBytesExt};
use std::cmp::Ordering;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Result, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Arc;

pub const BLOCK_SIZE: usize = 4096;

pub struct RegionFile {
    reader: BufReader<File>,
    writer: BufWriter<File>,
    locations: Locations,
    #[allow(dead_code)]
    timestamps: Timestamps,
}

impl RegionFile {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let fr = OpenOptions::new().read(true).open(path)?;
        let fw = OpenOptions::new().write(true).open(path)?;
        let mut reader = BufReader::with_capacity(BLOCK_SIZE, fr);
        let writer = BufWriter::with_capacity(2 * BLOCK_SIZE, fw);

        let mut locations_raw = [0u8; BLOCK_SIZE];
        let mut timestamps_raw = [0u8; BLOCK_SIZE];
        reader.read_exact(&mut locations_raw)?;
        reader.read_exact(&mut timestamps_raw)?;

        Ok(Self {
            locations: Locations::from_bytes(&locations_raw),
            timestamps: Timestamps::from_bytes(&timestamps_raw),
            reader,
            writer,
        })
    }

    /// Returns the number of chunks in the file
    pub fn count_chunks(&self) -> usize {
        return self.locations.valid_entries_enumerate().len();
    }

    /// Scans the chunk entries for possible errors
    pub fn scan_chunks(&mut self, options: &Arc<ScanOptions>) -> Result<ScanStatistics> {
        let mut statistic = ScanStatistics::new();

        let mut entries = self.locations.valid_entries_enumerate();
        entries.sort_by(|(_, (a, _)), (_, (b, _))| {
            if a > b {
                Ordering::Greater
            } else if a < b {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });
        statistic.total_chunks = entries.len() as u64;

        for (index, (offset, sections)) in entries {
            let reader_offset = offset as u64 * BLOCK_SIZE as u64;
            self.reader.seek(SeekFrom::Start(reader_offset))?;

            match Chunk::from_buf_reader(&mut self.reader) {
                Ok(chunk) => {
                    self.scan_chunk(index, offset, sections, chunk, &mut statistic, options)?;
                }
                Err(e) => {
                    statistic.failed_to_read += 1;
                    log::error!("Failed to read chunk at {}: {}", offset, e);
                }
            }
        }

        if options.fix || options.fix_delete {
            self.writer.seek(SeekFrom::Start(0))?;
            self.writer
                .write_all(self.locations.to_bytes().as_slice())?;
            self.writer.flush()?;
        }

        Ok(statistic)
    }

    /// Scans a single chunk for errors
    fn scan_chunk(
        &mut self,
        index: usize,
        offset: u32,
        sections: u8,
        mut chunk: Chunk,
        statistic: &mut ScanStatistics,
        options: &Arc<ScanOptions>,
    ) -> Result<()> {
        let chunk_sections = ((chunk.length + 4) as f64 / BLOCK_SIZE as f64).ceil();
        let reader_offset = offset as u64 * BLOCK_SIZE as u64;

        if chunk.compression_type > 3 {
            statistic.invalid_compression_method += 1;
            if options.fix {
                self.writer.seek(SeekFrom::Start(reader_offset + 4))?;
                self.writer.write_u8(1)?;
            }
        } else {
            self.reader.seek(SeekFrom::Start(reader_offset + 5))?;
            if let Err(e) = chunk.validate_nbt_data(&mut self.reader) {
                match e {
                    ChunkScanError::IO(e) => {
                        log::debug!("Compression error at chunk {}: {}", offset, e);
                        statistic.corrupted_compression += 1;
                    }
                    ChunkScanError::NBTError(e) => {
                        log::debug!("Corrupted nbt data for chunk {}: {}", offset, e);
                        statistic.corrupted_nbt += 1;
                    }
                    _ => {
                        log::debug!("Missing nbt data for chunk {}: {}", offset, e);
                        statistic.missing_nbt += 1;
                    }
                }
                self.delete_chunk(index)?;
            }
        }

        if sections != chunk_sections as u8 || chunk.length >= 1_048_576 {
            statistic.invalid_length += 1;
            self.locations
                .replace_entry_unchecked(index, (offset, chunk_sections as u8));
        }

        Ok(())
    }

    /// Deletes a chunk and shifts all other chunks
    pub fn delete_chunk(&mut self, index: usize) -> Result<()> {
        let (offset, sections) = self.locations.get_chunk_entry_unchecked(index);
        self.reader.seek(SeekFrom::Start(
            (offset as usize * BLOCK_SIZE + sections as usize * BLOCK_SIZE) as u64,
        ))?;
        self.writer
            .seek(SeekFrom::Start((offset as usize * BLOCK_SIZE) as u64))?;
        log::debug!(
            "Shifting chunk entries starting from {} by {} to the left",
            offset,
            sections as u32
        );
        loop {
            let mut buf = [0u8; BLOCK_SIZE];
            let read = self.reader.read(&mut buf)?;
            self.writer.write(&buf)?;
            if read < BLOCK_SIZE {
                break;
            }
        }
        self.locations.delete_chunk_entry_unchecked(index);
        self.locations.shift_entries(offset, -(sections as i32));

        Ok(())
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
    pub fn valid_entries_enumerate(&self) -> Vec<(usize, (u32, u8))> {
        self.inner
            .iter()
            .enumerate()
            .filter_map(|e| {
                if (*e.1).0 >= 2 {
                    Some((e.0, *e.1))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Replaces an entry with a new one. Panics if the index doesn't exist
    pub fn replace_entry_unchecked(&mut self, index: usize, entry: (u32, u8)) {
        self.inner[index] = entry;
    }

    /// Returns a chunk entry for an index. Panics if it doesn't exist.
    pub fn get_chunk_entry_unchecked(&self, index: usize) -> (u32, u8) {
        self.inner[index]
    }

    /// Sets a chunk entry to not generated. Panics if the index doesn't exists
    pub fn delete_chunk_entry_unchecked(&mut self, index: usize) {
        self.inner[index] = (0, 0);
    }

    /// Shifts all entries starting from `start_index` by `amount`
    pub fn shift_entries(&mut self, start_offset: u32, amount: i32) {
        log::debug!(
            "Shifting location entries starting from {} by {}",
            start_offset,
            amount
        );
        self.inner = self
            .inner
            .iter()
            .map(|e| {
                let mut entry = *e;

                if e.0 >= start_offset {
                    entry.0 = (entry.0 as i32 + amount) as u32;
                }

                entry
            })
            .collect();
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
