use std::fmt::{Display, Formatter, Result};
use std::ops::Add;

#[derive(Clone, Debug)]
pub struct ScanStatistics {
    pub total_chunks: u64,
    pub invalid_length: u64,
    pub invalid_compression_method: u64,
    pub missing_nbt: u64,
    pub failed_to_read: u64,
    pub corrupted_compression: u64,
}

impl ScanStatistics {
    pub fn new() -> Self {
        Self {
            total_chunks: 0,
            invalid_length: 0,
            invalid_compression_method: 0,
            missing_nbt: 0,
            corrupted_compression: 0,
            failed_to_read: 0,
        }
    }
}

impl Add for ScanStatistics {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.invalid_length += rhs.invalid_length;
        self.total_chunks += rhs.total_chunks;
        self.invalid_compression_method += rhs.invalid_compression_method;
        self.failed_to_read += rhs.failed_to_read;
        self.missing_nbt += rhs.missing_nbt;
        self.corrupted_compression += rhs.corrupted_compression;

        self
    }
}

impl Display for ScanStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Total Chunks: {}
            Failed to Read: {}
            Chunks with invalid length: {}
            Chunks with invalid compression method: {}
            Chunks with missing nbt data: {}
            Chunks with corrupted compressed data {}",
            self.total_chunks,
            self.failed_to_read,
            self.invalid_length,
            self.invalid_compression_method,
            self.missing_nbt,
            self.corrupted_compression
        )
    }
}
