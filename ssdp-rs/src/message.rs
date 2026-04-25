//! UPnP Messages as per [UPnP Device Architecture 2.0 (revision 2020-04-17)][spec]
//!
//! Message generation is strict to standard.
//!
//! Message parsing is lenient - I've have yet to see a single well-formed spec-conform UPnP
//! message flying around on my network.
//!
//! [spec]: https://openconnectivity.org/upnp-specs/UPnP-arch-DeviceArchitecture-v2.0-20200417.pdf

use std::{collections::HashMap, fmt::Display};

use uuid::Uuid;

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

/// Formats Message as per OCF specification (2020)
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Alive(_notification) => todo!(),
            Message::Search(msearch) => {
                write!(f, "{msearch}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MSearch {
    mx: u8,
    user_agent: Option<UserAgent>,
    friendly_name: String,
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
            mx,
            user_agent,
            friendly_name,
            uuid,
        } = self;
        writeln!(f, "M-SEARCH * HTTP/1.1")?;
        writeln!(f, "HOST: 239.255.255.250:1900")?;
        writeln!(f, r#"MAN: "ssdp:discover""#)?;
        writeln!(f, "MX: {}", mx)?;
        writeln!(f, "ST: ssdp:all")?;
        if let Some(user_agent) = user_agent {
            writeln!(f, "USER-AGENT: {}", user_agent)?;
        }
        writeln!(f, "CPFN.UPNP.ORG: {}", friendly_name)?;
        if let Some(uuid) = uuid {
            writeln!(f, "CPUUID.UPNP.ORG: {}", uuid)?;
        }
        writeln!(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UserAgent {
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
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
        mx: u8,
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
        Message::Search(MSearch {
            mx,
            user_agent: Some(user_agent),
            friendly_name: friendly_name.to_string(),
            uuid: Some(uuid),
        })
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
        let mx = 5;
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
}
