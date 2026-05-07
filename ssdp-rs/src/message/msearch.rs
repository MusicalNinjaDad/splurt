use std::fmt::Display;

use crate::message::{
    Header, ParseError, UPNP_VERSION1, UpnpHeader, UpnpPort,
    header::{ControlPointUuid, UpnpV2Ext, UserAgent},
};

use super::{FriendlyName, HeaderExt, Host, Man, Method, Mx, ST};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MSearch {
    Multicast(MulticastSearch),
}

// TODO - enum Multicast/Unicast
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MulticastSearch {
    pub mx: Mx,
    pub st: ST,
    pub user_agent: Option<UserAgent>,
    pub port: UpnpPort,
    pub friendly_name: Option<FriendlyName>,
    pub uuid: Option<ControlPointUuid>,
}

impl<'h> TryFrom<UpnpHeader<'h>> for MulticastSearch {
    type Error = ParseError;

    fn try_from(header: UpnpHeader<'h>) -> Result<Self, Self::Error> {
        Host::get_from(&header)?.check_multicast()?;
        Man::get_from(&header)?.check_discover()?;
        let mx = Mx::get_from(&header)?;
        let st = ST::get_from(&header)?;
        let user_agent = Option::<UserAgent>::get_from(&header)?;
        let upnp_version = match user_agent {
            Some(ref user_agent) => user_agent.upnp_version,
            None => UPNP_VERSION1,
        };
        let port: UpnpPort = header.get(UpnpPort::HEADER_KEY).try_into()?;
        let friendly_name = Option::<FriendlyName>::get_validated(&header, upnp_version)?;
        let uuid = Option::<ControlPointUuid>::get_from(&header)?;
        Ok(Self {
            mx,
            st,
            user_agent,
            port,
            friendly_name,
            uuid,
        })
    }
}

/// Entire valid M-SEARCH message including initial method line,
/// as per OCF specification (2020) section 1.3.2
///
/// #### Note:
/// I've rarely actually seen a well-formed spec-conform M-SEARCH flying around my network
/// but there's nothing wrong with actually being fully valid!
impl Display for MulticastSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            mx,
            st,
            user_agent,
            #[expect(unused_variables, reason = "todo handle port in MSearch output")]
            port,
            friendly_name,
            uuid,
        } = self;
        writeln!(f, "{}", Method::MSearch)?;
        Host::default().write_header(f)?;
        Man::Discover.write_header(f)?;
        mx.write_header(f)?;
        st.write_header(f)?;
        user_agent.write_header(f)?;
        friendly_name.write_header(f)?;
        uuid.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
    }
}
