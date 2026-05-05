use std::fmt::Display;

use uuid::Uuid;

use super::{FriendlyName, HeaderExt, Host, Man, Method, Mx, ProductTokens, ST};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MSearch {
    pub host: Host,
    pub mx: Mx,
    pub user_agent: Option<ProductTokens<"USER-AGENT">>,
    pub friendly_name: FriendlyName,
    pub uuid: Option<Uuid>,
}

/// Entire valid M-SEARCH message including initial method line,
/// as per OCF specification (2020) section 1.3.2
///
/// #### Note:
/// I've rarely actually seen a well-formed spec-conform M-SEARCH flying around my network
/// but there's nothing wrong with actually being fully valid!
impl Display for MSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            host,
            mx,
            user_agent,
            friendly_name,
            uuid,
        } = self;
        writeln!(f, "{}", Method::MSearch)?;
        host.write_header(f)?;
        Man::Discover.write_header(f)?;
        mx.write_header(f)?;
        ST::All.write_header(f)?;
        user_agent.write_header(f)?;
        friendly_name.write_header(f)?;
        uuid.write_header(f)?;
        // Must end with blank line as per spec:
        //   "Note: No body is present in requests with method M-SEARCH, but note that the
        //          message shall have a blank line following the last header field."
        writeln!(f)
    }
}
