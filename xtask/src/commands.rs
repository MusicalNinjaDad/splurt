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

/// Spawn `cargo build` (if no `glibc` specified) / `cargo zigbuild` (if `glibc` specified)
/// optionally performing a release build (default is cargo's default profile)
///
/// #### Build target (TODO: take a specific target & other cargo args)
/// - For a given `glibc`: `x86_64-unknown-linux-gnu`
/// - Otherwise: cargo default target
pub fn build(root: &Path, glibc: &Option<String>, release: &bool) -> Spawned {
    let (builder, target) = match glibc {
        Some(version) => {
            let target = format!("x86_64-unknown-linux-gnu.{}", version);
            ("zigbuild", vec!["--target".to_string(), target])
        }
        None => ("build", vec![]),
    };
    let release = if *release { Some("--release") } else { None };
    Command::new("cargo")
        .current_dir(root)
        .arg(builder)
        .args(release)
        .args(target)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .into_spawned("build")
}
