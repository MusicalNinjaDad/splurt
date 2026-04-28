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
};

use uuid::Uuid;

use crate::MULTICAST;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    InvalidMethod(String),
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidMethod(method) => write!(f, "{} is not a valid upnp method", method),
        }
    }
}

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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M-SEARCH * HTTP/1.1" => Ok(Self::MSearch),
            "NOTIFY * HTTP/1.1" => Ok(Self::Notify),
            "HTTP/1.1 200 OK" => Ok(Self::Response),
            _ => Err(Error::InvalidMethod(s.to_string())),
        }
    }
}

/// A valid & parsed ssdp message
///
/// Create with `Message::parse()`
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// NTS: ssdp:alive
    Alive(Notification),
    /// MAN: ssdp:discover
    Search(MSearch),
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

/// Formats Message as per OCF specification (2020)
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Alive(_notification) => todo!("display alive messages"),
            Message::Search(msearch) => {
                write!(f, "{msearch}")
            }
        }
    }
}

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
        writeln!(f, r#"MAN: "ssdp:discover""#)?;
        mx.write_header(f)?;
        writeln!(f, "ST: ssdp:all")?;
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

trait Header {
    const HEADER_KEY: &'static str;
}

trait HeaderExt {
    /// Output as a valid header line
    fn to_header(&self) -> String;

    /// Write a valid header line to `f` including new-line
    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<H: Header + Display> HeaderExt for H {
    /// Output as a valid header line
    fn to_header(&self) -> String {
        format!("{}: {}", Self::HEADER_KEY, self)
    }

    /// Write a valid header line to `f` including new-line
    fn write_header(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.to_header())
    }
}

impl<H: Header + HeaderExt> HeaderExt for Option<H> {
    fn to_header(&self) -> String {
        match self {
            Some(header) => header.to_header(),
            None => String::new(),
        }
    }

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
}
