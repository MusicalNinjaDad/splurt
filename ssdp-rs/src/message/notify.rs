//! `NOTIFY *` messages

use std::net::SocketAddr;

use url::Url;

use crate::{
    MULTICAST,
    message::{Header, MaxAge},
};

use super::{ErrorKind, ParseError, SsdpNss, UpnpHeader, Uri};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive(Alive),
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let nts = header.try_get("NTS")?.parse::<Uri>()?.try_into()?;
        let host =
            try bikeshed Result<_, ErrorKind> { header.try_get("HOST")?.parse::<SocketAddr>()? };
        // Host MUST be Multicast address as per spec
        match host {
            Ok(addr) if addr == MULTICAST => (),
            Ok(addr) => Err(ErrorKind::InvalidHost(addr.to_string()))?,
            Err(err) if matches!(err, ErrorKind::MissingField(_)) => Err(err)?,
            Err(_err) => todo!("chain"),
        }
        let max_age = header.try_get(MaxAge::HEADER_KEY)?.parse()?;
        let location = header.try_get("LOCATION")?;
        let location = location
            .parse()
            .map_err(|_| ErrorKind::InvalidLocation(location.to_string()))?;
        match nts {
            NTS::Alive => Ok(Self::Alive(Alive { max_age, location })),
            #[expect(unreachable_patterns)]
            _ => todo!("tryfrom header for notify other NTS e.g. byebye"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alive {
    /// `CACHE-CONTROL`: Duration (in seconds) until advertisement expires
    pub(crate) max_age: MaxAge,
    /// `URL` for UPnP description for root device
    pub(crate) location: Url,
}

/// The NTS values available for NOTIFY. This should usually be refered to as `notify::NTS`
/// and not brought directly into scope via `use notify::NTS` in order to disambiguate from
/// `NTS` values which may be added in future for other message types (e.g. for eventing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NTS {
    Alive,
}

impl TryFrom<Uri> for NTS {
    type Error = ErrorKind;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri {
            Uri::Ssdp(SsdpNss::Alive) => Ok(Self::Alive),
            _ => Err(ErrorKind::InvalidNTS(uri.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_ssdp_alive() {
        let output = format!("{}", Uri::Ssdp(SsdpNss::Alive));
        assert_eq!(output, "ssdp:alive");
    }
}
