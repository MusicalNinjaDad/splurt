use super::test_fixtures::*;
use super::*;

use crate::{
    devicemap::rootdevice::{
        IsKnown::{self, Inferred, Known},
        RootDevice,
    },
    message::{DeviceDetails, Message, ServiceDetails, UpnpPort},
};

use chrono::{DateTime, Utc};
use std::{collections::HashSet, time::Duration};

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
