use std::path::Path;

use clap::{Parser, Subcommand};
use try_v2_xtasks::{
    Exit,
    commands::{clippy, clippy_tests, fmt, git_add, test},
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
}

fn main() -> Exit<()> {
    let xtask = XTask::try_parse()?;

    match &xtask.command {
        Command::Add => {
            let root = Path::new(".");
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
    }
}
