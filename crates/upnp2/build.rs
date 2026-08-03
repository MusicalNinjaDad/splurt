use build_safely::nightly::UnstableFeature;
use build_safely::prelude::*;

fn main() -> Result<()> {
    let ac = AutoCfg::new()?;
    let allowed_features = cargo_allowed_features()?;

    ac.emit_unstable_feature(UnstableFeature::adt_const_params, &allowed_features);
    ac.emit_unstable_feature(UnstableFeature::assert_matches, &allowed_features);
    ac.emit_unstable_feature(UnstableFeature::doc_notable_trait, &allowed_features);
    ac.emit_unstable_feature(UnstableFeature::never_type, &allowed_features);
    ac.emit_unstable_feature(UnstableFeature::strip_circumfix, &allowed_features);
    ac.emit_unstable_feature(UnstableFeature::unsized_const_params, &allowed_features);
    Ok(())
}
