//! Edit (Ctrl+E) and Choose editor... (ticket 15): the document opens in the reader's editor,
//! which is the program they chose if it is still there, else the text file program, else
//! Notepad. Edit never opens Downlight itself, which may well be the program for `.md` files and
//! even for `.txt` ones.

use std::io;
use std::path::{Path, PathBuf};

use tauri::{Manager as _, State, WebviewWindow};

use crate::documents::{Documents, file_name_shown};
use crate::notice::{NamedFile, Notice};
use crate::platform;
use crate::settings::{SettingsError, SettingsStore};

/// A program Edit tries to open the document in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editor {
    /// The program the reader picked with Choose editor...
    Chosen(PathBuf),
    /// The program the system opens `.txt` files with, started through the shell so that a
    /// packaged one, such as Windows 11's Notepad, starts too.
    TextFileProgram,
    Notepad,
}

/// Opens the calling window's document in the reader's editor. The launch runs on a blocking
/// thread, since starting a program can take a while.
#[tauri::command]
pub async fn edit_document(
    window: WebviewWindow,
    settings: State<'_, SettingsStore>,
) -> Result<(), Notice> {
    let app = window.app_handle().clone();
    let chosen = settings.editor();
    tauri::async_runtime::spawn_blocking(move || {
        edit(
            &app.state::<Documents>(),
            window.label(),
            chosen.as_deref(),
            platform::launch_editor,
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
        keep_choice(&app.state::<SettingsStore>(), program)
    })
    .await
    .map_err(|source| EditError::DialogStopped { source })??;
    Ok(())
}

/// Opens `window`'s document in the first of the reader's editors that starts, with `launch`, the
/// OS's launch, passed in so a test sees what it is given without a program starting.
fn edit(
    documents: &Documents,
    window: &str,
    chosen: Option<&Path>,
    launch: impl Fn(&Editor, &Path) -> io::Result<()>,
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
    let text_file_program = platform::text_file_program();
    let order = editor_order(chosen, text_file_program.as_deref(), Path::is_file, is_own);
    let mut failure = io::Error::other("no editor to start");
    for editor in order {
        match launch(&editor, &document) {
            Ok(()) => return Ok(()),
            Err(error) => failure = error,
        }
    }
    Err(EditError::NoEditorStarted {
        document,
        source: failure,
    })
}

/// The editors Edit tries, in turn: the chosen editor if it is still a file, then the text file
/// program, then Notepad, leaving out any that is Downlight. `text_file_program` is the program's
/// file, or None when the system doesn't say which it is, as for some packaged programs; it is
/// tried then too, since Downlight is never a packaged program.
fn editor_order(
    chosen: Option<&Path>,
    text_file_program: Option<&Path>,
    is_file: impl Fn(&Path) -> bool,
    is_downlight: impl Fn(&Path) -> bool,
) -> Vec<Editor> {
    let mut order = Vec::with_capacity(3);
    if let Some(chosen) = chosen.filter(|chosen| is_file(chosen) && !is_downlight(chosen)) {
        order.push(Editor::Chosen(chosen.to_owned()));
    }
    if !text_file_program.is_some_and(&is_downlight) {
        order.push(Editor::TextFileProgram);
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
fn keep_choice(settings: &SettingsStore, program: PathBuf) -> Result<(), EditError> {
    let own_exe = std::env::current_exe().ok();
    check_choice(&program, own_exe.as_deref())?;
    settings
        .update(|settings| settings.editor = Some(program.clone()))
        .map_err(|source| EditError::NotSaved { program, source })
}

/// Whether `program` can be the reader's editor: a program (`.exe`, which also keeps Rust from
/// quoting the document's path as a batch file's argument), and never Downlight.
fn check_choice(program: &Path, own_exe: Option<&Path>) -> Result<(), EditError> {
    let is_exe = program
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    if !is_exe {
        return Err(EditError::NotAProgram(program.to_owned()));
    }
    if own_exe.is_some_and(|own| is_downlight(program, own)) {
        return Err(EditError::ChoseDownlight);
    }
    Ok(())
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

    use super::*;
    use crate::documents::recorder::recorded;
    use crate::documents::{DocumentAt, Target};
    use crate::settings::SettingsStore;

    const CHOSEN: &str = r"C:\Tools\edit.exe";
    const TEXT_FILE_PROGRAM: &str = r"C:\Program Files\Notepad++\notepad++.exe";

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
            [chosen(), Editor::TextFileProgram, Editor::Notepad]
        );
    }

    #[test]
    fn chosen_editor_that_no_longer_exists_falls_back_to_the_text_file_program() {
        assert_eq!(
            order(false, Some(TEXT_FILE_PROGRAM), "downlight.exe"),
            [Editor::TextFileProgram, Editor::Notepad]
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
        assert_eq!(order, [Editor::TextFileProgram, Editor::Notepad]);
    }

    #[test]
    fn chosen_editor_that_is_downlight_is_skipped() {
        assert_eq!(
            order(true, Some(TEXT_FILE_PROGRAM), CHOSEN),
            [Editor::TextFileProgram, Editor::Notepad]
        );
    }

    #[test]
    fn text_file_program_that_is_downlight_is_skipped() {
        assert_eq!(
            order(false, Some(TEXT_FILE_PROGRAM), TEXT_FILE_PROGRAM),
            [Editor::Notepad]
        );
    }

    #[test]
    fn unknown_text_file_program_is_still_tried() {
        // Some packaged programs, such as Windows 11's Notepad, have no file the system names.
        assert_eq!(
            order(false, None, "downlight.exe"),
            [Editor::TextFileProgram, Editor::Notepad]
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
        let (settings, _) = SettingsStore::load(None);
        assert!(keep_choice(&settings, own_exe()).is_err());
        assert_eq!(settings.editor(), None);
    }

    #[test]
    fn choice_that_cannot_be_saved_is_the_editor_until_downlight_closes() {
        let (settings, _) = SettingsStore::load(None);
        let program = PathBuf::from(r"C:\Tools\edit.exe");
        let kept = keep_choice(&settings, program.clone());
        assert_eq!(settings.editor(), Some(program));
        let notice = Notice::from(kept.unwrap_err()).to_string();
        assert!(
            notice.ends_with("edit.exe is the editor until Downlight closes."),
            "{notice}"
        );
    }

    fn repo_file(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)
    }

    /// Edits the document a window shows, or none, with a chosen editor that is a real file, and
    /// gives what Edit gave and the editors it tried, which fail as `starts` says.
    fn edit_shown(
        document: Option<&Path>,
        starts: impl Fn(&Editor) -> bool,
    ) -> (Result<(), EditError>, Vec<(Editor, PathBuf)>) {
        let (documents, _) = recorded();
        let window = documents.open_empty().unwrap();
        if let Some(document) = document {
            documents
                .give(DocumentAt::new(document), Target::Window(&window))
                .unwrap();
        }
        let chosen = repo_file("app/Cargo.toml");
        let tried = RefCell::new(Vec::new());
        let edited = edit(&documents, &window, Some(&chosen), |editor, document| {
            tried
                .borrow_mut()
                .push((editor.clone(), document.to_owned()));
            if starts(editor) {
                Ok(())
            } else {
                Err(io::Error::other("it didn't start"))
            }
        });
        (edited, tried.into_inner())
    }

    #[test]
    fn edit_without_a_document_says_so() {
        let (edited, tried) = edit_shown(None, |_| true);
        assert!(matches!(edited, Err(EditError::NoDocument)), "{edited:?}");
        assert!(tried.is_empty());
        assert_eq!(
            Notice::from(EditError::NoDocument).to_string(),
            "There is no document to edit."
        );
    }

    #[test]
    fn edit_opens_the_window_s_document_in_the_chosen_editor() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (edited, tried) = edit_shown(Some(&notes), |_| true);
        assert!(edited.is_ok(), "{edited:?}");
        assert_eq!(
            tried,
            [(Editor::Chosen(repo_file("app/Cargo.toml")), notes)]
        );
    }

    #[test]
    fn editor_that_does_not_start_falls_back_to_the_next() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (edited, tried) =
            edit_shown(Some(&notes), |editor| !matches!(editor, Editor::Chosen(_)));
        assert!(edited.is_ok(), "{edited:?}");
        let tried: Vec<_> = tried.into_iter().map(|(editor, _)| editor).collect();
        assert_eq!(tried[1..], [Editor::TextFileProgram]);
    }

    #[test]
    fn edit_that_starts_no_editor_names_the_document_and_why() {
        let notes = repo_file("core/tests/fixtures/notes.md");
        let (edited, tried) = edit_shown(Some(&notes), |_| false);
        assert_eq!(
            tried.last().map(|(editor, _)| editor),
            Some(&Editor::Notepad)
        );
        let notice = Notice::from(edited.unwrap_err()).to_string();
        assert!(
            notice.starts_with("Can't open notes.md in ")
                && notice.ends_with(" in an editor: it didn't start"),
            "{notice}"
        );
    }
}
