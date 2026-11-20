//! The contract samples (ticket 27): a sample of each answer and message the app sends a page, of
//! the failures that name a missing file (window design ticket 11), and of each argument the page
//! sends the app that is more than a number, string or flag (the menu entries, the timing reports
//! and the changes of the settings), and of the flags a window tells its page in the page's URL,
//! committed as JSON in `tests/fixtures`. The page's tests read the same files through the
//! types in `page/src/app.ts`: they drive each answer and message through the page, check the
//! arguments the page sends against their samples, and read the flags from the sampled URLs, so a
//! change to a shape on one side fails a test on the other. After an intended change to a shape,
//! `npm run fixtures` writes the samples again; review the diff.
//!
//! Each `contract_samples` starts with an exhaustive match over the variants of what it samples
//! (or, for the page's flags, a struct literal that names every field), so a new variant stops the
//! build there until it has a sample; Rust can't list an enum's variants itself without a
//! dependency.
//!
//! The names of a command's parameters aren't sampled: Tauri takes them from the Rust function,
//! and no Rust type holds them to serialize.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    documents, edit, full_screen, keep_on_top, links, menu, message, notice, settings, timing,
    title_bar, window,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

type Samples = fn() -> serde_json::Result<serde_json::Value>;

const HOW_TO_UPDATE: &str =
    "If the change is intended, run `npm run fixtures` and review the diff.";

/// Each sample file, with what writes it: the answers of the commands that answer with more than
/// nothing (among them `render_document`, `editor_choices` for the settings flyout's Edit with,
/// `wait_note` for a document slow to arrive and `stop_document` for Stop), the
/// failure of `open_named_folder` for a folder gone since, the messages, the arguments of
/// `show_document_menu`, the timing reports, `place_maximize_button` and `change_setting`, and
/// the flags a window tells its page in the page's URL.
const SAMPLE_FILES: [(&str, Samples); 14] = [
    ("render_document.json", documents::contract_samples),
    ("editor_choices.json", edit::contract_samples),
    ("wait_note.json", documents::wait_note_samples),
    ("stop_document.json", documents::arrival::contract_samples),
    ("follow_link.json", links::contract_samples),
    ("keep_on_top.json", keep_on_top::contract_samples),
    ("full_screen.json", full_screen::contract_samples),
    ("open_named_folder.json", notice::contract_samples),
    ("messages.json", message::contract_samples),
    ("menu_entries.json", menu::contract_samples),
    ("timing_reports.json", timing::contract_samples),
    ("title_bar.json", title_bar::contract_samples),
    ("page_url.json", window::contract_samples),
    ("setting_changes.json", settings::change::contract_samples),
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
