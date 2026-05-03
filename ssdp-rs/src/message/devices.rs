//! Device types (all known & handling for custom)

use std::{fmt::Display, str::FromStr};

use super::{ParseError, Vendor};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceDetails {
    pub vendor: Vendor,
    pub device: Device,
}

impl FromStr for DeviceDetails {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseError::InvalidDevice(s.to_string());
        let mut parts = s.split(":");
        match parts.next() {
            Some("urn") => (),
            _ => return Err(err()),
        };
        let Ok(vendor) = parts.next().ok_or_else(err)?.parse::<Vendor>();
        match parts.next() {
            Some("device") => (),
            _ => return Err(err()),
        };
        let device: String = parts.collect();
        let device: Device = device.parse()?;
        Ok(Self { vendor, device })
    }
}

impl Display for DeviceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:device:{}", self.vendor, self.device)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Device {
    Other { device_type: String, ver: String },
}

impl FromStr for Device {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (device_type, ver) = s
            .split_once(":")
            .ok_or(ParseError::InvalidDeviceDetails(s.to_string()))?;
        Ok(Self::Other {
            device_type: device_type.to_string(),
            ver: ver.to_string(),
        })
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::Other { device_type, ver } => write!(f, "{}:{}", device_type, ver),
        }
    }
}
