use std::collections::HashMap;

use uuid::Uuid;

use crate::devicemap::rootdevice::RootDevice;

pub mod rootdevice;

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
}

impl Default for DeviceMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{devicemap::rootdevice::RootDevice, message::Message};

    use super::*;

    #[test]
    fn add_new_root_device() {
        let msg = r#"HTTP/1.1 200 OK
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
        let msg = msg.parse::<Message>().expect("valid message");
        let root_device: RootDevice = msg.try_into().expect("a root device");
        let old_entry = devices.insert(root_device);
        assert!(old_entry.is_none());
    }
}
