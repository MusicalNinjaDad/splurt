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
    net::{AddrParseError, SocketAddr},
    str::FromStr,
    time::Duration,
};

use derive_more::{Display, From, FromStr, Into};
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

// TODO #54 benchmark this then try with [unicase = "2.9.0"::Ascii]
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
#[doc(notable_trait)]
pub trait Header {
    /// Key as per spec
    const HEADER_KEY: &'static str;
}

/// Handles constructing valid header lines.
///
/// This is a separate trait from [Header] to allow for it to also be implemented on `Option<H>`
#[doc(notable_trait)]
pub trait HeaderExt {
    /// Generate a valid header line
    fn to_header(&self) -> String;

    /// Write a valid header line to `f` including new-line
    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    /// Attempt to get and parse this from a UpnpHeader
    fn get_from(header: &UpnpHeader<'_>) -> Result<Self, ParseError>
    where
        Self: Sized;
}

impl<H, E> HeaderExt for H
where
    H: Header + Display + FromStr<Err = E>,
    ParseError: From<E>,
{
    fn to_header(&self) -> String {
        format!("{}: {}", Self::HEADER_KEY, self)
    }

    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.to_header())
    }

    fn get_from(header: &UpnpHeader<'_>) -> Result<Self, ParseError> {
        Ok(header.try_get(Self::HEADER_KEY)?.parse()?)
    }
}

