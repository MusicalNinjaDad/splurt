use std::{
    collections::{HashMap, hash_map::Values, hash_set},
    io,
    net::SocketAddr,
    ops::{FromResidual, Try},
};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    CompletedFrame, Terminal,
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph, StatefulWidget, Widget},
};
use ssdp_rs::{
    devicemap::{
        DeviceMap,
        rootdevice::{EmbeddedDevice, RootDevice},
    },
    message::{Message, ParseError, ServiceDetails, header::Lenient},
};
use url::Url;
use uuid::Uuid;

use crate::Exit;

pub(crate) struct Ui<B>
where
    B: Backend,
{
    terminal: Terminal<B>,
    devices: DeviceListing,
    errors: ErrorListing,
    focus: FocusHolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Default)]
enum FocusHolder {
    #[default]
    None,
    Devices,
    Errors,
}

impl FocusHolder {
    fn next(&mut self) {
        *self = match self {
            FocusHolder::None => Self::Devices,
            FocusHolder::Devices => Self::Errors,
            FocusHolder::Errors => Self::Devices,
        }
    }
}

impl Ui<CrosstermBackend<io::Stdout>> {
    pub fn new() -> Self {
        let terminal = ratatui::init();
        Self {
            terminal,
            devices: Default::default(),
            errors: Default::default(),
            focus: Default::default(),
        }
    }
}

pub trait HandleEvent {
    type Output;

    /// Handle the given event, returning Some(Output) if this leads to a situation which should
    /// be handled by the caller.
    fn handle_event(&mut self, event: Option<io::Result<Event>>) -> Option<Self::Output>;
}

impl<B: Backend> HandleEvent for Ui<B>
where
    Exit<()>: FromResidual<<Result<!, B::Error> as Try>::Residual>,
{
    type Output = Exit<()>;

    fn handle_event(&mut self, event: Option<io::Result<Event>>) -> Option<Self::Output> {
        match event {
            None => Some(Exit::IO("Keyboard handler closed".to_string())),
            Some(Err(e)) => Some(try bikeshed Exit<()> { Err(e)? }),
            Some(Ok(Event::Key(event))) if event == KeyCode::Esc.into() => Some(Exit::Ok(())),
            Some(Ok(Event::Key(event))) if event == KeyCode::Tab.into() => {
                self.focus.next();
                match self.render() {
                    Ok(_) => None,
                    Err(e) => Some(try bikeshed Exit<()> { Err(e)? }),
                }
            }
            _ => None,
        }
    }
}

impl<B: Backend> Ui<B> {
    pub fn render(&mut self) -> Result<CompletedFrame<'_>, B::Error> {
        self.terminal.draw(|frame| {
            let [device_listing, error_listing] =
                Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(frame.area());
            self.devices
                .render(device_listing, frame.buffer_mut(), &mut self.focus);
            self.errors.render(error_listing, frame.buffer_mut());
        })
    }

    pub fn process_device(&mut self, message: Message) {
        self.devices.devices.process(message);
    }

    pub fn process_error(&mut self, error: ParseError, sent_by: SocketAddr) {
        match self.errors.errors.entry(sent_by) {
            std::collections::hash_map::Entry::Occupied(mut grrr) => {
                grrr.get_mut().push(error);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(vec![error]);
            }
        }
    }
}

impl<B: Backend> Drop for Ui<B> {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DeviceListing {
    devices: DeviceMap,
}

impl StatefulWidget for &DeviceListing {
    type State = FocusHolder;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut FocusHolder)
    where
        Self: Sized,
    {
        let border_type = match state {
            FocusHolder::Devices => BorderType::Double,
            _ => BorderType::default(),
        };
        let device_text = Paragraph::new(DeviceLines::from(&self.devices))
            .block(Block::bordered().border_type(border_type).title("devices"));
        device_text.render(area, buf);
    }
}

struct DeviceLines<'devices> {
    rootdevices: Values<'devices, Url, RootDevice>,
    embedded_devices: Option<Values<'devices, Lenient<Uuid>, EmbeddedDevice>>,
    services: Option<hash_set::Iter<'devices, ServiceDetails>>,
}

