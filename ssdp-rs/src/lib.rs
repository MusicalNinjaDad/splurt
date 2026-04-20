#![cfg_attr(unstable_assert_matches, feature(assert_matches))]

use std::collections::HashMap;

/// A valid & parsed ssdp message
///
/// Create with `Message::parse()`
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Message {
    /// NTS: ssdp:alive
    Alive(Notification),
}

impl Message {
    /// Parse an ssdp message from given text
    pub fn parse(contents: &str) -> Option<Message> {
        let mut lines = contents.lines();
        if lines.next()? != "NOTIFY * HTTP/1.1" {
            return None;
        };
        let raw: RawNotification = lines
            .filter_map(|line| {
                line.split_once(": ")
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
            .collect();
        if *raw.get("NTS")? == "ssdp:alive" {
            //TODO: flaky - capitalisation
            let location = raw.get("Location").map(ToString::to_string);
            return Some(Message::Alive(Notification { location }));
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Notification {
    location: Option<String>,
}

/// `key: value` pairings, ideally from a NOTIFY * HTTP/1.1
///
/// look at [Message::parse] to see how to safely construct this yourself
type RawNotification = HashMap<String, String>;

#[cfg(test)]
mod tests {
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
        let expected_notification = Notification {
            location: Some("yeelight://192.168.1.239:55443".to_string()),
        };
        assert_matches!(msg, Message::Alive(notification) if notification == expected_notification);
    }
}
