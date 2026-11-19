//! The licenses of the syntax grammars the core embeds for highlighting (bat's, through two-face;
//! ADR 0004): each must be compatible with GPL-3.0. They ship in the third-party notices beside
//! the exe, which the app's `scripts/notices.js` assembles (ticket 36).

use two_face::acknowledgement::{License, LicenseType};

/// Only the grammars' licenses: the core uses none of two-face's themes.
fn grammar_licenses() -> Vec<License> {
    two_face::acknowledgement::listing().for_syntaxes().to_vec()
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
