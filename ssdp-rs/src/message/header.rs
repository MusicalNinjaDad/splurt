//! Header functionality:
//! - [UpnpHeader] for storing & finding values
//! - enums / structs for all standard header fields with standard key name, parsing & display logic

// impl ordering:
// - Header
// - Default
// - FromStr
// - (Try_)From T for Self (alphabetical)
// - (Try_)From Self for T (alphabetical)
// - Display

use std::{
    collections::HashMap,
    fmt::Display,
    io,
    net::{AddrParseError, SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
    time::Duration,
};

use uuid::Uuid;

use crate::{MULTICAST, SSDP_PORT};

use super::{DeviceDetails, ErrorKind, ParseError, ServiceDetails, SsdpNss, Target, UpnpNss, Uri};

pub struct UpnpHeader<'h>(HashMap<&'h str, &'h str>);

// TODO: #42 handle header key case sensitivity and maintain round-tripping
//   at the same time, also handle split_once(": ") skips headers without a value (like EXT:)
//   via split(":") & trim
impl<'h> FromIterator<&'h str> for UpnpHeader<'h> {
    fn from_iter<T: IntoIterator<Item = &'h str>>(iter: T) -> Self {
        let hashmap = iter
            .into_iter()
            .filter_map(|line| line.split_once(": "))
            .collect();
        Self(hashmap)
    }
}

impl<'h> UpnpHeader<'h> {
    /// Attempt to get the corresponding value for `key`, returning a [ParseError::MissingField]
    /// if unsuccessful.
    pub fn try_get(&self, key: &str) -> Result<&str, ErrorKind> {
        self.0
            .get(key)
            .ok_or_else(|| ErrorKind::MissingField(key.to_string()))
            .copied()
    }

    /// Attempt to get the value for `key`, returning `None` if unsuccessful.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).copied()
    }
}

/// Marker trait for Upnp header fields, with details of the relevant key
pub trait Header {
    /// Key as per spec
    const HEADER_KEY: &'static str;
}

/// Handles constructing valid header lines.
///
/// This is a separate trait from [Header] to allow for it to also be implemented on `Option<H>`
pub trait HeaderExt {
    /// Generate a valid header line
    fn to_header(&self) -> String;

    /// Write a valid header line to `f` including new-line
    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<H: Header + Display> HeaderExt for H {
    fn to_header(&self) -> String {
        format!("{}: {}", Self::HEADER_KEY, self)
    }

    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.to_header())
    }
}

impl<H: Header + HeaderExt> HeaderExt for Option<H> {
    /// Generate a valid header line
    ///
    /// #### Note
    /// - This will output an empty `String` for `None`.
    ///   If this is not what you want consider using `.map(|h| h.to_header())` which
    ///   will give you an `Option<String>` instead.
    fn to_header(&self) -> String {
        match self {
            Some(header) => header.to_header(),
            None => String::new(),
        }
    }

    /// Write a valid header line to `f` including new-line
    ///
    /// #### Note
    /// - `None` entries are handled nicely (no-op) *without* generating a blank line
    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Some(header) => header.write_header(f),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// TODO make private inner when impl FromStr
pub struct FriendlyName(pub String);

impl Header for FriendlyName {
    const HEADER_KEY: &'static str = "CPFN.UPNP.ORG";
}

// TODO Replace with FromStr
impl From<&str> for FriendlyName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Display for FriendlyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Host {
    V4(SocketAddrV4),
    /// IPv6 is currently untested & largely unimplemented
    _V6(SocketAddrV6),
}

impl Header for Host {
    const HEADER_KEY: &'static str = "HOST";
}

impl Default for Host {
    fn default() -> Self {
        MULTICAST.into()
    }
}

impl FromStr for Host {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let addr = SocketAddr::from_str(s)?;
        Ok(addr.into())
    }
}

