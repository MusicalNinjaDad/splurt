use crate::{
    devicemap::rootdevice::RootDevice,
    message::{Device, DeviceDetails, Message, Server, Service, UPNP_VERSION1, UpnpPort, Vendor},
};

use std::{collections::HashSet, time::Duration};

use super::*;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use url::Url;
use uuid::uuid;

const ROOT: &str = r#"HTTP/1.1 200 OK
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

const DEVICE: &str = r#"HTTP/1.1 200 OK
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

const SERVICE: &str = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
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

const ID: Uuid = uuid!("c4248768-d6b6-4232-a273-5b1701524493");

const DATE: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 22, 3).unwrap()),
    Utc,
);

const VALID_UNTIL: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 52, 3).unwrap()),
    Utc,
);

fn url() -> Url {
    Url::parse("http://192.168.0.84:1400/xml/device_description.xml").unwrap()
}

fn server() -> Server {
    Server {
        os: "Linux".to_string(),
        os_version: "".to_string(),
        upnp_version: UPNP_VERSION1,
        product_name: "Sonos".to_string(),
        product_version: "85.0-64200 (ZPS29)".to_string(),
    }
}

const BOOT_ID: Option<u32> = Some(6);

fn secure_url() -> Url {
    Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
}

const DEVICE_DETAILS: DeviceDetails = DeviceDetails {
    vendor: Vendor::Standard,
    device: Device::ZonePlayer { ver: 1 },
};

const SERVICE_DETAILS: ServiceDetails = ServiceDetails {
    vendor: Vendor::Standard,
    service: Service::MusicServices { ver: 1 },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum IsKnown {
    Inferred,
    Known,
}

use IsKnown::{Inferred, Known};

fn validate_root_device(
    devices: &DeviceMap,
    is_known: IsKnown,
    last_seen: Option<DateTime<Utc>>,
    valid_until: Option<DateTime<Utc>>,
    device: Option<DeviceDetails>,
    service: Option<ServiceDetails>,
) -> &RootDevice {
    let root_device = devices.inner.get(&url()).expect("device created");
    match is_known {
        Inferred => assert!(root_device.id.is_none()),
        Known => assert_eq!(root_device.id, Some(ID)),
    };
    if let Some(timestamp) = last_seen {
        assert_eq!(root_device.last_seen, timestamp);
    };
    if let Some(timestamp) = valid_until {
        assert_eq!(root_device.valid_until, timestamp);
    }
    assert_eq!(root_device.location, url());
    assert_eq!(root_device.product, Some(server()));
    assert_eq!(root_device.boot_id, BOOT_ID);
    assert!(root_device.config_id.is_none());
    assert_eq!(root_device.port, UpnpPort::Default);
    assert_eq!(root_device.secure_location, Some(secure_url()));
    let services = match service {
        Some(service) => {
            let mut services = HashSet::new();
            services.insert(service);
            services
        }
        None => HashSet::new(),
    };
    match (device, is_known) {
        (Some(device), Known) => {
            assert_eq!(root_device.device_type, Some(device));
            assert!(root_device.embedded_devices.is_empty());
            assert_eq!(root_device.services, services);
        }
        (Some(device), Inferred) => {
            assert!(root_device.device_type.is_none());
            let embedded_device = root_device
                .embedded_devices
                .get(&ID)
                .expect("device embedded");
            assert_eq!(
                embedded_device,
                &EmbeddedDevice {
                    id: ID,
                    device_type: Some(device),
                    services
                }
            );
            assert!(root_device.services.is_empty());
        }
        (None, _) => {
            assert!(root_device.device_type.is_none());
            assert!(root_device.embedded_devices.is_empty());
            assert_eq!(root_device.services, services);
        }
    };
    root_device
}

#[test]
fn root_from_response() {
    let mut devices = DeviceMap::new();

    let message = ROOT.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(&devices, Known, Some(DATE), Some(VALID_UNTIL), None, None);
}

#[test]
fn update_from_notify() {
    let mut devices = DeviceMap::new();

    let message = ROOT.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(&devices, Known, Some(DATE), Some(VALID_UNTIL), None, None);

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
    let root_device = validate_root_device(&devices, Known, None, None, None, None);
    assert!(root_device.last_seen > (Utc::now() - Duration::from_secs(60)));
    assert_eq!(
        root_device.valid_until,
        root_device.last_seen + Duration::from_secs(1800)
    );
}

#[test]
fn identify_root_device_type() {
    let mut devices = DeviceMap::new();

    let message = ROOT.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(&devices, Known, Some(DATE), Some(VALID_UNTIL), None, None);

    let message = DEVICE.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(
        &devices,
        Known,
        Some(DATE),
        Some(VALID_UNTIL),
        Some(DEVICE_DETAILS),
        None,
    );
}

#[test]
fn promote_device_to_root() {
    let mut devices = DeviceMap::new();

    let message = DEVICE.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(
        &devices,
        Inferred,
        Some(DATE),
        Some(VALID_UNTIL),
        Some(DEVICE_DETAILS),
        None,
    );

    let message = ROOT.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(
        &devices,
        Known,
        Some(DATE),
        Some(VALID_UNTIL),
        Some(DEVICE_DETAILS),
        None,
    );
}

#[test]
fn add_service() {
    let mut devices = DeviceMap::new();

    let message = ROOT.parse::<Message>().expect("valid message");
    devices.process(message).expect("process message");
    validate_root_device(&devices, Known, Some(DATE), Some(VALID_UNTIL), None, None);

    let service = SERVICE.parse::<Message>().expect("valid service");
    devices.process(service).expect("process service message");
    validate_root_device(
        &devices,
        Known,
        Some(DATE),
        Some(VALID_UNTIL),
        None,
        Some(SERVICE_DETAILS),
    );
}

#[test]
fn infer_root_from_service() {
    let mut devices = DeviceMap::new();
    let url = Url::parse("http://192.168.0.84:1400/xml/device_description.xml").expect("valid url");
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
    let root_device = devices.inner.get(&url).expect("root device inferred");
    assert!(root_device.id.is_none());
    assert!(
        root_device.last_seen > (Utc::now() - Duration::from_secs(60)),
        "root device last seen at {}",
        root_device.last_seen
    );
    assert_eq!(
        root_device.valid_until,
        root_device.last_seen + Duration::from_secs(2400)
    );
    assert!(root_device.services.is_empty());
    assert_eq!(root_device.embedded_devices.len(), 1);
    let embedded_device = root_device
        .embedded_devices
        .get(&id)
        .expect("inferred embedded device");
    assert_eq!(embedded_device.id, id);
    assert!(embedded_device.device_type.is_none());
    assert_eq!(embedded_device.services.len(), 1);
    let service = embedded_device.services.iter().next().unwrap();
    assert_eq!(
        service,
        &ServiceDetails {
            vendor: Vendor::Standard,
            service: Service::MusicServices { ver: 1 }
        }
    );
}
