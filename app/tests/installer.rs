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

/// Downlight's license and the third-party notices (ticket 36) go in the install folder beside
/// the exe, from files that exist. Tauri's NSIS installer writes each bundle resource to its
/// target under the install folder, and its uninstaller deletes them again. The license is
/// installed as `LICENSE.txt`, so a double-click opens it as text.
#[test]
fn installer_puts_the_license_and_the_notices_beside_the_exe() -> TestResult {
    let config: serde_json::Value = serde_json::from_str(&read_app_file("tauri.conf.json")?)?;
    let resources = config["bundle"]["resources"]
        .as_object()
        .ok_or("tauri.conf.json's bundle.resources isn't a map of sources to targets")?;
    let mut targets: Vec<(&str, &str)> = resources
        .iter()
        .map(|(source, target)| (source.as_str(), target.as_str().unwrap_or_default()))
        .collect();
    targets.sort_unstable();
    assert_eq!(
        targets,
        [
            ("../LICENSE", "LICENSE.txt"),
            ("notices/THIRD-PARTY-NOTICES.txt", "THIRD-PARTY-NOTICES.txt"),
        ]
    );
    for (source, _) in targets {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(source);
        assert!(file.is_file(), "{} doesn't exist", file.display());
    }
    Ok(())
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
