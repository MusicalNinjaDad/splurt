//! A heirarchical map of rootdevice[/device]/service

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use url::Url;
use uuid::Uuid;

use crate::message::{
    BootId, ConfigId, DeviceDetails, Location, MaxAge, SecureLocation, Server, ServiceDetails,
    UpnpPort, header::Lenient,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootDevice {
    /// None if this is an inferred root device
    pub id: Option<Lenient<Uuid>>,
    pub(crate) last_seen: DateTime<Utc>,
    pub(crate) valid_until: DateTime<Utc>,
    /// URL for UPnP description for root device
    pub location: Url,
    /// OS/version UPnP/2.0 product/version
    pub product: Option<Server>,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub boot_id: Option<u32>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub config_id: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    pub port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    pub secure_location: Option<Url>,
    /// The core device type of the root device, if known
    pub device_type: Option<DeviceDetails>,
    pub embedded_devices: HashMap<Lenient<Uuid>, EmbeddedDevice>,
    /// Services directly offered by this root device
    pub services: HashSet<ServiceDetails>,
}

impl RootDevice {
    #[allow(
        clippy::too_many_arguments,
        reason = "Need to construct RootDevice from deconstructed Message fields.
        Cannot use `From` implementations as nested matches on Messages need to own fields."
    )]
    pub(crate) fn new(
        id: Option<Lenient<Uuid>>,
        max_age: MaxAge,
        date: Option<DateTime<Utc>>,
        location: Location,
        server: Server,
        boot_id: Option<BootId>,
        config_id: Option<ConfigId>,
        port: UpnpPort,
        secure_location: Option<SecureLocation>,
    ) -> Self {
        let last_seen = date.unwrap_or_else(Utc::now);
        let valid_until = last_seen + *max_age.as_duration();

        Self {
            id,
            last_seen,
            valid_until,
            location: location.into_url(),
            product: Some(server),
            boot_id: boot_id.map(|id| *id.as_u32()),
            config_id: config_id.map(|id| *id.as_u32()),
            port,
            secure_location: secure_location.map(|loc| loc.into_url()),
            device_type: None,
            embedded_devices: Default::default(),
            services: Default::default(),
        }
    }

    pub fn is_known(&self) -> IsKnown {
        match self.id {
            Some(_) => IsKnown::Known,
            None => IsKnown::Inferred,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsKnown {
    Inferred,
    Known,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An embedded device, only containing the data which is not common across the entire RootDevice
pub struct EmbeddedDevice {
    pub id: Lenient<Uuid>,
    /// None if inferred as a home for a lonely service
    pub device_type: Option<DeviceDetails>,
    pub services: HashSet<ServiceDetails>,
}
