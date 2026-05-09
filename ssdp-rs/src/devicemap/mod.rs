use std::collections::HashMap;

use url::Url;
use uuid::Uuid;

use crate::{
    devicemap::rootdevice::RootDevice,
    message::{
        DeviceDetails, Message, Notify, Response, ST, ServiceDetails,
        notify::{Alive, NT},
    },
};

pub mod rootdevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Information {
    RootDevice(RootDevice),
    Device(DeviceInfo),
    Service(ServiceInfo),
    ControlPoint(Message),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    service: ServiceDetails,
    location: Url,
    inferred_root_device: RootDevice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    device: DeviceDetails,
    id: Uuid,
    location: Url,
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
                        None,
                        max_age,
                        None,
                        location.clone(),
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    Self::Service(ServiceInfo {
                        service,
                        location: location.into_url(),
                        inferred_root_device,
                    })
                }
                NT::RootDevice => Self::RootDevice(RootDevice::new(
                    Some(usn.uuid),
                    max_age,
                    None,
                    location,
                    server,
                    boot_id,
                    config_id,
                    port,
                    secure_location,
                )),
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
                        None,
                        max_age,
                        date,
                        location.clone(),
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    Self::Service(ServiceInfo {
                        service,
                        location: location.into_url(),
                        inferred_root_device,
                    })
                }
                ST::RootDevice => Self::RootDevice(RootDevice::new(
                    Some(usn.uuid),
                    max_age,
                    date,
                    location,
                    server,
                    boot_id,
                    config_id,
                    port,
                    secure_location,
                )),
                ST::Device(device) => {
                    let inferred_root_device = RootDevice::new(
                        None,
                        max_age,
                        date,
                        location.clone(),
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    Self::Device(DeviceInfo {
                        device,
                        id: usn.uuid,
                        location: location.into_url(),
                        inferred_root_device,
                    })
                }
                _ => todo!("other response"),
            },
            _ => todo!("other stuff"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceMap {
    inner: HashMap<Url, RootDevice>,
}

impl DeviceMap {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn insert(&mut self, root_device: RootDevice) -> Option<RootDevice> {
        self.inner.insert(root_device.location.clone(), root_device)
    }

    pub fn process(&mut self, message: Message) -> Result<(), Error> {
        let info = message.into();
        match info {
            Information::RootDevice(root_device) => {
                self.insert(root_device);
                Ok(())
            }
            Information::Device(deviceinfo) => {
                let root_device = self
                    .inner
                    .entry(deviceinfo.location)
                    // as long as a control point has received at least one advertisement that is still
                    // valid from a root device, any of its embedded devices or any of its services,
                    // then the control point can assume that all are available.
                    .and_modify(|rd| {
                        rd.last_seen = deviceinfo.inferred_root_device.last_seen;
                        rd.valid_until = deviceinfo.inferred_root_device.valid_until;
                    })
                    .or_insert(deviceinfo.inferred_root_device);
                match root_device.id {
                    Some(id) if id == deviceinfo.id => {
                        root_device.device_type = Some(deviceinfo.device)
                    }
                    _ => {
                        root_device
                            .embedded_devices
                            .insert(deviceinfo.id, deviceinfo.device);
                    }
                }
                Ok(())
            }
            Information::Service(serviceinfo) => {
                let root_device = self
                    .inner
                    .entry(serviceinfo.location)
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
    use crate::{
        devicemap::rootdevice::RootDevice,
        message::{Device, DeviceDetails, Message, Server, UPNP_VERSION1, UpnpPort, Vendor},
    };

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    use super::*;

    use chrono::{DateTime, Utc};
    use url::Url;
    use uuid::uuid;

    #[test]
    fn root_from_response() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");

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
        let message = response.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        let root_device = devices.inner.get(&url).expect("device created");
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
            device_type,
            embedded_devices,
            services,
        } = root_device;
        assert_eq!(id, &Some(uuid!("c4248768-d6b6-4232-a273-5b1701524493")));
        assert_eq!(
            last_seen,
            &DateTime::parse_from_rfc3339("2026-04-29T08:22:03+00:00").unwrap()
        );
        assert_eq!(
            valid_until,
            &DateTime::parse_from_rfc3339("2026-04-29T08:52:03+00:00").unwrap()
        );
        assert_eq!(location, &url);
        assert_matches!(product, Some(product) if product == &Server { os: "Linux".to_string(), os_version: "".to_string(),
         upnp_version: UPNP_VERSION1, product_name: "Sonos".to_string(), product_version: "85.0-64200 (ZPS29)".to_string() });
        assert_matches!(boot_id, Some(id) if id == &6);
        assert!(config_id.is_none());
        assert_matches!(port, UpnpPort::Default);
        assert_matches!(secure_location, Some(secure_location)
            if secure_location == &Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
        );
        assert!(device_type.is_none());
        assert!(services.is_empty());
    }

    #[test]
    fn update_from_notify() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");

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
        let message = response.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        assert!(devices.inner.contains_key(&url));

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
        let message = notify.parse::<Message>().expect("valid notify");
        devices.process(message).expect("process notify");
        let root_device = devices.inner.get(&url).expect("device created");
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
            device_type,
            embedded_devices,
            services,
        } = root_device;
        assert_eq!(id, &Some(uuid!("c4248768-d6b6-4232-a273-5b1701524493")));
        assert!(last_seen > &DateTime::parse_from_rfc3339("2026-04-29T08:22:03+00:00").unwrap());
        assert_eq!(valid_until, &(*last_seen + Duration::from_secs(1800)));
        assert_eq!(location, &url);
        assert_matches!(product, Some(product) if product == &Server { os: "Linux".to_string(), os_version: "".to_string(),
         upnp_version: UPNP_VERSION1, product_name: "Sonos".to_string(), product_version: "85.0-64200 (ZPS29)".to_string() });
        assert_matches!(boot_id, Some(id) if id == &6);
        assert!(config_id.is_none());
        assert_matches!(port, UpnpPort::Default);
        assert_matches!(secure_location, Some(secure_location)
            if secure_location == &Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
        );
        assert!(device_type.is_none());
        assert!(services.is_empty());
    }

    #[test]
    fn identify_root_device_type() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");

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
        let message = response.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        assert!(
            devices
                .inner
                .get(&url)
                .expect("root device registered")
                .device_type
                .is_none()
        );

        let device = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: urn:schemas-upnp-org:device:ZonePlayer:1
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::urn:schemas-upnp-org:device:ZonePlayer:1
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
        let message = device.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        assert_matches!(
            devices
                .inner
                .get(&url)
                .expect("root device registered")
                .device_type,
            Some(DeviceDetails {
                vendor: Vendor::Standard,
                device: Device::ZonePlayer { ver: 1 }
            })
        );
    }

    #[test]
    fn promote_device_to_root() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");
        let id = uuid!("c4248768-d6b6-4232-a273-5b1701524493");
        let devicedetails = DeviceDetails {
            vendor: Vendor::Standard,
            device: Device::ZonePlayer { ver: 1 },
        };

        let device = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: urn:schemas-upnp-org:device:ZonePlayer:1
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::urn:schemas-upnp-org:device:ZonePlayer:1
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
        let message = device.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        {
            // need &devices
            let root_device = devices.inner.get(&url).expect("root device registered");
            assert!(root_device.device_type.is_none());
            let embedded_device = root_device
                .embedded_devices
                .get(&id)
                .expect("device embedded");
            assert_eq!(embedded_device, &devicedetails);
            assert!(root_device.id.is_none());
        } // drop &devices

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
        let message = response.parse::<Message>().expect("valid message");
        devices.process(message).expect("process message");
        let root_device = devices.inner.get(&url).expect("root device registered");
        assert_eq!(root_device.device_type, Some(devicedetails));
        assert!(root_device.embedded_devices.is_empty());
        assert_eq!(root_device.id, Some(id));
    }

    #[test]
    fn add_service() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");

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
        let root_device = root_device.parse::<Message>().expect("valid message");
        devices
            .process(root_device)
            .expect("process root device message");
        assert!(devices.inner.contains_key(&url));
        {
            // acquire &devices
            let root_device = devices.inner.get(&url).expect("root device is there");
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
            let root_device = devices.inner.get(&url).expect("root device still there");
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
    //TODO: Work out new semantics - store in an inferred device in an inferred root device??
    fn infer_root_from_service() {
        let mut devices = DeviceMap::new();
        let url =
            Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");

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
        let root_device = devices.inner.get(&url).expect("root device still there");
        assert_eq!(root_device.services.len(), 1);
        assert!(
            root_device.last_seen > (Utc::now() - Duration::from_secs(60)),
            "root device last seen at {}",
            root_device.last_seen
        );
        assert_eq!(
            root_device.valid_until,
            root_device.last_seen + Duration::from_secs(2400)
        );
        assert!(root_device.id.is_none());
    }
}
