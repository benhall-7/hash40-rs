use crate::label_map::LabelMap;
use crate::{Hash40, Hash40Visitor, ReadHash40, WriteHash40, LABELS};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use crc::{Crc, CRC_32_CKSUM};
use diff::Diff;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use std::fmt::{Display, Error as fmtError, Formatter};
use std::io::{Error, Read, Write};
use std::num::ParseIntError;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

/// The central type of the crate, representing a string hashed using the hash40 algorithm
/// Hash40 is a combination of a crc32 checksum and string length appended to the top bits
impl Hash40 {
    pub const fn new(string: &str) -> Self {
        let length_byte = (string.len() as u8 as u64) << 32;
        let crc = Crc::<u32>::new(&CRC_32_CKSUM).checksum(string.as_bytes()) as u64;
        Hash40(crc | length_byte)
    }

    pub const fn crc(self) -> u32 {
        self.0 as u32
    }

    pub const fn strlen(self) -> u8 {
        (self.0 >> 32) as u8
    }

    // TODO: if the string isn't formatted with "0x"
    // return a real error instead of Err(None)
    pub fn from_hex_str(value: &str) -> Result<Self, Option<ParseIntError>> {
        if &value[0..2] == "0x" {
            Ok(Hash40(u64::from_str_radix(&value[2..], 16)?))
        } else {
            Err(None)
        }
    }
}

impl FromStr for Hash40 {
    type Err = ParseIntError;

    fn from_str(f: &str) -> Result<Self, ParseIntError> {
        if f.starts_with("0x") {
            // from_hex_str only returns None if it doesn't start with 0x
            // we can safely unwrap here
            Self::from_hex_str(f).map_err(|e| e.unwrap())
        } else {
            Ok(Hash40::new(f))
        }
    }
}

// Hash40 -> string
impl Display for Hash40 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmtError> {
        match LABELS.lock() {
            Ok(label_map) => match &*label_map {
                LabelMap::Pure(map) => {
                    if let Some(label) = map.get(&self) {
                        write!(f, "{}", label)
                    } else {
                        write!(f, "0x{:010x}", self.0)
                    }
                }
                LabelMap::Custom(bimap) => {
                    if let Some(label) = bimap.get_by_left(&self) {
                        write!(f, "{}", label)
                    } else {
                        write!(f, "0x{:010x}", self.0)
                    }
                }
                LabelMap::Unset => write!(f, "0x{:010x}", self.0),
            },
            Err(_) => write!(f, "0x{:010x}", self.0),
        }
    }
}

impl Deref for Hash40 {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Hash40 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<R: Read> ReadHash40 for R {
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, Error> {
        Ok(Hash40(self.read_u64::<T>()? & 0xff_ffff_ffff))
    }

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), Error> {
        let long = self.read_u64::<T>()?;
        Ok((Hash40(long & 0xff_ffff_ffff), (long >> 40) as u32))
    }
}

impl<W: Write> WriteHash40 for W {
    fn write_hash40<T: ByteOrder>(&mut self, hash: Hash40) -> Result<(), Error> {
        self.write_u64::<T>(hash.0)
    }

    fn write_hash40_with_meta<T: ByteOrder>(
        &mut self,
        hash: Hash40,
        meta: u32,
    ) -> Result<(), Error> {
        self.write_u64::<T>(hash.0 | (meta as u64) << 40)
    }
}

impl<'de> de::Visitor<'de> for Hash40Visitor {
    type Value = Hash40;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        formatter.write_str(
            "A hex-formatted integer hash value, or a string representing for its reversed form",
        )
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Hash40::from_str(value).map_err(E::custom)
    }
}

impl Serialize for Hash40 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(format!("{}", &self).as_ref())
    }
}

impl<'de> Deserialize<'de> for Hash40 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(Hash40Visitor)
    }
}

impl Diff for Hash40 {
    type Repr = Option<Hash40>;

    fn diff(&self, other: &Self) -> Self::Repr {
        if self == other {
            None
        } else {
            Some(*other)
        }
    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(other) = diff {
            *self = *other;
        }
    }

    fn identity() -> Self {
        Default::default()
    }
}
