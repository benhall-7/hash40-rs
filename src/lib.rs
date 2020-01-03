use bimap::BiHashMap;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use crc::crc32::checksum_ieee;
use lazy_static::lazy_static;
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::HashMap;
// use std::error::Error;
// use std::fs::File;
use std::io;
use std::io::{/*BufRead, BufReader, ErrorKind,*/ Read, Write};
// use std::path::Path;
use std::string::ToString;
use std::sync::Mutex;

pub use compile_time_crc32;

mod private;

#[macro_export]
macro_rules! hash40 {
    ($lit:literal) => {
        $crate::Hash40(
            ($crate::compile_time_crc32::crc32!($lit) as u64) | ($lit.len() as u64) << 32,
        )
    };
}

#[derive(Debug)]
pub enum LabelMap {
    Unset,
    Pure(HashMap<Hash40, String>),
    Custom(BiHashMap<Hash40, String>),
}

lazy_static! {
    static ref LABELS: Mutex<LabelMap> = { Mutex::new(LabelMap::Unset) };
}

pub fn set_labels<I: IntoIterator<Item = String>>(labels: I) {
    let mut map = LABELS.lock().unwrap();
    let mut hashmap = HashMap::<Hash40, String>::new();

    for l in labels {
        hashmap.insert(to_hash40(&l), l);
    }
    *map = LabelMap::Pure(hashmap);
}

// comma-separated hash40/value list ; read line-by-line
pub fn set_custom_labels<I: Iterator<Item = (Hash40, String)>>(labels: I) {
    let mut map = LABELS.lock().unwrap();
    let mut bimap = BiHashMap::<Hash40, String>::new();

    for (hash, label) in labels {
        bimap.insert(hash, label);
    }
    *map = LabelMap::Custom(bimap);
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hash40(pub u64);

impl Hash40 {
    #[inline]
    pub fn crc(self) -> u32 {
        self.0 as u32
    }

    #[inline]
    pub fn strlen(self) -> u8 {
        (self.0 >> 32) as u8
    }

    pub fn to_label(self) -> String {
        match LABELS.lock() {
            Ok(label_map) => match &*label_map {
                LabelMap::Pure(map) => {
                    if let Some(label) = map.get(&self) {
                        String::from(label)
                    } else {
                        self.to_string()
                    }
                }
                LabelMap::Custom(bimap) => {
                    if let Some(label) = bimap.get_by_left(&self) {
                        String::from(label)
                    } else {
                        self.to_string()
                    }
                }
                LabelMap::Unset => self.to_string(),
            },
            Err(_) => self.to_string(),
        }
    }

    pub fn from_hex_str(value: &str) -> Result<Self, std::num::ParseIntError> {
        Ok(Hash40(u64::from_str_radix(&value[2..], 16)?))
    }
}

// Hash40 -> string
impl ToString for Hash40 {
    fn to_string(&self) -> String {
        format!("0x{:010x}", self.0)
    }
}

// extension of io::Read capabilities to get Hash40 from stream
pub trait ReadHash40: ReadBytesExt {
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, io::Error>;

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), io::Error>;
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

// extension of io::Write capabilities to write Hash40 to stream
pub trait WriteHash40: WriteBytesExt {
    fn write_hash40<T: ByteOrder>(&mut self, hash: Hash40) -> Result<(), io::Error>;

    fn write_hash40_with_meta<T: ByteOrder>(
        &mut self,
        hash: Hash40,
        meta: u32,
    ) -> Result<(), io::Error>;
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

struct Hash40Visitor;

impl<'de> de::Visitor<'de> for Hash40Visitor {
    type Value = Hash40;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::result::Result<(), std::fmt::Error> {
        formatter.write_str(
            "A hex-formatted integer hash value, or a string standing for its reversed form",
        )
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.starts_with("0x") {
            Hash40::from_hex_str(value).map_err(E::custom)
        } else {
            Ok(to_hash40(value))
        }
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

//exposed function to compute a hash
pub fn to_hash40(word: &str) -> Hash40 {
    Hash40(crc32_with_len(word))
}

fn crc32_with_len(word: &str) -> u64 {
    ((word.len() as u64) << 32) | (checksum_ieee(word.as_bytes()) as u64)
}
