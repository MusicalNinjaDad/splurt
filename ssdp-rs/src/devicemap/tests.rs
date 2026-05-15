use crate::{
    devicemap::rootdevice::{
        IsKnown::{self, Inferred, Known},
        RootDevice,
    },
    message::{
        Device, DeviceDetails, Message, ProductTokens, Server, Service, ServiceDetails,
        UPNP_VERSION1, UpnpPort, Vendor, header::Lenient::Valid,
    },
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

fn root_msg() -> Message {
    ROOT.parse().expect("root device message")
}

const ROOT_UUID: &str = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: uuid:c4248768-d6b6-4232-a273-5b1701524493
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493
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

fn root_uuid_msg() -> Message {
    ROOT_UUID.parse().expect("root uuid message")
}

// TODO remove complication: different times, and add specific test
const DEVICE: &str = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1860
DATE: Wed, 29 Apr 2026 08:22:05 GMT
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

fn device_msg() -> Message {
    DEVICE.parse().expect("device message")
}

const EMBEDDED_DEVICE: &str = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: urn:schemas-upnp-org:device:MediaServer:1
USN: uuid:a4a60994-e188-4dd7-b3f5-3b5c6f47e036::urn:schemas-upnp-org:device:MediaServer:1
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

fn emb_dev_msg() -> Message {
    EMBEDDED_DEVICE.parse().expect("embedded device message")
}

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

fn service_msg() -> Message {
    SERVICE.parse().expect("service message")
}

const SERVICE_BYEBYE: &str = r#"NOTIFY * HTTP/1.1
HOST: 239.255.255.250:1900
NT: urn:schemas-upnp-org:service:MusicServices:1
NTS: ssdp:byebye
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::urn:schemas-upnp-org:service:MusicServices:1
X-RINCON-HOUSEHOLD: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3
X-RINCON-BOOTSEQ: 6
BOOTID.UPNP.ORG: 6
X-RINCON-WIFIMODE: 1
X-RINCON-VARIANT: 2
HOUSEHOLD.SMARTSPEAKER.AUDIO: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3.9LpAqreapUbAY1tsy5BF
LOCATION.SMARTSPEAKER.AUDIO: lc_4e8119cfb08d4c5083b6e0c75e47fe50

"#;

fn service_byebye() -> Message {
    SERVICE_BYEBYE.parse().expect("service byebye")
}

const ID: Lenient<Uuid> = Valid(uuid!("c4248768-d6b6-4232-a273-5b1701524493"));

const EMBEDDED_DEVICE_ID: Lenient<Uuid> = Valid(uuid!("a4a60994-e188-4dd7-b3f5-3b5c6f47e036"));

const ROOT_TIMESTAMP: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 22, 3).unwrap()),
    Utc,
);

const ROOT_VALIDITY: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 52, 3).unwrap()),
    Utc,
);

const DEVICE_TIMESTAMP: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 22, 5).unwrap()),
    Utc,
);

const DEVICE_VALIDITY: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 53, 5).unwrap()),
    Utc,
);

fn url() -> Url {
    Url::parse("http://192.168.0.84:1400/xml/device_description.xml").unwrap()
}

fn server() -> Server {
    Server::Valid(ProductTokens {
        os: "Linux".to_string(),
        os_version: "".to_string(),
        upnp_version: UPNP_VERSION1,
        product_name: "Sonos".to_string(),
        product_version: "85.0-64200 (ZPS29)".to_string(),
    })
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

fn embedded_devices() -> HashMap<Lenient<Uuid>, EmbeddedDevice> {
    let mut embedded_devices = HashMap::new();
    let embedded_device = EmbeddedDevice {
        id: EMBEDDED_DEVICE_ID,
        device_type: Some(DeviceDetails {
            vendor: Vendor::Standard,
            device: Device::MediaServer { ver: 1 },
        }),
        services: Default::default(),
    };
    embedded_devices.insert(EMBEDDED_DEVICE_ID, embedded_device);
    embedded_devices
}

#[track_caller]
#[allow(clippy::too_many_arguments)]
fn validate_root_device(
    devices: &DeviceMap,
    is_known: IsKnown,
    last_seen: Option<DateTime<Utc>>,
    valid_until: Option<DateTime<Utc>>,
    device_type: Option<DeviceDetails>,
    root_service: Option<ServiceDetails>,
    embedded_devices: Option<HashMap<Lenient<Uuid>, EmbeddedDevice>>,
    known_ids: Vec<Lenient<Uuid>>,
) -> &RootDevice {
    for id in known_ids {
        assert_eq!(devices.ids.get(&id), Some(&url()));
    }
    let root_device = devices.root_devices.get(&url()).expect("device exists");
    dbg!(root_device);
    match is_known {
        Inferred => assert!(root_device.id.is_none()),
        Known => assert_eq!(root_device.id, Some(ID)),
    };
    assert_eq!(root_device.is_known(), is_known);
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
    let root_services = match root_service {
        Some(service) => {
            let mut services = HashSet::new();
            services.insert(service);
            services
        }
        None => HashSet::new(),
    };
    let mut embedded_devices = embedded_devices.unwrap_or_default();
    match (device_type, is_known) {
        (Some(device), Known) => {
            assert_eq!(root_device.device_type, Some(device));
            assert_eq!(root_device.embedded_devices, embedded_devices);
            assert_eq!(root_device.services, root_services);
        }
        (Some(device), Inferred) => {
            assert!(root_device.device_type.is_none());
            let device = EmbeddedDevice {
                id: ID,
                device_type: Some(device),
                services: root_services,
            };
            embedded_devices.insert(ID, device);
            assert_eq!(root_device.embedded_devices, embedded_devices);
            assert!(root_device.services.is_empty());
        }
        (None, Inferred) if !root_services.is_empty() => {
            assert!(root_device.device_type.is_none());
            let device = EmbeddedDevice {
                id: ID,
                device_type: None,
                services: root_services,
            };
            embedded_devices.insert(ID, device);
            assert_eq!(root_device.embedded_devices, embedded_devices);
            assert!(root_device.services.is_empty());
        }
        (None, _) => {
            assert!(root_device.device_type.is_none());
            assert_eq!(root_device.embedded_devices, embedded_devices);
            assert_eq!(root_device.services, root_services);
        }
    };
    root_device
}

#[test]
fn root_from_response() {
    let mut devices = DeviceMap::new();

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );
}

