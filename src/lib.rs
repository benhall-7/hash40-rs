use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;

use std::io;
use std::sync::{Arc, Mutex};

mod r#impl;
pub mod label_map;

use label_map::LabelMap;

lazy_static! {
    /// The static map used for converting Hash40's between hash and string form.
    /// 
    /// Parent libraries should re-export this so that binaries can share a single
    /// label map between all instances of this crate
    pub static ref LABELS: Arc<Mutex<LabelMap>> = Arc::new(Mutex::new(LabelMap::Unset));
}

/// The central type of the crate, representing a string hashed using the hash40 algorithm
/// Hash40 is a combination of a crc32 checksum and string length appended to the top bits
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

/// Used to implement serde's Deserialize trait
struct Hash40Visitor;
