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
    net::{AddrParseError, SocketAddr},
    str::FromStr,
    time::Duration,
};

use derive_more::Display;
use url::Url;
use uuid::Uuid;

use crate::{MULTICAST, SSDP_PORT};

use super::{DeviceDetails, ErrorKind, ParseError, ServiceDetails, SsdpNss, Target, UpnpNss, Uri};

pub struct UpnpHeader<'h>(HashMap<String, HeaderEntry<'h>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeaderEntry<'h> {
    key: &'h str,
    val: &'h str,
}

// TODO: #42 maintain round-tripping
//   at the same time, also handle split_once(": ") skips headers without a value (like EXT:)
//   via split(":") & trim
impl<'h> FromIterator<&'h str> for UpnpHeader<'h> {
    fn from_iter<T: IntoIterator<Item = &'h str>>(iter: T) -> Self {
        let hashmap = iter
            .into_iter()
            .filter_map(|line| {
                line.split_once(": ")
                    .map(|(key, val)| (key.to_uppercase(), HeaderEntry { key, val }))
            })
            .collect();
        Self(hashmap)
    }
}

impl<'h> UpnpHeader<'h> {
    /// Attempt to get the corresponding value for `key`, returning an [ErrorKind::MissingField]
    /// if unsuccessful.
    pub fn try_get(&self, key: &str) -> Result<&str, ErrorKind> {
        self.get(key)
            .ok_or_else(|| ErrorKind::MissingField(key.to_string()))
    }

    /// Attempt to get the value for `key`, returning `None` if unsuccessful.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(&key.to_uppercase()).map(|entry| entry.val)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BootId(Option<u32>);

impl Header for BootId {
    const HEADER_KEY: &'static str = "BOOTID.UPNP.ORG";
}

impl TryFrom<Option<&str>> for BootId {
    type Error = ErrorKind;

    fn try_from(id: Option<&str>) -> Result<Self, Self::Error> {
        match id {
            Some(id) => Ok(Self(Some(
                id.parse()
                    .map_err(|_| ErrorKind::InvalidBootId(id.to_string()))?,
            ))),
            None => Ok(Self(None)),
        }
    }
}

impl BootId {
    pub fn as_option(&self) -> &Option<u32> {
        &self.0
    }

