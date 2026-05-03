//! Specific URI handling as used for the NT, ST & USN fields

use std::str::FromStr;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urn() {
        let st = "urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1";
        let urn: Target = st.parse().expect("is urn");
    }
}
