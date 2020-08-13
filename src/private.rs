use crate::Hash40;
use bimap::BiHashMap;
use crc::crc32::checksum_ieee;
use lazy_static::lazy_static;

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub enum LabelMap {
    Unset,
    Pure(HashMap<Hash40, String>),
    Custom(BiHashMap<Hash40, String>),
}

lazy_static! {
    pub static ref LABELS: Mutex<LabelMap> = Mutex::new(LabelMap::Unset);
}

pub fn crc32_with_len(word: &str) -> u64 {
    ((word.len() as u64) << 32) | (checksum_ieee(word.as_bytes()) as u64)
}
