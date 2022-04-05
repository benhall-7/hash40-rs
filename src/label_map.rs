use crate::Hash40;
use bimap::BiHashMap;
use crc::Crc;

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub enum LabelMap {
    Unset,
    Pure(HashMap<Hash40, String>),
    Custom(BiHashMap<Hash40, String>),
}
