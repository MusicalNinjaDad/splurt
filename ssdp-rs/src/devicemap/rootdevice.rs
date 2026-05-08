//! A heirarchical map of rootdevice[/device]/service

use chrono::{DateTime, Utc};
use url::Url;
use uuid::Uuid;

use crate::{
    Error,
    message::{
        Message, Notify, Response, ST, Server, UpnpPort,
        notify::{Alive, NT},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[expect(unused, reason = "nice to have, not yet implemented in message")]
enum Lenient<T> {
    Valid(T),
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootDevice {
    pub id: Uuid,
    last_seen: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    /// URL for UPnP description for root device
    location: Url,
    /// OS/version UPnP/2.0 product/version
    product: Option<Server>,
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

impl TryFrom<Message> for RootDevice {
    type Error = Error;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            // TODO hint & document `cold_path()` on error arms.
            // This should be called on something known to be about a RootDevice
            Message::Notify(notify) if matches!(notify.nt(), NT::RootDevice) => match notify {
                Notify::Alive(alive) => {
                    let Alive {
                        max_age,
                        location,
                        server,
                        usn,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    } = alive;
                    let last_seen = Utc::now();
                    let valid_until = last_seen + *max_age.as_duration();
                    Ok(Self {
                        id: usn.uuid,
                        last_seen,
                        valid_until,
                        location: location.into_url(),
                        product: Some(server),
                        boot_id: boot_id.map(|id| *id.as_u32()),
                        config_id: config_id.map(|id| *id.as_u32()),
                        port,
                        secure_location: secure_location.map(|loc| loc.into_url()),
                    })
                }
                #[expect(unused_variables, reason = "todo")]
                Notify::ByeBye(bye_bye) => todo!("remove root device based on bybebye"),
                #[expect(unused_variables, reason = "todo")]
                Notify::Update(update) => todo!("update or infer root device from update"),
            },
            Message::Response(response) if matches!(response.usn.ntst, ST::RootDevice) => {
                let Response {
                    max_age,
                    date,
                    location,
                    server,
                    usn,
                    boot_id,
                    config_id,
                    port,
                    secure_location,
                    ..
                } = response;
                let last_seen = date.unwrap_or_else(Utc::now);
                let valid_until = last_seen + *max_age.as_duration();
                Ok(Self {
                    id: usn.uuid,
                    last_seen,
                    valid_until,
                    location: location.into_url(),
                    product: Some(server),
                    boot_id: boot_id.map(|id| *id.as_u32()),
                    config_id: config_id.map(|id| *id.as_u32()),
                    port,
                    secure_location: secure_location.map(|loc| loc.into_url()),
                })
            }
            _ => todo!("error for parsing somthing that's not a root_device"),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;

    use uuid::uuid;

    use crate::message::UPNP_VERSION1;

    use super::*;

    #[test]
    fn root_from_response() {
        let sonos = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: upnp:rootdevice
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::upnp:rootdevice
X-RINCON-HOUSEHOLD: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3
X-RINCON-BOOTSEQ: 6
BOOTID.UPNP.ORG: 6
X-RINCON-WIFIMODE: 1
X-RINCON-VARIANT: 2
HOUSEHOLD.SMARTSPEAKER.AUDIO: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3.9LpAqreapUbAY1tsy5BF
LOCATION.SMARTSPEAKER.AUDIO: lc_4e8119cfb08d4c5083b6e0c75e47fe50
SECURELOCATION.UPNP.ORG: https://192.168.0.84:1443/xml/device_description.xml
X-SONOS-HHSECURELOCATION: https://192.168.0.84:1843/xml/device_description.xml

"#;
        let response = sonos.parse::<Message>().expect("valid message");
        let root_device: RootDevice = response.try_into().expect("a root device");
        let RootDevice {
            id,
            last_seen,
            valid_until,
            location,
            product,
            boot_id,
            config_id,
            port,
            secure_location,
        } = root_device;
        assert_eq!(id, uuid!("c4248768-d6b6-4232-a273-5b1701524493"));
        assert!(
            last_seen
                > DateTime::parse_from_rfc3339("2026-05-08T07:36:00+02:00")
                    .expect("when this test was written")
        );
        assert_eq!(valid_until, last_seen + Duration::from_secs(1800));
        assert_eq!(
            location,
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url")
        );
        assert_matches!(product, Some(product) if product == Server { os: "Linux".to_string(), os_version: "".to_string(),
         upnp_version: UPNP_VERSION1, product_name: "Sonos".to_string(), product_version: "85.0-64200 (ZPS29)".to_string() });
        assert_matches!(boot_id, Some(id) if id ==6);
        assert!(config_id.is_none());
        assert_matches!(port, UpnpPort::Default);
        assert_matches!(secure_location, Some(secure_location)
            if secure_location == Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
        );
    }

    #[test]
    fn root_from_alive() {
        let sonos = r#"NOTIFY * HTTP/1.1
HOST: 239.255.255.250:1900
CACHE-CONTROL: max-age = 1800
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
NT: upnp:rootdevice
NTS: ssdp:alive
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::upnp:rootdevice
X-RINCON-HOUSEHOLD: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3
X-RINCON-BOOTSEQ: 6
BOOTID.UPNP.ORG: 6
X-RINCON-WIFIMODE: 1
X-RINCON-VARIANT: 2
HOUSEHOLD.SMARTSPEAKER.AUDIO: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3.9LpAqreapUbAY1tsy5BF
LOCATION.SMARTSPEAKER.AUDIO: lc_4e8119cfb08d4c5083b6e0c75e47fe50
SECURELOCATION.UPNP.ORG: https://192.168.0.84:1443/xml/device_description.xml
X-SONOS-HHSECURELOCATION: https://192.168.0.84:1843/xml/device_description.xml

"#;
        let alive = sonos.parse::<Message>().expect("valid message");
        let root_device: RootDevice = alive.try_into().expect("a root device");
        let RootDevice {
            id,
            last_seen,
            valid_until,
            location,
            product,
            boot_id,
            config_id,
            port,
            secure_location,
        } = root_device;
        assert_eq!(id, uuid!("c4248768-d6b6-4232-a273-5b1701524493"));
        assert!(
            last_seen
                > DateTime::parse_from_rfc3339("2026-05-08T07:36:00+02:00")
                    .expect("when this test was written")
        );
        assert_eq!(valid_until, last_seen + Duration::from_secs(1800));
        assert_eq!(
            location,
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url")
        );
        assert_matches!(product, Some(product) if product == Server { os: "Linux".to_string(), os_version: "".to_string(),
         upnp_version: UPNP_VERSION1, product_name: "Sonos".to_string(), product_version: "85.0-64200 (ZPS29)".to_string() });
        assert_matches!(boot_id, Some(id) if id ==6);
        assert!(config_id.is_none());
        assert_matches!(port, UpnpPort::Default);
        assert_matches!(secure_location, Some(secure_location)
            if secure_location == Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
        );
    }
}
