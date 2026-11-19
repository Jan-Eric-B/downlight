//! Prints the licenses of the syntax grammars the core embeds for highlighting (bat's, through
//! two-face; ADR 0004) as JSON, an array of `{ "path", "text" }`, for `scripts/notices.js` to put
//! in the third-party notices (ticket 36). The core's `grammar_licenses` test checks that each is
//! compatible with GPL-3.0.

use std::error::Error;
use std::io::{self, Write};

use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    // Only the grammars' licenses: the core uses none of two-face's themes.
    let licenses: Vec<_> = two_face::acknowledgement::listing()
        .for_syntaxes()
        .iter()
        .map(|license| json!({ "path": license.rel_path, "text": license.text }))
        .collect();
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &licenses)?;
    stdout.flush()?;
    Ok(())
}
