//! Edit at the reading position (ticket 35): the editors Downlight knows to take a line, and the
//! arguments that open the document at it. The arguments are Downlight's own, never the reader's
//! or a document's: the line is a number, and the document's path is always one whole argument.
//! An absolute path never starts with `-`, so no editor reads it as an option.

use std::ffi::OsString;
use std::fmt;
use std::num::NonZeroU32;
use std::path::Path;

use serde::Deserialize;

/// The line of the document's file that Edit opens it at: the reading position, which the page
/// sends. It comes from the page, so it is only ever a positive number that fits in 32 bits;
/// anything else fails the command before it runs, as Tauri can't hand it over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Line(NonZeroU32);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// How an editor takes the line to open a file at, as its own documentation gives it (the sources
/// are in ticket 35's comments).
#[derive(Debug, Clone, Copy)]
enum TakesLine {
    /// `--goto <file>:<line>`: VS Code. `Code.exe` reads it itself, as its `code.cmd` passes it on.
    Goto,
    /// `-n<line> <file>`: Notepad++.
    DashN,
    /// `<file>:<line>`: Sublime Text, `sublime_text.exe` as `subl.exe`.
    Suffix,
    /// `--line <line> <file>`, the line before the file: JetBrains' IDEs.
    LineOption,
}

/// The editors Downlight knows to take a line, by their programs' file names. Windows' names: the
/// other systems' ports add theirs.
const KNOWN_EDITORS: [(&str, TakesLine); 17] = [
    ("Code.exe", TakesLine::Goto),
    ("Code - Insiders.exe", TakesLine::Goto),
    ("notepad++.exe", TakesLine::DashN),
    ("sublime_text.exe", TakesLine::Suffix),
    ("subl.exe", TakesLine::Suffix),
    ("idea64.exe", TakesLine::LineOption),
    ("pycharm64.exe", TakesLine::LineOption),
    ("webstorm64.exe", TakesLine::LineOption),
    ("rider64.exe", TakesLine::LineOption),
    ("clion64.exe", TakesLine::LineOption),
    ("goland64.exe", TakesLine::LineOption),
    ("phpstorm64.exe", TakesLine::LineOption),
    ("rubymine64.exe", TakesLine::LineOption),
    ("datagrip64.exe", TakesLine::LineOption),
    ("dataspell64.exe", TakesLine::LineOption),
    ("rustrover64.exe", TakesLine::LineOption),
    // RustRover's documentation names its launcher so.
    ("rustrover.exe", TakesLine::LineOption),
];

/// How the program at `program` takes a line, if Downlight knows. Windows ignores the case of
/// file names, so the table does too.
fn how_it_takes_line(program: &Path) -> Option<TakesLine> {
    let name = program.file_name()?;
    KNOWN_EDITORS
        .iter()
        .find(|(known, _)| name.eq_ignore_ascii_case(known))
        .map(|&(_, how)| how)
}

/// Whether Downlight knows how the program at `program` takes a line.
pub fn takes_line(program: &Path) -> bool {
    how_it_takes_line(program).is_some()
}

