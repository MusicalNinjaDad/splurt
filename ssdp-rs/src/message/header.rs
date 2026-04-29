//! Header functionality:
//! - [UpnpHeader] for storing & finding values
//! - enums / structs for all standard header fields with standard key name, parsing & display logic

use std::collections::HashMap;

use crate::message::ParseError;

pub struct UpnpHeader<'h>(HashMap<&'h str, &'h str>);

// TODO: #42 handle header key case sensitivity and maintain round-tripping
//   at the same time, also handle split_once(": ") skips headers without a value (like EXT:)
//   via split(":") & trim
impl<'h> FromIterator<&'h str> for UpnpHeader<'h> {
    fn from_iter<T: IntoIterator<Item = &'h str>>(iter: T) -> Self {
        let hashmap = iter
            .into_iter()
            .filter_map(|line| line.split_once(": "))
            .collect();
        Self(hashmap)
    }
}

impl<'h> UpnpHeader<'h> {
    /// Attempt to get the corresponding value for `key`, returning a [ParseError::MissingField]
    /// if unsuccessful.
    pub fn try_get(&self, key: &str) -> Result<&str, ParseError> {
        self.0
            .get(key)
            .ok_or_else(|| ParseError::MissingField(key.to_string()))
            .copied()
    }

    /// Attempt to get the value for `key`, returning `None` if unsuccessful.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).copied()
    }
}
