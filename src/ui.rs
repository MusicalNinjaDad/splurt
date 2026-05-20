use std::{collections::HashMap, io, net::SocketAddr};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    CompletedFrame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    text::Text,
    widgets::{Block, Paragraph, Widget},
};
use ssdp_rs::{devicemap::DeviceMap, message::ParseError};

use crate::{DeviceLines, Exit};

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

impl<B: Backend> Ui<B> {
    /// Returns `Some(Exit)` if an event occurs which leads to an exit condition.
    pub fn handle_event(&self, event: Option<io::Result<Event>>) -> Option<Exit<()>> {
        match event {
            None => Some(Exit::IO("Keyboard handler closed".to_string())),
            Some(Err(e)) => Some(try bikeshed Exit<()> { Err(e)? }),
            Some(Ok(Event::Key(event))) if event == KeyCode::Esc.into() => Some(Exit::Ok(())),
            _ => None,
        }
    }

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
