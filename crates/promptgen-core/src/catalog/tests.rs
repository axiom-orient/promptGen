use super::*;

const REPRESENTATIVE_IDS: [&str; 6] = ["C1", "C4", "C5", "C6", "C10", "C11"];

#[test]
fn catalog_is_the_six_outcome_taxonomy() {
    let catalog = catalog().expect("catalog must load");
    assert_eq!(catalog.entries.len(), 6);
    assert_eq!(catalog.metadata.schema_version, CATALOG_SCHEMA_VERSION);
    assert_eq!(catalog.metadata.source_repository, "promptGen");
    assert_eq!(catalog.metadata.source_license, "BSD-2-Clause");
    assert!(catalog.entries.iter().all(CatalogEntry::is_primary_output));
    assert_eq!(
        catalog
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        REPRESENTATIVE_IDS
    );
}

#[test]
fn catalog_navigation_exposes_one_step_output_cards() {
    let serialized = catalog_json().unwrap().to_compact_string();
    assert!(serialized.contains("one_step_representative_output_cards"));
    assert!(serialized.contains("\"asset_path\":\"assets/category/people-lifestyle.png\""));
    assert!(!serialized.contains("보조 스타일"));
    assert!(serialized.contains("\"tier_2\":"));
    assert!(!serialized.contains("\"source_commit\":"));
}

#[test]
fn finds_representative_ids_case_insensitively() {
    assert_eq!(find_entry("c10").unwrap().unwrap().id, "C10");
    assert_eq!(find_entry(" C4 ").unwrap().unwrap().id, "C4");
    assert!(find_entry("TP14").unwrap().is_none());
}

#[test]
fn catalog_index_covers_every_entry_once() {
    let store = catalog_store().unwrap();
    assert_eq!(store.entry_index.len(), store.catalog.entries.len());
    for (position, entry) in store.catalog.entries.iter().enumerate() {
        assert_eq!(store.entry_index.get(&entry.id), Some(&position));
    }
}

#[test]
fn public_catalog_separates_source_recipe_from_runtime_directives() {
    let serialized = find_entry("C5")
        .unwrap()
        .unwrap()
        .to_json()
        .to_compact_string();
    assert!(serialized.contains("\"source_recipe\":"));
    assert!(serialized.contains("\"prompt_directives\":"));
    assert!(!serialized.contains("\"recipe\":"));
}

#[test]
fn every_entry_has_three_executable_placeholder_free_directives() {
    let catalog = catalog().expect("catalog must load");
    assert_eq!(
        catalog
            .entries
            .iter()
            .map(|entry| entry.prompt_directives.len())
            .sum::<usize>(),
        18
    );
    for entry in &catalog.entries {
        assert_eq!(entry.prompt_directives.len(), 3, "{}", entry.id);
        for directive in &entry.prompt_directives {
            assert!(!directive.text.contains('{'), "{}", entry.id);
            assert!(!directive.text.contains('}'), "{}", entry.id);
            assert!(!directive.text.contains("references/"), "{}", entry.id);
            assert!(!directive.text.trim().is_empty(), "{}", entry.id);
        }
    }
}

#[test]
fn outcome_contracts_leave_style_to_typed_fields() {
    let catalog = catalog().expect("catalog must load");
    for entry in &catalog.entries {
        assert!(!entry.suggested_aspect_ratios.is_empty(), "{}", entry.id);
        assert_eq!(entry.reference_text_mode, ReferenceTextMode::None);
        assert!(entry.reference_exact_text.is_empty());
    }
}

#[test]
fn every_declared_asset_exists_and_is_claimed_once() {
    let catalog = catalog().expect("catalog must load");
    let catalog_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog");
    let mut claimed = BTreeSet::new();
    for entry in &catalog.entries {
        assert!(
            claimed.insert(entry.asset_path.clone()),
            "{} reuses asset {}",
            entry.id,
            entry.asset_path
        );
        assert!(
            catalog_root.join(&entry.asset_path).is_file(),
            "{} declares a missing asset: {}",
            entry.id,
            entry.asset_path
        );
    }
}

#[test]
fn directives_never_leak_authoring_scaffolding_or_override_typed_output() {
    let catalog = catalog().expect("catalog must load");
    for entry in &catalog.entries {
        for directive in &entry.prompt_directives {
            for marker in ["skills/", "```"] {
                assert!(
                    !directive.text.contains(marker),
                    "{} leaks authoring scaffolding {marker:?}",
                    entry.id
                );
            }
            let lowered = directive.text.to_lowercase();
            assert!(
                !lowered.starts_with("ar ") && !lowered.contains(" default ar"),
                "{} overrides typed output authority from a directive",
                entry.id
            );
        }
    }
}

#[test]
fn official_method_source_is_attributed() {
    let catalog = catalog().expect("catalog must load");
    let sources = &catalog.metadata.method_sources;
    assert_eq!(sources.len(), 1);
    let source = &sources[0];
    assert_eq!(source.repository, "openai/codex");
    assert_eq!(source.license, "Apache-2.0");
    assert_eq!(source.artifact_sha256.len(), 64);
    assert!(
        source
            .artifact_sha256
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
}

#[test]
fn taxonomy_selection_accepts_one_output() {
    let valid = inspect_catalog_selection(Some("C10")).unwrap();
    assert!(valid.issues.is_empty());
    assert_eq!(
        valid
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["C10"]
    );

    let invalid = inspect_catalog_selection(Some("unknown-category")).unwrap();
    assert!(matches!(
        invalid.issues.as_slice(),
        [CatalogSelectionIssue::UnknownCategory { id }] if id == "unknown-category"
    ));
}
