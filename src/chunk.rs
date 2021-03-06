use crate::nbt::{NBTError, NBTReader, NBTValue};
use byteorder::{BigEndian, ReadBytesExt};

use crate::constants::tags::{LEVEL_TAGS, TAG_LEVEL, TAG_X_POS, TAG_Z_POS};
use crate::region_file::BLOCK_SIZE;
use flate2::read::{GzDecoder, ZlibDecoder};
use std::fmt::{Display, Formatter};
use std::io::{self, BufReader, Error};

type IOResult<T> = io::Result<T>;

#[derive(Debug)]
pub struct Chunk {
    pub length: u32,
    pub compression_type: u8,
    pub x_pos: Option<i32>,
    pub z_pos: Option<i32>,
}

impl Chunk {
    pub fn from_buf_reader<R: io::Read + io::Seek>(reader: &mut R) -> IOResult<Self> {
        let length = reader.read_u32::<BigEndian>()?;
        if length > 128 * BLOCK_SIZE as u32 || length == 0 {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        let compression_type = reader.read_u8()?;

        Ok(Self {
            compression_type,
            length,
            x_pos: None,
            z_pos: None,
        })
    }

    pub fn validate_nbt_data<R: io::Read + io::Seek>(
        &mut self,
        reader: &mut R,
    ) -> Result<(), ChunkScanError> {
        let data = if self.compression_type == 1 {
            let mut nbt_reader = NBTReader::new(BufReader::new(GzDecoder::new(reader)));
            nbt_reader.parse()?
        } else if self.compression_type == 2 {
            let mut nbt_reader = NBTReader::new(BufReader::new(ZlibDecoder::new(reader)));
            nbt_reader.parse()?
        } else {
            let mut nbt_reader = NBTReader::new(reader);
            nbt_reader.parse()?
        };

        if !data.contains_key(TAG_LEVEL) {
            Err(ChunkScanError::MissingTag(TAG_LEVEL))
        } else {
            let lvl_data = &data[TAG_LEVEL];

            if let NBTValue::Compound(lvl_data) = lvl_data {
                for tag in LEVEL_TAGS {
                    if !lvl_data.contains_key(*tag) {
                        return Err(ChunkScanError::MissingTag(tag));
                    }
                }
                self.x_pos = lvl_data[TAG_X_POS].as_int().cloned();
                self.z_pos = lvl_data[TAG_Z_POS].as_int().cloned();

                Ok(())
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
    InvalidLength(u32),
}

impl Display for ChunkScanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::IO(io) => write!(f, "IO Error: {}", io),
            Self::NBTError(nbt) => write!(f, "NBT Error: {}", nbt),
            Self::MissingTag(tag) => write!(f, "Missing Tag in NBT Data: {}", tag),
            Self::InvalidFormat(tag) => write!(f, "Unexpected data format for NBT Tag {}", tag),
            Self::InvalidLength(length) => write!(f, "Invalid chunk data length: {}", length),
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
