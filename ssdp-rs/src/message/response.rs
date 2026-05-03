use chrono::{DateTime, NaiveDateTime, Utc};
use url::Url;

use super::{ErrorKind, Header, MaxAge, ParseError, RFC1123, ST, UpnpHeader, UpnpPort, UserAgent};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// A direct response to an `M-SEARCH` message.
///
/// "To be found by a network search, a device shall send a unicast UDP response to the source IP
/// address and port that sent the request to the multicast address." <- This represents one of
/// these messages.
pub struct Response {
    /// `CACHE-CONTROL`: Duration (in seconds) until advertisement expires
    max_age: MaxAge,
    /// `DATE`: when response was generated
    date: Option<DateTime<Utc>>,
    /// `EXT`: Required for backwards compatibility with UPnP 1.0. (Header field name only; no field value.)
    ext: Option<!>,
    /// `URL` for UPnP description for root device
    location: Url,
    /// `SERVER`: OS/version UPnP/2.0 product/version
    server: UserAgent<"SERVER">,
    /// `ST`: search target
    st: ST,
    /// `USN`: composite identifier for the advertisement
    ///
    /// **TODO** handle USN nicely
    usn: String,
    /// `BOOTID.UPNP.ORG`: the boot instance of the device expressed according to a monotonically
    /// increasing value. Control points can use this header field to detect the case when a device
    /// leaves and rejoins the network (“reboots” in UPnP terms). It can be used by
    /// control points for a number of purposes such as re-establishing desired event subscriptions,
    /// checking for changes to the device state that were not evented since the device was off-line.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    boot_id: Option<u32>,
    /// `CONFIGID.UPNP.ORG`: number used for caching description information.
    /// If a device sends out two messages with a `CONFIGID.UPNP.ORG` header field with the same field
    /// value, the configuration shall be the same at the moments that these messages were sent.
    /// This reduces peak loads on UPnP devices during startup and during network hiccups. Only if a
    /// control point receives an announcement of an unknown configuration is downloading required.
    ///
    /// Required for UPnPv2, not present in UPnPv1
    config_id: Option<u32>,
    /// `SEARCHPORT.UPNP.ORG`: number identifies port on which device responds to unicast M-SEARCH
    ///
    /// Optional (handled semantically in [UpnpPort])
    port: UpnpPort,
    /// `SECURELOCATION.UPNP.ORG`: provides a base URL, with `https:` scheme and a specific port.
    /// Required when device protection is implemented.
    secure_location: Option<Url>,
}
impl<'h> TryFrom<UpnpHeader<'h>> for Response {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let st = header.try_get(ST::HEADER_KEY)?.parse()?;
        let max_age = header.try_get(MaxAge::HEADER_KEY)?.parse()?;
        let date = header
            .get("DATE")
            .map(|date| {
                NaiveDateTime::parse_from_str(date, RFC1123)
                    .map(|tz| tz.and_utc())
                    .map_err(|_| ErrorKind::InvalidDate(date.to_string()))
            })
            .transpose()?;
        let ext = None;
        let location = header.try_get("LOCATION")?;
        let location = location
            .parse()
            .map_err(|_| ErrorKind::InvalidLocation(location.to_string()))?;
        let server: UserAgent<"SERVER"> = header.try_get("SERVER")?.parse()?;
        let usn = header.try_get("USN")?.to_string();
        let boot_id = header
            .get("BOOTID.UPNP.ORG")
            .map(|boot_id| {
                boot_id
                    .parse()
                    .map_err(|_| ErrorKind::InvalidBootId(boot_id.to_string()))
            })
            .transpose()?;
        let config_id = header
            .get("CONFIGID.UPNP.ORG")
            .map(|config_id| {
                config_id
                    .parse()
                    .map_err(|_| ErrorKind::InvalidConfigId(config_id.to_string()))
            })
            .transpose()?;
        match server.upnp_version.as_str() {
            // TODO parse the version number into Major,Minor
            "1.0" => (),
            _ => {
                if boot_id.is_none() {
                    Err(ErrorKind::MissingBootId)?;
                }
                if config_id.is_none() {
                    Err(ErrorKind::MissingConfigId)?;
                }
            }
        };
        let port = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let secure_location: Option<Url> = header
            .get("SECURELOCATION.UPNP.ORG")
            .map(|location| {
                location
                    .parse()
                    .map_err(|_| ErrorKind::InvalidSecureLocation(location.to_string()))
            })
            .transpose()?;
        if let Some(ref secure_location) = secure_location
            && (secure_location.scheme() != "https" || secure_location.port().is_none())
        {
            Err(ErrorKind::InvalidSecureLocation(
                secure_location.to_string(),
            ))?;
        };
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
