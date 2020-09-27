use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read};

const MAX_RECURSION: u64 = 100;

pub struct NBTReader<R> {
    inner: Box<R>,
    recursion: u64,
}

type NBTResult<T> = Result<T, NBTError>;

impl<R> NBTReader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner: Box::new(inner),
            recursion: 0,
        }
    }

    /// Parses the contents of the reader
    pub fn parse(&mut self) -> NBTResult<HashMap<String, NBTValue>> {
        let tag = self.inner.read_u8()?;

        if tag != 10 {
            return Err(NBTError::MissingRootTag);
        }
        let mut buf = [0u8; 2];
        self.inner.read(&mut buf)?;

        self.parse_compound()
    }

    /// Parses a compound tag
    fn parse_compound(&mut self) -> NBTResult<HashMap<String, NBTValue>> {
        self.recursion += 1;
        if self.recursion > MAX_RECURSION {
            return Err(NBTError::RecursionError);
        }
        let mut root_value = HashMap::new();
        loop {
            let tag = self.inner.read_u8()?;
            if tag == 0 {
                break;
            }
            let name = self.parse_string()?;

            let value = match tag {
                1 => NBTValue::Byte(self.inner.read_u8()?),
                2 => NBTValue::Short(self.inner.read_i16::<BigEndian>()?),
                3 => NBTValue::Int(self.inner.read_i32::<BigEndian>()?),
                4 => NBTValue::Long(self.inner.read_i64::<BigEndian>()?),
                5 => NBTValue::Float(self.inner.read_f32::<BigEndian>()?),
                6 => NBTValue::Double(self.inner.read_f64::<BigEndian>()?),
                7 => NBTValue::ByteArray(self.parse_byte_array()?),
                8 => NBTValue::String(self.parse_string()?),
                9 => NBTValue::List(self.parse_list()?),
                10 => NBTValue::Compound(self.parse_compound()?),
                11 => NBTValue::IntArray(self.parse_int_array()?),
                12 => NBTValue::LongArray(self.parse_long_array()?),
                _ => return Err(NBTError::InvalidTag(tag)),
            };
            root_value.insert(name, value);
        }
        self.recursion -= 1;
        Ok(root_value)
    }

    /// Parses an array of bytes
    fn parse_byte_array(&mut self) -> NBTResult<Vec<u8>> {
        let length = self.inner.read_u32::<BigEndian>()?;
        for _ in 0..length {
            self.inner.read_u8()?;
        }

        Ok(Vec::with_capacity(0))
    }

    /// Parses a string value
    fn parse_string(&mut self) -> NBTResult<String> {
        let length = self.inner.read_u16::<BigEndian>()?;
        if length == 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; length as usize];
        self.inner.read_exact(&mut buf)?;

        String::from_utf8(buf).map_err(|_| NBTError::InvalidName)
    }

    /// Parses a list of nbt values
    fn parse_list(&mut self) -> NBTResult<Vec<NBTValue>> {
        let tag = self.inner.read_u8()?;
        let length = self.inner.read_u32::<BigEndian>()?;

        let parse_fn: Box<dyn Fn(&mut Self) -> NBTResult<NBTValue>> = match tag {
            0 => Box::new(|_| Ok(NBTValue::Null)),
            1 => Box::new(|nbt| Ok(NBTValue::Byte(nbt.inner.read_u8()?))),
            2 => Box::new(|nbt| Ok(NBTValue::Short(nbt.inner.read_i16::<BigEndian>()?))),
            3 => Box::new(|nbt| Ok(NBTValue::Int(nbt.inner.read_i32::<BigEndian>()?))),
            4 => Box::new(|nbt| Ok(NBTValue::Long(nbt.inner.read_i64::<BigEndian>()?))),
            5 => Box::new(|nbt| Ok(NBTValue::Float(nbt.inner.read_f32::<BigEndian>()?))),
            6 => Box::new(|nbt| Ok(NBTValue::Double(nbt.inner.read_f64::<BigEndian>()?))),
            7 => Box::new(|nbt| Ok(NBTValue::ByteArray(nbt.parse_byte_array()?))),
            8 => Box::new(|nbt| Ok(NBTValue::String(nbt.parse_string()?))),
            9 => Box::new(|nbt| Ok(NBTValue::List(nbt.parse_list()?))),
            11 => Box::new(|nbt| Ok(NBTValue::IntArray(nbt.parse_int_array()?))),
            10 => Box::new(|nbt| Ok(NBTValue::Compound(nbt.parse_compound()?))),
            12 => Box::new(|nbt| Ok(NBTValue::LongArray(nbt.parse_long_array()?))),
            _ => return Err(NBTError::InvalidTag(tag)),
        };
        let mut items = Vec::new();
        for _ in 0..length {
            items.push(parse_fn(self)?);
        }

        Ok(items)
    }

    /// Parses an array of 32 bit integers
    fn parse_int_array(&mut self) -> NBTResult<Vec<i32>> {
        let length = self.inner.read_u32::<BigEndian>()?;
        let mut items = Vec::new();
        for _ in 0..length {
            items.push(self.inner.read_i32::<BigEndian>()?);
        }

        Ok(items)
    }

    /// Parses an array of 64 bit integers
    fn parse_long_array(&mut self) -> NBTResult<Vec<i64>> {
        let length = self.inner.read_u32::<BigEndian>()?;
        let mut items = Vec::new();
        for _ in 0..length {
            items.push(self.inner.read_i64::<BigEndian>()?);
        }

        Ok(items)
    }
}

#[derive(Clone, Debug)]
pub enum NBTValue {
    Null,
    Byte(u8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<NBTValue>),
    Compound(HashMap<String, NBTValue>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[derive(Debug)]
pub enum NBTError {
    IO(io::Error),
    MissingRootTag,
    InvalidTag(u8),
    InvalidName,
    RecursionError,
}

impl Display for NBTError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(io) => write!(f, "IO Error: {}", io),
            Self::InvalidTag(tag) => write!(f, "Invalid Tag: 0x{:x}", tag),
            Self::MissingRootTag => write!(f, "Missing root tag!"),
            Self::InvalidName => write!(f, "Encountered invalid tag name"),
            Self::RecursionError => write!(f, "Reached recursion limit"),
        }
    }
}

impl Error for NBTError {}

impl From<io::Error> for NBTError {
    fn from(io_err: io::Error) -> Self {
        Self::IO(io_err)
    }
}
