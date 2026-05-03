use std::{error::Error, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorKind {
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

#[derive(Debug)]
pub struct ParseError {
    pub kind: ErrorKind,
    source: Option<Box<dyn Error>>,
}

impl From<ErrorKind> for ParseError {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::EmptyMessage => write!(f, "empty message"),
            ErrorKind::InvalidBootId(boot_id) => {
                write!(f, "{boot_id} is not a valid boot instance")
            }
            ErrorKind::InvalidConfigId(config_id) => {
                write!(f, "{config_id} is not a valid configuration number")
            }
            ErrorKind::InvalidDate(date) => write!(f, "{date} is not a valid date"),
            ErrorKind::InvalidDuration(duration) => {
                write!(f, "{duration} is not a valid duration")
            }
            ErrorKind::InvalidDevice(device) => write!(
                f,
                "{} is not a valid upnp device specification (valid forms are `urn:domain-name:device:deviceType:ver` & `urn:schemas-upnp-org:device:deviceType:ver`)",
                device
            ),
            ErrorKind::InvalidDeviceDetails(device) => {
                write!(f, "{} is not a valid upnp device:ver specification", device)
            }
            ErrorKind::InvalidLocation(location) => write!(f, "{location} is not a valid url"),
            ErrorKind::InvalidMethod(method) => write!(f, "{} is not a valid upnp method", method),
            ErrorKind::InvalidPort(port) => write!(f, "{port} is not a valid IP port"),
            ErrorKind::InvalidSecureLocation(location) => write!(
                f,
                "{location} is not a valid secure location (must be a valid URL beginning with `https://` and containing a port number)"
            ),
            ErrorKind::InvalidST(st) => write!(f, "{} is not a valid upnp search type", st),
            ErrorKind::InvalidUrn(urn) => {
                write!(f, "{} is not a valid upnp universal resource name", urn)
            }
            ErrorKind::InvalidUserAgent(user_agent) => {
                write!(f, "{user_agent} is not a valid user agent")
            }
            ErrorKind::MissingBootId => {
                write!(f, "a boot instance is required from UPnp/2.0 onwards")
            }
            ErrorKind::MissingConfigId => write!(
                f,
                "a configuration number is required from UPnp/2.0 onwards"
            ),
            ErrorKind::MissingField(field) => write!(f, "header is missing field {field}"),
        }
    }
}