/// The arguments to start the program at `program` with, to open `document` at `line`: as the
/// program takes a line if Downlight knows, or else the document's path alone, as without a line.
pub fn editor_arguments(program: &Path, document: &Path, line: Option<Line>) -> Vec<OsString> {
    // The whole guard against a document named like an option (`-n1.md`): the window's record
    // holds only absolute paths, which never start with `-`.
    debug_assert!(document.is_absolute(), "{}", document.display());
    let document = document.as_os_str();
    let (Some(line), Some(how)) = (line, how_it_takes_line(program)) else {
        return vec![document.to_owned()];
    };
    let at_line = || {
        let mut argument = document.to_owned();
        argument.push(format!(":{line}"));
        argument
    };
    match how {
        TakesLine::Goto => vec!["--goto".into(), at_line()],
        TakesLine::DashN => vec![format!("-n{line}").into(), document.to_owned()],
        TakesLine::Suffix => vec![at_line()],
        TakesLine::LineOption => vec![
            "--line".into(),
            line.to_string().into(),
            document.to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::*;

    /// The line as the app takes it from the page's `edit_document`, which Tauri hands over as
    /// JSON.
    fn line_from(json: serde_json::Value) -> serde_json::Result<Option<Line>> {
        serde_json::from_value(json)
    }

    #[test]
    fn edit_takes_a_line_number_or_none() {
        assert_eq!(line_from(serde_json::json!(15)).unwrap(), line(15));
        assert_eq!(line_from(serde_json::Value::Null).unwrap(), None);
    }

    #[test]
    fn edit_refuses_a_line_that_is_not_a_line_number() {
        for json in [
            serde_json::json!(0),
            serde_json::json!(-3),
            serde_json::json!(1.5),
            serde_json::json!(4_294_967_296_u64),
            serde_json::json!("15"),
            serde_json::json!("15 --new-window"),
            serde_json::json!([15]),
        ] {
            assert!(line_from(json.clone()).is_err(), "{json}");
        }
    }

    const DOCUMENT: &str = r"C:\docs\notes.md";

    fn line(number: u32) -> Option<Line> {
        line_from(serde_json::json!(number)).unwrap()
    }

    /// The arguments Edit starts the program at `program` with, for `DOCUMENT` at `line`.
    fn arguments_for(program: &str, line: Option<Line>) -> Vec<String> {
        editor_arguments(Path::new(program), Path::new(DOCUMENT), line)
            .into_iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn vs_code_opens_the_document_at_the_line_with_goto() {
        for program in [
            r"C:\Users\reader\AppData\Local\Programs\Microsoft VS Code\Code.exe",
            r"C:\Program Files\Microsoft VS Code Insiders\Code - Insiders.exe",
        ] {
            assert_eq!(
                arguments_for(program, line(15)),
                ["--goto", r"C:\docs\notes.md:15"],
                "{program}"
            );
        }
    }

    #[test]
    fn notepad_plus_plus_opens_the_document_at_the_line_with_n() {
        assert_eq!(
            arguments_for(r"C:\Program Files\Notepad++\notepad++.exe", line(15)),
            ["-n15", DOCUMENT]
        );
    }

    #[test]
    fn sublime_text_opens_the_document_at_the_line_after_its_name() {
        for program in [
            r"C:\Program Files\Sublime Text\sublime_text.exe",
            r"C:\Program Files\Sublime Text\subl.exe",
        ] {
            assert_eq!(
                arguments_for(program, line(15)),
                [r"C:\docs\notes.md:15"],
                "{program}"
            );
        }
    }

    #[test]
    fn jetbrains_ides_open_the_document_at_the_line_with_line_before_it() {
        for ide in [
            "idea64.exe",
            "pycharm64.exe",
            "webstorm64.exe",
            "rider64.exe",
            "clion64.exe",
            "goland64.exe",
            "phpstorm64.exe",
            "rubymine64.exe",
            "datagrip64.exe",
            "dataspell64.exe",
            "rustrover64.exe",
            "rustrover.exe",
        ] {
            let program = format!(r"C:\Program Files\JetBrains\IDE\bin\{ide}");
            assert_eq!(
                arguments_for(&program, line(15)),
                ["--line", "15", DOCUMENT],
                "{ide}"
            );
        }
    }

    #[test]
    fn editor_is_known_by_its_file_name_in_any_case() {
        assert_eq!(
            arguments_for(r"C:\Tools\VS Code\CODE.EXE", line(7)),
            ["--goto", r"C:\docs\notes.md:7"]
        );
        assert_eq!(
            arguments_for(r"C:\Tools\Notepad++\Notepad++.Exe", line(7)),
            ["-n7", DOCUMENT]
        );
    }

    #[test]
    fn editor_not_known_to_take_a_line_opens_the_document_alone() {
        for program in [
            r"C:\Tools\edit.exe",
            r"C:\Windows\System32\notepad.exe",
            // Only the program's own file name counts, never a folder named like one.
            r"C:\Code.exe\edit.exe",
        ] {
            assert_eq!(arguments_for(program, line(15)), [DOCUMENT], "{program}");
            assert!(!takes_line(Path::new(program)), "{program}");
        }
    }

    #[test]
    fn editor_without_a_line_opens_the_document_alone() {
        assert_eq!(
            arguments_for(r"C:\Program Files\Notepad++\notepad++.exe", None),
            [DOCUMENT]
        );
    }

    #[test]
    fn document_path_stays_one_argument_whatever_its_name() {
        // A file named like options is still the document's path, in the one argument that holds
        // it.
        let document = Path::new(r"C:\docs\--new-window -n1 x.md");
        let arguments = editor_arguments(
            Path::new(r"C:\Program Files\JetBrains\IDE\bin\idea64.exe"),
            document,
            line(3),
        );
        assert_eq!(
            arguments,
            [OsStr::new("--line"), OsStr::new("3"), document.as_os_str()]
        );
    }
}
