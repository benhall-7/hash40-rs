pub mod errors;
pub mod label_map;

pub use binrw;
pub use diff;

mod algorithm;

use errors::*;
use label_map::LabelMap;

use binrw::binrw as binrw_attr;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use diff::Diff;
use lazy_static::lazy_static;

use std::fmt::{Display, Error as fmtError, Formatter};
use std::io::{self, Read, Write};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[cfg(feature = "serde")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

lazy_static! {
    /// The static map used for converting Hash40's between hash and string form.
    static ref LABELS: Arc<Mutex<LabelMap>> = Arc::new(Mutex::new(LabelMap::default()));
}

/// The central type of the crate, representing a string hashed using the hash40 algorithm
/// Hash40 is a combination of a crc32 checksum and string length appended to the top bits
#[binrw_attr]
#[repr(transparent)]
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hash40(pub u64);

/// An alias for Hash40::new, which creates a Hash40 from a string
pub const fn hash40(string: &str) -> Hash40 {
    Hash40::new(string)
}

// An extension of the byteorder trait, to read a Hash40 from a stream
pub trait ReadHash40: ReadBytesExt {
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, io::Error>;

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), io::Error>;
}

// An extension of the byteorder trait, to write a Hash40 into a stream
pub trait WriteHash40: WriteBytesExt {
    fn write_hash40<T: ByteOrder>(&mut self, hash: Hash40) -> Result<(), io::Error>;

    fn write_hash40_with_meta<T: ByteOrder>(
        &mut self,
        hash: Hash40,
        meta: u32,
    ) -> Result<(), io::Error>;
}

impl Hash40 {
    /// Computes a Hash40 from a string. This method does not respect the static label map,
    /// nor does it check to see if the provided string is in hexadecimal format already.
    pub const fn new(string: &str) -> Self {
        Self(algorithm::hash40(string))
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
    pub fn from_label(label: &str) -> Result<Self, FromLabelError> {
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
                        .ok_or_else(|| FromLabelError::LabelNotFound(String::from(label)))
                }
                ParseHashError::ParseError(err) => Err(err.into()),
            },
        }
    }

    /// Searches for the label associated with the hash value. If no label is found, returns
    /// the hexadecimal value, formatted as `0x0123456789`
    pub fn to_label(&self) -> String {
        let lock = LABELS.lock();
        let labels = match lock {
            Ok(labels) => labels,
            Err(err) => err.into_inner(),
        };
        labels
            .label_of(*self)
            .unwrap_or_else(|| format!("0x{:010x}", self.0))
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

    /// Concatenates two Hash40 values, so that the resulting length and CRC would be the same if
    /// the original data was all hashed together.
    pub const fn concat(self, other: Self) -> Self {
        Self(algorithm::hash40_concat(self.0, other.0))
    }

    /// A convenience method for concatenating a string to a Hash40
    pub const fn concat_str(self, other: &str) -> Self {
        self.concat(hash40(other))
    }

    /// A convenience method for concatenating two Hash40s separated by a path separator
    pub const fn join_path(self, other: Self) -> Self {
        self.concat_str("/").concat(other)
    }
}

impl FromStr for Hash40 {
    type Err = FromLabelError;

    fn from_str(f: &str) -> Result<Self, FromLabelError> {
        Hash40::from_label(f)
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

impl<R: Read> ReadHash40 for R {
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, io::Error> {
        Ok(Hash40(self.read_u64::<T>()? & 0xff_ffff_ffff))
    }

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), io::Error> {
        let long = self.read_u64::<T>()?;
        Ok((Hash40(long & 0xff_ffff_ffff), (long >> 40) as u32))
    }
}

impl<W: Write> WriteHash40 for W {
    fn write_hash40<T: ByteOrder>(&mut self, hash: Hash40) -> Result<(), io::Error> {
        self.write_u64::<T>(hash.0)
    }

    fn write_hash40_with_meta<T: ByteOrder>(
        &mut self,
        hash: Hash40,
        meta: u32,
    ) -> Result<(), io::Error> {
        self.write_u64::<T>(hash.0 | (meta as u64) << 40)
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

#[cfg(feature = "serde")]
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

#[cfg(feature = "serde")]
impl Serialize for Hash40 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_label())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Hash40 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(Hash40Visitor)
    }
}

#[cfg(feature = "serde")]
/// Used to implement serde's Deserialize trait
struct Hash40Visitor;