impl<H, E> HeaderExt for Option<H>
where
    H: Header + HeaderExt + FromStr<Err = E>,
    ParseError: From<E>,
{
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

    fn get_from(header: &UpnpHeader<'_>) -> Result<Self, ParseError>
    where
        Self: Sized,
    {
        Ok(header
            .get(H::HEADER_KEY)
            .map(|val| val.parse::<H>())
            .transpose()?)
    }
}

/// For types which are required in UPnP V2 but not V1 and can be represented as an `Option<T>`
#[doc(notable_trait)]
pub trait UpnpV2 {
    const ERR: ErrorKind;
}

pub trait UpnpV2Ext {
    fn get_validated(header: &UpnpHeader<'_>, upnp_version: Version) -> Result<Self, ParseError>
    where
        Self: Sized;
}

impl<H> UpnpV2Ext for Option<H>
where
    H: UpnpV2,
    Option<H>: HeaderExt,
{
    fn get_validated(header: &UpnpHeader<'_>, upnp_version: Version) -> Result<Self, ParseError> {
        let this = Option::<H>::get_from(header)?;
        match upnp_version.major {
            ..=1 => Ok(this),
            2.. => match this {
                Some(_) => Ok(this),
                None => Err(H::ERR)?,
            },
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into, FromStr,
)]
#[from_str(error(ErrorKind, |_| ErrorKind::InvalidBootId(s.to_string())))]
pub struct BootId(u32);

impl Header for BootId {
    const HEADER_KEY: &'static str = "BOOTID.UPNP.ORG";
}

impl UpnpV2 for BootId {
    const ERR: ErrorKind = ErrorKind::MissingBootId;
}

impl PartialEq<u32> for BootId {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl PartialEq<BootId> for u32 {
    fn eq(&self, other: &BootId) -> bool {
        *self == other.0
    }
}

impl PartialEq<NextBootId> for BootId {
    fn eq(&self, new: &NextBootId) -> bool {
        self.0 == new.0
    }
}

impl PartialOrd<NextBootId> for BootId {
    fn partial_cmp(&self, new: &NextBootId) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&new.0)
    }
}

impl BootId {
    pub fn as_u32(&self) -> &u32 {
        &self.0
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into, FromStr,
)]
#[from_str(error(ErrorKind, |_| ErrorKind::InvalidConfigId(s.to_string())))]
pub struct ConfigId(u32);

impl Header for ConfigId {
    const HEADER_KEY: &'static str = "CONFIGID.UPNP.ORG";
}

impl UpnpV2 for ConfigId {
    const ERR: ErrorKind = ErrorKind::MissingConfigId;
}

impl ConfigId {
    pub fn as_u32(&self) -> &u32 {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into)]
pub struct ControlPointUuid(Uuid);

impl Header for ControlPointUuid {
    const HEADER_KEY: &'static str = "CPUUID.UPNP.ORG";
}

impl FromStr for ControlPointUuid {
    type Err = ErrorKind;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!("fromstr controlpoint uuid")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
// TODO make private inner when impl FromStr
pub struct FriendlyName(pub String);

impl Header for FriendlyName {
    const HEADER_KEY: &'static str = "CPFN.UPNP.ORG";
}

impl UpnpV2 for FriendlyName {
    const ERR: ErrorKind = ErrorKind::MissingFriendlyName;
}

impl FromStr for FriendlyName {
    type Err = ErrorKind;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!("fromstr for friendly name")
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into)]
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

impl Host {
    /// Return [ErrorKind::InvalidHost] if not [MULTICAST]
    pub fn check_multicast(&self) -> Result<(), ErrorKind> {
        match self.0 {
            MULTICAST => Ok(()),
            _ => Err(ErrorKind::InvalidHost(self.0.to_string())),
        }
    }

    pub fn as_socket_addr(&self) -> &SocketAddr {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into, FromStr)]
#[from_str(error(ErrorKind, |_| ErrorKind::InvalidLocation(s.to_string())))]
pub struct Location(Url);

impl Header for Location {
    const HEADER_KEY: &'static str = "LOCATION";
}

impl Location {
    pub fn as_url(&self) -> &Url {
        &self.0
    }

    pub fn into_url(self) -> Url {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Man {
    Discover,
}

impl Header for Man {
    const HEADER_KEY: &'static str = "MAN";
}

impl FromStr for Man {
    type Err = ParseError;

    fn from_str(man: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidMan(man.to_string());
        let man = man.strip_circumfix('"', '"').ok_or_else(err)?;
        let man = man.parse::<Uri>()?;
        Ok(man.try_into()?)
    }
}

impl TryFrom<Uri> for Man {
    type Error = ErrorKind;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri {
            Uri::Ssdp(SsdpNss::Discover) => Ok(Man::Discover),
            _ => Err(ErrorKind::InvalidMan(uri.to_string())),
        }
    }
}

impl Man {
    /// `MAN` can (currently) only be `"ssdp:discover"`. While parsing effectively confirms this
    /// this function is provided to make code more readable in expressing this invariant and
    /// future-proof against any additions to the spec. It is a no-op which the compiler should
    /// optimise away.
    pub fn check_discover(&self) -> Result<(), ErrorKind> {
        Ok(())
    }
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
pub struct MaxAge(pub(crate) Duration);

impl Header for MaxAge {
    const HEADER_KEY: &'static str = "CACHE-CONTROL";
}

impl FromStr for MaxAge {
    // TODO: #50 Move all FromStr impls to use Err = ParseError
    type Err = ErrorKind;

    fn from_str(max_age: &str) -> Result<Self, Self::Err> {
        let err = || ErrorKind::InvalidDuration(max_age.to_string());
        let (_, secs) = max_age.split_once("max-age").ok_or_else(err)?;
        let secs = secs.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
        let duration = Duration::from_secs(secs.parse().map_err(|_| err())?);
        Ok(Self(duration))
    }
}

impl PartialEq<Duration> for MaxAge {
    fn eq(&self, other: &Duration) -> bool {
        self.0 == *other
    }
}

impl PartialEq<MaxAge> for Duration {
    fn eq(&self, other: &MaxAge) -> bool {
        *self == other.0
    }
}

impl MaxAge {
    pub fn as_duration(&self) -> &Duration {
        &self.0
    }
}

impl Display for MaxAge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_duration().as_secs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into, Display)]
/// A valid MX value (0..=5) (see UPnP spec para 1.3.2)
///
/// - Construct via `TryFrom<u8>`
/// - Desconstruct via `Into<u8>`
/// - Invalid values will result in an `io::ErrorKind::InvalidInput`.
pub struct Mx(u8);

impl Header for Mx {
    const HEADER_KEY: &'static str = "MX";
}

impl FromStr for Mx {
    type Err = ErrorKind;

    fn from_str(mx: &str) -> Result<Self, Self::Err> {
        let err = |_| ErrorKind::InvalidMx(mx.to_string());
        let mx = mx.parse::<u8>().map_err(err)?;
        let mx = mx.try_into()?;
        Ok(mx)
    }
}

impl TryFrom<u8> for Mx {
    type Error = ErrorKind;

    fn try_from(mx: u8) -> Result<Self, Self::Error> {
        match mx {
            0..=5 => Ok(Self(mx)),
            _ => Err(ErrorKind::InvalidMx(mx.to_string())),
        }
    }
}

impl Mx {
    pub fn as_u8(&self) -> &u8 {
        &self.0
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into, FromStr,
)]
#[from_str(error(ErrorKind, |_| ErrorKind::InvalidNextBootId(s.to_string())))]
pub struct NextBootId(u32);

impl Header for NextBootId {
    const HEADER_KEY: &'static str = "NEXTBOOTID.UPNP.ORG";
}

impl UpnpV2 for NextBootId {
    const ERR: ErrorKind = ErrorKind::MissingNextBootId;
}

impl PartialEq<u32> for NextBootId {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl PartialEq<NextBootId> for u32 {
    fn eq(&self, other: &NextBootId) -> bool {
        *self == other.0
    }
}

impl PartialEq<BootId> for NextBootId {
    fn eq(&self, old: &BootId) -> bool {
        self.0 == old.0
    }
}

impl PartialOrd<BootId> for NextBootId {
    fn partial_cmp(&self, old: &BootId) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&old.0)
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
        let (os_token, rest) = user_agent.split_once("UPnP/").ok_or_else(err)?;
        // TODO: How to handle removing "," while allowing "OS Foo/6.3 (wibblish)"
        // https://datatracker.ietf.org/doc/html/rfc9110#name-server for formal grammar
        let os_token = os_token.trim_matches(|c: char| !c.is_alphanumeric());
        let (os, os_version) = match os_token.split_once("/") {
            Some((os, os_version)) => (os.to_string(), os_version.to_string()),
            None => (os_token.to_string(), "".to_string()),
        };
        let (upnp_version, product) = rest.split_once(" ").ok_or_else(err)?;
        let upnp_version: Version = upnp_version
            .trim_matches(|c: char| !c.is_alphanumeric())
            .parse()?;
        let (product_name, product_version) = product
            .split_once("/")
            .ok_or_else(err)
            .map(|(name, ver)| (name.to_string(), ver.to_string()))?;
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Into)]
pub struct SecureLocation(Url);

impl Header for SecureLocation {
    const HEADER_KEY: &'static str = "SECURELOCATION.UPNP.ORG";
}

impl FromStr for SecureLocation {
    type Err = ErrorKind;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        let err = |_| ErrorKind::InvalidSecureLocation(url.to_string());
        let url = url.parse::<Url>().map_err(err)?;
        let secure_location = url.try_into()?;
        Ok(secure_location)
    }
}