    pub fn validate(self, upnp_version: Version) -> Result<Self, ErrorKind> {
        match upnp_version.major {
            ..=1 => Ok(self),
            2.. => match self.as_option() {
                Some(_) => Ok(self),
                None => Err(ErrorKind::MissingBootId),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfigId(Option<u32>);

impl Header for ConfigId {
    const HEADER_KEY: &'static str = "CONFIGID.UPNP.ORG";
}

impl TryFrom<Option<&str>> for ConfigId {
    type Error = ErrorKind;

    fn try_from(id: Option<&str>) -> Result<Self, Self::Error> {
        match id {
            Some(id) => {
                Ok(Self(Some(id.parse().map_err(|_| {
                    ErrorKind::InvalidConfigId(id.to_string())
                })?)))
            }
            None => Ok(Self(None)),
        }
    }
}

impl ConfigId {
    pub fn as_option(&self) -> &Option<u32> {
        &self.0
    }

    pub fn validate(self, upnp_version: Version) -> Result<Self, ErrorKind> {
        match upnp_version.major {
            ..=1 => Ok(self),
            2.. => match self.as_option() {
                Some(_) => Ok(self),
                None => Err(ErrorKind::MissingBootId),
            },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
// TODO: A way to get the addr back out
pub struct Host(SocketAddr);

impl Header for Host {
    const HEADER_KEY: &'static str = "HOST";
}

impl Default for Host {
    fn default() -> Self {
        MULTICAST.into()
    }
}

impl FromStr for Host {
    type Err = ParseError;

    fn from_str(hostname: &str) -> Result<Self, Self::Err> {
        let addr = hostname
            .parse::<SocketAddr>()
            .map_err(|err: AddrParseError| {
                ParseError::chain_from(
                    ErrorKind::from(err).into(),
                    ErrorKind::InvalidHost(hostname.to_string()),
                )
            })?;
        Ok(addr.into())
    }
}

impl From<SocketAddr> for Host {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

impl Host {
    /// Return [ErrorKind::InvalidHost] if not [MULTICAST]
    pub fn check_multicast(&self) -> Result<(), ErrorKind> {
        match self.0 {
            MULTICAST => Ok(()),
            _ => Err(ErrorKind::InvalidHost(self.0.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub struct Location(Url);

impl Header for Location {
    const HEADER_KEY: &'static str = "LOCATION";
}

impl FromStr for Location {
    type Err = ErrorKind;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        match url.parse() {
            Ok(url) => Ok(Self(url)),
            Err(_) => Err(ErrorKind::InvalidLocation(url.to_string())),
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaxAge(pub(crate) Duration);

impl Header for MaxAge {
    const HEADER_KEY: &'static str = "CACHE-CONTROL";
}

impl FromStr for MaxAge {
    // TODO: #50 Move all FromStr impls to use Err = ParseError
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductTokens<const FIELD_NAME: &'static str> {
    pub os: String,
    pub os_version: String,
    pub upnp_version: Version,
    pub product_name: String,
    pub product_version: String,
}

impl<const FIELD_NAME: &'static str> Header for ProductTokens<FIELD_NAME> {
    const HEADER_KEY: &'static str = FIELD_NAME;
}

impl<const _FLD: &'static str> FromStr for ProductTokens<_FLD> {
    type Err = ErrorKind;

    fn from_str(user_agent: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidUserAgent(user_agent.to_string());
        // Wierd & slightly backwards splitting as product_name may contain spaces
        let mut token_ish = user_agent.split("/");
        let os = token_ish.next().ok_or_else(err)?.to_string();
        // TODO: Check there was a "Upnp" in the right place
        let (os_version, _upnp) = token_ish
            .next()
            .ok_or_else(err)?
            .split_once(" ")
            .ok_or_else(err)
            .map(|(ver, upnp)| {
                (
                    ver.trim_end_matches(|c: char| !c.is_alphanumeric())
                        .to_string(),
                    upnp,
                )
            })?;
        let (ver, prod) = token_ish
            .next()
            .ok_or_else(err)?
            .split_once(" ")
            .ok_or_else(err)?;
        let upnp_version = ver
            .trim_end_matches(|c: char| !c.is_alphanumeric())
            .parse()?;
        let product_name = prod.to_string();
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
impl<const _FLD: &'static str> Display for ProductTokens<_FLD> {
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecureLocation(Option<Url>);

impl Header for SecureLocation {
    const HEADER_KEY: &'static str = "SECURELOCATION.UPNP.ORG";
}

impl TryFrom<Option<&str>> for SecureLocation {
    type Error = ErrorKind;

    fn try_from(url: Option<&str>) -> Result<Self, Self::Error> {
        match url {
            Some(url) => {
                Ok(Self(Some(url.parse().map_err(|_| {
                    ErrorKind::InvalidSecureLocation(url.to_string())
                })?)))
            }
            None => Ok(Self(None)),
        }
    }
}

impl SecureLocation {
    pub fn as_option(&self) -> &Option<Url> {
        &self.0
    }

    pub fn validate(&self) -> Result<&Self, ErrorKind> {
        match self.as_option() {
            None => Ok(self),
            Some(secure_location)
                if secure_location.scheme() == "https" && secure_location.port().is_some() =>
            {
                Ok(self)
            }
            Some(insecure_location) => Err(ErrorKind::InvalidSecureLocation(
                insecure_location.to_string(),
            )),
        }
    }
}

pub type Server = ProductTokens<"SERVER">;

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
            _ => Err(ErrorKind::InvalidST(s.to_string()))?,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

pub type UserAgent = ProductTokens<"USER-AGENT">;

impl Header for Uuid {
    const HEADER_KEY: &'static str = "CPUUID.UPNP.ORG";
}

/// Upnp version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl FromStr for Version {
    type Err = ErrorKind;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidVersion(v.to_string());
        let mut parts = v.split(".");
        let major = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
        let minor = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
        if parts.next().is_some() {
            return Err(err());
        }
        Ok(Self { major, minor })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
