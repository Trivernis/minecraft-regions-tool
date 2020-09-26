use crate::nbt::{NBTError, NBTReader, NBTValue};
use crate::region_file::BLOCK_SIZE;
use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

use flate2::bufread::ZlibDecoder;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{self, BufReader, Error, Read, Seek, SeekFrom};

type IOResult<T> = io::Result<T>;

const TAG_LEVEL: &str = "Level";
const TAG_X_POS: &str = "xPos";
const TAG_Z_POS: &str = "zPos";

#[derive(Debug)]
pub struct Chunk {
    pub length: u32,
    pub compression_type: u8,
    nbt_raw: Vec<u8>,
}

impl Chunk {
    pub fn from_buf_reader(reader: &mut BufReader<File>, include_nbt: bool) -> IOResult<Self> {
        let mut length_raw = [0u8; 4];
        reader.read_exact(&mut length_raw)?;
        let length = BigEndian::read_u32(&length_raw);
        let compression_type = reader.read_u8()?;

        let mut nbt_raw = Vec::new();
        if include_nbt {
            for _ in 0..((length - 1) as f32 / BLOCK_SIZE as f32).ceil() as u8 {
                let mut buffer = [0u8; BLOCK_SIZE];
                reader.read(&mut buffer)?;
                nbt_raw.append(&mut buffer.to_vec());
            }
            nbt_raw.truncate((length - 1) as usize);
        }

        if length > 0 {
            reader.seek(SeekFrom::Current((length - 1) as i64))?;
        } else {
            reader.seek(SeekFrom::Current((length) as i64))?;
        }

        Ok(Self {
            compression_type,
            length,
            nbt_raw,
        })
    }

    pub fn validate_nbt_data(&mut self) -> Result<(), ChunkScanError> {
        if self.compression_type == 2 {
            let mut decoder = ZlibDecoder::new(&self.nbt_raw[..]);
            let mut data = Vec::new();
            decoder.read_to_end(&mut data)?;
            self.nbt_raw = data;
        }
        let mut reader = NBTReader::new(&self.nbt_raw[..]);
        let data = reader.parse()?;

        if !data.contains_key(TAG_LEVEL) {
            Err(ChunkScanError::MissingTag(TAG_LEVEL))
        } else {
            let lvl_data = &data[TAG_LEVEL];

            if let NBTValue::Compound(lvl_data) = lvl_data {
                if !lvl_data.contains_key(TAG_X_POS) {
                    Err(ChunkScanError::MissingTag(TAG_X_POS))
                } else if !lvl_data.contains_key(TAG_Z_POS) {
                    Err(ChunkScanError::MissingTag(TAG_Z_POS))
                } else {
                    Ok(())
                }
            } else {
                Err(ChunkScanError::InvalidFormat(TAG_LEVEL))
            }
        }
    }
}

#[derive(Debug)]
pub enum ChunkScanError {
    String(String),
    IO(io::Error),
    NBTError(NBTError),
    MissingTag(&'static str),
    InvalidFormat(&'static str),
}

impl Display for ChunkScanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::IO(io) => write!(f, "IO Error: {}", io),
            Self::NBTError(nbt) => write!(f, "NBT Error: {}", nbt),
            Self::MissingTag(tag) => write!(f, "Missing Tag in NBT Data: {}", tag),
            Self::InvalidFormat(tag) => write!(f, "Unexpected data format for NBT Tag {}", tag),
        }
    }
}

impl From<io::Error> for ChunkScanError {
    fn from(io_err: Error) -> Self {
        Self::IO(io_err)
    }
}

impl From<NBTError> for ChunkScanError {
    fn from(nbt: NBTError) -> Self {
        Self::NBTError(nbt)
    }
}

impl From<String> for ChunkScanError {
    fn from(err: String) -> Self {
        Self::String(err)
    }
}
