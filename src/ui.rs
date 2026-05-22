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

pub trait HandleEvent<B: Backend> {
    type Output: Try;

    /// Handle the given event, returning Some(Output) if this leads to a situation which should
    /// be handled by the caller.
    fn handle_event(&mut self, event: Option<io::Result<Event>>) -> Option<Self::Output>
    where
        Self::Output: FromResidual<<Result<!, B::Error> as Try>::Residual>;
}

impl<B: Backend> HandleEvent<B> for Ui<B> {
    type Output = Exit<()>;

    fn handle_event(&mut self, event: Option<io::Result<Event>>) -> Option<Self::Output>
    where
        Self::Output: FromResidual<<Result<!, B::Error> as Try>::Residual>,
    {
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
