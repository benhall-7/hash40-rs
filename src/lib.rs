use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use crc::crc32::checksum_ieee;
use lazy_static::lazy_static;
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::path::Path;
use std::string::ToString;
use std::sync::Mutex;

pub use compile_time_crc32;

#[macro_export]
macro_rules! hash40 {
    ($lit:literal) => {
        $crate::Hash40(($crate::compile_time_crc32::crc32!($lit) as u64) | ($lit.len() as u64) << 32)
    };
}

lazy_static! {
    static ref LABELS: Mutex<HashMap<Hash40, String>> = {
        let l = HashMap::new();
        //TODO: populate the dictionary at compile time with all documented labels
        Mutex::new(l)
    };
}

// value list, automatically converted to hash40's ; read line-by-line
pub fn load_labels<P: AsRef<Path>>(file: P) -> Result<(), Error> {
    match LABELS.lock() {
        Ok(ref mut map) => {
            for l in BufReader::new(File::open(file)?).lines() {
                match l {
                    Ok(line) => {
                        map.insert(to_hash40(&line), line);
                    }
                    Err(_) => continue,
                }
            }
            Ok(())
        }
        // TODO: returning a io:Error here is bad
        Err(_) => Err(Error::new(
            ErrorKind::Other,
            "Failed to access global: LABELS",
        )),
    }
}

// comma-separated hash40/value list ; read line-by-line
pub fn load_custom_labels<P: AsRef<Path>>(file: P) -> Result<(), Error> {
    match LABELS.lock() {
        Ok(ref mut map) => {
            for l in BufReader::new(File::open(file)?).lines() {
                match l {
                    Ok(line) => {
                        let split: Vec<&str> = line.split(',').collect();
                        if split.len() < 2 {
                            continue;
                        }
                        let hash = if split[0].starts_with("0x") {
                            match Hash40::from_hex_str(split[0]) {
                                Ok(h) => h,
                                Err(_) => {
                                    continue;
                                }
                            }
                        } else {
                            continue;
                        };
                        map.insert(hash, String::from(split[1]));
                    }
                    Err(_) => continue,
                }
            }
            Ok(())
        }
        // TODO: returning a io:Error here is bad
        Err(_) => Err(Error::new(
            ErrorKind::Other,
            "Failed to access global: LABELS",
        )),
    }
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
            Ok(x) => match x.get(&self) {
                Some(l) => String::from(l),
                None => self.to_string(),
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
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, Error>;

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), Error>;
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

// extension of io::Write capabilities to write Hash40 to stream
pub trait WriteHash40: WriteBytesExt {
    fn write_hash40<T: ByteOrder>(&mut self, hash: Hash40) -> Result<(), Error>;

    fn write_hash40_with_meta<T: ByteOrder>(
        &mut self,
        hash: Hash40,
        meta: u32,
    ) -> Result<(), Error>;
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
            Hash40::from_hex_str(value).map_err(|e| { E::custom(e) })
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