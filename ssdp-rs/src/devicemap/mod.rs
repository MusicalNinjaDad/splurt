use std::collections::{HashMap, hash_map::Entry};

use url::Url;
use uuid::Uuid;

use crate::{
    devicemap::rootdevice::{EmbeddedDevice, RootDevice},
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
    Device(DeviceInfo),
    Service(ServiceInfo),
    ControlPoint(Message),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    service: ServiceDetails,
    id: Uuid,
    inferred_root_device: RootDevice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    embedded_device: EmbeddedDevice,
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
                        location,
                        server,
                        boot_id,
                        config_id,
                        port,
                        secure_location,
                    );
                    let embedded_device = EmbeddedDevice {
                        id: usn.uuid,
                        device_type: Some(device),
                        services: Default::default(),
                    };
                    Self::Device(DeviceInfo {
                        embedded_device,
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

    pub fn process(&mut self, message: Message) -> Result<(), Error> {
        let info = message.into();
        match info {
            Information::RootDevice(this_rd) => {
                self.inner
                    .entry(this_rd.location.clone())
                    .and_modify(|known_rd| {
                        known_rd.update_validity(this_rd.last_seen, this_rd.valid_until);
                        match (this_rd.id, known_rd.id) {
                            (Some(this_id), Some(known_id)) if this_id != known_id => {
                                todo!("previously known with a different id")
                            }
                            (Some(this_id), _)
                                if let Some(this_device) =
                                    known_rd.embedded_devices.remove(&this_id) =>
                            {
                                known_rd.device_type = this_device.device_type;
                                known_rd.services.extend(this_device.services);
                                // Relies on invariant from previous arm: this_id != known_id
                                // Is a no-op if root_device was already known.
                                // Current code should mean we never get here in that case but best
                                // to leave this here for future saftey.
                                known_rd.id = Some(this_id);
                            }
                            _ => (),
                        }
                    })
                    .or_insert(this_rd);
                Ok(())
            }
            Information::Device(deviceinfo) => {
                let root_device = self
                    .inner
                    .entry(deviceinfo.inferred_root_device.location.clone())
                    // as long as a control point has received at least one advertisement that is still
                    // valid from a root device, any of its embedded devices or any of its services,
                    // then the control point can assume that all are available.
                    .and_modify(|rd| {
                        rd.update_validity(
                            deviceinfo.inferred_root_device.last_seen,
                            deviceinfo.inferred_root_device.valid_until,
                        );
                    })
                    .or_insert(deviceinfo.inferred_root_device);
                match root_device.id {
                    Some(id) if id == deviceinfo.embedded_device.id => {
                        root_device.device_type = deviceinfo.embedded_device.device_type
                    }
                    _ => {
                        root_device
                            .embedded_devices
                            .insert(deviceinfo.embedded_device.id, deviceinfo.embedded_device);
                    }
                }
                Ok(())
            }
            Information::Service(serviceinfo) => {
                match self
                    .inner
                    .entry(serviceinfo.inferred_root_device.location.clone())
                {
                    Entry::Occupied(mut known_rd) => {
                        let known_rd = known_rd.get_mut();
                        // as long as a control point has received at least one advertisement that is still
                        // valid from a root device, any of its embedded devices or any of its services,
                        // then the control point can assume that all are available.
                        known_rd.update_validity(
                            serviceinfo.inferred_root_device.last_seen,
                            serviceinfo.inferred_root_device.valid_until,
                        );
                        known_rd.services.insert(serviceinfo.service);
                    }
                    Entry::Vacant(entry) => {
                        let mut rd = serviceinfo.inferred_root_device;
                        let mut inferred_device = EmbeddedDevice {
                            id: serviceinfo.id,
                            device_type: rd.device_type.take(),
                            services: Default::default(),
                        };
                        inferred_device.services.insert(serviceinfo.service);
                        rd.id = None;
                        rd.embedded_devices.insert(serviceinfo.id, inferred_device);
                        entry.insert(rd);
                    }
                }
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
mod tests;