impl TryFrom<Url> for SecureLocation {
    type Error = ErrorKind;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.scheme() == "https" && url.port().is_some() && !url.cannot_be_a_base() {
            Ok(Self(url))
        } else {
            Err(ErrorKind::InvalidSecureLocation(url.to_string()))
        }
    }
}

impl PartialEq<Url> for SecureLocation {
    fn eq(&self, other: &Url) -> bool {
        self.0 == *other
    }
}

impl PartialEq<SecureLocation> for Url {
    fn eq(&self, other: &SecureLocation) -> bool {
        *self == other.0
    }
}

impl SecureLocation {
    pub fn as_url(&self) -> &Url {
        &self.0
    }
    pub fn into_url(self) -> Url {
        self.0
    }
}

pub type Server = ProductTokens<"SERVER">;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Search Target
pub enum ST {
    /// `ssdp:all`: Search for all devices and services.
    All,
    /// `upnp:rootdevice`: Search for root devices only.
    RootDevice,
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
        let uri = s.parse::<Uri>()?;
        Ok(uri.try_into()?)
    }
}

impl TryFrom<Uri> for ST {
    type Error = ErrorKind;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri {
            Uri::Ssdp(SsdpNss::All) => Ok(ST::All),
            Uri::Upnp(UpnpNss::RootDevice) => Ok(ST::RootDevice),
            Uri::Urn(Target::Device(device)) => Ok(ST::Device(device)),
            Uri::Urn(Target::Service(service)) => Ok(ST::Service(service)),
            Uri::Uuid { uuid, suffix: None } => Ok(Self::Uuid(uuid)),
            _ => Err(ErrorKind::InvalidST(uri.to_string()))?,
        }
    }
}

