use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseError {
    EmptyMessage,
    InvalidBootId(String),
    InvalidConfigId(String),
    InvalidDate(String),
    InvalidDevice(String),
    InvalidDeviceDetails(String),
    InvalidDuration(String),
    InvalidLocation(String),
    InvalidMethod(String),
    InvalidPort(String),
    InvalidSecureLocation(String),
    InvalidST(String),
    InvalidUrn(String),
    InvalidUserAgent(String),
    MissingBootId,
    MissingConfigId,
    MissingField(String),
}

impl std::error::Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyMessage => write!(f, "empty message"),
            ParseError::InvalidBootId(boot_id) => {
                write!(f, "{boot_id} is not a valid boot instance")
            }
            ParseError::InvalidConfigId(config_id) => {
                write!(f, "{config_id} is not a valid configuration number")
            }
            ParseError::InvalidDate(date) => write!(f, "{date} is not a valid date"),
            ParseError::InvalidDuration(duration) => {
                write!(f, "{duration} is not a valid duration")
            }
            ParseError::InvalidDevice(device) => write!(
                f,
                "{} is not a valid upnp device specification (valid forms are `urn:domain-name:device:deviceType:ver` & `urn:schemas-upnp-org:device:deviceType:ver`)",
                device
            ),
            ParseError::InvalidDeviceDetails(device) => {
                write!(f, "{} is not a valid upnp device:ver specification", device)
            }
            ParseError::InvalidLocation(location) => write!(f, "{location} is not a valid url"),
            ParseError::InvalidMethod(method) => write!(f, "{} is not a valid upnp method", method),
            ParseError::InvalidPort(port) => write!(f, "{port} is not a valid IP port"),
            ParseError::InvalidSecureLocation(location) => write!(
                f,
                "{location} is not a valid secure location (must be a valid URL beginning with `https://` and containing a port number)"
            ),
            ParseError::InvalidST(st) => write!(f, "{} is not a valid upnp search type", st),
            ParseError::InvalidUrn(urn) => {
                write!(f, "{} is not a valid upnp universal resource name", urn)
            }
            ParseError::InvalidUserAgent(user_agent) => {
                write!(f, "{user_agent} is not a valid user agent")
            }
            ParseError::MissingBootId => {
                write!(f, "a boot instance is required from UPnp/2.0 onwards")
            }
            ParseError::MissingConfigId => write!(
                f,
                "a configuration number is required from UPnp/2.0 onwards"
            ),
            ParseError::MissingField(field) => write!(f, "header is missing field {field}"),
        }
    }
}
