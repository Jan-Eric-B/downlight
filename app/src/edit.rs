//! Edit (Ctrl+E) and Choose editor... (ticket 15): the document opens in the reader's editor,
//! which is the program they chose if it is still there, else the text file program, else
//! Notepad. Edit never opens Downlight itself, which may well be the program for `.md` files and
//! even for `.txt` ones. An editor Downlight knows to take a line opens the document at the
//! reading position's line (ticket 35, `line`).

mod line;

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{Manager as _, State, WebviewWindow};

use self::line::{Line, editor_arguments, takes_line};
use crate::documents::Documents;
use crate::notice::{NamedFile, Notice, file_name_shown};
use crate::platform;
use crate::settings::{SettingsError, SettingsStore};

/// A program Edit tries to open the document in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editor {
    /// The program the reader picked with Choose editor...
    Chosen(PathBuf),
    /// The program the system opens `.txt` files with, by its file, or None when the system
    /// doesn't say which it is, as for some packaged programs.
    TextFileProgram(Option<PathBuf>),
    Notepad,
}

/// The programs Edit can open a document in besides Notepad: the reader's chosen editor, and the
/// system's text file program, or None for either when there is none or the system doesn't say
/// which it is.
#[derive(Debug, Clone, Copy)]
struct Editors<'a> {
    chosen: Option<&'a Path>,
    text_file_program: Option<&'a Path>,
}

/// How Edit starts one editor.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Launch {
    /// A program by its path, with its arguments.
    Program {
        program: PathBuf,
        arguments: Vec<OsString>,
    },
    /// The text file program through the shell, with this document and no line.
    AsTextFile(PathBuf),
}

/// Opens the calling window's document in the reader's editor, at `line` for an editor known to
/// take one (ticket 35). The launch runs on a blocking thread, since starting a program can take a
/// while.
#[tauri::command]
pub async fn edit_document(
    window: WebviewWindow,
    settings: State<'_, Arc<SettingsStore>>,
    line: Option<Line>,
) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    let chosen = settings.get().editor;
    tauri::async_runtime::spawn_blocking(move || {
        let text_file_program = platform::text_file_program();
        let editors = Editors {
            chosen: chosen.as_deref(),
            text_file_program: text_file_program.as_deref(),
        };
        edit(
            &app.state::<Documents>(),
            window.label(),
            editors,
            line,
            Path::is_file,
            start,
        )
    })
    .await
    .map_err(|source| EditError::Stopped { source })??;
    Ok(())
}

/// Shows a file dialog for a program and keeps the one chosen as the reader's editor. Choosing
/// nothing changes nothing.
#[tauri::command]
pub async fn choose_editor(window: WebviewWindow) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(program) = rfd::FileDialog::new()
            .set_parent(&window)
            .set_title("Choose editor")
            .add_filter("Programs", &["exe"])
            .pick_file()
        else {
            return Ok(());
        };
        keep_choice(&app.state::<Documents>(), program)
    })
    .await
    .map_err(|source| EditError::DialogStopped { source })??;
    Ok(())
}

/// What Edit with in the settings flyout offers (window design ticket 08, `EditorChoices` in
/// `page/src/app.ts`): Default, named after the program Edit opens a document in without a chosen
/// editor, and the chosen editor, while Edit would open it. Asked for each time the flyout opens,
/// so a change of the program for `.txt` files shows without a restart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorChoices {
    /// The name of the text file program, or of Notepad when the text file program is Downlight;
    /// None when the system names neither.
    default_name: Option<String>,
    /// The editor the reader chose, unless Edit skips it: it is gone, or isn't a program.
    editor: Option<ChosenEditor>,
}

/// The editor the reader chose, as Edit with shows it: the program's own name, and its file for
/// the tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChosenEditor {
    name: String,
    path: PathBuf,
}

