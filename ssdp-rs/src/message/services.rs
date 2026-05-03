//! Device types (all known & handling for custom)

use std::fmt::Display;

use super::{ErrorKind, Vendor};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceDetails {
    pub vendor: Vendor,
    pub service: Service,
}

impl Display for ServiceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:service:{}", self.vendor, self.service)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Service {
    Other { service_type: String, ver: String },
}

impl Service {
    pub fn from_parts<'s, P: IntoIterator<Item = &'s str>>(parts: P) -> Result<Self, ErrorKind> {
        let mut parts = parts.into_iter();
        let service_type = parts
            .next()
            .ok_or(ErrorKind::InvalidDevice("".to_string()))?
            .to_string();
        let ver = parts.collect();
        Ok(Self::Other { service_type, ver })
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Service::Other { service_type, ver } => write!(f, "{}:{}", service_type, ver),
        }
    }
}