#[test]
fn update_from_notify() {
    let mut devices = DeviceMap::new();

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

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
    devices.process(message);
    let root_device = validate_root_device(&devices, Known, None, None, None, None, None, vec![ID]);
    assert!(root_device.last_seen > (Utc::now() - Duration::from_secs(60)));
    assert_eq!(
        root_device.valid_until,
        root_device.last_seen + Duration::from_secs(1800)
    );
}

#[test]
fn identify_root_device_type() {
    let mut devices = DeviceMap::new();

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

    devices.process(device_msg());
    validate_root_device(
        &devices,
        Known,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        None,
        None,
        vec![ID],
    );
}

#[test]
fn promote_device_to_root() {
    let mut devices = DeviceMap::new();

    devices.process(device_msg());
    validate_root_device(
        &devices,
        Inferred,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        None,
        None,
        vec![ID],
    );

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        None,
        None,
        vec![ID],
    );
}

#[test]
fn add_service() {
    let mut devices = DeviceMap::new();

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

    devices.process(service_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        Some(SERVICE_DETAILS),
        None,
        vec![ID],
    );
}

#[test]
fn infer_root_from_service() {
    let mut devices = DeviceMap::new();
    devices.process(service_msg());
    validate_root_device(
        &devices,
        Inferred,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        Some(SERVICE_DETAILS),
        None,
        vec![ID],
    );
}

#[test]
fn add_embedded_device() {
    let mut devices = DeviceMap::new();

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

    devices.process(emb_dev_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        Some(embedded_devices()),
        vec![ID, EMBEDDED_DEVICE_ID],
    );
}

#[test]
fn service_device_embedded_root() {
    let mut devices = DeviceMap::new();
    devices.process(service_msg());
    validate_root_device(
        &devices,
        Inferred,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        Some(SERVICE_DETAILS),
        None,
        vec![ID],
    );

    devices.process(device_msg());
    validate_root_device(
        &devices,
        Inferred,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        Some(SERVICE_DETAILS),
        None,
        vec![ID],
    );

    devices.process(emb_dev_msg());
    validate_root_device(
        &devices,
        Inferred,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        Some(SERVICE_DETAILS),
        Some(embedded_devices()),
        vec![ID, EMBEDDED_DEVICE_ID],
    );

    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(DEVICE_TIMESTAMP),
        Some(DEVICE_VALIDITY),
        Some(DEVICE_DETAILS),
        Some(SERVICE_DETAILS),
        Some(embedded_devices()),
        vec![ID, EMBEDDED_DEVICE_ID],
    );
}

#[test]
fn root_then_uuid() {
    let mut devices = DeviceMap::new();
    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

    devices.process(root_uuid_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );
}

#[test]
fn byebye_service() {
    let mut devices = DeviceMap::new();
    devices.process(root_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        None,
        None,
        vec![ID],
    );

    devices.process(service_msg());
    validate_root_device(
        &devices,
        Known,
        Some(ROOT_TIMESTAMP),
        Some(ROOT_VALIDITY),
        None,
        Some(SERVICE_DETAILS),
        None,
        vec![ID],
    );

    assert!(devices.root_devices.contains_key(&url()));
    assert!(devices.ids.contains_key(&ID));
    devices.process(service_byebye());
    assert!(devices.root_devices.is_empty());
    assert!(devices.ids.is_empty());
}
