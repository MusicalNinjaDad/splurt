use chrono::{DateTime, NaiveDateTime, Utc};

use crate::message::{
    HeaderExt,
    header::{BootId, ConfigId, Location, SecureLocation, Server, UpnpV2Ext_},
};

use super::{ErrorKind, Header, MaxAge, ParseError, RFC1123, ST, UpnpHeader, UpnpPort};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// A direct response to an `M-SEARCH` message.
///
/// "To be found by a network search, a device shall send a unicast UDP response to the source IP
/// address and port that sent the request to the multicast address." <- This represents one of
/// these messages.
pub struct Response {
    /// `CACHE-CONTROL`: Duration (in seconds) until advertisement expires
    pub max_age: MaxAge,
    /// `DATE`: when response was generated
    pub date: Option<DateTime<Utc>>,
    /// `EXT`: Required for backwards compatibility with UPnP 1.0. (Header field name only; no field value.)
    ext: Option<!>,
    /// `URL` for UPnP description for root device
    pub location: Location,
    /// `SERVER`: OS/version UPnP/2.0 product/version
    pub server: Server,
    /// `ST`: search target
    pub st: ST,
    /// `USN`: composite identifier for the advertisement
    ///
    /// **TODO** handle USN nicely
    pub usn: String,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub boot_id: BootId,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    pub config_id: ConfigId,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    pub port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    pub secure_location: Option<SecureLocation>,
}
impl<'h> TryFrom<UpnpHeader<'h>> for Response {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let max_age = MaxAge::get_from(&header)?;
        let date = header
            .get("DATE")
            .map(|date| {
                NaiveDateTime::parse_from_str(date, RFC1123)
                    .map(|tz| tz.and_utc())
                    .map_err(|_| ErrorKind::InvalidDate(date.to_string()))
            })
            .transpose()?;
        let ext = None;
        let location = Location::get_from(&header)?;
        let server = Server::get_from(&header)?;
        let st = ST::get_from(&header)?;
        let usn = header.try_get("USN")?.to_string();
        let boot_id = BootId::get_validated(&header, server.upnp_version)?;
        let config_id = ConfigId::get_validated(&header, server.upnp_version)?;
        let port = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let secure_location = Option::<SecureLocation>::get_from(&header)?;
        Ok(Self {
            max_age,
            date,
            ext,
            location,
            server,
            st,
            usn,
            boot_id,
            config_id,
            port,
            secure_location,
        })
    }
}
