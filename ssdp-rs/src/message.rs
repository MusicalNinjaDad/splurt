//! UPnP Messages as per [UPnP Device Architecture 2.0 (revision 2020-04-17)][spec]
//!
//! Message generation is strict to standard.
//!
//! Message parsing is lenient - I've have yet to see a single well-formed spec-conform UPnP
//! message flying around on my network.
//!
//! [spec]: https://openconnectivity.org/upnp-specs/UPnP-arch-DeviceArchitecture-v2.0-20200417.pdf

use std::{
    collections::HashMap,
    error,
    fmt::Display,
    io,
    net::{AddrParseError, SocketAddr, SocketAddrV4, SocketAddrV6},
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Utc};
use url::Url;
use uuid::Uuid;

use crate::{MULTICAST, SSDP_PORT};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseError {
    EmptyMessage,
    InvalidMethod(String),
    InvalidST(String),
    InvalidDevice(String),
    InvalidDeviceDetails(String),
}

impl error::Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyMessage => write!(f, "empty message"),
            ParseError::InvalidMethod(method) => write!(f, "{} is not a valid upnp method", method),
            ParseError::InvalidST(st) => write!(f, "{} is not a valid upnp search type", st),
            ParseError::InvalidDevice(device) => write!(
                f,
                "{} is not a valid upnp device specification (valid forms are `urn:domain-name:device:deviceType:ver` & `urn:schemas-upnp-org:device:deviceType:ver`)",
                device
            ),
            ParseError::InvalidDeviceDetails(device) => {
                write!(f, "{} is not a valid upnp device:ver specification", device)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    MSearch,
    Notify,
    Response,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::MSearch => write!(f, "M-SEARCH * HTTP/1.1"),
            Method::Notify => write!(f, "NOTIFY * HTTP/1.1"),
            Method::Response => write!(f, "HTTP/1.1 200 OK"),
        }
    }
}

impl FromStr for Method {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M-SEARCH * HTTP/1.1" => Ok(Self::MSearch),
            "NOTIFY * HTTP/1.1" => Ok(Self::Notify),
            "HTTP/1.1 200 OK" => Ok(Self::Response),
            _ => Err(ParseError::InvalidMethod(s.to_string())),
        }
    }
}

/// A valid & parsed ssdp message
///
/// Create with `Message::parse()`
#[derive(Debug, Clone, PartialEq)]
#[expect(clippy::large_enum_variant)]
// TODO: #39 Consider boxing `Message::Response`
//       Contents are `Box`ed as they contain many large pointers to heap-allocated
//       information e.g. `String`s (each is a 24b pointer to data that is on the heap anyway)
pub enum Message {
    /// NTS: ssdp:alive
    Alive(Notification),
    /// MAN: ssdp:discover
    Search(MSearch),
    /// A direct response to an `M-SEARCH` request
    Response(Response),
}

impl Message {
    /// Parse an ssdp message from given text
    pub fn parse(contents: &str) -> Option<Message> {
        let mut lines = contents.lines();
        if lines.next()? != "NOTIFY * HTTP/1.1" {
            return None;
        };
        let header: RawHeader = lines
            .filter_map(|line| {
                line.split_once(": ")
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
            .collect();
        if *header.get("NTS")? == "ssdp:alive" {
            //TODO: flaky - capitalisation
            let location = header.get("Location").map(ToString::to_string);
            return Some(Message::Alive(Notification { location, header }));
        }
        None
    }

    /// Construct a new M-SEARCH message.
    ///
    /// While details of the user agent are technically optional we are going to include them
    /// in our searches.
    pub fn new_search(
        mx: Mx,
        os: &str,
        os_version: &str,
        product_name: &str,
        product_version: &str,
        friendly_name: &str,
        uuid: Uuid,
    ) -> Self {
        let user_agent = UserAgent {
            os: os.to_string(),
            os_version: os_version.to_string(),
            product_name: product_name.to_string(),
            product_version: product_version.to_string(),
        };
        let host = Default::default();
        Message::Search(MSearch {
            host,
            mx,
            user_agent: Some(user_agent),
            friendly_name: friendly_name.into(),
            uuid: Some(uuid),
        })
    }
}

impl FromStr for Message {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let method: Method = lines.next().ok_or(ParseError::EmptyMessage)?.parse()?;
        let header: UpnpHeader = lines.collect();
        match method {
            Method::MSearch => todo!("parse MSearch"),
            Method::Notify => todo!("parse Notify"),
            Method::Response => todo!("parse Response"),
        }
    }
}

struct UpnpHeader<'h>(HashMap<&'h str, &'h str>);

impl<'h> FromIterator<&'h str> for UpnpHeader<'h> {
    fn from_iter<T: IntoIterator<Item = &'h str>>(iter: T) -> Self {
        let hashmap = iter
            .into_iter()
            .filter_map(|line| line.split_once(": "))
            .collect();
        Self(hashmap)
    }
}

