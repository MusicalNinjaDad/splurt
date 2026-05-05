//! Specific URI handling as used for the NT, ST & USN fields

use std::{fmt::Display, str::FromStr};

use derive_more::{Display, FromStr};
use uuid::Uuid;

use super::{Device, DeviceDetails, ErrorKind, ParseError, Service, ServiceDetails};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromStr)]
/// Known valuable URI tokens.
pub enum UriToken {
    Ssdp,
    Upnp,
    Urn,
    Uuid,
    Device,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Known Urn schemes
pub enum Uri {
    Ssdp(SsdpNss),
    Upnp(UpnpNss),
    Urn(Target),
    Uuid {
        uuid: Uuid,
        suffix: Option<Box<Uri>>,
    },
}

impl Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Uri::Ssdp(ssdp_nss) => write!(f, "{ssdp_nss}"),
            Uri::Upnp(upnp_nss) => write!(f, "{upnp_nss}"),
            Uri::Urn(target) => write!(f, "{target}"),
            Uri::Uuid { uuid, suffix } => match suffix {
                Some(suffix) => write!(f, "uuid:{uuid}::{suffix}"),
                None => write!(f, "uuid:{uuid}"),
            },
        }
    }
}

/// For types which represent a subset or specific usage of a valid Upnp Uri.
pub trait UriExt {
    /// Provides infallible conversion to Uri e.g. for comparison with other `UriExt` types,
    /// `Display` etc.
    fn to_uri(&self) -> Uri;
}

impl UriExt for Uri {
    fn to_uri(&self) -> Uri {
        self.clone()
    }
}

impl FromStr for Uri {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(":");
        Self::from_parts(&mut parts, s)
    }
}

impl Uri {
    ///Construct a Uri from an iterator over the &str parts. Expecting this to be coming from a
    /// call to `.split(":")`
    pub fn from_parts<'s, P>(parts: &mut P, s: &'s str) -> Result<Self, ParseError>
    where
        P: Iterator<Item = &'s str>,
    {
        let err = || ErrorKind::InvalidUrn(s.to_string());
        let chain = |e: ErrorKind| ParseError::chain_from(e.into(), err());
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
            UriToken::Uuid => {
                let uuid = try bikeshed Result<_, ErrorKind> {
                    parts.next().ok_or_else(err)?.parse::<Uuid>()?
                }?;
                match parts.next() {
                    None => Ok(Self::Uuid { uuid, suffix: None }),
                    Some("") => {
                        let nt = Some(Box::new(Uri::from_parts(parts, s)?));
                        Ok(Self::Uuid { uuid, suffix: nt })
                    }
                    Some(_) => Err(ErrorKind::InvalidUsn(s.to_string()))?,
                }
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
    ByeBye,
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
    use super::*;

    use crate::message::{Device, Vendor};

    use uuid::uuid;

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

    #[test]
    fn display_usn_none() {
        let usn = Uri::Uuid {
            uuid: uuid!("fd6e74c3-9c89-4fd0-bf52-994af57b5d40"),
            suffix: None,
        };
        assert_eq!(
            format!("{usn}"),
            "uuid:fd6e74c3-9c89-4fd0-bf52-994af57b5d40"
        );
    }

    #[test]
    fn display_usn_root() {
        let usn = Uri::Uuid {
            uuid: uuid!("fd6e74c3-9c89-4fd0-bf52-994af57b5d40"),
            suffix: Some(Box::new(Uri::Upnp(UpnpNss::RootDevice))),
        };
        assert_eq!(
            format!("{usn}"),
            "uuid:fd6e74c3-9c89-4fd0-bf52-994af57b5d40::upnp:rootdevice"
        );
    }

    #[test]
    fn display_usn_device() {
        let target = DeviceDetails {
            vendor: Vendor::Standard,
            device: Device::MediaServer { ver: "4".into() },
        };
        let usn = Uri::Uuid {
            uuid: uuid!("fd6e74c3-9c89-4fd0-bf52-994af57b5d40"),
            suffix: Some(Box::new(Uri::Urn(Target::Device(target)))),
        };
        assert_eq!(
            format!("{usn}"),
            "uuid:fd6e74c3-9c89-4fd0-bf52-994af57b5d40::schemas-upnp-org:device:MediaServer:4"
        );
    }
}
