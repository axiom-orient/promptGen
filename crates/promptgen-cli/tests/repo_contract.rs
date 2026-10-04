//! Repository consistency checks that the compiler cannot make.
//!
//! `cargo` proves the code builds and behaves; it cannot prove the prose still points at
//! files that exist, or that the version stated in the docs, `REVISION`, and the packaging
//! script have not drifted apart. These checks used to live in an external Python script, so
//! they only ran when someone remembered to invoke it. Here they run with `cargo test`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root resolves")
}

fn workspace_version(root: &Path) -> String {
    let manifest =
        fs::read_to_string(root.join("Cargo.toml")).expect("readable workspace manifest");
    manifest
        .lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .expect("workspace manifest declares a version")
        .to_owned()
}

/// Every document the project authors by hand, as opposed to generated output.
fn authored_documents(root: &Path) -> Vec<PathBuf> {
    let mut documents = vec![
        root.join("README.md"),
        root.join("crates/promptgen-cli/README.md"),
        root.join("NOTICE.md"),
        root.join("SECURITY.md"),
    ];
    let mut docs = fs::read_dir(root.join("docs"))
        .expect("readable docs directory")
        .filter_map(|entry| {
            let path = entry.expect("readable directory entry").path();
            (path.extension().is_some_and(|value| value == "md")).then_some(path)
        })
        .collect::<Vec<_>>();
    docs.sort();
    documents.extend(docs);
    documents
}

/// Targets of inline markdown links, excluding anchors and external schemes.
fn local_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut index = 0usize;
    while let Some(offset) = text[index..].find("](") {
        let start = index + offset + 2;
        let Some(close) = text[start..].find(')') else {
            break;
        };
        let target = text[start..start + close].trim();
        index = start + close;
        let target = target.split_whitespace().next().unwrap_or(target);
        if target.is_empty()
            || target.starts_with('#')
            || target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
            || target.starts_with("data:")
        {
            continue;
        }
        targets.push(target.split('#').next().unwrap_or(target).to_owned());
    }
    targets
}

#[test]
fn revision_and_cli_document_state_the_workspace_version() {
    let root = repo_root();
    let version = workspace_version(&root);

    let revision = fs::read_to_string(root.join("REVISION")).expect("readable REVISION");
    let revision = revision.trim();
    assert!(
        revision.starts_with(&format!("promptgen-{version}-")),
        "REVISION {revision:?} does not match workspace version {version}"
    );

    let cli_document = fs::read_to_string(root.join("crates/promptgen-cli/README.md"))
        .expect("readable CLI contract");
    assert!(
        cli_document
            .lines()
            .any(|line| line == format!("# promptgen-cli {version}")),
        "crates/promptgen-cli/README.md does not state version {version} in its title"
    );
}

#[test]
fn authored_documents_exist_and_carry_no_superseded_version() {
    let root = repo_root();
    let version = workspace_version(&root);
    let superseded = [
        "promptGen 1.0.0",
        "promptGen 1.1.0",
        "promptgen-1.0.0-",
        "promptgen-1.1.0-",
    ];

    for document in authored_documents(&root) {
        assert!(
            document.is_file(),
            "required authored document is missing: {}",
            document.display()
        );
        let text = fs::read_to_string(&document).expect("readable authored document");
        for stale in superseded {
            assert!(
                !text.contains(stale),
                "superseded version {stale:?} in {} (workspace is {version})",
                document.display()
            );
        }
    }
}

#[test]
fn authored_documents_do_not_reference_removed_audit_scripts() {
    let root = repo_root();
    let removed = "scripts/audit-prompt-contracts.py";
    let offenders = authored_documents(&root)
        .into_iter()
        .filter_map(|document| {
            let text = fs::read_to_string(&document).expect("readable authored document");
            text.contains(removed)
                .then(|| document.display().to_string())
        })
        .collect::<Vec<_>>();
    assert!(
        offenders.is_empty(),
        "removed audit script must not be documented: {}",
        offenders.join(", ")
    );
}

#[test]
fn every_local_document_link_resolves_inside_the_repository() {
    let root = repo_root();
    let mut broken = Vec::new();

    for document in authored_documents(&root) {
        let text = fs::read_to_string(&document).expect("readable authored document");
        let parent = document.parent().expect("document has a parent");
        for target in local_link_targets(&text) {
            let relative = document.strip_prefix(&root).unwrap_or(&document);
            let resolved = parent.join(&target);
            let Ok(resolved) = resolved.canonicalize() else {
                broken.push(format!("{} -> {target} (missing)", relative.display()));
                continue;
            };
            if !resolved.starts_with(&root) {
                broken.push(format!(
                    "{} -> {target} (escapes repository)",
                    relative.display()
                ));
            }
        }
    }

    assert!(
        broken.is_empty(),
        "document links must resolve inside the repository:\n  {}",
        broken.join("\n  ")
    );
}

#[test]
fn checked_in_schemas_declare_draft_2020_12() {
    let root = repo_root();
    let declaration = "\"$schema\": \"https://json-schema.org/draft/2020-12/schema\"";
    let mut names = BTreeSet::new();

    for entry in fs::read_dir(root.join("schemas")).expect("readable schemas directory") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().is_none_or(|value| value != "json") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable schema");
        let compact = text.replace(['\n', ' '], "");
        assert!(
            compact.contains(&declaration.replace(' ', "")),
            "schema does not declare Draft 2020-12: {}",
            path.display()
        );
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("schema file name is UTF-8");
        names.insert(
            file_name
                .strip_suffix(".schema.json")
                .unwrap_or(file_name)
                .to_owned(),
        );
    }

    for required in ["image", "interview", "compilation"] {
        assert!(
            names.contains(required),
            "checked-in schema is missing: {required}.schema.json"
        );
    }
}

/// Recognizes a bare `major.minor.patch` literal without pulling in a regex crate.
fn contains_semver_literal(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut groups = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            let followed_by_dot = bytes.get(index) == Some(&b'.');
            groups.push((start, followed_by_dot));
            if followed_by_dot {
                index += 1;
            }
        } else {
            groups.clear();
            index += 1;
        }
        if groups.len() >= 3 && groups[groups.len() - 3].1 && groups[groups.len() - 2].1 {
            return true;
        }
    }
    false
}

/// A version literal copied into a script is stale the moment the workspace version
/// moves, and the copy is only discovered when a release gate fails. The contract is
/// therefore that no script carries one: each derives the version from `Cargo.toml`.
#[test]
fn scripts_derive_the_workspace_version_instead_of_hardcoding_it() {
    let root = repo_root();
    for name in ["scripts/smoke.sh", "scripts/smoke-web.py"] {
        let script = fs::read_to_string(root.join(name))
            .unwrap_or_else(|_| panic!("readable script: {name}"));
        for (number, line) in script.lines().enumerate() {
            let lowercase = line.to_ascii_lowercase();
            // Dotted numbers appear in addresses and shell patterns too; only a line
            // that also names the product or a version can be a version literal.
            let version_context = lowercase.contains("version") || lowercase.contains("promptgen");
            assert!(
                !version_context || !contains_semver_literal(line) || line.contains("Cargo.toml"),
                "{name}:{} hardcodes a version literal: {line}",
                number + 1
            );
        }
        assert!(
            script.contains("Cargo.toml"),
            "{name} must read the workspace version from Cargo.toml"
        );
    }
}
