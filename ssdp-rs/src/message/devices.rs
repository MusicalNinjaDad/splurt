//! Device types (all known & handling for custom)

use std::fmt::Display;

use super::{ParseError, Vendor};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceDetails {
    pub vendor: Vendor,
    pub device: Device,
}

impl Display for DeviceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:device:{}", self.vendor, self.device)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Device {
    MediaServer { ver: String },
    Other { device_type: String, ver: String },
}

impl Device {
    pub fn from_parts<'s, P: IntoIterator<Item = &'s str>>(parts: P) -> Result<Self, ParseError> {
        let mut parts = parts.into_iter();
        let device_type = parts
            .next()
            .ok_or(ParseError::InvalidDevice("".to_string()))?
            .to_string();
        let ver = parts.collect();
        let device = match device_type.as_str() {
            "MediaServer" => Device::MediaServer { ver },
            _ => Device::Other { device_type, ver },
        };
        Ok(device)
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::MediaServer { ver } => write!(f, "MediaServer:{}", ver),
            Device::Other { device_type, ver } => write!(f, "{}:{}", device_type, ver),
        }
    }
}