impl PartialEq<Uri> for ST {
    fn eq(&self, uri: &Uri) -> bool {
        match self {
            Self::All => matches!(uri, Uri::Ssdp(SsdpNss::All)),
            Self::RootDevice => matches!(uri, Uri::Upnp(UpnpNss::RootDevice)),
            Self::Uuid(this_uuid) => {
                matches!(uri, Uri::Uuid { uuid, suffix: None } if uuid == this_uuid)
            }
            Self::Device(this_device) => {
                matches!(uri, Uri::Urn(Target::Device(device)) if device == this_device)
            }
            Self::Service(this_service) => {
                matches!(uri, Uri::Urn(Target::Service(service)) if service == this_service)
            }
        }
    }
}

impl PartialEq<ST> for Uri {
    fn eq(&self, st: &ST) -> bool {
        match self {
            Uri::Ssdp(SsdpNss::All) => matches!(st, ST::All),
            Uri::Upnp(UpnpNss::RootDevice) => matches!(st, ST::RootDevice),
            Uri::Uuid {
                uuid: this_uuid,
                suffix: None,
            } => matches!(st, ST::Uuid(uuid) if uuid == this_uuid),
            Uri::Urn(Target::Device(this_device)) => {
                matches!(st, ST::Device(device) if device == this_device)
            }
            Uri::Urn(Target::Service(this_service)) => {
                matches!(st, ST::Service(service) if service == this_service)
            }
            _ => false,
        }
    }
}

impl Display for ST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ST::All => write!(f, "ssdp:all"),
            ST::RootDevice => write!(f, "upnp:rootdevice"),
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

/// USN as a type to validate invariances
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Usn<NTST> {
    pub uuid: Uuid,
    pub ntst: NTST,
}

impl<_NTST> Header for Usn<_NTST> {
    const HEADER_KEY: &'static str = "USN";
}

impl<NTST, E> FromStr for Usn<NTST>
where
    NTST: TryFrom<Uri, Error = E>,
    ParseError: From<E>,
{
    type Err = ParseError;

    fn from_str(usn: &str) -> Result<Self, Self::Err> {
        let uri = usn.parse::<Uri>()?;
        match uri {
            Uri::Uuid {
                uuid,
                suffix: Some(ntst),
            } => Ok(Self {
                uuid,
                ntst: NTST::try_from(*ntst)?,
            }),
            Uri::Uuid { uuid, suffix: None } => Ok(Self {
                uuid,
                ntst: NTST::try_from(uri)?,
            }),
            _ => Err(ErrorKind::InvalidUsn(usn.to_string()))?,
        }
    }
}

