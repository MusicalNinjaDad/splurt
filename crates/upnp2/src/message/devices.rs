//! Device types (all known & handling for custom)

use std::fmt::Display;

use super::{ErrorKind, Vendor};

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
    Basic { ver: u8 },
    BinaryLight { ver: u8 },
    MediaServer { ver: u8 },
    ZonePlayer { ver: u8 },
    Other { device_type: String, ver: String },
}

impl Device {
    pub fn from_parts<'s, P>(parts: &mut P) -> Result<Self, ErrorKind>
    where
        P: Iterator<Item = &'s str>,
    {
        let device_type = parts
            .next()
            .ok_or(ErrorKind::InvalidDevice("''".to_string()))?
            .to_string();
        let ver = |v: String| {
            v.as_str()
                .parse()
                .map_err(|_| ErrorKind::InvalidDevice(format!("{}:{}", device_type, v)))
        };
        let v = parts.collect();
        let device = match device_type.as_str() {
            // TODO: Case sensitivity
            "basic" => Device::Basic { ver: ver(v)? },
            "BinaryLight" => Device::BinaryLight { ver: ver(v)? },
            "MediaServer" => Device::MediaServer { ver: ver(v)? },
            "ZonePlayer" => Device::ZonePlayer { ver: ver(v)? },
            _ => Device::Other {
                device_type,
                ver: v,
            },
        };
        Ok(device)
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::Basic { ver } => write!(f, "Basic:{}", ver),
            Device::BinaryLight { ver } => write!(f, "BinaryLight:{}", ver),
            Device::MediaServer { ver } => write!(f, "MediaServer:{}", ver),
            Device::ZonePlayer { ver } => write!(f, "ZonePlayer:{}", ver),
            Device::Other { device_type, ver } => write!(f, "{}:{}", device_type, ver),
        }
    }
}
