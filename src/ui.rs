use std::{
    collections::{HashMap, hash_map::Values, hash_set},
    io,
    net::SocketAddr,
};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    CompletedFrame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use ssdp_rs::{
    devicemap::{
        DeviceMap,
        rootdevice::{EmbeddedDevice, RootDevice},
    },
    message::{ParseError, ServiceDetails, header::Lenient},
};
use url::Url;
use uuid::Uuid;

use crate::Exit;

pub(crate) struct Ui<B>
where
    B: Backend,
{
    terminal: Terminal<B>,
}

impl Ui<CrosstermBackend<io::Stdout>> {
    pub fn new() -> Self {
        let terminal = ratatui::init();
        Self { terminal }
    }
}

pub trait HandleEvent {
    type Output;

    /// Handle the given event, returning Some(Output) if this leads to a situation which should
    /// be handled by the caller.
    fn handle_event(&self, event: Option<io::Result<Event>>) -> Option<Self::Output>;
}

impl<B: Backend> HandleEvent for Ui<B> {
    type Output = Exit<()>;

    fn handle_event(&self, event: Option<io::Result<Event>>) -> Option<Self::Output> {
        match event {
            None => Some(Exit::IO("Keyboard handler closed".to_string())),
            Some(Err(e)) => Some(try bikeshed Exit<()> { Err(e)? }),
            Some(Ok(Event::Key(event))) if event == KeyCode::Esc.into() => Some(Exit::Ok(())),
            _ => None,
        }
    }
}

impl<B: Backend> Ui<B> {
    pub fn render(
        &mut self,
        devices: &DeviceMap,
        errors: &HashMap<SocketAddr, Vec<ParseError>>,
    ) -> Result<CompletedFrame<'_>, B::Error> {
        let device_text =
            Paragraph::new(DeviceLines::from(devices)).block(Block::bordered().title("devices"));
        let error_text = Text::from_iter(errors.iter().map(|(addr, errs)| {
            format!(
                "{addr}: has {} errors. First is: {:?}",
                errs.len(),
                errs.first().unwrap()
            )
        }));
        let error_text = Paragraph::new(error_text).block(Block::bordered().title("errors"));
        self.terminal.draw(|frame| {
            let [device_listing, error_listing] =
                Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(frame.area());
            device_text.render(device_listing, frame.buffer_mut());
            error_text.render(error_listing, frame.buffer_mut());
        })
    }
}

impl<B: Backend> Drop for Ui<B> {
    fn drop(&mut self) {
        ratatui::restore();
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
