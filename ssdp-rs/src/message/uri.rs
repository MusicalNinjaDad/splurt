//! Specific URI handling as used for the NT, ST & USN fields

use std::str::FromStr;

use derive_more::FromStr;

use crate::message::Service;

use super::{DeviceDetails, ParseError, ServiceDetails};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Device(DeviceDetails),
    Service(ServiceDetails),
}

impl FromStr for Target {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseError::InvalidUrn(s.to_string());
        let mut parts = s.split(":");
        match parts.next().ok_or(ParseError::EmptyMessage)?.parse() {
            Ok(UriToken::Urn) => (),
            _ => return Err(err()),
        };
        let vendor = parts.next().ok_or_else(err)?;
        let offering: UriToken = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
        let name = parts.next().ok_or_else(err)?;
        let ver = parts.next().ok_or_else(err)?;
        match parts.next() {
            None => (),
            Some(_) => return Err(err()),
        };
        let target = match offering {
            UriToken::Service => Target::Service(ServiceDetails {
                vendor: super::Vendor::Custom(vendor.to_string()),
                service: Service::Other {
                    service_type: name.to_string(),
                    ver: ver.to_string(),
                },
            }),
            UriToken::Device => Target::Device(DeviceDetails {
                vendor: super::Vendor::Standard,
                device: super::Device::Other {
                    device_type: name.to_string(),
                    ver: ver.to_string(),
                },
            }),
            _ => return Err(err()),
        };
        Ok(target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr)]
/// Known valuable URI tokens.
pub enum UriToken {
    Urn,
    Device,
    Service,
}

#[cfg(test)]
mod tests {
    use crate::message::{Device, Vendor};

    use super::*;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;

    #[test]
    fn known_uri_token() {
        let prefix = "urn";
        let token: UriToken = prefix.parse().expect("urn");
        assert_matches!(token, UriToken::Urn);
    }

    #[test]
    fn urn_for_service() {
        let st = "urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1";
        let urn: Target = st.parse().expect("is urn");
        assert_matches!(urn, Target::Service(ref s)
            if matches!(&s.vendor, Vendor::Custom(v)
                if v == "microsoft.com"
            )
            && matches!(&s.service, Service::Other { service_type, ver }
                if service_type == "X_MS_MediaReceiverRegistrar" && ver == "1"
            )
        );
    }

    #[test]
    fn urn_for_std_device() {
        let st = "urn:schemas-upnp-org:device:MediaServer:1";
        let urn: Target = st.parse().expect("is urn");
        assert_matches!(urn, Target::Device(ref d)
            if matches!(&d.vendor, Vendor::Standard)
            && matches!(&d.device, Device::Other { device_type, ver }
                if device_type == "MediaServer" && ver == "1"
            )
        );
    }
}
