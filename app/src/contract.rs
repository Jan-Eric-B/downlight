//! The contract samples (ticket 27): a sample of each answer and message the app sends a page,
//! and of each argument the page sends the app that is more than a number, string or flag (the
//! menu entries and the timing reports), committed as JSON in `tests/fixtures`. The page's tests
//! read the same files through the types in `page/src/app.ts`: they drive each answer and message
//! through the page, and check the arguments the page sends against their samples, so a change to
//! a shape on one side fails a test on the other. After an intended change to a shape,
//! `npm run fixtures` writes the samples again; review the diff.
//!
//! Each `contract_samples` starts with an exhaustive match over the variants of what it samples,
//! so a new variant stops the build there until it has a sample; Rust can't list an enum's
//! variants itself without a dependency.
//!
//! The names of a command's parameters aren't sampled: Tauri takes them from the Rust function,
//! and no Rust type holds them to serialize.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{documents, links, menu, message, timing};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

type Samples = fn() -> serde_json::Result<serde_json::Value>;

const HOW_TO_UPDATE: &str =
    "If the change is intended, run `npm run fixtures` and review the diff.";

/// Each sample file, with what writes it: the answers of the commands that answer with more than
/// nothing, the messages, and the arguments of `show_document_menu` and the timing reports.
const SAMPLE_FILES: [(&str, Samples); 5] = [
    ("render_document.json", documents::contract_samples),
    ("follow_link.json", links::contract_samples),
    ("messages.json", message::contract_samples),
    ("menu_entries.json", menu::contract_samples),
    ("timing_reports.json", timing::contract_samples),
];

fn sample_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// The samples as the file holds them.
fn written(samples: Samples) -> TestResult<String> {
    Ok(serde_json::to_string_pretty(&samples()?)? + "\n")
}

#[test]
fn contract_samples_match_their_committed_json() -> TestResult {
    for (name, samples) in SAMPLE_FILES {
        let file = sample_file(name);
        let committed = fs::read_to_string(&file).map_err(|error| {
            format!(
                "Can't read {}: {error}. `npm run fixtures` writes it.",
                file.display()
            )
        })?;
        assert_eq!(
            written(samples)?,
            committed,
            "The app and its page no longer exchange what {} holds. {HOW_TO_UPDATE}",
            file.display()
        );
    }
    Ok(())
}

#[test]
#[ignore = "writes the contract samples; run it with `npm run fixtures`"]
fn write_contract_samples() -> TestResult {
    for (name, samples) in SAMPLE_FILES {
        let file = sample_file(name);
        if let Some(folder) = file.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::write(file, written(samples)?)?;
    }
    Ok(())
}
