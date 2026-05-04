//! UPnP Messages as per [UPnP Device Architecture 2.0 (revision 2020-04-17)][spec]
//!
//! Message generation is strict to standard.
//!
//! Message parsing is lenient - I've have yet to see a single well-formed spec-conform UPnP
//! message flying around on my network.
//!
//! [spec]: https://openconnectivity.org/upnp-specs/UPnP-arch-DeviceArchitecture-v2.0-20200417.pdf

use std::{collections::HashMap, fmt::Display, str::FromStr};

use uuid::Uuid;

mod devices;
mod error;
mod header;
mod msearch;
mod response;
mod services;
mod uri;

pub use devices::{Device, DeviceDetails};
pub use error::{ErrorKind, ParseError};
pub use header::{
    FriendlyName, Header, HeaderExt, Host, Man, MaxAge, Mx, ST, UpnpHeader, UpnpPort, UserAgent,
};
pub use msearch::MSearch;
pub use response::Response;
pub use services::{Service, ServiceDetails};
pub use uri::{SsdpNss, Target, UpnpNss, Uri, UriToken};

const UPNP_VERSION: &str = "2.0";
/// RFC1123 date format, e.g.: "Wed, 29 Apr 2026 08:22:03 GMT"
///
/// ## Note
/// - Will only parse to [chrono::NaiveDateTime]. Parsing MUST ignore timezone specified ("GMT" as
///   per spec) as abbreviations are not standardised unique values.
///   See: https://docs.rs/chrono/latest/chrono/format/strftime/index.html#fn6
const RFC1123: &str = "%a, %d %b %Y %T %Z";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    MSearch,
    Notify,
    Response,
}
impl FromStr for Method {
    type Err = ErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M-SEARCH * HTTP/1.1" => Ok(Self::MSearch),
            "NOTIFY * HTTP/1.1" => Ok(Self::Notify),
            "HTTP/1.1 200 OK" => Ok(Self::Response),
            _ => Err(ErrorKind::InvalidMethod(s.to_string())),
        }
    }
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
            upnp_version: UPNP_VERSION.to_string(),
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
        let method: Method = lines.next().ok_or(ErrorKind::EmptyMessage)?.parse()?;
        let header: UpnpHeader = lines.collect();
        match method {
            Method::MSearch => todo!("parse MSearch"),
            Method::Notify => todo!("parse Notify"),
            Method::Response => Ok(Message::Response(header.try_into()?)),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vendor {
    Standard,
    Custom(String),
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

impl Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vendor::Standard => write!(f, "schemas-upnp-org"),
            Vendor::Custom(s) => write!(f, "{}", s),
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
        let msg = ALIVE.parse().expect("parsed as NOTIFY");
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
HOST: 239.255.255.250:1900
EXT:
CACHE-CONTROL: max-age=100
LOCATION: http://192.168.0.71:80/description.xml
SERVER: Hue/1.0 UPnP/1.0 IpBridge/1.76.0
hue-bridgeid: ECB55AF4FE12E2C4
ST: upnp:rootdevice
USN: uuid:2f402f80-da50-11e1-9b23-ecb55af4fe12e2c4::upnp:rootdevice
"#;
        let response: Message = raw_response.parse().expect("parsed as response");
        assert_matches!(response, Message::Response(_));
    }

    #[test]
    fn parse_service() {
        let raw_response = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age=1900
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.5.12:50001/desc/device.xml
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
