use build_safely::prelude::*;

fn main() -> Result<()> {
    let ac = AutoCfg::new()?;
    let allowed_features = cargo_allowed_features()?;

    ac.emit_unstable_feature(OtherFeature("adt_const_params".into()), &allowed_features);
    ac.emit_unstable_feature(never_type, &allowed_features);
    ac.emit_unstable_feature(assert_matches, &allowed_features);

    Ok(())
}
