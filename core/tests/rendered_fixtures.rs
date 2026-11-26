//! The rendered fixtures: each fixture document in `tests/fixtures` is committed next to the HTML
//! the core renders for it, and its counts in `counts.json`. The page's tests load that same HTML
//! (ticket 03) and those counts (window design ticket 10), so the core and the page agree on their
//! shape. After an intended change to rendering, `npm run fixtures` writes them again; their diff
//! shows what the change does to real documents.

mod common;

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use common::Nothing;
use downlight_core::{Counts, RenderedDocument};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const HOW_TO_UPDATE: &str =
    "If the change is intended, run `npm run fixtures` and review the diff.";

fn fixtures_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// The fixture documents, sorted so that a failure names the same document on every run.
fn fixture_documents() -> TestResult<Vec<PathBuf>> {
    let mut documents = Vec::new();
    for entry in fs::read_dir(fixtures_folder())? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            documents.push(path);
        }
    }
    documents.sort();
    Ok(documents)
}

/// Renders a fixture document with only its file name as its path and [`Nothing`] on the disk, so
/// that the HTML doesn't depend on where the repository is checked out or what is next to it.
fn render(document: &Path) -> TestResult<RenderedDocument> {
    let bytes = fs::read(document)?;
    Ok(downlight_core::render(
        &bytes,
        Path::new(file_name(document)?),
        "http://image.localhost",
        Nothing,
        &downlight_core::Leave::new(),
    )?)
}

fn file_name(document: &Path) -> TestResult<&str> {
    Ok(document
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("a fixture document has no file name")?)
}

fn rendered_fixture_of(document: &Path) -> PathBuf {
    document.with_extension("html")
}

/// The file with every fixture document's counts, by its file name.
fn counts_fixture() -> PathBuf {
    fixtures_folder().join("counts.json")
}

/// The counts of the fixture documents, as `counts.json` holds them.
fn counts_of_fixtures() -> TestResult<String> {
    let mut counts = BTreeMap::<String, Counts>::new();
    for document in fixture_documents()? {
        counts.insert(
            file_name(&document)?.to_owned(),
            render(&document)?.counts(),
        );
    }
    Ok(serde_json::to_string_pretty(&counts)? + "\n")
}

#[test]
fn fixture_documents_render_as_their_committed_html() -> TestResult {
    let documents = fixture_documents()?;
    assert!(!documents.is_empty(), "No fixture documents found.");
    for document in documents {
        let fixture = rendered_fixture_of(&document);
        let committed = fs::read_to_string(&fixture).map_err(|error| {
            format!(
                "Can't read {}: {error}. `npm run fixtures` writes it.",
                fixture.display()
            )
        })?;
        assert_eq!(
            render(&document)?.as_str(),
            committed,
            "{} renders differently from {}. {HOW_TO_UPDATE}",
            document.display(),
            fixture.display()
        );
    }
    Ok(())
}

#[test]
fn fixture_documents_count_as_their_committed_counts() -> TestResult {
    let fixture = counts_fixture();
    let committed = fs::read_to_string(&fixture).map_err(|error| {
        format!(
            "Can't read {}: {error}. `npm run fixtures` writes it.",
            fixture.display()
        )
    })?;
    assert_eq!(
        counts_of_fixtures()?,
        committed,
        "The fixture documents count differently from {}. {HOW_TO_UPDATE}",
        fixture.display()
    );
    Ok(())
}

#[test]
#[ignore = "writes the rendered fixtures; run it with `npm run fixtures`"]
fn write_rendered_fixtures() -> TestResult {
    for document in fixture_documents()? {
        fs::write(rendered_fixture_of(&document), render(&document)?.as_str())?;
    }
    fs::write(counts_fixture(), counts_of_fixtures()?)?;
    Ok(())
}
