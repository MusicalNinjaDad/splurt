#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    fmt::Debug,
    io,
    process::{Child, Output, Termination as _T},
};

use exit_safely::Termination;
use try_v2::{Try, Try_ConvertResult};

pub mod commands;

#[derive(Debug, Termination, Try, Try_ConvertResult, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
#[must_use]
pub enum Exit<T: _T> {
    Ok(T) = 0,
    Error(String) = 1,
    InvocationError(String) = 2,
    IO(String) = 3,
}

impl Exit<()> {
    fn message(&self) -> &str {
        match self {
            Exit::Ok(_) => "",
            Exit::Error(m) => m,
            Exit::InvocationError(m) => m,
            Exit::IO(m) => m,
        }
    }

    fn replace_message(self, msg: String) -> Option<Self> {
        match self {
            Exit::Ok(_) => None,
            Exit::Error(_) => Some(Exit::Error(msg)),
            Exit::InvocationError(_) => Some(Exit::InvocationError(msg)),
            Exit::IO(_) => Some(Exit::IO(msg)),
        }
    }
}

impl FromIterator<Exit<()>> for Exit<()> {
    fn from_iter<I: IntoIterator<Item = Exit<()>>>(iter: I) -> Self {
        let mut msg = String::new();
        iter.into_iter()
            .filter_map(|e| {
                if let Exit::Ok(_) = e {
                    None
                } else {
                    msg.push_str(e.message());
                    msg.push('\n');
                    Some(e)
                }
            })
            .min()
            .and_then(|e| e.replace_message(msg))
            .unwrap_or(Exit::Ok(()))
    }
}

impl<T: _T> From<clap::Error> for Exit<T> {
    fn from(e: clap::Error) -> Self {
        Self::InvocationError(e.to_string())
    }
}

#[derive(Debug)]
pub struct Cmd {
    pub name: &'static str,
    pub result: Result<Output, io::Error>,
}

trait CmdExt {
    fn into_cmd(self, name: &'static str) -> Cmd;
}

impl CmdExt for Result<Output, io::Error> {
    fn into_cmd(self, name: &'static str) -> Cmd {
        Cmd { name, result: self }
    }
}

impl From<Cmd> for Exit<()> {
    fn from(cmd: Cmd) -> Self {
        match cmd.result {
            Ok(output) => {
                if output.status.success() {
                    println!("{}: OK", cmd.name);
                    Self::Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Self::Error(stderr.to_string())
                }
            }
            Err(e) => {
                let msg = format!("{} failed: {}", cmd.name, e);
                Self::IO(msg)
            }
        }
    }
}

#[derive(Debug)]
pub struct Spawned {
    pub name: &'static str,
    pub child: Result<Child, io::Error>,
}

impl Spawned {
    pub fn wait(self) -> Cmd {
        match self.child {
            Ok(child) => child.wait_with_output().into_cmd(self.name),
            Err(e) => Cmd {
                name: self.name,
                result: Err(e),
            },
        }
    }
}

trait SpawnedExt {
    fn into_spawned(self, name: &'static str) -> Spawned;
}

impl SpawnedExt for Result<Child, io::Error> {
    fn into_spawned(self, name: &'static str) -> Spawned {
        Spawned { name, child: self }
    }
}

impl From<Vec<Spawned>> for Exit<()> {
    fn from(spawns: Vec<Spawned>) -> Self {
        spawns
            .into_iter()
            .map(|spawn| spawn.wait())
            .map(Exit::from)
            .collect()
    }
}

impl From<Spawned> for Exit<()> {
    fn from(spawn: Spawned) -> Self {
        spawn.wait().into()
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn exit_from_404() {
        let splat: Cmd = Command::new("splat").output().into_cmd("splat");
        assert_eq!(splat.name, "splat");
        assert!(
            matches!(splat.result, Result::Err(ref e) if matches!(e.kind(), io::ErrorKind::NotFound))
        );
        let exit: Exit<()> = Exit::from(splat);
        let Exit::IO(ref msg) = exit else {
            panic!("not an IO2")
        };
        eprintln!("{}", msg);
        assert!(msg.starts_with("splat failed: "));
    }

    #[test]
    fn collect_exit() {
        let exits = [
            Exit::Ok(()),
            Exit::IO("one".to_string()),
            Exit::Error("two".to_string()),
            Exit::Error("three".to_string()),
        ];
        let exit: Exit<()> = exits.into_iter().collect();
        let expected = "one\ntwo\nthree\n";
        dbg!(&exit);
        assert!(matches!(exit, Exit::Error(s) if s == expected));
    }
}