impl<'d> From<&'d DeviceMap> for DeviceLines<'d> {
    fn from(devicemap: &'d DeviceMap) -> Self {
        Self {
            rootdevices: devicemap.devices().values(),
            embedded_devices: None,
            services: None,
        }
    }
}

impl<'d> From<DeviceLines<'d>> for Text<'d> {
    fn from(devicelines: DeviceLines<'d>) -> Self {
        Text::from_iter(devicelines)
    }
}

impl<'d> Iterator for DeviceLines<'d> {
    type Item = Line<'d>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(services) = self.services.as_mut() {
            match services.next() {
                Some(s) => return Some(format!("   └[ ] {}", s).into()),
                None => self.services = None,
            }
        }
        if let Some(embedded_devices) = self.embedded_devices.as_mut() {
            match embedded_devices.next() {
                Some(ed) => {
                    if !ed.services.is_empty() {
                        self.services = Some(ed.services.iter());
                    }
                    let dt = match &ed.device_type {
                        Some(d) => d.to_string(),
                        None => "Unknown".to_string(),
                    };
                    return Some(
                        format!(
                            " └[ ] {}: {} offering {} services",
                            ed.id,
                            dt,
                            ed.services.len()
                        )
                        .into(),
                    );
                }
                None => self.embedded_devices = None,
            }
        };
        let rd = self.rootdevices.next()?;
        if !rd.embedded_devices.is_empty() {
            self.embedded_devices = Some(rd.embedded_devices.values());
        }
        if !rd.services.is_empty() {
            self.services = Some(rd.services.iter());
        }
        let dt = match &rd.device_type {
            Some(d) => d.to_string(),
            None => "Unknown".to_string(),
        };
        Some(
            format!(
                "[ ] {}: {} with {} embedded devices",
                rd.location,
                dt,
                rd.embedded_devices.len()
            )
            .into(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ErrorListing {
    errors: HashMap<SocketAddr, Vec<ParseError>>,
}

impl Widget for &ErrorListing {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let error_text = Text::from_iter(self.errors.iter().map(|(addr, errs)| {
            format!(
                "{addr}: has {} errors. First is: {:?}",
                errs.len(),
                errs.first().unwrap()
            )
        }));
        let error_text = Paragraph::new(error_text).block(Block::bordered().title("errors"));
        error_text.render(area, buf);
    }
}

#[cfg(test)]
mod tests {

    use ratatui::style::Style;

    use super::*;

    #[test]
    fn list_root_devices() {
        let device = r#"HTTP/1.1 200 OK
CACHE-CONTROL: max-age = 1800
DATE: Wed, 29 Apr 2026 08:22:03 GMT
EXT:
LOCATION: http://192.168.0.84:1400/xml/device_description.xml
SERVER: Linux UPnP/1.0 Sonos/85.0-64200 (ZPS29)
ST: upnp:rootdevice
USN: uuid:c4248768-d6b6-4232-a273-5b1701524493::upnp:rootdevice
X-RINCON-HOUSEHOLD: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3
X-RINCON-BOOTSEQ: 6
BOOTID.UPNP.ORG: 6
X-RINCON-WIFIMODE: 1
X-RINCON-VARIANT: 2
HOUSEHOLD.SMARTSPEAKER.AUDIO: Sonos_J9hfdYcBvSBCyHLo5tPwpI9Cm3.9LpAqreapUbAY1tsy5BF
LOCATION.SMARTSPEAKER.AUDIO: lc_4e8119cfb08d4c5083b6e0c75e47fe50
SECURELOCATION.UPNP.ORG: https://192.168.0.84:1443/xml/device_description.xml
X-SONOS-HHSECURELOCATION: https://192.168.0.84:1843/xml/device_description.xml

"#;
        let mut devices = DeviceMap::new();
        let message = device.parse().expect("root device message");
        devices.process(message);
        let listing = DeviceListing { devices };
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let expected_text = "[ ] http://192.168.0.84:1400/xml/device_description.xml: Unknown";
        let mut expected_buf = buf.clone();
        expected_buf.set_string(0, 0, expected_text, Style::default());
        let mut state = Default::default();
        listing.render(area, &mut buf, &mut state);
        assert_eq!(buf, expected_buf);
    }
}
