use build_safely::prelude::*;

include!("./src/cli.rs");

fn main() -> Result<()> {
    if get_var("PROFILE")? == "release" {
        use clap_builder::{CommandFactory, ValueEnum};
        use clap_complete::Shell;

        let mut cmd = Rip::command();

        let out_dir = std::path::PathBuf::from(get_var("OUT_DIR")?);
        let bin_name = get_var("CARGO_PKG_NAME")?;

        clap_mangen::generate_to(cmd.clone(), &out_dir)?;

        for &shell in Shell::value_variants() {
            clap_complete::generate_to(shell, &mut cmd, &bin_name, &out_dir)?;
        }
    }

    Ok(())
}
