use crate::errors::ParseHashError;
use crate::{hash40, Hash40};
use bimap::BiHashMap;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct LabelMap {
    /// A bidirectional map to associate hashes and their labels
    pub map: BiHashMap<Hash40, String>,

    /// Controls whether the default hash40 method is used instead of returning None
    /// when you try to find the hash of a label which is not present in the map.
    ///
    /// By default, set to false
    pub strict: bool,
}

/// The type of error returned when reading from custom label files
#[derive(Debug)]
pub enum CustomLabelError {
    Io(io::Error),
    MisingColumn,
    ParseHashError(ParseHashError),
}

impl LabelMap {
    /// Convenience method to clear the labels within the map
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Inserts labels into the map, using the default hash40 method for the hash
    pub fn add_labels<I: IntoIterator<Item = String>>(&mut self, labels: I) {
        for l in labels {
            self.map.insert(Hash40::new(&l), l);
        }
    }

    /// Inserts labels into the map, providing both the hash and the associated label.
    ///
    /// Users can insert a label for a hash, even if the hash of the label inserted doesn't
    /// match the paired hash. This allows custom descriptive labels when the true label is
    /// not known for the hash.
    pub fn add_custom_labels<I: Iterator<Item = (Hash40, String)>>(&mut self, labels: I) {
        for (hash, label) in labels {
            self.map.insert(hash, label);
        }
    }

    /// Opens a file and returns a list of newline-separated labels
    pub fn read_labels<P: AsRef<Path>>(path: P) -> Result<Vec<String>, io::Error> {
        let reader = BufReader::new(File::open(path)?);
        reader.lines().collect()
    }

    /// Opens a file and returns a list of line-separated pairs of hashes and labels.
    /// Each hash-label pair is separated by a comma, and the hash must be formatted
    /// in hexadecimal, beginning with "0x"
    pub fn read_custom_labels<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<(Hash40, String)>, CustomLabelError> {
        let reader = BufReader::new(File::open(path)?);
        reader
            .lines()
            .map(|line_result| {
                let line = line_result?;
                let mut split = line.split(',');
                split
                    .next()
                    .zip(split.next())
                    .ok_or(CustomLabelError::MisingColumn)
                    .and_then(|(hash, label)| {
                        Ok((Hash40::from_hex_str(hash)?, String::from(label)))
                    })
            })
            .collect()
    }

    /// A combination of the two functions [`Self::add_labels`] and [`Self::read_labels`]
    pub fn add_labels_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), io::Error> {
        self.add_labels(Self::read_labels(path)?);
        Ok(())
    }

    /// A combination of the two functions [`Self::add_custom_labels`] and
    /// [`Self::read_custom_labels`]
    pub fn add_custom_labels_from_path<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), CustomLabelError> {
        self.add_custom_labels(Self::read_custom_labels(path)?.into_iter());
        Ok(())
    }

    pub fn label_of(&self, hash: Hash40) -> Option<String> {
        self.map.get_by_left(&hash).map(Into::into)
    }

    pub fn hash_of(&self, label: &str) -> Option<Hash40> {
        self.map
            .get_by_right(label)
            .copied()
            .or_else(|| (!self.strict).then(|| hash40(label)))
    }
}

impl From<io::Error> for CustomLabelError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseHashError> for CustomLabelError {
    fn from(err: ParseHashError) -> Self {
        Self::ParseHashError(err)
    }
}
