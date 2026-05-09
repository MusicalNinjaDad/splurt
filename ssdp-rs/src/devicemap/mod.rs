use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    devicemap::rootdevice::RootDevice,
    message::{
        Message, Notify, Response, ST, ServiceDetails,
        notify::{Alive, NT},
    },
};

pub mod rootdevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Information {
    RootDevice(RootDevice),
    Device(Message),
    Service(ServiceInfo),
    ControlPoint(Message),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    service: ServiceDetails,
    id: Uuid,
    inferred_root_device: RootDevice,
}

impl From<Message> for Information {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Notify(Notify::Alive(Alive {
                max_age,
                location,
                server,
                usn,
                boot_id,
                config_id,
                port,
                secure_location,
            })) => match usn.ntst {
                NT::Service(service) => {
                    let inferred_root_device = RootDevice::new(
                        usn.uuid,
                        max_age,
                        None,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    Self::Service(ServiceInfo {
                        service,
                        id: usn.uuid,
                        inferred_root_device,
                    })
                }
                _ => todo!("other alive"),
            },
            Message::Search(_) => Self::ControlPoint(msg),
            Message::Response(Response {
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
            }) => match usn.ntst {
                ST::Service(service) => {
                    let inferred_root_device = RootDevice::new(
                        usn.uuid,
                        max_age,
                        date,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    Self::Service(ServiceInfo {
                        service,
                        id: usn.uuid,
                        inferred_root_device,
                    })
                }
                ST::RootDevice => Self::RootDevice(RootDevice::new(
                    usn.uuid,
                    max_age,
                    date,
                    location,
                    server,
                    boot_id,
                    config_id,
                    port,
                    secure_location,
                )),
                _ => todo!("other response"),
            },
            _ => todo!("other stuff"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceMap {
    inner: HashMap<Uuid, RootDevice>,
}

impl DeviceMap {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn insert(&mut self, root_device: RootDevice) -> Option<RootDevice> {
        self.inner.insert(root_device.id, root_device)
    }

    pub fn process(&mut self, message: Message) -> Result<(), Error> {
        let info = message.into();
        match info {
            Information::RootDevice(root_device) => {
                self.insert(root_device);
                Ok(())
            }
            #[expect(unused_variables, reason = "todo")]
            Information::Device(message) => todo!("process devices"),
            Information::Service(serviceinfo) => {
                let root_device = self
                    .inner
                    .entry(serviceinfo.id)
                    // as long as a control point has received at least one advertisement that is still
                    // valid from a root device, any of its embedded devices or any of its services,
                    // then the control point can assume that all are available.
                    .and_modify(|rd| {
                        rd.last_seen = serviceinfo.inferred_root_device.last_seen;
                        rd.valid_until = serviceinfo.inferred_root_device.valid_until;
                    })
                    .or_insert(serviceinfo.inferred_root_device);
                root_device.services.insert(serviceinfo.service);
                Ok(())
            }
            #[expect(unused_variables, reason = "todo")]
            Information::ControlPoint(message) => todo!("process control points"),
        }
    }
}

impl Default for DeviceMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{devicemap::rootdevice::RootDevice, message::Message};

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    use super::*;

    use chrono::{DateTime, Utc};
    use uuid::uuid;

    #[test]
    fn add_new_root_device() {
        let response = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
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
        let mut devices = DeviceMap::new();
        let msg = response.parse::<Message>().expect("valid message");
        let root_device: RootDevice = msg.try_into().expect("a root device");
        let old_entry = devices.insert(root_device);
        assert!(old_entry.is_none());
    }

    #[test]
    fn update_from_notify() {
        let response = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
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
        let mut devices = DeviceMap::new();
        let response = response.parse::<Message>().expect("valid message");
        let root_device: RootDevice = response.try_into().expect("a root device");
        let root_device1 = root_device.clone();
        let old_entry = devices.insert(root_device);
        assert!(old_entry.is_none());
        let notify = r#"NOTIFY * HTTP/1.1
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
        let notify = notify.parse::<Message>().expect("valid message");
        let root_device: RootDevice = notify.try_into().expect("a root device");
        let old_entry = devices.insert(root_device);
        assert_matches!(old_entry, Some(old_entry) if old_entry == root_device1);
    }

    #[test]
    fn add_service() {
        let mut devices = DeviceMap::new();

        let root_device = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
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
        let id = uuid!("c4248768-d6b6-4232-a273-5b1701524493");
        let root_device = root_device.parse::<Message>().expect("valid message");
        devices
            .process(root_device)
            .expect("process root device message");
        assert!(devices.inner.contains_key(&id));
        {
            // acquire &devices
            let root_device = devices.inner.get(&id).expect("root device is there");
            assert_eq!(root_device.services.len(), 0);
            assert_eq!(
                root_device.last_seen,
                DateTime::parse_from_rfc3339("2026-04-29T08:22:03+00:00")
                    .expect("when this test was written")
                    .to_utc()
            );
            assert_eq!(
                root_device.valid_until,
                (DateTime::parse_from_rfc3339("2026-04-29T08:22:03+00:00")
                    .expect("when this test was written")
                    .to_utc()
                    + Duration::from_secs(1800))
            )
        } // drop &devices
        let service = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 2400
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: urn:schemas-upnp-org:service:MusicServices:1
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::urn:schemas-upnp-org:service:MusicServices:1
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
        let service = service.parse::<Message>().expect("valid service");
        // needs &mut devices
        devices.process(service).expect("process service message");
        {
            let root_device = devices.inner.get(&id).expect("root device still there");
            assert_eq!(root_device.services.len(), 1);
            assert!(
                root_device.last_seen > (Utc::now() - Duration::from_secs(60)),
                "root device last seen at {}",
                root_device.last_seen
            );
            assert_eq!(
                root_device.valid_until,
                root_device.last_seen + Duration::from_secs(2400)
            )
        }
    }

    #[test]
    fn infer_root_from_service() {
        let mut devices = DeviceMap::new();
        let id = uuid!("c4248768-d6b6-4232-a273-5b1701524493");

        let service = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 2400
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: urn:schemas-upnp-org:service:MusicServices:1
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::urn:schemas-upnp-org:service:MusicServices:1
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
        let service = service.parse::<Message>().expect("valid service");
        devices.process(service).expect("process service message");
        let root_device = devices.inner.get(&id).expect("root device still there");
        assert_eq!(root_device.services.len(), 1);
        assert!(
            root_device.last_seen > (Utc::now() - Duration::from_secs(60)),
            "root device last seen at {}",
            root_device.last_seen
        );
        assert_eq!(
            root_device.valid_until,
            root_device.last_seen + Duration::from_secs(2400)
        )
    }
}
