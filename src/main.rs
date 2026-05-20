#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(future_join)]
#![feature(never_type)]
#![feature(try_blocks_heterogeneous)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    collections::{HashMap, hash_map::Values, hash_set},
    fmt::Debug,
    future::join,
    io,
    net::{Ipv4Addr, SocketAddr},
    pin::pin,
    process::Termination as _T,
};

use clap::Parser;
use crossterm::event::EventStream;
use exit_safely::Termination;
use futures::{FutureExt, SinkExt, StreamExt, channel::mpsc::unbounded, select};
use ratatui::text::{Line, Text};
use try_v2::{Try, Try_ConvertResult};
use url::Url;
use uuid::Uuid;

use ssdp_rs::{
    Listener, Searcher,
    devicemap::{
        DeviceMap,
        rootdevice::{EmbeddedDevice, RootDevice},
    },
    message::{Message, ParseError, ServiceDetails, header::Lenient},
};

mod cli;
use cli::*;
mod ui;
use ui::Ui;

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
