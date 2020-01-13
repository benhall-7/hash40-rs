use crate::{Hash40, Hash40Visitor, ReadHash40, WriteHash40, to_hash40};
use crate::private::{LABELS, LabelMap};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use serde::{de, Serialize, Serializer, Deserialize, Deserializer};

use std::num::ParseIntError;
use std::io::{Error, Read, Write};

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

// Hash40 -> string
impl ToString for Hash40 {
    fn to_string(&self) -> String {
        format!("0x{:010x}", self.0)
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
            "A hex-formatted integer hash value, or a string standing for its reversed form",
        )
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.starts_with("0x") {
            // from_hex_str only returns None if it doesn't start with 0x
            // we can safely unwrap here
            Hash40::from_hex_str(value).map_err(|e| E::custom(e.unwrap()))
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