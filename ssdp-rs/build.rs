use ninja_build_rs::{
    Result,
    nightly::{AssertMatchesLocation, Nightly},
};

fn main() -> Result<()> {
    let ac = autocfg::new();
    ac.emit_unstable_feature("adt_const_params");
    ac.emit_unstable_feature("if_let_guard");
    ac.emit_unstable_feature("let_chains");
    ac.emit_unstable_feature("never_type");
    ac.emit_unstable_feature("assert_matches");
    AssertMatchesLocation::emit_possibilities();
    if let Some(location) = ac.assert_matches_location() {
        autocfg::emit(&location.to_string())
    }

    Ok(())
}
