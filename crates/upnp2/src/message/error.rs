use std::{
    error::Error,
    fmt::Display,
    net::{self, AddrParseError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    EmptyMessage,
    InvalidBootId(String),
    InvalidConfigId(String),
    InvalidDate(String),
    InvalidDevice(String),
    InvalidDeviceDetails(String),
    InvalidService(String),
    InvalidDuration(String),
    InvalidHost(String),
    InvalidIPAddress(net::AddrParseError),
    InvalidLocation(String),
    InvalidMan(String),
    InvalidMethod(String),
    InvalidMx(String),
    InvalidNT(String),
    InvalidNTS(String),
    InvalidNextBootId(String),
    InvalidPort(String),
    InvalidProductTokens(String),
    InvalidSecureLocation(String),
    InvalidST(String),
    InvalidUrn(String),
    InvalidUsn(String),
    InvalidUUID(uuid::Error),
    InvalidVersion(String),
    MissingBootId,
    MissingConfigId,
    MissingField(String),
    MissingFriendlyName,
    MissingNextBootId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ErrorKind,
    source: Option<Box<ParseError>>,
}

impl From<AddrParseError> for ErrorKind {
    fn from(err: AddrParseError) -> Self {
        Self::InvalidIPAddress(err)
    }
}

impl From<uuid::Error> for ErrorKind {
    fn from(err: uuid::Error) -> Self {
        Self::InvalidUUID(err)
    }
}

impl From<ErrorKind> for ParseError {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }
}

impl ParseError {
    pub fn chain_from(err: ParseError, kind: ErrorKind) -> Self {
        Self {
            kind,
            source: Some(Box::new(err)),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.source.as_deref() {
            Some(parse_error) => Some(parse_error),
            None => None,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            None => write!(f, "{}", self.kind),
            Some(source) => write!(f, "{}, {}", self.kind, source),
        }
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
            ErrorKind::InvalidDevice(device) => {
                write!(f, "{} is not a valid upnp device specification", device)
            }
            ErrorKind::InvalidDeviceDetails(device) => {
                write!(f, "{} is not a valid upnp device:ver specification", device)
            }
            ErrorKind::InvalidHost(host) => write!(f, "{host} is not a valid host in this context"),
            ErrorKind::InvalidIPAddress(err) => write!(f, "{err}"),
            ErrorKind::InvalidLocation(location) => write!(f, "{location} is not a valid url"),
            ErrorKind::InvalidMan(man) => write!(f, "{man} is not a valid MAN value"),
            ErrorKind::InvalidMethod(method) => write!(f, "{} is not a valid upnp method", method),
            ErrorKind::InvalidMx(mx) => write!(
                f,
                "{mx} is not a valid MX value. Only integers 0..=5 are valid."
            ),
            ErrorKind::InvalidNextBootId(id) => write!(
                f,
                "{id} is not a valid new BootId. It must be an integer and larger than the old BootId"
            ),
            ErrorKind::InvalidNT(nt) => write!(f, "{} is not a valid NT in this context", nt),
            ErrorKind::InvalidNTS(nts) => write!(f, "{} is not a valid NTS in this context", nts),
            ErrorKind::InvalidPort(port) => write!(f, "{port} is not a valid IP port"),
            ErrorKind::InvalidProductTokens(tokens) => {
                write!(f, "{tokens} is not valid as a UPnP user agent / server")
            }
            ErrorKind::InvalidSecureLocation(location) => write!(
                f,
                "{location} is not a valid secure location (must be a valid URL beginning with `https://` and containing a port number)"
            ),
            ErrorKind::InvalidService(service) => {
                write!(f, "{} is not a valid upnp service specification", service)
            }
            ErrorKind::InvalidST(st) => write!(f, "{} is not a valid upnp search type", st),
            ErrorKind::InvalidUrn(urn) => {
                write!(f, "{} is not a valid upnp universal resource name", urn)
            }
            ErrorKind::InvalidUsn(usn) => write!(
                f,
                "{usn} is not a valid USN (Unique Search Name). Valid forms are: `uuid:device-UUID::upnp:rootdevice`, `uuid:device-UUID` & `uuid:device-UUID::urn:...`"
            ),
            ErrorKind::InvalidUUID(err) => write!(f, "{err}"),
            ErrorKind::InvalidVersion(ver) => write!(
                f,
                "{ver} is not a valid Version. Valid form is `major.minor`"
            ),
            ErrorKind::MissingBootId => {
                write!(f, "a boot instance is required from UPnp/2.0 onwards")
            }
            ErrorKind::MissingConfigId => write!(
                f,
                "a configuration number is required from UPnp/2.0 onwards"
            ),
            ErrorKind::MissingField(field) => write!(f, "header is missing field {field}"),
            ErrorKind::MissingFriendlyName => {
                write!(f, "a friendly name is required from UPnp/2.0 onwards")
            }
            ErrorKind::MissingNextBootId => {
                write!(f, "a new boot instance is required from UPnp/2.0 onwards")
            }
        }
    }
}