/// Samples of what Edit with offers, which the page's tests drive through the settings flyout
/// (`app/src/contract.rs`).
#[cfg(test)]
pub fn contract_samples() -> serde_json::Result<serde_json::Value> {
    let chosen = PathBuf::from(r"C:\Program Files\Notepad++\notepad++.exe");
    Ok(serde_json::json!({
        "withEditor": EditorChoices {
            default_name: Some("Notepad".to_owned()),
            editor: Some(ChosenEditor {
                name: "Notepad++ : a free (GNU) source code editor".to_owned(),
                path: chosen,
            }),
        },
        "withoutNames": EditorChoices {
            default_name: None,
            editor: None,
        },
    }))
}

/// What Edit with offers now: the reader's choice of editor, from the settings, against what the
/// system has. Asks the shell for the programs' names on a blocking thread, as Edit does.
#[tauri::command]
pub async fn editor_choices(
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<EditorChoices, Notice> {
    let chosen = settings.get().editor;
    tauri::async_runtime::spawn_blocking(move || {
        let own_exe = std::env::current_exe().ok();
        let is_own = |program: &Path| {
            own_exe
                .as_deref()
                .is_some_and(|own| is_downlight(program, own))
        };
        let text_file_program = platform::text_file_program();
        let order = editor_order(
            chosen.as_deref(),
            text_file_program.as_deref(),
            Path::is_file,
            is_own,
        );
        choices_of(&order, |editor| match editor {
            Editor::Chosen(program) => platform::program_name(program),
            Editor::TextFileProgram(_) => platform::text_file_program_name(),
            Editor::Notepad => {
                platform::notepad().and_then(|notepad| platform::program_name(&notepad))
            }
        })
    })
    .await
    .map_err(|source| format!("Edit with's choices couldn't be read: {source}").into())
}

/// What Edit with offers for the editors Edit tries in `order` (`editor_order`), each named by
/// `name_of`: the chosen editor if Edit tries it, and as Default the first one after it.
fn choices_of(order: &[Editor], name_of: impl Fn(&Editor) -> Option<String>) -> EditorChoices {
    let editor = order.iter().find_map(|editor| match editor {
        Editor::Chosen(path) => Some(ChosenEditor {
            // A program without a description of its own is named as the file manager names it.
            name: name_of(editor).unwrap_or_else(|| file_name_shown(path)),
            path: path.clone(),
        }),
        Editor::TextFileProgram(_) | Editor::Notepad => None,
    });
    let default_name = order
        .iter()
        .find(|editor| !matches!(editor, Editor::Chosen(_)))
        .and_then(&name_of);
    EditorChoices {
        default_name,
        editor,
    }
}

/// Opens `window`'s document at `line` in the first of the reader's `editors` that starts. The
/// OS's side is passed in, so a test sees what Edit does without a program starting: `is_file`,
/// whether a program's file is there, and `start`, the launch.
fn edit(
    documents: &Documents,
    window: &str,
    editors: Editors<'_>,
    line: Option<Line>,
    is_file: impl Fn(&Path) -> bool,
    start: impl Fn(&Launch) -> io::Result<()>,
) -> Result<(), EditError> {
    let document = documents
        .document_path(window)
        .ok_or(EditError::NoDocument)?;
    let own_exe = std::env::current_exe().ok();
    let is_own = |program: &Path| {
        own_exe
            .as_deref()
            .is_some_and(|own| is_downlight(program, own))
    };
    let order = editor_order(editors.chosen, editors.text_file_program, &is_file, is_own);
    let mut failure = io::Error::other("no editor to start");
    for editor in order {
        match launch_of(&editor, &document, line, &is_file).and_then(|launch| start(&launch)) {
            Ok(()) => return Ok(()),
            Err(error) => failure = error,
        }
    }
    Err(EditError::NoEditorStarted {
        document,
        source: failure,
    })
}

/// How Edit starts `editor` to open `document` at `line`: a program with the arguments that open
/// it there if Downlight knows how the program takes a line, or with the document's path alone;
/// the text file program the way the OS starts it, which passes no line.
///
/// A text file program that Downlight knows to take a line is started by its path instead, so
/// that it gets the line (ticket 35), when its file is there and named in full, as a chosen
/// editor must be: a bare name would be searched for on the PATH. A file the system names that
/// is gone is still left to the shell, as before.
fn launch_of(
    editor: &Editor,
    document: &Path,
    line: Option<Line>,
    is_file: impl Fn(&Path) -> bool,
) -> io::Result<Launch> {
    let program = |program: PathBuf| Launch::Program {
        arguments: editor_arguments(&program, document, line),
        program,
    };
    Ok(match editor {
        Editor::Chosen(path) => program(path.clone()),
        Editor::TextFileProgram(Some(path))
            if path.is_absolute() && takes_line(path) && is_file(path) =>
        {
            program(path.clone())
        }
        Editor::TextFileProgram(_) => Launch::AsTextFile(document.to_owned()),
        Editor::Notepad => program(platform::notepad().ok_or(io::ErrorKind::Unsupported)?),
    })
}

/// Starts an editor as `launch` says, and doesn't wait for it.
fn start(launch: &Launch) -> io::Result<()> {
    match launch {
        Launch::Program { program, arguments } => std::process::Command::new(program)
            .args(arguments)
            .spawn()
            .map(drop),
        Launch::AsTextFile(document) => platform::open_as_text_file(document),
    }
}

/// The editors Edit tries, in turn: the chosen editor if it is still a file, then the text file
/// program, then Notepad, leaving out any that is Downlight. `text_file_program` is the program's
/// file, or None when the system doesn't say which it is, as for some packaged programs; it is
/// tried then too, since Downlight is never a packaged program.
///
/// The chosen editor is checked again as Choose editor... checks it, since a hand-edited settings
/// file can name anything, and it needs a full path too: a bare name would be searched for on the
/// PATH (ticket 15's review).
fn editor_order(
    chosen: Option<&Path>,
    text_file_program: Option<&Path>,
    is_file: impl Fn(&Path) -> bool,
    is_downlight: impl Fn(&Path) -> bool,
) -> Vec<Editor> {
    let mut order = Vec::with_capacity(3);
    let can_start = |chosen: &Path| {
        chosen.is_absolute() && is_program(chosen) && is_file(chosen) && !is_downlight(chosen)
    };
    if let Some(chosen) = chosen.filter(|chosen| can_start(chosen)) {
        order.push(Editor::Chosen(chosen.to_owned()));
    }
    if !text_file_program.is_some_and(&is_downlight) {
        order.push(Editor::TextFileProgram(
            text_file_program.map(Path::to_owned),
        ));
    }
    order.push(Editor::Notepad);
    order
}

/// Whether `program` is Downlight's own `own_exe`, under whatever name reaches it: another case,
/// a short name, a link.
fn is_downlight(program: &Path, own_exe: &Path) -> bool {
    let file = |path: &Path| path.canonicalize().ok().map(platform::fold_case);
    file(program).is_some_and(|program| Some(program) == file(own_exe))
}

/// Keeps `program` as the reader's editor, if it can be one.
fn keep_choice(documents: &Documents, program: PathBuf) -> Result<(), EditError> {
    let own_exe = std::env::current_exe().ok();
    check_choice(&program, own_exe.as_deref())?;
    documents
        .change_settings(|settings| settings.editor = Some(program.clone()))
        .map_err(|source| EditError::NotSaved { program, source })
}

/// Whether `program` can be the reader's editor: a program, and never Downlight.
fn check_choice(program: &Path, own_exe: Option<&Path>) -> Result<(), EditError> {
    if !is_program(program) {
        return Err(EditError::NotAProgram(program.to_owned()));
    }
    if own_exe.is_some_and(|own| is_downlight(program, own)) {
        return Err(EditError::ChoseDownlight);
    }
    Ok(())
}

/// Whether `program` is named as a program, an `.exe`, which also keeps Rust from quoting the
/// document's path as a batch file's argument.
fn is_program(program: &Path) -> bool {
    program
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

/// Why Edit or Choose editor... didn't do what the reader asked. The page shows it as a notice.
#[derive(Debug)]
pub enum EditError {
    /// The window shows no document.
    NoDocument,
    /// The reader chose Downlight as the editor.
    ChoseDownlight,
    /// The reader chose a file that isn't a program.
    NotAProgram(PathBuf),
    /// No editor started; `source` is why the last one tried, Notepad, didn't.
    NoEditorStarted {
        document: PathBuf,
        source: io::Error,
    },
    /// The choice holds until Downlight closes, but wasn't saved.
    NotSaved {
        program: PathBuf,
        source: SettingsError,
    },
    /// The background task that starts the editor panicked or was cancelled.
    Stopped { source: tauri::Error },
    /// The background task that shows the dialog panicked or was cancelled.
    DialogStopped { source: tauri::Error },
}

impl From<EditError> for Notice {
    fn from(error: EditError) -> Self {
        match error {
            EditError::NoDocument => "There is no document to edit.".into(),
            EditError::ChoseDownlight => {
                "Downlight can't be the editor: Edit opens the document in another program.".into()
            }
            EditError::NotAProgram(program) => Self::naming(
                "",
                NamedFile::at(&program),
                " isn't a program. Choose an .exe file as the editor.",
            ),
            EditError::NoEditorStarted { document, source } => Self::naming(
                "Can't open ",
                NamedFile::at(&document),
                format!(" in an editor: {source}"),
            ),
            EditError::NotSaved { program, source } => format!(
                "{source}. {} is the editor until Downlight closes.",
                file_name_shown(&program)
            )
            .into(),
            EditError::Stopped { source } => format!("Edit stopped: {source}").into(),
            EditError::DialogStopped { source } => {
                format!("The Choose editor dialog stopped: {source}").into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::ffi::OsStr;

    use super::*;
    use crate::documents::recorder::recorded;
    use crate::documents::{DocumentAt, Target};

    const CHOSEN: &str = r"C:\Tools\edit.exe";
    /// A text file program that Downlight doesn't know to take a line (ticket 35).
    const TEXT_FILE_PROGRAM: &str = r"C:\Program Files\EmEditor\EmEditor.exe";
    /// A text file program that Downlight knows to take a line, which no test machine has.
    const VS_CODE: &str = r"C:\Tests\Microsoft VS Code\Code.exe";

    fn text_file_program() -> Editor {
        Editor::TextFileProgram(Some(PathBuf::from(TEXT_FILE_PROGRAM)))
    }

    fn order(chosen_exists: bool, text_file_program: Option<&str>, downlight: &str) -> Vec<Editor> {
        editor_order(
            Some(Path::new(CHOSEN)),
            text_file_program.map(Path::new),
            |_| chosen_exists,
            |program| program == Path::new(downlight),
        )
    }

    fn chosen() -> Editor {
        Editor::Chosen(PathBuf::from(CHOSEN))
    }

    #[test]
    fn chosen_editor_that_exists_comes_first() {
        assert_eq!(
            order(true, Some(TEXT_FILE_PROGRAM), "downlight.exe"),
            [chosen(), text_file_program(), Editor::Notepad]
        );
    }

    #[test]
    fn chosen_editor_that_no_longer_exists_falls_back_to_the_text_file_program() {
        assert_eq!(
            order(false, Some(TEXT_FILE_PROGRAM), "downlight.exe"),
            [text_file_program(), Editor::Notepad]
        );
    }

    #[test]
    fn no_chosen_editor_tries_the_text_file_program_then_notepad() {
        let order = editor_order(
            None,
            Some(Path::new(TEXT_FILE_PROGRAM)),
            |_| true,
            |_| false,
        );
        assert_eq!(order, [text_file_program(), Editor::Notepad]);
    }

    #[test]
    fn chosen_editor_that_is_downlight_is_skipped() {
        assert_eq!(
            order(true, Some(TEXT_FILE_PROGRAM), CHOSEN),
            [text_file_program(), Editor::Notepad]
        );
    }

    #[test]
    fn text_file_program_that_is_downlight_is_skipped() {
        assert_eq!(
            order(false, Some(TEXT_FILE_PROGRAM), TEXT_FILE_PROGRAM),
            [Editor::Notepad]
        );
    }

    /// The editors Edit tries for a chosen editor that is a file and isn't Downlight, as a
    /// hand-edited settings file may name it.
    fn order_for_saved(chosen: &str) -> Vec<Editor> {
        editor_order(
            Some(Path::new(chosen)),
            Some(Path::new(TEXT_FILE_PROGRAM)),
            |_| true,
            |_| false,
        )
    }

    #[test]
    fn saved_editor_that_is_not_an_exe_is_not_tried() {
        // Choose editor... refuses it; a settings file edited by hand may still name it.
        assert_eq!(
            order_for_saved(r"C:\Tools\edit.cmd"),
            [text_file_program(), Editor::Notepad]
        );
    }

    #[test]
    fn saved_editor_without_a_full_path_is_not_tried() {
        // Starting it would search the folders on the PATH for it.
        for chosen in [
            "edit.exe",
            r"Tools\edit.exe",
            r"\Tools\edit.exe",
            r"C:edit.exe",
        ] {
            assert_eq!(
                order_for_saved(chosen),
                [text_file_program(), Editor::Notepad],
                "{chosen}"
            );
        }
    }

    #[test]
    fn unknown_text_file_program_is_still_tried() {
        // Some packaged programs, such as Windows 11's Notepad, have no file the system names.
        assert_eq!(
            order(false, None, "downlight.exe"),
            [Editor::TextFileProgram(None), Editor::Notepad]
        );
    }

    /// What Edit with offers for `order`, with each program named after its file and the text
    /// file program named `Text editor`.
    fn choices(order: &[Editor]) -> EditorChoices {
        choices_of(order, |editor| match editor {
            Editor::Chosen(program) => program
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned()),
            Editor::TextFileProgram(_) => Some("Text editor".to_owned()),
            Editor::Notepad => Some("Notepad".to_owned()),
        })
    }

    #[test]
    fn edit_with_offers_the_chosen_editor_and_the_text_file_program_as_default() {
        assert_eq!(
            choices(&order(true, Some(TEXT_FILE_PROGRAM), "downlight.exe")),
            EditorChoices {
                default_name: Some("Text editor".to_owned()),
                editor: Some(ChosenEditor {
                    name: "edit".to_owned(),
                    path: PathBuf::from(CHOSEN),
                }),
            }
        );
    }

    #[test]
    fn edit_with_offers_no_chosen_editor_that_edit_would_skip() {
        assert_eq!(
            choices(&order(false, Some(TEXT_FILE_PROGRAM), "downlight.exe")).editor,
            None
        );
    }

    #[test]
    fn edit_with_names_notepad_as_default_when_the_text_file_program_is_downlight() {
        assert_eq!(
            choices(&order(false, Some(TEXT_FILE_PROGRAM), TEXT_FILE_PROGRAM)).default_name,
            Some("Notepad".to_owned())
        );
    }

    #[test]
    fn chosen_editor_without_a_name_of_its_own_is_named_after_its_file() {
        let order = [chosen(), text_file_program(), Editor::Notepad];
        let unnamed = choices_of(&order, |_| None);
        assert_eq!(unnamed.default_name, None);
        assert_eq!(
            unnamed.editor.map(|editor| editor.name),
            Some("edit.exe".to_owned())
        );
    }

    fn own_exe() -> PathBuf {
        std::env::current_exe().unwrap()
    }

    #[test]
    fn own_exe_in_another_case_is_downlight() {
        let own = own_exe();
        let other_case = PathBuf::from(own.to_string_lossy().to_uppercase());
        assert!(is_downlight(&other_case, &own));
    }

    #[test]
    fn another_program_is_not_downlight() {
        let cargo_toml = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(!is_downlight(&cargo_toml, &own_exe()));
    }

    #[test]
    fn missing_program_is_not_downlight() {
        let own = own_exe();
        assert!(!is_downlight(&own.with_file_name("missing.exe"), &own));
    }

    #[test]
    fn choosing_downlight_is_refused() {
        let own = own_exe();
        assert!(matches!(
            check_choice(&own, Some(&own)),
            Err(EditError::ChoseDownlight)
        ));
    }

    #[test]
    fn choosing_a_file_that_is_not_an_exe_is_refused() {
        let script = Path::new(r"C:\Tools\edit.cmd");
        let refused = check_choice(script, Some(&own_exe()));
        assert!(
            matches!(&refused, Err(EditError::NotAProgram(path)) if path == script),
            "{refused:?}"
        );
        assert_eq!(
            Notice::from(refused.unwrap_err()).to_string(),
            r"edit.cmd in C:\Tools isn't a program. Choose an .exe file as the editor."
        );
    }

    #[test]
    fn choosing_another_program_is_allowed_in_any_case() {
        assert!(check_choice(Path::new(r"C:\Tools\EDIT.EXE"), Some(&own_exe())).is_ok());
    }

    #[test]
    fn choice_that_is_refused_leaves_the_editor_as_it_was() {
        let (documents, _) = recorded();
        assert!(keep_choice(&documents, own_exe()).is_err());
        assert_eq!(documents.settings().editor, None);
    }

    #[test]
    fn choice_that_cannot_be_saved_is_the_editor_until_downlight_closes() {
        let (documents, _) = recorded();
        let program = PathBuf::from(r"C:\Tools\edit.exe");
        let kept = keep_choice(&documents, program.clone());
        assert_eq!(documents.settings().editor, Some(program));
        let notice = Notice::from(kept.unwrap_err()).to_string();
        assert!(
            notice.ends_with("edit.exe is the editor until Downlight closes."),
            "{notice}"
        );
    }

    fn repo_file(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)
    }

    fn notes() -> PathBuf {
        repo_file("core/tests/fixtures/notes.md")
    }

    /// A real program, by its full path, that is never Downlight and that Downlight doesn't know
    /// to take a line: the Cargo running the tests.
    fn chosen_program() -> PathBuf {
        PathBuf::from(env!("CARGO"))
    }

    /// The reading position's line the page sends in these tests.
    fn line_15() -> Option<Line> {
        serde_json::from_value(serde_json::json!(15)).unwrap()
    }

    /// Edits the document a window shows, or none, at `line`, with the reader's `chosen` editor and
    /// the system's `text_file_program`, and gives what Edit gave and the launches it tried, which
    /// fail as `starts` says. The test machine's files are there, and `VS_CODE` too.
    fn edit_shown(
        document: Option<&Path>,
        (chosen, text_file_program): (Option<&Path>, Option<&Path>),
        line: Option<Line>,
        starts: impl Fn(&Launch) -> bool,
    ) -> (Result<(), EditError>, Vec<Launch>) {
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        if let Some(document) = document {
            documents
                .give(DocumentAt::new(document), Target::Window(&window))
                .unwrap();
        }
        let tried = RefCell::new(Vec::new());
        let editors = Editors {
            chosen,
            text_file_program,
        };
        let is_file = |path: &Path| path == Path::new(VS_CODE) || path.is_file();
        let edited = edit(&documents, &window, editors, line, is_file, |launch| {
            tried.borrow_mut().push(launch.clone());
            if starts(launch) {
                Ok(())
            } else {
                Err(io::Error::other("it didn't start"))
            }
        });
        (edited, tried.into_inner())
    }

    /// The reader's editor is Cargo, and the text file program one Downlight doesn't know.
    fn chosen_and_unknown() -> (PathBuf, PathBuf) {
        (chosen_program(), PathBuf::from(TEXT_FILE_PROGRAM))
    }

    fn program(program: impl Into<PathBuf>, arguments: &[&OsStr]) -> Launch {
        Launch::Program {
            program: program.into(),
            arguments: arguments
                .iter()
                .map(|&argument| argument.to_owned())
                .collect(),
        }
    }

    #[test]
    fn edit_without_a_document_says_so() {
        let (chosen, text_file_program) = chosen_and_unknown();
        let (edited, tried) = edit_shown(
            None,
            (Some(&chosen), Some(&text_file_program)),
            line_15(),
            |_| true,
        );
        assert!(matches!(edited, Err(EditError::NoDocument)), "{edited:?}");
        assert!(tried.is_empty());
        assert_eq!(
            Notice::from(EditError::NoDocument).to_string(),
            "There is no document to edit."
        );
    }

    #[test]
    fn edit_opens_the_window_s_document_in_the_chosen_editor() {
        // Cargo isn't known to take a line, so it gets the document alone, as before ticket 35.
        let (chosen, text_file_program) = chosen_and_unknown();
        let (edited, tried) = edit_shown(
            Some(&notes()),
            (Some(&chosen), Some(&text_file_program)),
            line_15(),
            |_| true,
        );
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried, [program(chosen, &[notes().as_os_str()])]);
    }

    /// VS Code's launch at line 15 of `notes()`.
    fn vs_code_at_line_15() -> Launch {
        let mut at_line = notes().into_os_string();
        at_line.push(":15");
        program(VS_CODE, &[OsStr::new("--goto"), &at_line])
    }

    #[test]
    fn edit_opens_the_document_at_the_line_in_a_text_file_program_that_takes_one() {
        // The shell passes the text file program no line, so it is started as a chosen editor is.
        let vs_code = Path::new(VS_CODE);
        let (edited, tried) =
            edit_shown(Some(&notes()), (None, Some(vs_code)), line_15(), |_| true);
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried, [vs_code_at_line_15()]);
    }

    #[test]
    fn text_file_program_that_takes_a_line_opens_at_it_after_a_chosen_editor_that_does_not_start() {
        let chosen = chosen_program();
        let (edited, tried) = edit_shown(
            Some(&notes()),
            (Some(&chosen), Some(Path::new(VS_CODE))),
            line_15(),
            |launch| *launch != program(&chosen, &[notes().as_os_str()]),
        );
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried[1..], [vs_code_at_line_15()]);
    }

    #[test]
    fn text_file_program_that_takes_a_line_but_is_gone_opens_through_the_shell() {
        // The registry may still name a program that was uninstalled; the shell may know better.
        let gone = Path::new(r"C:\Program Files\Gone\Code.exe");
        let (edited, tried) = edit_shown(Some(&notes()), (None, Some(gone)), line_15(), |_| true);
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried, [Launch::AsTextFile(notes())]);
    }

    #[test]
    fn edit_opens_another_text_file_program_through_the_shell_without_a_line() {
        let (edited, tried) = edit_shown(
            Some(&notes()),
            (None, Some(Path::new(TEXT_FILE_PROGRAM))),
            line_15(),
            |_| true,
        );
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried, [Launch::AsTextFile(notes())]);
    }

    #[test]
    fn editor_that_does_not_start_falls_back_to_the_next() {
        let (chosen, text_file_program) = chosen_and_unknown();
        let (edited, tried) = edit_shown(
            Some(&notes()),
            (Some(&chosen), Some(&text_file_program)),
            line_15(),
            |launch| matches!(launch, Launch::AsTextFile(_)),
        );
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(tried[1..], [Launch::AsTextFile(notes())]);
    }

    #[test]
    fn edit_that_starts_no_editor_names_the_document_and_why() {
        let (chosen, text_file_program) = chosen_and_unknown();
        let (edited, tried) = edit_shown(
            Some(&notes()),
            (Some(&chosen), Some(&text_file_program)),
            line_15(),
            |_| false,
        );
        let notepad = platform::notepad().unwrap();
        assert_eq!(
            tried.last(),
            Some(&program(notepad, &[notes().as_os_str()]))
        );
        let notice = Notice::from(edited.unwrap_err()).to_string();
        assert!(
            notice.starts_with("Can't open notes.md in ")
                && notice.ends_with(" in an editor: it didn't start"),
            "{notice}"
        );
    }
}
