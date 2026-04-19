use std::{env::VarError, ffi::OsString, fmt::Display};

use autocfg::AutoCfg;

include!("./src/cli.rs");

fn main() -> Result<(), BuildError> {
    let ac = autocfg::new();
    ac.emit_unstable_feature("let_chains");

    if get_var("PROFILE")? == "release" {
        use clap_builder::{CommandFactory, ValueEnum};
        use clap_complete::Shell;

        let mut cmd = Splurt::command();

        let out_dir = std::path::PathBuf::from(get_var("OUT_DIR")?);
        let bin_name = get_var("CARGO_PKG_NAME")?;

        clap_mangen::generate_to(cmd.clone(), &out_dir)?;

        for &shell in Shell::value_variants() {
            clap_complete::generate_to(shell, &mut cmd, &bin_name, &out_dir)?;
        }
    };

    Ok(())
}

fn get_var(key: &str) -> Result<String, BuildError> {
    std::env::var(key).map_err(|e| BuildError::from_var_error(key, e))
}

#[derive(Debug)]
#[allow(
    unused,
    reason = "error contents output to stderr in Termination for Result"
)]
enum BuildError {
    VarNotSet(OsString),
    VarInvalid(OsString, OsString),
    IOError(Box<std::io::Error>),
}

impl BuildError {
    fn from_var_error(var: &str, e: VarError) -> BuildError {
        match e {
            VarError::NotPresent => BuildError::VarNotSet(var.into()),
            VarError::NotUnicode(contents) => BuildError::VarInvalid(var.into(), contents),
        }
    }
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        BuildError::IOError(Box::new(e))
    }
}

/// Location of assert_matches!() macro. Stabilisation was reverted at last minute
/// on 2026-04-10, leaving the macro in the new planned location.
enum AssertMatchesLocation {
    /// Macro is at `std::assert_matches`
    Root,
    /// Macro is at `std::assert_matches::assert_matches`
    Module,
}

impl Display for AssertMatchesLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssertMatchesLocation::Root => write!(f, "assert_matches_in_root"),
            AssertMatchesLocation::Module => write!(f, "assert_matches_in_module"),
        }
    }
}

#[expect(dead_code, reason = "not using assert_matches right now")]
impl AssertMatchesLocation {
    fn emit_possibilities() {
        autocfg::emit_possibility(&AssertMatchesLocation::Root.to_string());
        autocfg::emit_possibility(&AssertMatchesLocation::Module.to_string());
    }
}

trait Nightly {
    /// Identify whether a an experimental feature flag is available _and_ required on nightly.
    /// Always fails if feature flags are unavailable.
    ///
    /// ## Usage:
    /// To be used at top-level crate via `#![cfg_attr(unstable_foo, feature(foo))]`
    fn emit_unstable_feature(&self, feature: &'static str);

    #[expect(dead_code, reason = "not using assert_matches right now")]
    /// Location of assert_matches!() macro. Stabilisation was reverted at last minute
    /// on 2026-04-10, leaving the macro in the new planned location.
    ///
    /// #Recommended usage
    /// ```
    /// AssertMatchesLocation::emit_possibilities();
    /// if let Some(location) = ac.assert_matches_location() {
    ///     autocfg::emit(&location.to_string())
    /// }
    /// ```
    fn assert_matches_location(&self) -> Option<AssertMatchesLocation>;
}

impl Nightly for AutoCfg {
    fn emit_unstable_feature(&self, feature: &'static str) {
        let cfg = format!("unstable_{feature}");
        let code = format!(
            r#"
        #![deny(stable_features)]
        #![feature({feature})]
        "#
        );
        autocfg::emit_possibility(&cfg);
        if self.probe_raw(&code).is_ok() {
            autocfg::emit(&cfg);
        }
    }

    fn assert_matches_location(&self) -> Option<AssertMatchesLocation> {
        let in_root = r#"
        #![allow(stable_features)]
        #![feature(assert_matches)]
        use std::assert_matches;

        fn main() {
            assert_matches!(Some(4), Some(_));
        }
            "#;

        let in_module = r#"
        #![allow(stable_features)]
        #![feature(assert_matches)]
        use std::assert_matches::assert_matches;

        fn main() {
            assert_matches!(Some(4), Some(_));
        }
            "#;

        if self.probe_raw(in_root).is_ok() {
            Some(AssertMatchesLocation::Root)
        } else if self.probe_raw(in_module).is_ok() {
            Some(AssertMatchesLocation::Module)
        } else {
            None
        }
    }
}
