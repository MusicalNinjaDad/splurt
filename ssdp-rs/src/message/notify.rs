//! `NOTIFY *` messages

use std::{fmt::Display, str::FromStr};

use derive_more::Display;
use uuid::Uuid;

use crate::message::{
    DeviceDetails, Header, HeaderExt, Host, MaxAge, Method, ServiceDetails, Target, UpnpNss,
    UpnpPort,
    header::{BootId, ConfigId, Location, NextBootId, SecureLocation, Server, UpnpV2Ext, Usn},
};

use super::{ErrorKind, ParseError, SsdpNss, UpnpHeader, Uri};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Notify {
    Alive(Alive),
    ByeBye(ByeBye),
    Update(Update),
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        header
            .try_get(Host::HEADER_KEY)?
            .parse::<Host>()?
            .check_multicast()?;
        let nts = header.try_get(NTS::HEADER_KEY)?.parse()?;

        match nts {
            NTS::Alive => Ok(Self::Alive(header.try_into()?)),
            NTS::ByeBye => Ok(Self::ByeBye(header.try_into()?)),
            NTS::Update => Ok(Self::Update(header.try_into()?)),
        }
    }
}

impl Notify {
    pub fn nt(&self) -> &NT {
        match self {
            Notify::Alive(alive) => &alive.usn.ntst,
            Notify::ByeBye(bye_bye) => &bye_bye.usn.ntst,
            Notify::Update(update) => &update.usn.ntst,
        }
    }

    pub fn into_nt(self) -> NT {
        match self {
            Notify::Alive(alive) => alive.usn.ntst,
            Notify::ByeBye(bye_bye) => bye_bye.usn.ntst,
            Notify::Update(update) => update.usn.ntst,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// TODO consider derive-getters = "0.5.0" and removing pub access to fields
pub struct Alive {
    /// `CACHE-CONTROL`: Duration (in seconds) until advertisement expires
    pub max_age: MaxAge,
    /// `URL` for UPnP description for root device
    pub location: Location,
    /// `SERVER`: OS/version UPnP/2.0 product/version
    pub server: Server,
    /// `USN`: Field value contains Unique Service Name. Identifies a unique instance of a device
    /// or service. Obeys strict rules in relation to `NT` and therefore acts as the primary store
    /// of both the NT and the UUID.
    pub usn: Usn<NT>,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub boot_id: Option<BootId>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub config_id: Option<ConfigId>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    pub port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    pub secure_location: Option<SecureLocation>,
}

impl<'h> TryFrom<UpnpHeader<'h>> for Alive {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let max_age = MaxAge::get_from(&header)?;
        let location = Location::get_from(&header)?;
        let nt = NT::get_from(&header)?;
        let server = Server::get_from(&header)?;
        let usn = Usn::get_validated(&header, &nt)?;
        let boot_id = Option::<BootId>::get_validated(&header, server.upnp_version)?;
        let config_id = Option::<ConfigId>::get_validated(&header, server.upnp_version)?;
        let port = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let secure_location = Option::<SecureLocation>::get_from(&header)?;
        Ok(Self {
            max_age,
            location,
            server,
            usn,
            boot_id,
            config_id,
            port,
            secure_location,
        })
    }
}

impl Display for Alive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            max_age,
            location,
            server,
            usn,
            boot_id,
            config_id,
            #[expect(unused_variables, reason = "todo handle port in message output")]
            port,
            secure_location,
        } = self;
        writeln!(f, "{}", Method::Notify)?;
        Host::default().write_header(f)?;
        max_age.write_header(f)?;
        location.write_header(f)?;
        usn.ntst.write_header(f)?;
        NTS::Alive.write_header(f)?;
        server.write_header(f)?;
        usn.write_header(f)?;
        boot_id.write_header(f)?;
        config_id.write_header(f)?;
        secure_location.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// When a device is about to be removed from the network, it should explicitly revoke its discovery
/// messages by sending one multicast message for each ssdp:alive message it sent.
///
/// If a control point has received at least one ssdp:byebye message of a root device, any of its
/// embedded devices or any of its services then the control point can assume that all are no longer
/// available.
pub struct ByeBye {
    /// `USN`: Field value contains Unique Service Name. Identifies a unique instance of a device
    /// or service. Obeys strict rules in relation to `NT` and therefore acts as the primary store
    /// of both the NT and the UUID.
    pub usn: Usn<NT>,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub boot_id: Option<BootId>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub config_id: Option<ConfigId>,
}

impl<'h> TryFrom<UpnpHeader<'h>> for ByeBye {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let nt = NT::get_from(&header)?;
        let usn = Usn::get_validated(&header, &nt)?;
        // TODO - document Boot & ConfigID validation must be done by something that has a
        // suitable cache from previous Alive & Update notifications
        let boot_id = Option::<BootId>::get_from(&header)?;
        let config_id = Option::<ConfigId>::get_from(&header)?;
        Ok(Self {
            usn,
            boot_id,
            config_id,
        })
    }
}

