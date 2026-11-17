//! The installer (ticket 17): its NSIS hooks register Downlight for the extensions the core opens
//! as Markdown, and Tauri's own file associations stay unused (`app/installer/hooks.nsh` says why).

use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn read_app_file(name: &str) -> TestResult<String> {
    Ok(fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(name),
    )?)
}

/// The extensions in the hooks' `DOWNLIGHT_FOR_EACH_EXTENSION`, in their order.
fn extensions_the_installer_registers() -> TestResult<Vec<String>> {
    let hooks = read_app_file("installer/hooks.nsh")?;
    let body = hooks
        .split("!macro DOWNLIGHT_FOR_EACH_EXTENSION MACRO")
        .nth(1)
        .and_then(|rest| rest.split("!macroend").next())
        .ok_or("hooks.nsh has no DOWNLIGHT_FOR_EACH_EXTENSION macro")?;
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("!insertmacro ${MACRO} \"")
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_owned)
                .ok_or_else(|| format!("unexpected line in DOWNLIGHT_FOR_EACH_EXTENSION: {line}"))
        })
        .collect::<Result<_, _>>()
        .map_err(Into::into)
}

#[test]
fn installer_registers_every_markdown_extension_and_no_other() -> TestResult {
    assert_eq!(
        extensions_the_installer_registers()?,
        downlight_core::MARKDOWN_EXTENSIONS
    );
    Ok(())
}

#[test]
fn installer_registers_the_associations_only_through_its_hooks() -> TestResult {
    let config: serde_json::Value = serde_json::from_str(&read_app_file("tauri.conf.json")?)?;
    let bundle = &config["bundle"];
    assert!(
        bundle.get("fileAssociations").is_none(),
        "Tauri's fileAssociations would claim each extension's default; hooks.nsh registers them"
    );
    assert_eq!(
        bundle["windows"]["nsis"]["installerHooks"],
        "installer/hooks.nsh"
    );
    Ok(())
}