/// Formats Message as per OCF specification (2020)
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Alive(_notification) => todo!("display alive messages"),
            Message::Search(msearch) => {
                write!(f, "{msearch}")
            }
            Message::Response(_response) => todo!("display response messages"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Man {
    Discover,
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

impl Header for Man {
    const HEADER_KEY: &'static str = "MAN";
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Search Target
enum ST {
    /// `ssdp:all`: Search for all devices and services.
    All,
    /// `upnp:rootdevice`: Search for root devices only.
    #[expect(unused)]
    Root,
    /// uuid:device-UUID: Search for a particular device.
    #[expect(unused)]
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
    #[expect(unused)]
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
    #[expect(unused)]
    Service(ServiceDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DeviceDetails {
    vendor: Vendor,
    device: Device,
}

impl Display for DeviceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:device:{}", self.vendor, self.device)
    }
}

impl FromStr for DeviceDetails {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseError::InvalidDevice(s.to_string());
        let mut parts = s.split(":");
        match parts.next() {
            Some("urn") => (),
            _ => return Err(err()),
        };
        let Ok(vendor) = parts.next().ok_or_else(err)?.parse::<Vendor>();
        match parts.next() {
            Some("device") => (),
            _ => return Err(err()),
        };
        let device: String = parts.collect();
        let device: Device = device.parse()?;
        Ok(Self { vendor, device })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Vendor {
    Standard,
    Custom(String),
}

impl Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vendor::Standard => write!(f, "schemas-upnp-org"),
            Vendor::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for Vendor {
    type Err = !;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "schemas-upnp-org" => Ok(Self::Standard),
            _ => Ok(Self::Custom(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Device {
    Other { device_type: String, ver: String },
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::Other { device_type, ver } => write!(f, "{}:{}", device_type, ver),
        }
    }
}

impl FromStr for Device {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (device_type, ver) = s
            .split_once(":")
            .ok_or(ParseError::InvalidDeviceDetails(s.to_string()))?;
        Ok(Self::Other {
            device_type: device_type.to_string(),
            ver: ver.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ServiceDetails {
    vendor: Vendor,
    service: Service,
}

impl Display for ServiceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:service:{}", self.vendor, self.service)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Service {
    #[expect(unused)]
    Other { service_type: String, ver: String },
}

impl Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Service::Other { service_type, ver } => write!(f, "{}:{}", service_type, ver),
        }
    }
}

impl Header for ST {
    const HEADER_KEY: &'static str = "ST";
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

//TODO: impl FromStr for ST {
//     type Err = Error;
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         todo!("from str ST")
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MSearch {
    host: Host,
    mx: Mx,
    user_agent: Option<UserAgent>,
    friendly_name: FriendlyName,
    uuid: Option<Uuid>,
}

/// Entire valid M-SEARCH message including initial method line,
/// as per OCF specification (2020) section 1.3.2
///
/// #### Note:
/// I've rarely actually seen a well-formed spec-conform M-SEARCH flying around my network
/// but there's nothing wrong with actually being fully valid!
impl Display for MSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            host,
            mx,
            user_agent,
            friendly_name,
            uuid,
        } = self;
        writeln!(f, "{}", Method::MSearch)?;
        host.write_header(f)?;
        Man::Discover.write_header(f)?;
        mx.write_header(f)?;
        ST::All.write_header(f)?;
        user_agent.write_header(f)?;
        friendly_name.write_header(f)?;
        uuid.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
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

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Host::V4(socket_addr_v4) => write!(f, "{socket_addr_v4}"),
            Host::_V6(socket_addr_v6) => write!(f, "{socket_addr_v6}"),
        }
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

impl FromStr for Host {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let addr = SocketAddr::from_str(s)?;
        Ok(addr.into())
    }
}

impl Default for Host {
    fn default() -> Self {
        MULTICAST.into()
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

impl Display for Mx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UserAgent {
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
}

impl Header for UserAgent {
    const HEADER_KEY: &'static str = "USER-AGENT";
}

/// Formatted as per OCF specification (2020) section 1.3.2 for the `USER-AGENT` *value*,
/// does NOT include the header key
impl Display for UserAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            os,
            os_version,
            product_name,
            product_version,
        } = self;
        write!(
            f,
            "{os}/{os_version} UPnP/2.0 {product_name}/{product_version}"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FriendlyName(String);

impl Header for FriendlyName {
    const HEADER_KEY: &'static str = "CPFN.UPNP.ORG";
}

impl Display for FriendlyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FriendlyName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Header for Uuid {
    const HEADER_KEY: &'static str = "CPUUID.UPNP.ORG";
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// A direct response to an `M-SEARCH` message.
///
/// "To be found by a network search, a device shall send a unicast UDP response to the source IP
/// address and port that sent the request to the multicast address." <- This represents one of
/// these messages.
pub struct Response {
    /// `CACHE-CONTROL`: Duration until advertisement expires
    max_age: Duration,
    /// `DATE`: when response was generated
    date: Option<DateTime<Utc>>,
    /// `EXT`: Required for backwards compatibility with UPnP 1.0. (Header field name only; no field value.)
    ext: Option<!>,
    /// `URL` for UPnP description for root device
    location: Url,
    /// `SERVER`: OS/version UPnP/2.0 product/version
    server: UserAgent,
    /// `ST`: search target
    st: ST,
    /// `USN`: composite identifier for the advertisement
    usn: !,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    boot_id: u32,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    config_id: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    secure_location: Option<Url>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// The port specified in a Upnp message.
///
/// Treat this like a semantically specific `Option<u16>` with a valuable implementation of
/// `Default`, `From<Option<u16>>` & `Into<u16>`.
enum UpnpPort {
    /// Specifically defined value.
    ///
    /// If this is set to the default [SSDP_PORT], then it means the message specifically
    /// defined that value.
    Defined(u16),
    /// A value was not defined. Conversion to a `u16` will provide the default [SSDP_PORT]
    #[default]
    Default,
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

/// `None` maps to `Default`
impl From<Option<u16>> for UpnpPort {
    fn from(port: Option<u16>) -> Self {
        match port {
            Some(port) => Self::Defined(port),
            None => Self::Default,
        }
    }
}

/// Marker trait for Upnp header fields, with details of the relevant key
trait Header {
    /// Key as per spec
    const HEADER_KEY: &'static str;
}

/// Handles constructing valid header lines.
///
/// This is a separate trait from [Header] to allow for it to also be implemented on `Option<H>`
trait HeaderExt {
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
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    location: Option<String>,
    header: RawHeader,
}

/// `key: value` pairings, ideally from a NOTIFY * HTTP/1.1
///
/// look at [Message::parse] to see how to safely construct this yourself
type RawHeader = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use uuid::uuid;

    use super::*;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;

    const ALIVE: &str = r#"NOTIFY * HTTP/1.1
Host: 239.255.255.250:1982
Cache-Control: max-age=3600
Location: yeelight://192.168.1.239:55443
NTS: ssdp:alive
Server: POSIX, UPnP/1.0 YGLC/1
id: 0x000000000015243f
model: color
fw_ver: 18
support: get_prop set_default set_power toggle set_bright start_cf stop_cf set_scene cron_add cron_get cron_del set_ct_abx set_rgb
power: on
bright: 100
color_mode: 2
ct: 4000
rgb: 16711680
hue: 100
sat: 35
name: my_bulb
"#;

    #[test]
    fn parse_alive() {
        let msg = Message::parse(ALIVE).unwrap();
        let alive_header = HashMap::from([
            ("Host", "239.255.255.250:1982"),
            ("Cache-Control", "max-age=3600"),
            ("Location", "yeelight://192.168.1.239:55443"),
            ("NTS", "ssdp:alive"),
            ("Server", "POSIX, UPnP/1.0 YGLC/1"),
            ("id", "0x000000000015243f"),
            ("model", "color"),
            ("fw_ver", "18"),
            (
                "support",
                "get_prop set_default set_power toggle set_bright start_cf stop_cf set_scene cron_add cron_get cron_del set_ct_abx set_rgb",
            ),
            ("power", "on"),
            ("bright", "100"),
            ("color_mode", "2"),
            ("ct", "4000"),
            ("rgb", "16711680"),
            ("hue", "100"),
            ("sat", "35"),
            ("name", "my_bulb"),
        ]);
        let alive_header = alive_header
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let expected_notification = Notification {
            location: Some("yeelight://192.168.1.239:55443".to_string()),
            header: alive_header,
        };
        assert_matches!(msg, Message::Alive(notification) if notification == expected_notification);
    }

    #[test]
    fn generate_search() {
        let expected = r#"M-SEARCH * HTTP/1.1
HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
MX: 5
ST: ssdp:all
USER-AGENT: linux/6.6.87 UPnP/2.0 splurt/0.0.1
CPFN.UPNP.ORG: splurt SSDP repeater
CPUUID.UPNP.ORG: 2fac1234-31f8-11b4-a222-08002b34c003

"#;
        let mx = 5.try_into().expect("valid mx");
        let os = "linux";
        let os_version = "6.6.87";
        let product_name = "splurt";
        let product_version = "0.0.1";
        let friendly_name = "splurt SSDP repeater";
        let uuid = uuid!("2fac1234-31f8-11b4-a222-08002b34c003");
        let msg = Message::new_search(
            mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            uuid,
        );
        let msg_text = msg.to_string();
        assert_eq!(msg_text, expected);
    }

    #[test]
    fn search_no_user_agent() {
        let expected = r#"M-SEARCH * HTTP/1.1
HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
MX: 5
ST: ssdp:all
CPFN.UPNP.ORG: splurt SSDP repeater
CPUUID.UPNP.ORG: 2fac1234-31f8-11b4-a222-08002b34c003

"#;
        let mx = 5.try_into().expect("valid mx");
        let friendly_name = "splurt SSDP repeater";
        let uuid = uuid!("2fac1234-31f8-11b4-a222-08002b34c003");
        let msg = MSearch {
            host: Default::default(),
            mx,
            user_agent: None,
            friendly_name: FriendlyName(friendly_name.to_string()),
            uuid: Some(uuid),
        };
        let msg_text = msg.to_string();
        assert_eq!(msg_text, expected);
    }

    #[test]
    fn parse_response() {
        // TODO: Will likely need indexmap = "2.14.0" for round-trip conversion with non-std entries
        let raw_response = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age=1900
DATE: Tue, 28 Apr 2026 12:56:35 GMT
EXT:
LOCATION: http://192.168.0.129:50001/desc/device.xml
OPT: "http://schemas.upnp.org/upnp/1/0/"; ns=01
01-NLS: 88ccb70e-32ec-11f1-8533-ec2b50e32df5
SERVER: Linux/2.6.32.12, UPnP/1.0, Portable SDK for UPnP devices/1.6.21
X-User-Agent: redsonic
ST: urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1
USN: uuid:00113214-9943-0011-4399-439914321100::urn:microsoft.com:service:X_MS_MediaReceiverRegistrar:1
"#;
        let response: Message = raw_response.parse().expect("parsed as response");
        assert_matches!(response, Message::Response(_));
    }
}
