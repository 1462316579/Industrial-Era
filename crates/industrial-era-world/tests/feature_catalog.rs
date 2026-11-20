use industrial_era_world::worldgen::{feature_catalog::FeatureCatalog, tags::TagDefinitions};
use std::collections::BTreeMap;
fn map(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
#[test]
fn canonical_duplicates_cycles_and_unknown_unselected_children_fail() {
    let tags = TagDefinitions::default();
    assert!(
        FeatureCatalog::from_resources(
            &map(&[
                ("a", r#"{"type":"no_op"}"#),
                ("minecraft:a", r#"{"type":"no_op"}"#)
            ]),
            &BTreeMap::new(),
            &tags
        )
        .is_err()
    );
    let features = map(&[
        (
            "industrial-era:loop",
            r#"{"type":"sequence","features":["industrial-era:loop"]}"#,
        ),
        ("industrial-era:unused", r#"{"type":"not_implemented"}"#),
    ]);
    let placed = map(&[(
        "industrial-era:loop",
        r#"{"feature":"industrial-era:loop","placement":[]}"#,
    )]);
    let catalog = FeatureCatalog::from_resources(&features, &placed, &tags).unwrap();
    assert!(catalog.feature(r#""industrial-era:loop""#).is_err());
    assert!(catalog.placed_feature(r#""industrial-era:loop""#).is_err());
    assert!(catalog.feature(r#"{"type":"no_op"}"#).is_ok());
    assert!(catalog.feature(r#"{"type":"weighted_random_selector","features":[{"data":"industrial-era:missing","weight":0}]}"#).is_err());
}
#[test]
fn shared_reference_expansion_is_bounded_before_building_a_large_tree() {
    let leaf = r#"{"feature":{"type":"no_op"},"placement":[]}"#;
    let branch = format!(
        r#"{{"type":"sequence","features":[{}]}}"#,
        vec![r#""industrial-era:leaf""#; 8192].join(",")
    );
    let features = BTreeMap::from([("industrial-era:branch".into(), branch)]);
    let placed = map(&[
        ("industrial-era:leaf", leaf),
        (
            "industrial-era:branch",
            r#"{"feature":"industrial-era:branch","placement":[]}"#,
        ),
    ]);
    let catalog =
        FeatureCatalog::from_resources(&features, &placed, &TagDefinitions::default()).unwrap();
    let root = format!(
        r#"{{"type":"sequence","features":[{}]}}"#,
        [r#""industrial-era:branch""#; 9].join(",")
    );
    assert!(catalog.feature(&root).is_err());
}
