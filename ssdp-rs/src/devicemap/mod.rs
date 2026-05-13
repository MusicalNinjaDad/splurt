use std::collections::{HashMap, hash_map::Entry};

use url::Url;
use uuid::Uuid;

use crate::{
    devicemap::rootdevice::{EmbeddedDevice, RootDevice},
    message::{
        Message, Notify, Response, ST,
        notify::{Alive, NT},
    },
};

pub mod rootdevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Information {
    Device { root_device: RootDevice, id: Uuid },
    ControlPoint(Message),
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
                NT::RootDevice => Self::Device {
                    root_device: RootDevice::new(
                        Some(usn.uuid),
                        max_age,
                        None,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    ),
                    id: usn.uuid,
                },
                NT::Device(device) => {
                    let id = usn.uuid;
                    let embedded_device = EmbeddedDevice {
                        id,
                        device_type: Some(device),
                        services: Default::default(),
                    };
                    let mut root_device = RootDevice::new(
                        None,
                        max_age,
                        None,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    root_device.embedded_devices.insert(id, embedded_device);
                    Self::Device { root_device, id }
                }
                NT::Service(service) => {
                    let id = usn.uuid;
                    let mut embedded_device = EmbeddedDevice {
                        id,
                        device_type: None,
                        services: Default::default(),
                    };
                    embedded_device.services.insert(service);
                    let mut root_device = RootDevice::new(
                        None,
                        max_age,
                        None,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    root_device.embedded_devices.insert(id, embedded_device);
                    Self::Device { root_device, id }
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
                ST::RootDevice => Self::Device {
                    root_device: RootDevice::new(
                        Some(usn.uuid),
                        max_age,
                        date,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    ),
                    id: usn.uuid,
                },
                ST::Device(device) => {
                    let id = usn.uuid;
                    let embedded_device = EmbeddedDevice {
                        id,
                        device_type: Some(device),
                        services: Default::default(),
                    };
                    let mut root_device = RootDevice::new(
                        None,
                        max_age,
                        date,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    root_device.embedded_devices.insert(id, embedded_device);
                    Self::Device { root_device, id }
                }
                ST::Service(service) => {
                    let id = usn.uuid;
                    let mut embedded_device = EmbeddedDevice {
                        id,
                        device_type: None,
                        services: Default::default(),
                    };
                    embedded_device.services.insert(service);
                    let mut root_device = RootDevice::new(
                        None,
                        max_age,
                        date,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    root_device.embedded_devices.insert(id, embedded_device);
                    Self::Device { root_device, id }
                }
                ST::Uuid(id) => {
                    let embedded_device = EmbeddedDevice {
                        id,
                        device_type: None,
                        services: Default::default(),
                    };
                    let mut root_device = RootDevice::new(
                        None,
                        max_age,
                        date,
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    root_device.embedded_devices.insert(id, embedded_device);
                    Self::Device { root_device, id }
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

    pub fn process(&mut self, message: Message) -> Result<(), Error> {
        let info = message.into();
        match info {
            Information::Device { root_device, id } => {
                match self.inner.entry(root_device.location.clone()) {
                    Entry::Occupied(mut known_rd) => {
                        let known_rd = known_rd.get_mut();
                        known_rd.update_based_on(root_device, id);
                        Ok(())
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(root_device);
                        Ok(())
                    }
                }
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
mod tests;
