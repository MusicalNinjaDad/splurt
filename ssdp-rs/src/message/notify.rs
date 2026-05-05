//! `NOTIFY *` messages

use std::{fmt::Display, net::AddrParseError, str::FromStr};

use url::Url;
use uuid::Uuid;

use crate::{
    MULTICAST,
    message::{
        DeviceDetails, Header, MaxAge, ServiceDetails, Target, UpnpNss, UpnpPort, UserAgent,
        uri::UriExt,
    },
};

use super::{ErrorKind, ParseError, SsdpNss, UpnpHeader, Uri};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive(Alive),
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let hostname = header.try_get("HOST")?;
        let host = hostname.parse().map_err(|err: AddrParseError| {
            ParseError::chain_from(
                ErrorKind::from(err).into(),
                ErrorKind::InvalidHost(hostname.to_string()),
            )
        })?;
        // Host MUST be Multicast address as per spec
        match host {
            MULTICAST => (),
            _ => Err(ErrorKind::InvalidHost(hostname.to_string()))?,
        }
        let max_age = header.try_get(MaxAge::HEADER_KEY)?.parse()?;
        let location = header.try_get("LOCATION")?;
        let location = location
            .parse()
            .map_err(|_| ErrorKind::InvalidLocation(location.to_string()))?;
        let nt = header.try_get(NT::HEADER_KEY)?.parse()?;
        let nts = header
            .try_get(NTS::HEADER_KEY)?
            .parse::<Uri>()?
            .try_into()?;
        let server: UserAgent<"SERVER"> = header.try_get("SERVER")?.parse()?;
        let usn = header.try_get("USN")?.parse()?;
        let uuid = match usn {
            Uri::Uuid { uuid, suffix }
                if matches!(nt, NT::Uuid(nt_uuid) if uuid == nt_uuid) && suffix.is_none()
                    || matches!(&suffix, Some(uri) if **uri == nt.to_uri()) =>
            {
                uuid
            }
            _ => Err(ErrorKind::InvalidUsn(usn.to_string()))?,
        };
        let boot_id = header
            .get("BOOTID.UPNP.ORG")
            .map(|boot_id| {
                boot_id
                    .parse()
                    .map_err(|_| ErrorKind::InvalidBootId(boot_id.to_string()))
            })
            .transpose()?;
        let config_id = header
            .get("CONFIGID.UPNP.ORG")
            .map(|config_id| {
                config_id
                    .parse()
                    .map_err(|_| ErrorKind::InvalidConfigId(config_id.to_string()))
            })
            .transpose()?;
        let port = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let secure_location: Option<Url> = header
            .get("SECURELOCATION.UPNP.ORG")
            .map(|location| {
                location
                    .parse()
                    .map_err(|_| ErrorKind::InvalidSecureLocation(location.to_string()))
            })
            .transpose()?;
        if let Some(ref secure_location) = secure_location
            && (secure_location.scheme() != "https" || secure_location.port().is_none())
        {
            Err(ErrorKind::InvalidSecureLocation(
                secure_location.to_string(),
            ))?;
        };
        match server.upnp_version.as_str() {
            // TODO parse the version number into Major,Minor
            "1.0" => (),
            _ => {
                if boot_id.is_none() {
                    Err(ErrorKind::MissingBootId)?;
                }
                if config_id.is_none() {
                    Err(ErrorKind::MissingConfigId)?;
                }
            }
        };
        match nts {
            NTS::Alive => Ok(Self::Alive(Alive {
                max_age,
                location,
                nt,
                server,
                uuid,
                boot_id,
                config_id,
                port,
                secure_location,
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
    /// UUID extracted from `USN`
    /// TODO: Validate match for NT::Uuid
    pub(crate) uuid: Uuid,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    boot_id: Option<u32>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    config_id: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    secure_location: Option<Url>,
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

impl UriExt for NT {
    fn to_uri(&self) -> Uri {
        match self {
            NT::RootDevice => Uri::Upnp(UpnpNss::RootDevice),
            NT::Uuid(uuid) => Uri::Uuid {
                uuid: *uuid,
                suffix: None,
            },
            NT::Device(device_details) => Uri::Urn(Target::Device(device_details.clone())),
            NT::Service(service_details) => Uri::Urn(Target::Service(service_details.clone())),
        }
    }
}

// TODO: Reflexive version for uri::NT
impl PartialEq<Uri> for NT {
    fn eq(&self, uri_nt: &Uri) -> bool {
        self.to_uri() == *uri_nt
    }
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

impl Header for NTS {
    const HEADER_KEY: &'static str = "NTS";
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
