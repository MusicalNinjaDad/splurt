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
        let mut parts = s.split(":");
        match parts.next() {
            Some(token) => match token.parse() {
                Ok(UriToken::Urn) => (),
                _ => return Err(ParseError::InvalidUrn(s.to_string())),
            },
            None => return Err(ParseError::EmptyMessage),
        }
        todo!("parse target from")
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
