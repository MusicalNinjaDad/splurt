use super::*;

use crate::message::{
    Device, DeviceDetails, Message, ProductTokens, Server, Service, ServiceDetails, UPNP_VERSION1,
    Vendor, header::Lenient::Valid,
};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use uuid::uuid;

pub const ROOT: &str = r#"HTTP/1.1 200 OK
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

pub fn root_msg() -> Message {
    ROOT.parse().expect("root device message")
}

pub const ROOT_UUID: &str = r#"HTTP/1.1 200 OK
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

pub fn root_uuid_msg() -> Message {
    ROOT_UUID.parse().expect("root uuid message")
}

// TODO remove complication: different times, and add specific test
pub const DEVICE: &str = r#"HTTP/1.1 200 OK
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

pub fn device_msg() -> Message {
    DEVICE.parse().expect("device message")
}

pub const EMBEDDED_DEVICE: &str = r#"HTTP/1.1 200 OK
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

pub fn emb_dev_msg() -> Message {
    EMBEDDED_DEVICE.parse().expect("embedded device message")
}

pub const SERVICE: &str = r#"HTTP/1.1 200 OK
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

pub fn service_msg() -> Message {
    SERVICE.parse().expect("service message")
}

pub const SERVICE_BYEBYE: &str = r#"NOTIFY * HTTP/1.1
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

pub fn service_byebye() -> Message {
    SERVICE_BYEBYE.parse().expect("service byebye")
}

pub const ID: Lenient<Uuid> = Valid(uuid!("c4248768-d6b6-4232-a273-5b1701524493"));

pub const EMBEDDED_DEVICE_ID: Lenient<Uuid> = Valid(uuid!("a4a60994-e188-4dd7-b3f5-3b5c6f47e036"));

pub const ROOT_TIMESTAMP: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 22, 3).unwrap()),
    Utc,
);

pub const ROOT_VALIDITY: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 52, 3).unwrap()),
    Utc,
);

pub const DEVICE_TIMESTAMP: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 22, 5).unwrap()),
    Utc,
);

pub const DEVICE_VALIDITY: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
    NaiveDate::from_ymd_opt(2026, 4, 29)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(8, 53, 5).unwrap()),
    Utc,
);

pub fn url() -> Url {
    Url::parse("http://192.168.0.84:1400/xml/device_description.xml").unwrap()
}

pub fn server() -> Server {
    Server::Valid(ProductTokens {
        os: "Linux".to_string(),
        os_version: "".to_string(),
        upnp_version: UPNP_VERSION1,
        product_name: "Sonos".to_string(),
        product_version: "85.0-64200 (ZPS29)".to_string(),
    })
}

pub const BOOT_ID: Option<u32> = Some(6);

pub fn secure_url() -> Url {
    Url::parse("https://192.168.0.84:1443/xml/device_description.xml").expect("valid https url")
}

pub const DEVICE_DETAILS: DeviceDetails = DeviceDetails {
    vendor: Vendor::Standard,
    device: Device::ZonePlayer { ver: 1 },
};

pub const SERVICE_DETAILS: ServiceDetails = ServiceDetails {
    vendor: Vendor::Standard,
    service: Service::MusicServices { ver: 1 },
};

pub fn embedded_devices() -> HashMap<Lenient<Uuid>, EmbeddedDevice> {
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
