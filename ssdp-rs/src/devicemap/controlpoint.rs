//! Control Points on the network

use uuid::Uuid;

use crate::message::header::UserAgent;
use crate::message::{FriendlyName, ST, UpnpPort};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControlPoint {
    pub interested_in: Vec<ST>,
    pub product: Option<UserAgent>,
    pub port: UpnpPort,
    pub friendly_name: Option<FriendlyName>,
    pub uuid: Option<Uuid>,
}
