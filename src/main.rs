#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(future_join)]
#![feature(never_type)]
#![feature(try_blocks_heterogeneous)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    collections::HashMap,
    fmt::Debug,
    future::join,
    io,
    net::{Ipv4Addr, SocketAddr},
    ops::{Deref, DerefMut},
    pin::pin,
    process::Termination as _T,
};

use clap::Parser;
use crossterm::event::EventStream;
use exit_safely::Termination;
use futures::{FutureExt, SinkExt, StreamExt, channel::mpsc::unbounded, select};
use ratatui::{
    CompletedFrame, Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Layout},
    text::Text,
    widgets::{Block, Paragraph, Widget},
};
use try_v2::{Try, Try_ConvertResult};

use ssdp_rs::{
    Listener, Searcher,
    devicemap::DeviceMap,
    message::{Message, ParseError},
};

mod cli;
use cli::*;

struct Ui<B>
where
    B: Backend,
{
    terminal: Terminal<B>,
}

impl<B: Backend> Deref for Ui<B> {
    type Target = Terminal<B>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl<B: Backend> DerefMut for Ui<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Ui<CrosstermBackend<io::Stdout>> {
    fn new() -> Self {
        let terminal = ratatui::init();
        Self { terminal }
    }
}

impl<B: Backend> Ui<B> {
    /// Returns `Some(Exit)` if an event occurs which leads to an exit condition.
    fn handle_event(&self, event: Option<io::Result<Event>>) -> Option<Exit<()>> {
        match event {
            None => Some(Exit::IO("Keyboard handler closed".to_string())),
            Some(Err(e)) => Some(try bikeshed Exit<()> { Err(e)? }),
            Some(Ok(Event::Key(event))) if event == KeyCode::Esc.into() => Some(Exit::Ok(())),
            _ => None,
        }
    }

    fn render(
        &mut self,
        devices: &DeviceMap,
        errors: &HashMap<SocketAddr, Vec<ParseError>>,
    ) -> Result<CompletedFrame<'_>, B::Error> {
        let device_text = Text::from_iter(devices.devices().values().map(|rd| {
            format!(
                "{}: {:?} with {} embedded devices",
                rd.location,
                rd.device_type,
                rd.embedded_devices.len()
            )
        }));
        let device_text = Paragraph::new(device_text).block(Block::bordered().title("devices"));
        let error_text = Text::from_iter(errors.iter().map(|(addr, errs)| {
            format!(
                "{addr}: has {} errors. First is: {:?}",
                errs.len(),
                errs.first().unwrap()
            )
        }));
        let error_text = Paragraph::new(error_text).block(Block::bordered().title("errors"));
        self.draw(|frame| {
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

fn main() -> Exit<()> {
    let splurt = Splurt::try_parse()?;

    match &splurt.command {
        Command::Snoop => {
            let (mut messages_tx, mut messages_rx) =
                unbounded::<(Result<Message, ParseError>, SocketAddr)>();

            let listen_loop = async {
                let mut listener = Listener::new(Ipv4Addr::UNSPECIFIED)?;
                try bikeshed Exit<!> {
                    loop {
                        let (msg, sent_by) = listener.next().await.expect("a message")?;
                        messages_tx.send((msg.parse(), sent_by)).await?;
                    }
                }
            };

            let render_loop = async {
                let mut ui = Ui::new();
                let mut devices = DeviceMap::new();
                let mut errors: HashMap<SocketAddr, Vec<ParseError>> = HashMap::new();
                ui.render(&devices, &errors)?;

                let mut events = EventStream::new();

                try bikeshed Exit<()> {
                    loop {
                        let mut messages = messages_rx.recv().fuse();
                        let mut events = events.next().fuse();
                        select! {
                            message = messages => {
                                let (msg, sent_by) = message?;
                                match msg {
                                    Ok(message) => devices.process(message),
                                    Err(e) => match errors.entry(sent_by) {
                                        std::collections::hash_map::Entry::Occupied(mut grrr) => {
                                            grrr.get_mut().push(e);
                                        }
                                        std::collections::hash_map::Entry::Vacant(entry) => {
                                            entry.insert(vec![e]);
                                        }
                                    },
                                }
                            },
                            event = events => if let Some(exit) = ui.handle_event(event) {
                                break exit?;
                            },
                        };
                        ui.render(&devices, &errors)?;
                    }
                }
            };

            let mut listen = pin!(listen_loop.fuse());
            let mut render = pin!(render_loop.fuse());
            let try_join = async {
                try bikeshed Exit<()> {
                    select!(
                        err = listen => err?,
                        exit = render => exit?,
                    )
                }
            };
            let exit = futures::executor::block_on(try_join);

            return exit;
        }

        Command::Listen => {
            let mut searcher = Searcher::new("splurt", "v0.0.1", "splurt ssdp repeater")?;
            let search = async {
                try bikeshed Exit<()> {
                    println!("sending an M-SEARCH");
                    searcher.search().await?
                }
            };

            let mut listener = Listener::new(Ipv4Addr::UNSPECIFIED)?;
            let listen_loop = async {
                try bikeshed Exit<()> {
                    loop {
                        println!("listening ...");
                        let (msg, sent_by) = listener.next().await.expect("a message")?;
                        println!("received: {} from {}", msg, sent_by);
                    }
                }
            };

            let both = join!(search, listen_loop);
            let (search, listen) = futures::executor::block_on(both);
            search?;
            listen?;
        }
    }
    Exit::Ok(())
}

#[derive(Debug, Termination, Try, Try_ConvertResult, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
#[must_use]
pub enum Exit<T: _T> {
    Ok(T) = 0,
    Error(String) = 1,
    InvocationError(String) = 2,
    IO(String) = 3,
}

impl<T: _T> From<clap::Error> for Exit<T> {
    fn from(e: clap::Error) -> Self {
        Self::InvocationError(e.to_string())
    }
}

impl<T: _T> From<io::Error> for Exit<T> {
    fn from(e: io::Error) -> Self {
        Self::IO(e.to_string())
    }
}

impl<T: _T> From<futures_util::task::SpawnError> for Exit<T> {
    fn from(e: futures_util::task::SpawnError) -> Self {
        Self::Error(e.to_string())
    }
}

impl<T: _T> From<ssdp_rs::Error> for Exit<T> {
    fn from(e: ssdp_rs::Error) -> Self {
        Self::Error(e.to_string())
    }
}

impl<T: _T> From<futures::channel::mpsc::RecvError> for Exit<T> {
    fn from(e: futures::channel::mpsc::RecvError) -> Self {
        Self::Error(e.to_string())
    }
}

impl<T: _T> From<futures::channel::mpsc::SendError> for Exit<T> {
    fn from(e: futures::channel::mpsc::SendError) -> Self {
        Self::Error(e.to_string())
    }
}
