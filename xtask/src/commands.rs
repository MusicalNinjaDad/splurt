use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::{Cmd, CmdExt as _, Spawned, SpawnedExt as _};

pub fn fmt(root: &Path) -> Cmd {
    Command::new("cargo")
        .current_dir(root)
        .arg("fmt")
        .output()
        .into_cmd("fmt")
}

pub fn git_add(root: &Path) -> Cmd {
    Command::new("git")
        .current_dir(root)
        .arg("add")
        .arg(".")
        .output()
        .into_cmd("git add")
}

pub fn clippy(root: &Path) -> Spawned {
    Command::new("cargo")
        .current_dir(root)
        .arg("clippy")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .into_spawned("clippy")
}

pub fn clippy_tests(root: &Path) -> Spawned {
    Command::new("cargo")
        .current_dir(root)
        .arg("clippy")
        .arg("--tests")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .into_spawned("clippy the tests")
}

pub fn test(root: &Path) -> Spawned {
    Command::new("cargo")
        .current_dir(root)
        .arg("test")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .into_spawned("tests")
}
