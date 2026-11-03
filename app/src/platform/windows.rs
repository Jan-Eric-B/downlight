use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Runtime, WebviewWindowBuilder};

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
