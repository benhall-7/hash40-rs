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
use std::sync::{Mutex, Arc};

impl Hash40 {
    /// Computes a Hash40 from a string. This method does not respect the static label map,
    /// nor does it check to see if the provided string is in hexadecimal format already.
    pub const fn new(string: &str) -> Self {
        let length_byte = (string.len() as u8 as u64) << 32;
        let crc = Crc::<u32>::new(&CRC_32_CKSUM).checksum(string.as_bytes()) as u64;
        Hash40(crc | length_byte)
    }

    /// Converts a hexadecimal string representation of a hash to a Hash40
    pub fn from_hex_str(value: &str) -> Result<Self, ParseHashError> {
        if let Some(stripped) = value.strip_prefix("0x") {
            Ok(Hash40(u64::from_str_radix(stripped, 16)?))
        } else {
            Err(ParseHashError::MissingPrefix)
        }
    }

    /// Computes a Hash40 from a string. This method checks if the string is a hexadecimal
    /// value first. If not, it either searches for a reverse label from the static map or
    /// computes a new hash, depending on the form of the static label map.
    pub fn from_label(label: &String) -> Result<Self, FromLabelError> {
        match Self::from_hex_str(label) {
            Ok(hash) => Ok(hash),
            Err(err) => match err {
                ParseHashError::MissingPrefix => {
                    let lock = LABELS.lock();
                    let labels = match lock {
                        Ok(labels) => labels,
                        Err(err) => err.into_inner(),
                    };
                    labels
                        .hash_of(label)
                        .ok_or_else(|| FromLabelError::LabelNotFound(label.clone()))
                }
                ParseHashError::ParseError(err) => Err(err.into()),
            },
        }
    }

    pub fn to_label(&self) -> String {
        let lock = LABELS.lock();
        let labels = match lock {
            Ok(labels) => labels,
            Err(err) => err.into_inner(),
        };
        labels.label_of(*self).unwrap_or_else(|| format!("0x{:010x}", self.0))
    }

    /// Returns the CRC32 part of the hash
    pub const fn crc(self) -> u32 {
        self.0 as u32
    }

    /// Returns the string length part of the hash
    pub const fn str_len(self) -> u8 {
        (self.0 >> 32) as u8
    }

    /// A convenience method provided to access the static label map
    pub fn label_map() -> Arc<Mutex<LabelMap>> {
        LABELS.clone()
    }
}

impl FromStr for Hash40 {
    type Err = FromLabelError;

    fn from_str(f: &str) -> Result<Self, FromLabelError> {
        Hash40::from_label(&f.to_string())
    }
}

// Hash40 -> string
impl Display for Hash40 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmtError> {
        write!(f, "{}", self.to_label())
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

#[derive(Debug, Clone, PartialEq)]
pub enum ParseHashError {
    /// The error returned when the numeric hash string doesn't begin with "0x"
    MissingPrefix,
    /// The error returned when the hexadecimal part of the hash string cannot be parsed
    ParseError(ParseIntError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FromLabelError {
    /// The error returned only when the static label map is bidirectional, and a label
    /// cannot be matched to a hash
    LabelNotFound(String),
    /// The error returned when the hexadecimal part of the hash string cannot be parsed
    ParseError(ParseIntError),
}

impl From<ParseIntError> for ParseHashError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseError(err)
    }
}

impl From<ParseIntError> for FromLabelError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseError(err)
    }
}

impl Display for FromLabelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
        Hash40::from_label(&String::from(value)).map_err(de::Error::custom)
    }
}

impl Serialize for Hash40 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_label())
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
