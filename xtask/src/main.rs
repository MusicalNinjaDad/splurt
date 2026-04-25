use std::path::Path;

use clap::{Parser, Subcommand};
use try_v2_xtasks::{
    Exit,
    commands::{build, clippy, clippy_tests, fmt, git_add, test},
};

#[derive(Parser)]
#[command(version)]
struct XTask {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// git add if all is good
    Add,
    /// build (optionally with zigbuild for a given glibc version)
    Build {
        /// build for a specific glibc version (WSL-Ubuntu is 2.35)
        #[arg(short, long)]
        glibc: Option<String>,
        /// build a release build (default is cargo's default profile, usually debug)
        #[arg(short, long)]
        release: bool,
    },
}

fn main() -> Exit<()> {
    let xtask = XTask::try_parse()?;
    let root = Path::new(".");

    match &xtask.command {
        Command::Add => {
            let fmt = fmt(root);
            Exit::from(fmt)?;
            let clippy = clippy(root);
            let clippy_tests = clippy_tests(root);
            let tests = test(root);
            let checks = vec![clippy, clippy_tests, tests];
            Exit::from(checks)?;
            let git = git_add(root);
            Exit::from(git)
        }
        Command::Build { glibc, release } => {
            let build = build(root, glibc, release);
            Exit::from(build)
        }
    }
}