impl From<SocketAddr> for Host {
    fn from(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(socket_addr_v4) => Self::V4(socket_addr_v4),
            SocketAddr::V6(socket_addr_v6) => Self::_V6(socket_addr_v6),
        }
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::V4(socket_addr_v4) => write!(f, "{socket_addr_v4}"),
            Host::_V6(socket_addr_v6) => write!(f, "{socket_addr_v6}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Man {
    Discover,
}

impl Header for Man {
    const HEADER_KEY: &'static str = "MAN";
}

impl Display for Man {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Man::Discover => "ssdp:discover",
        };
        // MAN values are enclosed in double-quotes
        write!(f, r#""{}""#, str)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaxAge(Duration);

impl Header for MaxAge {
    const HEADER_KEY: &'static str = "CACHE-CONTROL";
}

impl FromStr for MaxAge {
    type Err = ErrorKind;

    fn from_str(max_age: &str) -> Result<Self, Self::Err> {
        let (_, secs) = max_age
            .split_once("max-age=")
            .ok_or_else(|| ErrorKind::InvalidDuration(max_age.to_string()))?;
        let duration = Duration::from_secs(
            secs.parse()
                .map_err(|_| ErrorKind::InvalidDuration(max_age.to_string()))?,
        );
        Ok(Self(duration))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// A valid MX value (0..=5) (see UPnP spec para 1.3.2)
///
/// - Construct via `TryFrom<u8>`
/// - Desconstruct via `Into<u8>`
/// - Invalid values will result in an `io::ErrorKind::InvalidInput`.
pub struct Mx(u8);

impl Header for Mx {
    const HEADER_KEY: &'static str = "MX";
}

impl TryFrom<u8> for Mx {
    type Error = io::Error;

    fn try_from(mx: u8) -> Result<Self, Self::Error> {
        match mx {
            0..=5 => Ok(Self(mx)),
            _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
        }
    }
}

impl From<Mx> for u8 {
    fn from(mx: Mx) -> Self {
        mx.0
    }
}

impl Display for Mx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Search Target
pub enum ST {
    /// `ssdp:all`: Search for all devices and services.
    All,
    /// `upnp:rootdevice`: Search for root devices only.
    Root,
    /// uuid:device-UUID: Search for a particular device.
    Uuid(Uuid),
    /// `urn:schemas-upnp-org:device:deviceType:ver`:
    ///     Search for any device of this type where `deviceType` and `ver` are
    ///     defined by the UPnP Forum working committee.
    /// `urn:domain-name:device:deviceType:ver`:
    ///     Search for any device of this typewhere domain-name (a Vendor Domain Name),
    ///     deviceType and ver are defined by the UPnP vendor and ver specifies the highest
    ///     specifies the highest supported version of the device type. Period characters in
    ///     the Vendor Domain Name shall be replaced with hyphens in accordance with RFC 2141.
    /// TODO: #36 DeviceTypes
    Device(DeviceDetails),
    /// `urn:schemas-upnp-org:service:serviceType:ver`:
    ///     Search for any service of this type where serviceType and ver are
    ///     defined by the UPnP Forum working committee.
    /// `urn:domain-name:service:serviceType:ver`:
    ///     Search for any service of this type. Where domain-name (a Vendor Domain Name),
    ///     serviceType and ver are defined by the UPnP vendor and ver specifies the highest
    ///     specifies the highest supported version of the service type. Period characters in
    ///     the Vendor Domain Name shall be replaced with hyphens in accordance with RFC 2141.
    /// TODO: #37 ServiceTypes
    Service(ServiceDetails),
}

impl Header for ST {
    const HEADER_KEY: &'static str = "ST";
}

impl FromStr for ST {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = s.parse()?;
        match uri {
            Uri::Ssdp(SsdpNss::All) => Ok(ST::All),
            Uri::Upnp(UpnpNss::RootDevice) => Ok(ST::Root),
            Uri::Urn(Target::Device(device)) => Ok(ST::Device(device)),
            Uri::Urn(Target::Service(service)) => Ok(ST::Service(service)),
            // TODO: parse UUID
        }
    }
}

impl Display for ST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ST::All => write!(f, "ssdp:all"),
            ST::Root => write!(f, "upnp:rootdevice"),
            ST::Uuid(uuid) => write!(f, "uuid:device-{}", uuid),
            ST::Device(device_details) => write!(f, "urn:{device_details}"),
            ST::Service(service_details) => write!(f, "urn:{service_details}"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// The port specified in a Upnp message.
///
/// Treat this like a semantically specific `Option<u16>` with a valuable implementation of
/// `Default`, `From<Option<u16>>` & `Into<u16>`.
pub enum UpnpPort {
    /// Specifically defined value.
    ///
    /// If this is set to the default [SSDP_PORT], then it means the message specifically
    /// defined that value.
    Defined(u16),
    /// A value was not defined. Conversion to a `u16` will provide the default [SSDP_PORT]
    #[default]
    Default,
}

impl Header for UpnpPort {
    const HEADER_KEY: &'static str = "SEARCHPORT.UPNP.ORG";
}

/// `None` maps to `Default`
impl TryFrom<Option<&str>> for UpnpPort {
    type Error = ErrorKind;

    fn try_from(port: Option<&str>) -> Result<Self, Self::Error> {
        Ok(port
            .map(|port| {
                port.parse::<u16>()
                    .map_err(|_| ErrorKind::InvalidPort(port.to_string()))
            })
            .transpose()?
            .into())
    }
}
/// `None` maps to `Default`
impl From<Option<u16>> for UpnpPort {
    fn from(port: Option<u16>) -> Self {
        match port {
            Some(port) => Self::Defined(port),
            None => Self::Default,
        }
    }
}

/// Provides:
/// - `Defined(port)`: the defined port
/// - `Default`: [SSDP_PORT]
impl From<UpnpPort> for u16 {
    fn from(port: UpnpPort) -> Self {
        match port {
            UpnpPort::Defined(p) => p,
            UpnpPort::Default => SSDP_PORT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserAgent<const FIELD_NAME: &'static str> {
    pub os: String,
    pub os_version: String,
    pub upnp_version: String,
    pub product_name: String,
    pub product_version: String,
}

impl<const FIELD_NAME: &'static str> Header for UserAgent<FIELD_NAME> {
    const HEADER_KEY: &'static str = FIELD_NAME;
}

impl<const _FN: &'static str> FromStr for UserAgent<_FN> {
    type Err = ErrorKind;

    fn from_str(user_agent: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidUserAgent(user_agent.to_string());
        // Wierd & slightly backwards splitting as product_name may contain spaces
        let mut token_ish = user_agent.split("/");
        let os = token_ish.next().ok_or_else(err)?.to_string();
        // TODO: Check there was a "Upnp" in the right place
        // TODO: Handle cases with comma separation "Linux/2.6.32.12, UPnP/1.0, Portable SDK for UPnP devices/1.6.21"
        let (os_version, _upnp) = token_ish
            .next()
            .ok_or_else(err)?
            .split_once(" ")
            .ok_or_else(err)
            .map(|(ver, upnp)| (ver.to_string(), upnp))?;
        let (upnp_version, product_name) = token_ish
            .next()
            .ok_or_else(err)?
            .split_once(" ")
            .ok_or_else(err)
            .map(|(ver, prod)| (ver.to_string(), prod.to_string()))?;
        let product_version = token_ish.next().ok_or_else(err)?.to_string();
        if token_ish.next().is_some() {
            return Err(err());
        };
        Ok(Self {
            os,
            os_version,
            upnp_version,
            product_name,
            product_version,
        })
    }
}

/// Formatted as per OCF specification (2020) section 1.3.2 for the `USER-AGENT` *value*,
/// does NOT include the header key
impl<const _FN: &'static str> Display for UserAgent<_FN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            os,
            os_version,
            upnp_version,
            product_name,
            product_version,
        } = self;
        write!(
            f,
            "{os}/{os_version} UPnP/{upnp_version} {product_name}/{product_version}"
        )
    }
}

impl Header for Uuid {
    const HEADER_KEY: &'static str = "CPUUID.UPNP.ORG";
}
