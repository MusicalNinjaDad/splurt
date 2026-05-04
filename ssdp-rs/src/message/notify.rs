//! `NOTIFY *` messages

use std::{fmt::Display, net::SocketAddr, str::FromStr};

use url::Url;
use uuid::Uuid;

use crate::{
    MULTICAST,
    message::{DeviceDetails, Header, MaxAge, ServiceDetails, Target, UpnpNss, UserAgent},
};

use super::{ErrorKind, ParseError, SsdpNss, UpnpHeader, Uri};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive(Alive),
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let nts = header.try_get("NTS")?.parse::<Uri>()?.try_into()?;
        let host =
            try bikeshed Result<_, ErrorKind> { header.try_get("HOST")?.parse::<SocketAddr>()? };
        // Host MUST be Multicast address as per spec
        match host {
            Ok(addr) if addr == MULTICAST => (),
            Ok(addr) => Err(ErrorKind::InvalidHost(addr.to_string()))?,
            Err(err) if matches!(err, ErrorKind::MissingField(_)) => Err(err)?,
            Err(_err) => todo!("chain"),
        }
        let max_age = header.try_get(MaxAge::HEADER_KEY)?.parse()?;
        let location = header.try_get("LOCATION")?;
        let location = location
            .parse()
            .map_err(|_| ErrorKind::InvalidLocation(location.to_string()))?;
        let nt = header.try_get(NT::HEADER_KEY)?.parse()?;
        let server = header.try_get("SERVER")?.parse()?;
        match nts {
            NTS::Alive => Ok(Self::Alive(Alive {
                max_age,
                location,
                nt,
                server,
            })),
            #[expect(unreachable_patterns)]
            _ => todo!("tryfrom header for notify other NTS e.g. byebye"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alive {
    /// `CACHE-CONTROL`: Duration (in seconds) until advertisement expires
    pub(crate) max_age: MaxAge,
    /// `URL` for UPnP description for root device
    pub(crate) location: Url,
    /// `NT`: notification type
    pub(crate) nt: NT,
    /// `SERVER`: OS/version UPnP/2.0 product/version
    pub(crate) server: UserAgent<"SERVER">,
}

/// The NT values available for NOTIFY. This should usually be refered to as `notify::NT`
/// and not brought directly into scope via `use notify::NT` in order to disambiguate from
/// `NT` values for other message types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NT {
    /// `upnp:rootdevice`: Sent once for root device.
    RootDevice,
    /// `uuid:device-UUID`: Sent once for each device, root or embedded, where
    /// `device-UUID` is specified by the UPnP vendor.
    Uuid(Uuid),
    /// `urn:schemas-upnp-org:device:deviceType:ver`:
    ///     Sent once for each device, root or embedded, where deviceType and ver are defined by
    ///     UPnP Forum working committee, and ver specifies the version of the device type.
    /// `urn:domain-name:device:deviceType:ver`:
    ///     Sent once for each device, root or embedded, where domain-name is a Vendor Domain
    ///     Name, deviceType and ver are defined by the UPnP vendor, and ver specifies the version
    ///     of the device type. Period characters in the Vendor Domain Name shall be replaced with
    ///     hyphens in accordance with RFC 2141.
    /// TODO: #36 DeviceTypes
    Device(DeviceDetails),
    /// `urn:schemas-upnp-org:service:serviceType:ver`:
    ///     Sent once for each service where serviceType and ver are defined by UPnP Forum working
    ///     committee and ver specifies the version of the service type.
    /// `urn:domain-name:service:serviceType:ver`:
    ///     Sent once for each service where domain-name is a Vendor Domain Name, serviceType and
    ///     ver are defined by UPnP vendor, and ver specifies the version of the service type.
    ///     Period characters in the Vendor Domain Name shall be replaced with hyphens in
    ///     accordance with RFC 2141.
    /// TODO: #37 ServiceTypes
    Service(ServiceDetails),
}

impl Header for NT {
    const HEADER_KEY: &'static str = "NT";
}

impl FromStr for NT {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = s.parse()?;
        match uri {
            Uri::Upnp(UpnpNss::RootDevice) => Ok(Self::RootDevice),
            Uri::Urn(Target::Device(device)) => Ok(Self::Device(device)),
            Uri::Urn(Target::Service(service)) => Ok(Self::Service(service)),
            // TODO: parse UUID
            _ => Err(ErrorKind::InvalidNT(s.to_string()))?,
        }
    }
}

impl Display for NT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootDevice => write!(f, "upnp:rootdevice"),
            Self::Uuid(uuid) => write!(f, "uuid:device-{}", uuid),
            Self::Device(device_details) => write!(f, "urn:{device_details}"),
            Self::Service(service_details) => write!(f, "urn:{service_details}"),
        }
    }
}

/// The NTS values available for NOTIFY. This should usually be refered to as `notify::NTS`
/// and not brought directly into scope via `use notify::NTS` in order to disambiguate from
/// `NTS` values which may be added in future for other message types (e.g. for eventing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NTS {
    Alive,
}

impl TryFrom<Uri> for NTS {
    type Error = ErrorKind;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri {
            Uri::Ssdp(SsdpNss::Alive) => Ok(Self::Alive),
            _ => Err(ErrorKind::InvalidNTS(uri.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_ssdp_alive() {
        let output = format!("{}", Uri::Ssdp(SsdpNss::Alive));
        assert_eq!(output, "ssdp:alive");
    }
}
