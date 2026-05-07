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
    AVTransport { ver: u8 },
    Other { service_type: String, ver: String },
}

impl Service {
    pub fn from_parts<'s, P>(parts: &mut P) -> Result<Self, ErrorKind>
    where
        P: Iterator<Item = &'s str>,
    {
        let service_type = parts
            .next()
            .ok_or(ErrorKind::InvalidService("''".to_string()))?;
        let ver: String = parts.collect();
        let service = match service_type {
            "AVTransport" => Self::AVTransport {
                ver: ver
                    .as_str()
                    .parse()
                    .map_err(|_| ErrorKind::InvalidService(format!("{}:{}", service_type, ver)))?,
            },
            _ => Self::Other {
                service_type: service_type.to_string(),
                ver,
            },
        };
        Ok(service)
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AVTransport { ver } => write!(f, "AVTransport:{ver}"),
            Service::Other { service_type, ver } => write!(f, "{}:{}", service_type, ver),
        }
    }
}
