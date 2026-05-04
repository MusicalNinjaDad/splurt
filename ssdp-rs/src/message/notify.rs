//! `NOTIFY *` messages

use super::{ParseError, UpnpHeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive,
}

impl<'h> TryFrom<UpnpHeader<'h>> for Notify {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        let _nts = header.try_get("NTS")?;
        todo!("tryfrom header for notify")
    }
}