impl<NTST> Usn<NTST>
where
    Self: HeaderExt + Display,
    NTST: PartialEq,
{
    pub fn get_validated(header: &UpnpHeader<'_>, ntst: &NTST) -> Result<Self, ParseError> {
        let usn = Self::get_from(header)?;
        if usn.ntst == *ntst {
            Ok(usn)
        } else {
            Err(ErrorKind::InvalidUsn(usn.to_string()))?
        }
    }
}

impl<NTST> Display for Usn<NTST>
where
    NTST: Display + PartialEq<Uri>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "uuid:{}", self.uuid)?;
        if self.ntst
            != (Uri::Uuid {
                uuid: self.uuid,
                suffix: None,
            })
        {
            write!(f, "::{}", self.ntst)?;
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;
    use std::error::Error;

    #[test]
    fn get_option_some() {
        let msg = r#"HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
MX: 5
ST: ssdp:all
USER-AGENT: linux/6.6.87 UPnP/2.0 splurt/0.0.1
CPFN.UPNP.ORG: splurt SSDP repeater
CPUUID.UPNP.ORG: 2fac1234-31f8-11b4-a222-08002b34c003"#;
        let header: UpnpHeader = msg.lines().collect();
        let mx = Option::<Mx>::get_from(&header).expect("optional MX");
        assert_matches!(mx, Some(mx) if mx == Mx(5));
    }

    #[test]
    fn get_option_none() {
        let msg = r#"HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
ST: ssdp:all
USER-AGENT: linux/6.6.87 UPnP/2.0 splurt/0.0.1
CPFN.UPNP.ORG: splurt SSDP repeater
CPUUID.UPNP.ORG: 2fac1234-31f8-11b4-a222-08002b34c003"#;
        let header: UpnpHeader = msg.lines().collect();
        let mx = Option::<Mx>::get_from(&header).expect("optional MX");
        assert_matches!(mx, None);
    }

    #[test]
    fn get_option_invalid() {
        let msg = r#"HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
MX: 6
ST: ssdp:all
USER-AGENT: linux/6.6.87 UPnP/2.0 splurt/0.0.1
CPFN.UPNP.ORG: splurt SSDP repeater
CPUUID.UPNP.ORG: 2fac1234-31f8-11b4-a222-08002b34c003"#;
        let header: UpnpHeader = msg.lines().collect();
        let err = Option::<Mx>::get_from(&header).expect_err("invalid MX");
        assert_matches!(err.kind, ErrorKind::InvalidMx(ref mx) if mx =="6");
        assert!(err.source().is_none());
    }

    #[test]
    fn secure_location() {
        let msg = r#"SECURELOCATION.UPNP.ORG: https://192.168.0.15:8001/xml/device_description.xml
"#;
        let header: UpnpHeader = msg.lines().collect();
        let secure_location =
            Option::<SecureLocation>::get_from(&header).expect("has SecureLocation");
        assert_matches!(secure_location, Some(_));
    }

    #[test]
    fn secure_location_invalid_scheme() {
        let msg = r#"SECURELOCATION.UPNP.ORG: http://192.168.0.15:8001/xml/device_description.xml
"#;
        let header: UpnpHeader = msg.lines().collect();
        let err =
            Option::<SecureLocation>::get_from(&header).expect_err("has invalid SecureLocation");
        assert_matches!(err.kind, ErrorKind::InvalidSecureLocation(ref loc)
            if loc == "http://192.168.0.15:8001/xml/device_description.xml"
        );
        assert!(err.source().is_none());
    }

    #[test]
    fn secure_location_no_port() {
        let msg = r#"SECURELOCATION.UPNP.ORG: http://192.168.0.15/xml/device_description.xml
"#;
        let header: UpnpHeader = msg.lines().collect();
        let err =
            Option::<SecureLocation>::get_from(&header).expect_err("has invalid SecureLocation");
        assert_matches!(err.kind, ErrorKind::InvalidSecureLocation(ref loc)
            if loc == "http://192.168.0.15/xml/device_description.xml"
        );
        assert!(err.source().is_none());
    }
}