impl Display for ByeBye {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            usn,
            boot_id,
            config_id,
        } = self;
        writeln!(f, "{}", Method::Notify)?;
        Host::default().write_header(f)?;
        usn.ntst.write_header(f)?;
        NTS::ByeBye.write_header(f)?;
        usn.write_header(f)?;
        boot_id.write_header(f)?;
        config_id.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Update {
    /// `URL` for UPnP description for root device
    pub location: Location,
    /// `USN`: Field value contains Unique Service Name. Identifies a unique instance of a device
    /// or service. Obeys strict rules in relation to `NT` and therefore acts as the primary store
    /// of both the NT and the UUID.
    pub usn: Usn<NT>,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub boot_id: Option<BootId>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub config_id: Option<ConfigId>,
    /// `NEXTBOOTID.UPNP.ORG`: contains the new BOOTID.UPNP.ORG field value that the device intends
    /// to use in the subsequent device and service announcement messages. It shall be greater than
    /// the field value of the BOOTID.UPNP.ORG header field.
    pub next_boot_id: Option<NextBootId>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    pub port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    pub secure_location: Option<SecureLocation>,
}

impl<'h> TryFrom<UpnpHeader<'h>> for Update {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let location = Location::get_from(&header)?;
        let nt = NT::get_from(&header)?;
        let usn = Usn::get_validated(&header, &nt)?;
        // No Server line so validation must happen downstream where info is cached.
        let boot_id = Option::<BootId>::get_from(&header)?;
        let config_id = Option::<ConfigId>::get_from(&header)?;
        let next_boot_id = Option::<NextBootId>::get_from(&header)?;
        let valid_next_boot_id = match next_boot_id {
            Some(new_id)
                if let Some(old_id) = boot_id
                    && new_id > old_id =>
            {
                Ok(())
            }
            Some(new_id) => Err(ErrorKind::InvalidNextBootId(new_id.to_string())),
            None if boot_id.is_none() => Ok(()),
            None => Err(ErrorKind::MissingNextBootId),
        };
        valid_next_boot_id?;
        let port = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let secure_location = Option::<SecureLocation>::get_from(&header)?;
        Ok(Self {
            location,
            usn,
            boot_id,
            config_id,
            next_boot_id,
            port,
            secure_location,
        })
    }
}

impl Display for Update {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            location,
            usn,
            boot_id,
            config_id,
            next_boot_id,
            #[expect(unused_variables, reason = "todo handle port in message output")]
            port,
            secure_location,
        } = self;
        writeln!(f, "{}", Method::Notify)?;
        Host::default().write_header(f)?;
        location.write_header(f)?;
        usn.ntst.write_header(f)?;
        NTS::Update.write_header(f)?;
        usn.write_header(f)?;
        boot_id.write_header(f)?;
        config_id.write_header(f)?;
        next_boot_id.write_header(f)?;
        secure_location.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
    }
}

/// The NT values available for NOTIFY. This should usually be refered to as `notify::NT`
/// and not brought directly into scope via `use notify::NT` in order to disambiguate from
/// `NT` values for other message types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NT {
    /// `upnp:rootdevice`: Sent once for root device.
    RootDevice,
    /// `uuid:device-UUID`: Sent once for each device, root or embedded, where
    /// `device-UUID` is specified by the UPnP vendor.
    Uuid(Uuid),
    /// `urn:schemas-upnp-org:device:deviceType:ver`:
    ///     Sent once for each device, root or embedded, where deviceType and ver are defined by
    ///     UPnP Forum working committee, and ver specifies the version of the device type.
    /// `urn:domain-name:device:deviceType:ver`:
    ///     Sent once for each device, root or embedded, where domain-name is a Vendor Domain
    ///     Name, deviceType and ver are defined by the UPnP vendor, and ver specifies the version
    ///     of the device type. Period characters in the Vendor Domain Name shall be replaced with
    ///     hyphens in accordance with RFC 2141.
    /// TODO: #36 DeviceTypes
    Device(DeviceDetails),
    /// `urn:schemas-upnp-org:service:serviceType:ver`:
    ///     Sent once for each service where serviceType and ver are defined by UPnP Forum working
    ///     committee and ver specifies the version of the service type.
    /// `urn:domain-name:service:serviceType:ver`:
    ///     Sent once for each service where domain-name is a Vendor Domain Name, serviceType and
    ///     ver are defined by UPnP vendor, and ver specifies the version of the service type.
    ///     Period characters in the Vendor Domain Name shall be replaced with hyphens in
    ///     accordance with RFC 2141.
    /// TODO: #37 ServiceTypes
    Service(ServiceDetails),
}

impl Header for NT {
    const HEADER_KEY: &'static str = "NT";
}

