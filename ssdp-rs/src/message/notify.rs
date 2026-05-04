//! `NOTIFY *` messages

use super::{ErrorKind, ParseError, UpnpHeader, Uri};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive,
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let _nts: NTS = header.try_get("NTS")?.parse::<Uri>()?.try_into()?;
        todo!("tryfrom header for notify")
    }
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

    fn try_from(_uri: Uri) -> Result<Self, Self::Error> {
        todo!("tryfrom uri for nts")
    }
}
