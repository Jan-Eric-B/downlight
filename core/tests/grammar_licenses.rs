//! The licenses of the syntax grammars the core embeds for highlighting (bat's, through two-face;
//! ADR 0004). They ship in the page's build next to the other notices (`licenses.md`), from the
//! committed `page/public/grammar-licenses.md`, and each must be compatible with GPL-3.0. After
//! two-face is updated, `npm run fixtures` writes the file again; review its diff.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use two_face::acknowledgement::{License, LicenseType};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn grammar_licenses_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("page")
        .join("public")
        .join("grammar-licenses.md")
}

/// Only the grammars' licenses: the core uses none of two-face's themes.
fn grammar_licenses() -> Vec<License> {
    two_face::acknowledgement::listing().for_syntaxes().to_vec()
}

fn grammar_licenses_text() -> String {
    let mut text = String::from(
        "# Licenses of the syntax grammars\n\n\
         Downlight highlights code with the syntax grammars collected by the \
         [bat project](https://github.com/sharkdp/bat), packaged by \
         [two-face](https://codeberg.org/CosmicHarper/two-face). These are the licenses of the \
         grammars that ask to be acknowledged.\n\n",
    );
    for license in grammar_licenses() {
        license.write_md(&mut text);
    }
    text
}

/// Whether software under `license` may be combined with GPL-3.0 code and conveyed under GPL-3.0.
/// A license two-face adds later fails until it is checked and named here.
fn is_compatible_with_gpl_3(license: LicenseType) -> bool {
    matches!(
        license,
        LicenseType::Mit
            | LicenseType::Bsd2Clause
            | LicenseType::Bsd2ClauseFreeBsd
            | LicenseType::Bsd3Clause
            | LicenseType::Apache2
            | LicenseType::Unlicense
            | LicenseType::Wtfpl
            | LicenseType::Sublime
    )
}

#[test]
fn every_grammar_license_is_compatible_with_gpl_3() {
    let licenses = grammar_licenses();
    assert!(!licenses.is_empty(), "two-face lists no grammar licenses");
    for license in licenses {
        assert!(
            is_compatible_with_gpl_3(license.ty),
            "{} is under {:?}, which isn't checked against GPL-3.0",
            license.rel_path.display(),
            license.ty
        );
    }
}

#[test]
fn grammar_licenses_ship_as_committed() -> TestResult {
    let file = grammar_licenses_file();
    let committed = fs::read_to_string(&file).map_err(|error| {
        format!(
            "Can't read {}: {error}. `npm run fixtures` writes it.",
            file.display()
        )
    })?;
    assert!(
        committed == grammar_licenses_text(),
        "{} doesn't list the grammars' licenses two-face gives. If two-face was updated, run \
         `npm run fixtures` and review the diff.",
        file.display()
    );
    Ok(())
}

#[test]
#[ignore = "writes the grammar licenses; run it with `npm run fixtures`"]
fn write_grammar_licenses() -> TestResult {
    let file = grammar_licenses_file();
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(file, grammar_licenses_text())?;
    Ok(())
}
