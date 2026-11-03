use std::time::{Duration, SystemTime};

use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindowBuilder};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

/// Puts Mica behind the whole window. The window and its webview are transparent and the page
/// draws no background of its own, so the document sits on Mica. `Effect::Mica` follows the
/// system's light or dark mode.
pub fn with_backdrop<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder
        .transparent(true)
        .effects(EffectsBuilder::new().effect(Effect::Mica).build())
}

/// When Windows created this process, so first paint counts the loader's work before `main` too.
pub fn process_start() -> Option<SystemTime> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: GetCurrentProcess returns a pseudo handle that is always valid and needs no
    // closing, and each pointer points to a live FILETIME that GetProcessTimes only writes.
    let succeeded = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if succeeded == 0 {
        return None;
    }
    // A FILETIME counts 100 ns ticks since 1601-01-01, 11,644,473,600 s before the Unix epoch.
    const TICKS_BEFORE_UNIX_EPOCH: u64 = 116_444_736_000_000_000;
    let ticks = (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
    let since_epoch = ticks.checked_sub(TICKS_BEFORE_UNIX_EPOCH)?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(since_epoch.checked_mul(100)?))
}