impl FromStr for NT {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = s.parse::<Uri>()?;
        Ok(uri.try_into()?)
    }
}

impl TryFrom<Uri> for NT {
    type Error = ErrorKind;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri {
            Uri::Upnp(UpnpNss::RootDevice) => Ok(Self::RootDevice),
            Uri::Urn(Target::Device(device)) => Ok(Self::Device(device)),
            Uri::Urn(Target::Service(service)) => Ok(Self::Service(service)),
            Uri::Uuid { uuid, suffix: None } => Ok(Self::Uuid(uuid)),
            _ => Err(ErrorKind::InvalidNT(uri.to_string())),
        }
    }
}

impl PartialEq<Uri> for NT {
    fn eq(&self, uri: &Uri) -> bool {
        match self {
            Self::RootDevice => matches!(uri, Uri::Upnp(UpnpNss::RootDevice)),
            Self::Uuid(this_uuid) => {
                matches!(uri, Uri::Uuid { uuid, suffix: None } if uuid == this_uuid)
            }
            Self::Device(this_device) => {
                matches!(uri, Uri::Urn(Target::Device(device)) if device == this_device)
            }
            Self::Service(this_service) => {
                matches!(uri, Uri::Urn(Target::Service(service)) if service == this_service)
            }
        }
    }
}

impl PartialEq<NT> for Uri {
    fn eq(&self, nt: &NT) -> bool {
        match self {
            Uri::Upnp(UpnpNss::RootDevice) => matches!(nt, NT::RootDevice),
            Uri::Uuid {
                uuid: this_uuid,
                suffix: None,
            } => matches!(nt, NT::Uuid(uuid) if uuid == this_uuid),
            Uri::Urn(Target::Device(this_device)) => {
                matches!(nt, NT::Device(device) if device == this_device)
            }
            Uri::Urn(Target::Service(this_service)) => {
                matches!(nt, NT::Service(service) if service == this_service)
            }
            _ => false,
        }
    }
}

impl Display for NT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootDevice => write!(f, "upnp:rootdevice"),
            Self::Uuid(uuid) => write!(f, "uuid:{}", uuid),
            Self::Device(device_details) => write!(f, "urn:{device_details}"),
            Self::Service(service_details) => write!(f, "urn:{service_details}"),
        }
    }
}

/// The NTS values available for NOTIFY. This should usually be refered to as `notify::NTS`
/// and not brought directly into scope via `use notify::NTS` in order to disambiguate from
/// `NTS` values which may be added in future for other message types (e.g. for eventing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NTS {
    Alive,
    ByeBye,
    Update,
}

impl Header for NTS {
    const HEADER_KEY: &'static str = "NTS";
}

impl FromStr for NTS {
    type Err = ParseError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri.parse()? {
            Uri::Ssdp(SsdpNss::Alive) => Ok(Self::Alive),
            Uri::Ssdp(SsdpNss::ByeBye) => Ok(Self::ByeBye),
            Uri::Ssdp(SsdpNss::Update) => Ok(Self::Update),
            _ => Err(ErrorKind::InvalidNTS(uri.to_string()))?,
        }
    }
}

impl Display for NTS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NTS::Alive => write!(f, "{}", Uri::Ssdp(SsdpNss::Alive)),
            NTS::ByeBye => write!(f, "{}", Uri::Ssdp(SsdpNss::ByeBye)),
            NTS::Update => write!(f, "{}", Uri::Ssdp(SsdpNss::Update)),
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::uuid;

    use crate::message::{Device, Vendor};

    use super::*;

    #[test]
    fn display_ssdp_alive() {
        let output = format!("{}", Uri::Ssdp(SsdpNss::Alive));
        assert_eq!(output, "ssdp:alive");
    }

    #[test]
    #[should_panic(
        expected = r#"valid notify: ParseError { kind: InvalidUsn("uuid:f3d5b7e9-77f3-497e-ab39-ce2bc90001e8"), source: None }"#
    )]
    fn parse_alive() {
        let msg = r#"HOST: 239.255.255.250:1900
CACHE-CONTROL: max-age=100
LOCATION: http://192.168.5.26:80/description.xml
SERVER: Hue/1.0 UPnP/1.0 IpBridge/1.77.0
NTS: ssdp:alive
hue-bridgeid: CE2BC90001E8
NT: urn:schemas-upnp-org:device:basic:1
USN: uuid:f3d5b7e9-77f3-497e-ab39-ce2bc90001e8

"#;
        let header: UpnpHeader = msg.lines().collect();
        let alive: Alive = header.try_into().expect("valid notify");
        assert_eq!(
            alive.usn.ntst,
            NT::Device(DeviceDetails {
                vendor: Vendor::Standard,
                device: Device::Basic { ver: 1 }
            })
        );
        assert_eq!(
            alive.usn.uuid,
            uuid!("f3d5b7e9-77f3-497e-ab39-ce2bc90001e8")
        );
    }
}
