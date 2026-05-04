//! Specific URI handling as used for the NT, ST & USN fields

use std::str::FromStr;

use derive_more::{Display, FromStr};

use super::{Device, DeviceDetails, ErrorKind, ParseError, Service, ServiceDetails};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr)]
/// Known valuable URI tokens.
pub enum UriToken {
    Ssdp,
    Upnp,
    Urn,
    Device,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
/// Known Urn schemes
pub enum Uri {
    Ssdp(SsdpNss),
    Upnp(UpnpNss),
    Urn(Target),
}

impl FromStr for Uri {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidUrn(s.to_string());
        let chain = |e: ErrorKind| ParseError::chain_from(e.into(), err());
        let mut parts = s.split(":");

        let prefix = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
        match prefix {
            UriToken::Ssdp => {
                let nss = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
                Ok(Self::Ssdp(nss))
            }
            UriToken::Upnp => {
                let nss = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
                Ok(Self::Upnp(nss))
            }
            UriToken::Urn => {
                let Ok(vendor) = parts.next().ok_or_else(err)?.parse();
                let offering = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;

                let target = match offering {
                    UriToken::Service => Target::Service(ServiceDetails {
                        vendor,
                        service: Service::from_parts(parts).map_err(chain)?,
                    }),
                    UriToken::Device => Target::Device(DeviceDetails {
                        vendor,
                        device: Device::from_parts(parts).map_err(chain)?,
                    }),
                    _ => Err(err())?,
                };
                Ok(Self::Urn(target))
            }
            _ => todo!("parse other types"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr, Display)]
#[display("ssdp:{_variant}")]
#[display(rename_all = "lowercase")]
/// Known ssdp namespace specific strings
pub enum SsdpNss {
    All,
    Alive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
// TODO: Check derived Display output
pub enum Target {
    Device(DeviceDetails),
    Service(ServiceDetails),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr, Display)]
#[display("upnp:{_variant}")]
#[display(rename_all = "lowercase")]
/// Known upnp namespace specific strings
pub enum UpnpNss {
    RootDevice,
}

#[cfg(test)]
mod tests {
    use crate::message::{Device, Vendor};

    use super::*;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;
    use std::error::Error;

    #[test]
    fn known_uri_token() {
        let prefix = "urn";
        let token: UriToken = prefix.parse().expect("urn");
        assert_matches!(token, UriToken::Urn);
    }

    #[test]
    fn urn_for_service() {
        let st = "urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1";
        let urn = st.parse().expect("is urn");
        assert_matches!(urn, Uri::Urn(target)
            if matches!(target, Target::Service(ref s)
                if matches!(&s.vendor, Vendor::Custom(v)
                    if v == "microsoft.com"
                )
                && matches!(&s.service, Service::Other { service_type, ver }
                    if service_type == "X_MS_MediaReceiverRegistrar" && ver == "1"
                )
            )
        );
    }

    #[test]
    fn urn_for_std_device() {
        let st = "urn:schemas-upnp-org:device:MediaServer:1";
        let urn = st.parse().expect("is urn");
        assert_matches!(urn, Uri::Urn(target)
            if matches!(target, Target::Device(ref d)
                if matches!(&d.vendor, Vendor::Standard)
                && matches!(&d.device, Device::MediaServer { ver } if ver == "1")
            )
        );
    }

    #[test]
    fn urn_no_device() {
        let st = "urn:schemas-upnp-org:device";
        let err = st.parse::<Uri>().expect_err("no device details");
        assert_matches!(&err.kind, ErrorKind::InvalidUrn(s) if s == "urn:schemas-upnp-org:device");
        let device_err = err.source().expect("inner error").downcast_ref();
        assert_matches!(device_err, Some(ParseError { kind, .. })
            if matches!(kind, ErrorKind::InvalidDevice(d) if d == "''")
        );
        println!("{err}");
    }

    #[test]
    fn test_ssdp_all() {
        let st = "ssdp:all";
        let uri = st.parse().expect("is urn");
        assert_matches!(uri, Uri::Ssdp(t) if matches!(t, SsdpNss::All))
    }

    #[test]
    fn test_upnp_rootdevice() {
        let st = "upnp:rootdevice";
        let uri = st.parse().expect("is urn");
        assert_matches!(uri, Uri::Upnp(t) if matches!(t, UpnpNss::RootDevice))
    }
}
