use std::fmt::{Display, Formatter, Result};
use std::ops::Add;

#[derive(Clone, Debug)]
pub struct ScanStatistics {
    pub total_chunks: u64,
    pub invalid_length: u64,
    pub invalid_compression_method: u64,
}

impl ScanStatistics {
    pub fn new() -> Self {
        Self {
            total_chunks: 0,
            invalid_length: 0,
            invalid_compression_method: 0,
        }
    }
}

impl Add for ScanStatistics {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.invalid_length += rhs.invalid_length;
        self.total_chunks += rhs.total_chunks;
        self.invalid_compression_method += rhs.invalid_compression_method;

        self
    }
}

impl Display for ScanStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Total Chunks: {}\nChunks with invalid length: {}\nChunks with invalid compression method: {}",
            self.total_chunks, self.invalid_length, self.invalid_compression_method
        )
    }
}
