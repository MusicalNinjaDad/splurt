//! Device types (all known & handling for custom)

use std::{fmt::Display, str::FromStr};

use super::{Device, ParseError, Vendor};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
