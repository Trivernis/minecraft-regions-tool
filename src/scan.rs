use std::fmt::{Display, Formatter, Result};
use std::ops::Add;

#[derive(Clone, Debug)]
pub struct ScanStatistics {
    pub total_chunks: u64,
    pub invalid_length: u64,
    pub invalid_compression_method: u64,
    pub missing_nbt: u64,
    pub corrupted_nbt: u64,
    pub failed_to_read: u64,
    pub corrupted_compression: u64,
    pub invalid_chunk_pointer: u64,
    pub shrunk_size: u64,
    pub unused_space: u64,
}

impl ScanStatistics {
    pub fn new() -> Self {
        Self {
            total_chunks: 0,
            invalid_length: 0,
            invalid_compression_method: 0,
            missing_nbt: 0,
            corrupted_nbt: 0,
            corrupted_compression: 0,
            invalid_chunk_pointer: 0,
            failed_to_read: 0,
            shrunk_size: 0,
            unused_space: 0,
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
        self.invalid_chunk_pointer += rhs.invalid_chunk_pointer;
        self.corrupted_nbt += rhs.corrupted_nbt;
        self.unused_space += rhs.unused_space;

        self
    }
}

impl Display for ScanStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "
            Total Chunks: {}
            Failed to Read: {}
            Invalid chunk pointers: {}
            Chunks with invalid length: {}
            Chunks with invalid compression method: {}
            Chunks with missing nbt data: {}
            Chunks with corrupted nbt data: {}
            Chunks with corrupted compressed data: {}
            Unused space: {} KiB",
            self.total_chunks,
            self.failed_to_read,
            self.invalid_chunk_pointer,
            self.invalid_length,
            self.invalid_compression_method,
            self.missing_nbt,
            self.corrupted_nbt,
            self.corrupted_compression,
            self.unused_space / 1024,
        )
    }
}

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub fix: bool,
    pub fix_delete: bool,
}

impl ScanOptions {
    pub fn new() -> Self {
        ScanOptions {
            fix: false,
            fix_delete: false,
        }
    }

    pub fn fix(mut self, fix: bool) -> Self {
        self.fix = fix;

        self
    }

    pub fn fix_delete(mut self, fix_delete: bool) -> Self {
        self.fix_delete = fix_delete;

        self
    }
}
