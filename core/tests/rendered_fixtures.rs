//! The rendered fixtures: each fixture document in `tests/fixtures` is committed next to the HTML
//! the core renders for it. The page's tests load that same HTML (ticket 03), so the core and the
//! page agree on its shape. After an intended change to rendering, `npm run fixtures` writes the
//! HTML again; its diff shows what the change does to real documents.

mod common;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use common::Nothing;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const HOW_TO_UPDATE: &str =
    "If the change is intended, run `npm run fixtures` and review the diff.";

/// The fixture documents, sorted so that a failure names the same document on every run.
fn fixture_documents() -> TestResult<Vec<PathBuf>> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut documents = Vec::new();
    for entry in fs::read_dir(folder)? {
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
fn render(document: &Path) -> TestResult<String> {
    let bytes = fs::read(document)?;
    let name = document
        .file_name()
        .ok_or("a fixture document has no file name")?;
    Ok(
        downlight_core::render(&bytes, Path::new(name), "http://image.localhost", Nothing)
            .as_str()
            .to_owned(),
    )
}

fn rendered_fixture_of(document: &Path) -> PathBuf {
    document.with_extension("html")
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
            render(&document)?,
            committed,
            "{} renders differently from {}. {HOW_TO_UPDATE}",
            document.display(),
            fixture.display()
        );
    }
    Ok(())
}

#[test]
#[ignore = "writes the rendered fixtures; run it with `npm run fixtures`"]
fn write_rendered_fixtures() -> TestResult {
    for document in fixture_documents()? {
        fs::write(rendered_fixture_of(&document), render(&document)?)?;
    }
    Ok(())
}
