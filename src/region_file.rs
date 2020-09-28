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
    path: PathBuf,
    reader: BufReader<File>,
    writer: BufWriter<File>,
    locations: Locations,
    #[allow(dead_code)]
    timestamps: Timestamps,
    length: u64,
}

impl RegionFile {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let fr = OpenOptions::new().read(true).open(path)?;
        let fw = OpenOptions::new().write(true).open(path)?;
        let file_size = fr.metadata()?.len();
        let mut reader = BufReader::with_capacity(BLOCK_SIZE, fr);
        let writer = BufWriter::with_capacity(2 * BLOCK_SIZE, fw);

        let mut locations_raw = [0u8; BLOCK_SIZE];
        let mut timestamps_raw = [0u8; BLOCK_SIZE];
        reader.read_exact(&mut locations_raw)?;
        reader.read_exact(&mut timestamps_raw)?;

        Ok(Self {
            path: path.clone(),
            locations: Locations::from_bytes(&locations_raw),
            timestamps: Timestamps::from_bytes(&timestamps_raw),
            reader,
            writer,
            length: file_size,
        })
    }

    /// Returns the number of chunks in the file
    pub fn count_chunks(&self) -> usize {
        return self.locations.valid_entries_enumerate().len();
    }

    /// Scans the chunk entries for possible errors
    pub fn scan_chunks(&mut self, options: &Arc<ScanOptions>) -> Result<ScanStatistics> {
        let mut statistic = ScanStatistics::new();
        let mut shift_operations: Vec<(usize, isize)> = Vec::new();

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
        let mut previous_offset = 2;
        let mut previous_sections = 0;

        for (index, (offset, sections)) in entries {
            // Calculate and seek to the start of the chunk
            let reader_offset = offset as u64 * BLOCK_SIZE as u64;
            self.reader.seek(SeekFrom::Start(reader_offset))?;

            let offset_diff = offset as i32 - (previous_offset as i32 + previous_sections as i32);
            // Check if there is wasted space between the chunks
            // since the chunks are iterated ordered by offset the previous chunk is the closest
            if offset_diff > 0 {
                statistic.unused_space += (BLOCK_SIZE * offset_diff as usize) as u64;
                log::debug!(
                    "Gap of unused {:.2} KiB detected between {} and {}",
                    (BLOCK_SIZE as f32 * offset_diff as f32) / 1024.0,
                    previous_offset,
                    offset
                );
                if options.fix {
                    shift_operations.push((offset as usize, -(offset_diff as isize)));
                }
            }
            // Check if the chunk is longer than the file
            if offset < 2 || self.length < (offset + sections as u32) as u64 * BLOCK_SIZE as u64 {
                statistic.invalid_chunk_pointer += 1;
                log::debug!(
                    "Invalid chunk offset and sections at index {}: {} + {}",
                    index,
                    offset,
                    sections
                );
                if options.fix_delete {
                    self.delete_chunk(index)?;
                }
                continue;
            }
            match Chunk::from_buf_reader(&mut self.reader) {
                Ok(chunk) => {
                    let exists =
                        self.scan_chunk(index, offset, sections, chunk, &mut statistic, options)?;
                    // If scan_chunk returns false the chunk entry was deleted
                    if !exists && options.fix {
                        shift_operations
                            .push((offset as usize + sections as usize, -(sections as isize)))
                    }
                }
                Err(e) => {
                    statistic.failed_to_read += 1;
                    log::error!(
                        "Failed to read chunk at {} in {:?}: {}",
                        offset,
                        self.path,
                        e
                    );
                    if options.fix_delete {
                        self.delete_chunk(index)?;
                        shift_operations
                            .push((offset as usize + sections as usize, -(sections as isize)));
                    }
                }
            }

            previous_offset = offset;
            previous_sections = sections as u32;
        }

        if options.fix || options.fix_delete {
            self.perform_shift_operations(shift_operations)?;

            // The new size of the file is the estimated size based on the highest chunk offset + sections
            statistic.shrunk_size = self.locations.estimated_size();
            self.writer.seek(SeekFrom::Start(0))?;
            self.writer
                .write_all(self.locations.to_bytes().as_slice())?;
            self.writer.flush()?;
        }

        Ok(statistic)
    }

    /// Performs shift operations defined in the shift_operations vector
    fn perform_shift_operations(
        &mut self,
        mut shift_operations: Vec<(usize, isize)>,
    ) -> Result<()> {
        // sort the shift operations by resulting offset to have them in the right order
        shift_operations.sort_by(|(o1, a1), (o2, a2)| {
            let to_offset1 = *o1 as isize + *a1;
            let to_offset2 = *o2 as isize + *a2;
            if to_offset1 > to_offset1 {
                Ordering::Greater
            } else if to_offset1 < to_offset2 {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });
        let mut shifted = 0isize;

        // perform shifting of chunks to close gaps between them
        let mut operations = shift_operations.iter().peekable();

        while let Some((offset, amount)) = operations.next() {
            shifted += *amount;
            let end_offset = if let Some((o, a)) = operations.peek() {
                (*o as isize + *a) as usize
            } else {
                self.locations.max_offset() as usize
            };
            if *offset > end_offset {
                log::error!("Invalid shift ({} - {}) -> {}", offset, end_offset, shifted);
                break;
            }
            self.shift_right(*offset, end_offset, shifted)?;
            self.locations
                .shift_entries(*offset as u32, end_offset as u32, shifted as i32);
        }

        Ok(())
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
    ) -> Result<bool> {
        let chunk_sections = ((chunk.length + 4) as f64 / BLOCK_SIZE as f64).ceil();
        let reader_offset = offset as u64 * BLOCK_SIZE as u64;

        // Valid compression types are:
        // 0 - uncompressed
        // 1 - GZIP
        // 2 - ZLIB
        if chunk.compression_type > 3 {
            statistic.invalid_compression_method += 1;
            if options.fix {
                self.writer.seek(SeekFrom::Start(reader_offset + 4))?;
                self.writer.write_u8(1)?;
            }
        } else {
            // seek to the start of the actual chunk data
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
                if options.fix_delete {
                    self.delete_chunk(index)?;
                    return Ok(false);
                }
            } else {
                // validate that the chunk is the one the index should be pointing at
                if let Some(x) = chunk.x_pos {
                    if let Some(z) = chunk.z_pos {
                        if get_chunk_index(x as isize, z as isize) != index {
                            statistic.invalid_chunk_pointer += 1;
                            log::debug!("Pointer {} pointing to wrong chunk ({},{})", index, x, z);

                            if options.fix_delete {
                                // Delete the entry of the chunk from the locations table
                                self.delete_chunk(index)?;
                            }
                        }
                    }
                }
            }
        }

        if sections != chunk_sections as u8 || chunk.length >= 1_048_576 {
            statistic.invalid_length += 1;
            if options.fix {
                self.locations
                    .replace_entry_unchecked(index, (offset, chunk_sections as u8));
            }
        }

        Ok(true)
    }

    /// Deletes a chunk and shifts all other chunks
    pub fn delete_chunk(&mut self, index: usize) -> Result<()> {
        log::debug!(
            "Deleting chunk at {}",
            self.locations.get_chunk_entry_unchecked(index).0
        );
        self.locations.delete_chunk_entry_unchecked(index);
        Ok(())
    }

    /// Shifts the file from the `offset` position `amount` blocks to the right
    pub fn shift_right(
        &mut self,
        start_offset: usize,
        end_offset: usize,
        amount: isize,
    ) -> Result<()> {
        log::debug!(
            "Shifting chunk blocks starting from {} by {} until {}",
            start_offset,
            amount,
            end_offset,
        );
        // seek to the start of the data to be shifted
        self.reader
            .seek(SeekFrom::Start((start_offset * BLOCK_SIZE) as u64))?;
        // seek to the start of the data to be shifted
        self.writer
            .seek(SeekFrom::Start((start_offset * BLOCK_SIZE) as u64))?;
        // seek the amount the data should be shifted
        self.writer
            .seek(SeekFrom::Current(amount as i64 * BLOCK_SIZE as i64))?;

        for _ in 0..(end_offset - start_offset) {
            // since the offset is based on the fixed BLOCK_SIZE we can use that as our buffer size
            let mut buf = [0u8; BLOCK_SIZE];
            let read = self.reader.read(&mut buf)?;
            self.writer.write(&buf)?;

            if read < BLOCK_SIZE {
                break;
            }
        }

        Ok(())
    }

    /// Closes the region file by flushing the writer
    pub fn close(&mut self) -> Result<()> {
        self.writer.flush()
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
            // construct a 4-byte number from 3 bytes
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
    pub fn get_chunk_offset(&self, x: isize, z: isize) -> Option<u32> {
        self.inner.get(get_chunk_index(x, z)).map(|e| (*e).0)
    }

    /// Returns the number of sectors for a chunk
    pub fn get_chunk_sectors(&self, x: isize, z: isize) -> Option<u8> {
        self.inner.get(get_chunk_index(x, z)).map(|e| (*e).1)
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

    /// The maximum offset in the file
    pub fn max_offset(&self) -> u32 {
        let largest = self
            .inner
            .iter()
            .max_by(|(a, _), (b, _)| {
                if a > b {
                    Ordering::Greater
                } else if a < b {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .cloned()
            .unwrap_or((2, 0));

        largest.0 + largest.1 as u32
    }

    /// Returns the estimated of all chunks combined including the header
    pub fn estimated_size(&self) -> u64 {
        self.max_offset() as u64 * BLOCK_SIZE as u64
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
    pub fn shift_entries(&mut self, start_offset: u32, end_offset: u32, amount: i32) {
        log::debug!(
            "Shifting location entries starting from {} by {} until {}",
            start_offset,
            amount,
            end_offset
        );
        self.inner = self
            .inner
            .iter()
            .map(|e| {
                let mut entry = *e;

                if e.0 >= start_offset && e.0 <= end_offset {
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

#[inline]
fn get_chunk_index(x: isize, z: isize) -> usize {
    let mut x = x % 32;
    let mut z = z % 32;
    if x < 0 {
        x += 32;
    }
    if z < 0 {
        z += 32;
    }

    x as usize + z as usize * 32
}
