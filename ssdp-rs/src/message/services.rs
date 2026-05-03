//! Device types (all known & handling for custom)

use std::fmt::Display;

use super::Vendor;

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

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Service::Other { service_type, ver } => write!(f, "{}:{}", service_type, ver),
        }
    }
}
