//! Specific URI handling as used for the NT, ST & USN fields

use std::str::FromStr;

use derive_more::FromStr;

use super::{DeviceDetails, ParseError, ServiceDetails};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Device(DeviceDetails),
    Service(ServiceDetails),
}

impl FromStr for Target {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("parse string as target")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr)]
/// Known valuable URI tokens.
enum UriToken {
    Urn,
    Service,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_uri_token() {
        let prefix = "urn";
        let token: UriToken = prefix.parse().expect("urn");
    }

    #[test]
    fn urn() {
        let st = "urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1";
        let urn: Target = st.parse().expect("is urn");
    }
}
