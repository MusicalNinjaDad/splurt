use std::{
    cmp::max,
    collections::{HashMap, HashSet, hash_map::Entry},
};

use url::Url;
use uuid::Uuid;

use crate::{
    devicemap::{
        controlpoint::ControlPoint,
        rootdevice::{EmbeddedDevice, IsKnown, RootDevice},
    },
    message::{
        Message, MulticastSearch, Notify, Response, ST,
        msearch::{MSearch, UnicastSearch},
        notify::{Alive, ByeBye, NT, Update},
    },
};

pub mod controlpoint;
pub mod rootdevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(clippy::large_enum_variant, reason = "most likely case is Device")]
pub enum Information {
    Device { root_device: RootDevice, id: Uuid },
    // TODO handle BootID & ConfigID in byebye
    Removal { id: Uuid },
    // TODO handle updates to BootID
    Update,
    ControlPoint { control_point: ControlPoint },
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
            })) => {
                let id = usn.uuid;
                let mut embedded_device = EmbeddedDevice {
                    id,
                    device_type: None,
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
                match usn.ntst {
                    NT::RootDevice => {
                        root_device.id = Some(id);
                        Self::Device { root_device, id }
                    }
                    NT::Device(device) => {
                        embedded_device.device_type = Some(device);
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                    NT::Service(service) => {
                        embedded_device.services.insert(service);
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                    NT::Uuid(_) => {
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                }
            }
            Message::Search(MSearch::Multicast(MulticastSearch {
                mx: _,
                st,
                user_agent,
                port,
                friendly_name,
                uuid,
            })) => Self::ControlPoint {
                control_point: ControlPoint {
                    interested_in: vec![st],
                    product: user_agent,
                    port,
                    friendly_name,
                    uuid: uuid.map(Into::into),
                },
            },
            Message::Search(MSearch::Unicast(UnicastSearch {
                host: _,
                st,
                user_agent,
            })) => Self::ControlPoint {
                control_point: ControlPoint {
                    interested_in: vec![st],
                    product: user_agent,
                    port: Default::default(),
                    friendly_name: None,
                    uuid: None,
                },
            },
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
            }) => {
                let id = usn.uuid;
                let mut embedded_device = EmbeddedDevice {
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
                match usn.ntst {
                    ST::RootDevice => {
                        root_device.id = Some(usn.uuid);
                        Self::Device { root_device, id }
                    }
                    ST::Device(device) => {
                        embedded_device.device_type = Some(device);
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                    ST::Service(service) => {
                        embedded_device.services.insert(service);
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                    ST::Uuid(_) | ST::All => {
                        root_device.embedded_devices.insert(id, embedded_device);
                        Self::Device { root_device, id }
                    }
                }
            }
            Message::Notify(Notify::ByeBye(ByeBye {
                usn,
                #[expect(unused_variables, reason = "todo handle BootID & ConfigID in byebye")]
                boot_id,
                #[expect(unused_variables, reason = "todo handle BootID & ConfigID in byebye")]
                config_id,
            })) => Self::Removal { id: usn.uuid },
            #[expect(unused_variables, reason = "todo Update")]
            Message::Notify(Notify::Update(Update {
                location,
                usn,
                boot_id,
                config_id,
                next_boot_id,
                port,
                secure_location,
            })) => Self::Update,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceMap {
    root_devices: HashMap<Url, RootDevice>,
    ids: HashMap<Uuid, Url>,
    // TODO Properly handle control points
    control_points: HashSet<ControlPoint>,
}

impl DeviceMap {
    pub fn new() -> Self {
        Self {
            root_devices: Default::default(),
            ids: Default::default(),
            control_points: Default::default(),
        }
    }

    pub fn process(&mut self, message: Message) {
        let info = message.into();
        match info {
            Information::Device { root_device, id } => {
                if matches!(root_device.is_known(), IsKnown::Known)
                    && let Some(known_root_device) = self.root_devices.get(&root_device.location)
                    && let Some(known_id) = known_root_device.id
                    && known_id != id
                {
                    // Something else was previously known at this address!
                    self.ids.remove(&id);
                    for embedded_device in known_root_device.embedded_devices.keys() {
                        self.ids.remove(embedded_device);
                    }
                    self.root_devices.remove(&root_device.location);
                }
                // TODO: handling non-UUID IDs - check validity of insert (see docs re: update key)
                self.ids.insert(id, root_device.location.clone());
                match self.root_devices.entry(root_device.location.clone()) {
                    Entry::Occupied(mut known_rd) => {
                        let known_rd = known_rd.get_mut();
                        enum About {
                            RootDevice,
                            DeviceOrService,
                        }
                        let update_describes = match root_device.is_known() {
                            IsKnown::Inferred => About::DeviceOrService,
                            IsKnown::Known => About::RootDevice,
                        };
                        let RootDevice {
                            id: update_rd_id,
                            last_seen,
                            valid_until,
                            location: _,
                            product,
                            boot_id,
                            config_id,
                            port,
                            secure_location,
                            device_type: _,
                            mut embedded_devices,
                            services: _,
                        } = root_device;
                        match (known_rd.is_known(), update_describes) {
                            // Confirmation of root device details
                            (IsKnown::Inferred, About::RootDevice) => {
                                known_rd.id = update_rd_id;
                                // If we already have more details about device type & direct services,
                                // promote that embedded device.
                                if let Some(device) = known_rd.embedded_devices.remove(&id) {
                                    known_rd.device_type = device.device_type;
                                    known_rd.services.extend(device.services);
                                }
                            }

                            // Info on the device_type or direct services for a known root device
                            (IsKnown::Known, About::DeviceOrService)
                                if known_rd.id == Some(id)
                                    && let Some(device) = embedded_devices.remove(&id) =>
                            {
                                if device.device_type.is_some() {
                                    known_rd.device_type = device.device_type;
                                }
                                known_rd.services.extend(device.services);
                            }

                            // We already have some info on this embedded device
                            _ if let Some(known_device) = known_rd.embedded_devices.remove(&id)
                                && let Some(update_device) = embedded_devices.get_mut(&id) =>
                            {
                                // Update it accordingly before it gets inserted later
                                if update_device.device_type.is_none() {
                                    update_device.device_type = known_device.device_type;
                                }
                                update_device.services.extend(known_device.services);
                            }

                            // Either a direct update to a known root device, or a previously unknown embedded device
                            _ => {
                                // Do nothing except the general updates below
                            }
                        }
                        known_rd.last_seen = max(known_rd.last_seen, last_seen);
                        known_rd.valid_until = max(known_rd.valid_until, valid_until);
                        if known_rd.config_id.is_none() || config_id > known_rd.config_id {
                            known_rd.config_id = config_id;
                            known_rd.product = product;
                            known_rd.boot_id = boot_id;
                            known_rd.port = port;
                            known_rd.secure_location = secure_location;
                        }
                        known_rd.embedded_devices.extend(embedded_devices);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(root_device);
                    }
                }
            }
            Information::Removal { id } => {
                let location = self.ids.remove(&id);
                if let Some(location) = location {
                    self.root_devices.remove(&location);
                }
            }
            Information::Update => {
                // TODO: when handling BootId & Config ID properly. Until then irrelevant.
            }
            Information::ControlPoint { control_point } => {
                self.control_points.insert(control_point);
            }
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
