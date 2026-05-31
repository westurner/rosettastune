use insta::assert_json_snapshot;
use rosettastune::LexiconRegistry;

#[test]
fn lexicon_snapshot_matches_expected_units() {
    let registry = LexiconRegistry::from_jsonld_str(include_str!(
        "../python/rosettastune/data/lexicon.jsonld"
    ))
    .expect("lexicon should parse");

    assert_json_snapshot!(registry
        .snapshot_entries()
        .expect("snapshot should serialize"));
}
