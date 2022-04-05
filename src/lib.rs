use bimap::BiHashMap;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Mutex;

mod r#impl;
pub mod label_map;

use label_map::LabelMap;

lazy_static! {
    pub(crate) static ref LABELS: Mutex<LabelMap> = Mutex::new(LabelMap::Unset);
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hash40(u64);

pub fn set_labels<I: IntoIterator<Item = String>>(labels: I) {
    let mut map = LABELS.lock().unwrap();
    let mut hashmap = HashMap::<Hash40, String>::new();

    for l in labels {
        hashmap.insert(Hash40::new(&l), l);
    }
    *map = LabelMap::Pure(hashmap);
}

pub fn set_custom_labels<I: Iterator<Item = (Hash40, String)>>(labels: I) {
    let mut map = LABELS.lock().unwrap();
    let mut bimap = BiHashMap::<Hash40, String>::new();

    for (hash, label) in labels {
        bimap.insert(hash, label);
    }
    *map = LabelMap::Custom(bimap);
}

pub fn read_labels<P: AsRef<Path>>(path: P) -> Result<Vec<String>, io::Error> {
    let reader = BufReader::new(File::open(path)?);
    reader.lines().collect::<Result<Vec<_>, _>>()
}

pub fn read_custom_labels<P: AsRef<Path>>(path: P) -> Result<Vec<(Hash40, String)>, io::Error> {
    let reader = BufReader::new(File::open(path)?);
    reader
        .lines()
        .filter_map(|line_result| match line_result {
            Ok(line) => {
                let mut split = line.split(',');
                let hash_opt = split.next();
                let label_opt = split.next();

                if let Some(hash_str) = hash_opt {
                    if let Some(label) = label_opt {
                        if let Ok(hash) = Hash40::from_hex_str(hash_str) {
                            return Some(Ok((hash, String::from(label))));
                        }
                    }
                }

                None
            }
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()
}

// extension of io::Read capabilities to get Hash40 from stream
pub trait ReadHash40: ReadBytesExt {
    fn read_hash40<T: ByteOrder>(&mut self) -> Result<Hash40, io::Error>;

    fn read_hash40_with_meta<T: ByteOrder>(&mut self) -> Result<(Hash40, u32), io::Error>;
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

/// Used to implement serde's Deserialize trait
struct Hash40Visitor;
