use crate::r#impl::ParseHashError;
use crate::{hash40, Hash40};
use bimap::BiHashMap;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub enum LabelMap {
    Unset,
    Pure(HashMap<Hash40, String>),
    Custom(BiHashMap<Hash40, String>),
}

#[derive(Debug)]
pub enum CustomLabelError {
    Io(io::Error),
    MisingColumn,
    ParseHashError(ParseHashError),
}

impl LabelMap {
    pub fn set_labels<I: IntoIterator<Item = String>>(&mut self, labels: I) {
        let mut hashmap = HashMap::<Hash40, String>::new();
        for l in labels {
            hashmap.insert(Hash40::new(&l), l);
        }

        *self = LabelMap::Pure(hashmap);
    }

    pub fn set_custom_labels<I: Iterator<Item = (Hash40, String)>>(&mut self, labels: I) {
        let mut bimap = BiHashMap::<Hash40, String>::new();
        for (hash, label) in labels {
            bimap.insert(hash, label);
        }

        *self = LabelMap::Custom(bimap);
    }

    pub fn set_labels_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), io::Error> {
        let reader = BufReader::new(File::open(path)?);
        self.set_labels(reader.lines().collect::<Result<Vec<_>, _>>()?);
        Ok(())
    }

    pub fn set_custom_labels_from_path<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), CustomLabelError> {
        let reader = BufReader::new(File::open(path)?);
        let labels = reader
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
            .collect::<Result<Vec<_>, _>>()?;
        self.set_custom_labels(labels.into_iter());
        Ok(())
    }

    pub fn label_of(&self, hash: Hash40) -> Option<String> {
        match self {
            LabelMap::Unset => None,
            LabelMap::Pure(labels) => labels.get(&hash).map(Into::into),
            LabelMap::Custom(labels) => labels.get_by_left(&hash).map(Into::into),
        }
    }

    pub fn hash_of(&self, label: &str) -> Option<Hash40> {
        match self {
            LabelMap::Unset | LabelMap::Pure(..) => Some(hash40(label)),
            LabelMap::Custom(labels) => labels.get_by_right(label).copied(),
        }
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
