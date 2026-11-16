//! The window behind the page's maximize button (window design ticket 01). WebView2's windows
//! belong to another process, so Windows never asks Downlight's window what is under the pointer
//! while it is over the page. This small child window of the window's own thread sits over the
//! page's maximize button and answers `HTMAXBUTTON`, which is what makes Windows show the snap
//! layouts flyout, as Windows Terminal's drag bar does. Tauri covers the top resize border the
//! same way (`TAURI_DRAG_RESIZE_BORDERS`).
//!
//! The pointer over it never reaches the page, so the window does what the page's button would:
//! it tells the page when to show the button hovered or pressed, and maximizes or restores the
//! window on a click.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Once;

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowExW, GWLP_USERDATA, GetParent, GetWindowLongPtrW,
    GetWindowRect, HTMAXBUTTON, HTTRANSPARENT, HWND_TOP, IsWindow, IsZoomed, PostMessageW,
    RegisterClassExW, SC_MAXIMIZE, SC_RESTORE, SM_CYFRAME, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WM_NCDESTROY, WM_NCHITTEST, WM_NCLBUTTONDBLCLK,
    WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_SYSCOMMAND, WNDCLASSEXW,
    WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
};

use crate::title_bar::{ButtonRect, MaximizeButtonState};

/// The class of the maximize button's window, by which `place` finds it again.
const CLASS_NAME: *const u16 = windows_sys::w!("DOWNLIGHT_MAXIMIZE_BUTTON");

/// What the maximize button's window keeps, from its creation until `WM_NCDESTROY`. It is used
/// only on the window's thread.
struct ButtonWindow {
    button: Cell<MaximizeButton>,
    /// Whether Windows tells the window when the pointer leaves it (`TrackMouseEvent`).
    tracking: Cell<bool>,
    /// Tells the page how to show its button.
    on_state: Box<dyn Fn(MaximizeButtonState)>,
}

thread_local! {
    /// Where the page placed its maximize button before the window behind it was there, by the
    /// address of the window it goes in. Only the main thread, which creates and places the
    /// button's windows, uses it.
    static PLACED_EARLY: RefCell<HashMap<usize, ButtonRect>> = RefCell::new(HashMap::new());
}

/// Puts the maximize button's window over `window`'s page, where the page placed its button, or
/// at no size until it does (`place_maximize_button`). `on_state` tells the page how to show its
/// button. Created on the window's own thread, since Windows only lets a window that belongs to
/// it answer for it.
///
/// Without it the window works as before, the page's button maximizes it, and only the snap
/// layouts flyout is missing, so a failure is left at that.
pub fn add_maximize_button(
    window: &WebviewWindow,
    on_state: impl Fn(MaximizeButtonState) + Send + 'static,
) {
    let Ok(parent) = window.hwnd() else {
        return;
    };
    // A window handle is an address that any thread may use, but a raw pointer isn't Send.
    let parent = parent.0.expose_provenance();
    let _ = window.run_on_main_thread(move || {
        create(
            std::ptr::with_exposed_provenance_mut(parent),
            Box::new(on_state),
        );
    });
}

fn create(parent: HWND, on_state: Box<dyn Fn(MaximizeButtonState)>) {
    // Taken out whether or not the window can be created, so nothing is left behind for a parent
    // that is gone.
    let rect = PLACED_EARLY
        .with_borrow_mut(|placed| placed.remove(&parent.addr()))
        .unwrap_or_default();
    // SAFETY: a null name asks for the module of the process's own executable.
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_procedure),
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        // SAFETY: the class's name is a static NUL-terminated string, and its procedure lives as
        // long as the process. A failure shows as CreateWindowExW failing below.
        unsafe { RegisterClassExW(&class) };
    });
    // SAFETY: the class and its name are registered above, `parent` is the window it goes into
    // (CreateWindowExW fails if it is gone), and the null name, menu and creation data are
    // allowed. The window draws nothing, as Tauri's resize border doesn't, so the page shows
    // through it.
    let button = unsafe {
        CreateWindowExW(
            0,
            CLASS_NAME,
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            rect.x,
            rect.y,
            rect.width.max(0),
            rect.height.max(0),
            parent,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if button.is_null() {
        return;
    }
    let state = Box::new(ButtonWindow {
        button: Cell::new(MaximizeButton::default()),
        tracking: Cell::new(false),
        on_state,
    });
    // SAFETY: `button` is the live window just created, which owns the state from now on and
    // frees it on WM_NCDESTROY. Its procedure reads null until then, and does nothing with it.
    unsafe { SetWindowLongPtrW(button, GWLP_USERDATA, Box::into_raw(state) as isize) };
    // SAFETY: plain calls on the live window. Above the page's webview, so the pointer over the
    // button reaches it.
    unsafe {
        SetWindowPos(
            button,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Moves the maximize button's window over the page's maximize button at `rect`, which the page
/// measured as it laid its title bar out.
///
/// A window opened off the main thread (a second launch, a link opened in a new window) gets its
/// button's window later, from the main thread's queue, and the page reports its button again only
/// when the title bar resizes. So placing runs on the main thread too, and a rectangle that comes
/// first is kept for `create`: whichever comes first, the flyout is there without the reader
/// resizing the window.
pub fn place_maximize_button(window: &WebviewWindow, rect: ButtonRect) {
    let Ok(parent) = window.hwnd() else {
        return;
    };
    // As in `add_maximize_button`.
    let parent = parent.0.expose_provenance();
    let _ = window.run_on_main_thread(move || {
        place(std::ptr::with_exposed_provenance_mut(parent), rect);
    });
}

fn place(parent: HWND, rect: ButtonRect) {
    // SAFETY: `parent` is the window's handle, and the null child and name search all its
    // children for one of the class. A parent that is gone has none.
    let button =
        unsafe { FindWindowExW(parent, std::ptr::null_mut(), CLASS_NAME, std::ptr::null()) };
    if button.is_null() {
        // SAFETY: a plain call; a handle that is gone answers false. A rectangle is kept only for
        // a window still there, so none is kept for a window that closed before its page did.
        if unsafe { IsWindow(parent) } != 0 {
            PLACED_EARLY.with_borrow_mut(|placed| placed.insert(parent.addr(), rect));
        }
        return;
    }
    // SAFETY: a plain call on the live window, on its own thread.
    unsafe {
        SetWindowPos(
            button,
            std::ptr::null_mut(),
            rect.x,
            rect.y,
            rect.width.max(0),
            rect.height.max(0),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the window's user data is null or the ButtonWindow `create` gave it, which lives
    // until WM_NCDESTROY below.
    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *mut ButtonWindow;
    if message == WM_NCDESTROY && !state.is_null() {
        // SAFETY: the window is going, and with it the only owner of its state; the user data is
        // cleared first, so nothing reads it again.
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, 0);
            drop(Box::from_raw(state));
        }
    } else if let Some(state) = {
        // SAFETY: as above; the state is only used on this thread, so a shared borrow is safe.
        unsafe { state.as_ref() }
    } && let Some(answer) = answer(window, state, message, wparam, lparam)
    {
        return answer;
    }
    // SAFETY: the arguments are the message's own.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// What the maximize button's window does with `message`, or None to leave it to Windows.
fn answer(
    window: HWND,
    state: &ButtonWindow,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    if message == WM_NCHITTEST {
        return Some(hit_test_window(window, lparam));
    }
    let pointer = Pointer::from_message(message, wparam)?;
    match pointer {
        Pointer::Moved => track_leaving(window, state),
        Pointer::Left => state.tracking.set(false),
        Pointer::Pressed | Pointer::Released => {}
    }
    // Copied out and back rather than borrowed, so a message that comes in while the page is told
    // finds the button as it is now and no borrow held.
    let mut button = state.button.get();
    let reaction = button.on(pointer);
    state.button.set(button);
    if reaction.toggle {
        toggle_maximized(window);
    }
    if let Some(look) = reaction.show {
        (state.on_state)(look);
    }
    (!pointer.is_left_to_windows()).then_some(0)
}

/// What the pointer does to the maximize button, as Windows tells its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pointer {
    Moved,
    Left,
    Pressed,
    Released,
}

impl Pointer {
    /// The pointer event `message` is, where `hit` (its `wparam`) is what Windows' hit test
    /// answered for the pointer, or None for any other message. A double-click is a press: Windows
    /// sends the second press of one as `WM_NCLBUTTONDBLCLK` in place of `WM_NCLBUTTONDOWN`.
    fn from_message(message: u32, hit: WPARAM) -> Option<Self> {
        let on_button = hit == HTMAXBUTTON as WPARAM;
        match message {
            WM_NCMOUSEMOVE if on_button => Some(Self::Moved),
            // Carries no hit test answer: the pointer has left the window.
            WM_NCMOUSELEAVE => Some(Self::Left),
            WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if on_button => Some(Self::Pressed),
            WM_NCLBUTTONUP if on_button => Some(Self::Released),
            _ => None,
        }
    }

    /// Whether Windows handles the message too. Hovering the button is what shows the snap
    /// layouts flyout, so moves go on to Windows; presses and releases don't, or Windows would
    /// track the press with a caption button of its own.
    fn is_left_to_windows(self) -> bool {
        matches!(self, Self::Moved | Self::Left)
    }
}

/// What the maximize button's window does about a `Pointer`.
#[derive(Debug, Default, PartialEq, Eq)]
struct Reaction {
    /// The look to show the page, only when it changed.
    show: Option<MaximizeButtonState>,
    /// Whether to maximize or restore the window.
    toggle: bool,
}

/// What the pointer has done to the maximize button so far, kept apart from Windows' messages so
/// that hovering, pressing and clicking it can be tested.
#[derive(Debug, Clone, Copy)]
struct MaximizeButton {
    /// The look the page was last told to show.
    shown: MaximizeButtonState,
    /// Whether a press started on the button and hasn't been released or left yet.
    pressed: bool,
}

impl Default for MaximizeButton {
    fn default() -> Self {
        Self {
            shown: MaximizeButtonState::Normal,
            pressed: false,
        }
    }
}

impl MaximizeButton {
    /// A click toggles the window only when the press started on the button and the pointer
    /// stayed on it: leaving forgets the press.
    fn on(&mut self, pointer: Pointer) -> Reaction {
        let mut toggle = false;
        let look = match pointer {
            Pointer::Moved if self.pressed => MaximizeButtonState::Pressed,
            Pointer::Moved => MaximizeButtonState::Hovered,
            Pointer::Left => {
                self.pressed = false;
                MaximizeButtonState::Normal
            }
            Pointer::Pressed => {
                self.pressed = true;
                MaximizeButtonState::Pressed
            }
            Pointer::Released => {
                toggle = std::mem::take(&mut self.pressed);
                MaximizeButtonState::Hovered
            }
        };
        let show = (self.shown != look).then_some(look);
        self.shown = look;
        Reaction { show, toggle }
    }
}

/// The answer to `WM_NCHITTEST` at the screen point in `lparam`, from where the window and its
/// parent are now.
fn hit_test_window(window: HWND, lparam: LPARAM) -> LRESULT {
    // SAFETY: a plain call on the live window.
    let parent = unsafe { GetParent(window) };
    let (Some(point), Some(rect)) = (
        to_client(parent, screen_point(lparam)),
        client_rect_of(window, parent),
    ) else {
        return HTTRANSPARENT as LRESULT;
    };
    hit_test((point.x, point.y), rect, resize_border(parent))
}

/// The screen point a mouse message carries in `lparam`: two signed 16-bit coordinates.
fn screen_point(lparam: LPARAM) -> POINT {
    let bits = lparam.cast_unsigned();
    POINT {
        x: i32::from((bits & 0xFFFF) as u16 as i16),
        y: i32::from(((bits >> 16) & 0xFFFF) as u16 as i16),
    }
}

/// `point` on the screen, in `parent`'s client coordinates.
fn to_client(parent: HWND, mut point: POINT) -> Option<POINT> {
    // SAFETY: `point` is a live POINT that ScreenToClient only reads and writes.
    (unsafe { ScreenToClient(parent, &mut point) } != 0).then_some(point)
}

/// Where `window` is, in `parent`'s client coordinates.
fn client_rect_of(window: HWND, parent: HWND) -> Option<ButtonRect> {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a live RECT that GetWindowRect only writes.
    if unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return None;
    }
    let top_left = to_client(
        parent,
        POINT {
            x: rect.left,
            y: rect.top,
        },
    )?;
    Some(ButtonRect {
        x: top_left.x,
        y: top_left.y,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}

/// How far the top resize border reaches into `window`: none while it is maximized, and
/// otherwise Windows' frame height at its DPI, as Tauri's resize border takes it.
fn resize_border(window: HWND) -> i32 {
    // SAFETY: plain calls on the live window.
    unsafe {
        if IsZoomed(window) != 0 {
            0
        } else {
            GetSystemMetricsForDpi(SM_CYFRAME, GetDpiForWindow(window))
        }
    }
}

/// Asks Windows to tell the window when the pointer leaves it, once per stay.
fn track_leaving(window: HWND, state: &ButtonWindow) {
    if state.tracking.replace(true) {
        return;
    }
    let mut track = TRACKMOUSEEVENT {
        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE | TME_NONCLIENT,
        hwndTrack: window,
        dwHoverTime: 0,
    };
    // SAFETY: `track` is a live, filled-in TRACKMOUSEEVENT for the live window.
    if unsafe { TrackMouseEvent(&mut track) } == 0 {
        state.tracking.set(false);
    }
}

/// Maximizes the window the button is in, or restores it if it is maximized, as Windows' own
/// maximize button does.
fn toggle_maximized(button: HWND) {
    // SAFETY: plain calls on live windows; the command goes through the window's own queue.
    unsafe {
        let parent = GetParent(button);
        let command = if IsZoomed(parent) != 0 {
            SC_RESTORE
        } else {
            SC_MAXIMIZE
        };
        PostMessageW(parent, WM_SYSCOMMAND, command as WPARAM, 0);
    }
}

/// What the maximize button's window answers Windows' hit test with at `point`, in the client
/// coordinates of the window it sits in. On the button it is the maximize button. Beside it, and
/// in the top `resize_border` pixels of a window that isn't maximized, it is transparent, so
/// Windows asks the window below on the same thread: there the top resize border is Tauri's, and
/// stays where Windows 11 has it. A maximized window has no resize border (0), so its maximize
/// button reaches the screen's top edge.
fn hit_test(point: (i32, i32), button: ButtonRect, resize_border: i32) -> LRESULT {
    if button.contains(point) && point.1 >= resize_border {
        HTMAXBUTTON as LRESULT
    } else {
        HTTRANSPARENT as LRESULT
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::UI::WindowsAndMessaging::HTCAPTION;

    use super::*;

    const MAXIMIZE: LRESULT = HTMAXBUTTON as LRESULT;
    const TRANSPARENT: LRESULT = HTTRANSPARENT as LRESULT;

    /// A maximize button at Windows 11's size, second from the right of a 1000 px wide window.
    const BUTTON: ButtonRect = ButtonRect {
        x: 908,
        y: 0,
        width: 46,
        height: 32,
    };

    #[test]
    fn pointer_on_the_button_is_on_maximize() {
        assert_eq!(hit_test((930, 16), BUTTON, 4), MAXIMIZE);
        assert_eq!(hit_test((908, 31), BUTTON, 4), MAXIMIZE);
    }

    #[test]
    fn pointer_beside_the_button_is_left_to_the_window_below() {
        for point in [(907, 16), (954, 16), (930, 32)] {
            assert_eq!(hit_test(point, BUTTON, 4), TRANSPARENT, "{point:?}");
        }
    }

    #[test]
    fn top_resize_border_of_a_restored_window_is_left_to_the_border() {
        assert_eq!(hit_test((930, 0), BUTTON, 4), TRANSPARENT);
        assert_eq!(hit_test((930, 3), BUTTON, 4), TRANSPARENT);
        assert_eq!(hit_test((930, 4), BUTTON, 4), MAXIMIZE);
    }

    #[test]
    fn top_edge_of_a_maximized_window_is_on_maximize() {
        assert_eq!(hit_test((930, 0), BUTTON, 0), MAXIMIZE);
    }

    #[test]
    fn button_the_page_has_not_placed_yet_is_nowhere() {
        assert_eq!(hit_test((0, 0), ButtonRect::default(), 0), TRANSPARENT);
    }

    fn shows(look: MaximizeButtonState) -> Reaction {
        Reaction {
            show: Some(look),
            toggle: false,
        }
    }

    #[test]
    fn hovering_the_button_shows_it_hovered_once() {
        let mut button = MaximizeButton::default();
        assert_eq!(
            button.on(Pointer::Moved),
            shows(MaximizeButtonState::Hovered)
        );
        assert_eq!(button.on(Pointer::Moved), Reaction::default());
    }

    #[test]
    fn pressing_the_button_shows_it_pressed() {
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        assert_eq!(
            button.on(Pointer::Pressed),
            shows(MaximizeButtonState::Pressed)
        );
    }

    #[test]
    fn release_after_a_press_toggles_the_window_and_shows_the_button_hovered() {
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        button.on(Pointer::Pressed);
        assert_eq!(
            button.on(Pointer::Released),
            Reaction {
                show: Some(MaximizeButtonState::Hovered),
                toggle: true,
            }
        );
        // The press is used up: a second release toggles nothing.
        assert_eq!(button.on(Pointer::Released), Reaction::default());
    }

    #[test]
    fn release_without_a_press_on_the_button_does_not_toggle_the_window() {
        // A press that started beside the button and was dragged onto it.
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        assert_eq!(button.on(Pointer::Released), Reaction::default());
    }

    #[test]
    fn leaving_the_button_shows_it_normal_and_forgets_the_press() {
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        button.on(Pointer::Pressed);
        assert_eq!(button.on(Pointer::Left), shows(MaximizeButtonState::Normal));
        assert_eq!(
            button.on(Pointer::Moved),
            shows(MaximizeButtonState::Hovered)
        );
    }

    #[test]
    fn press_that_left_the_button_and_came_back_does_not_toggle_the_window() {
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        button.on(Pointer::Pressed);
        button.on(Pointer::Left);
        button.on(Pointer::Moved);
        assert_eq!(button.on(Pointer::Released), Reaction::default());
    }

    #[test]
    fn moving_over_a_pressed_button_keeps_it_pressed() {
        let mut button = MaximizeButton::default();
        button.on(Pointer::Moved);
        button.on(Pointer::Pressed);
        assert_eq!(button.on(Pointer::Moved), Reaction::default());
    }

    const ON_BUTTON: WPARAM = HTMAXBUTTON as WPARAM;

    #[test]
    fn double_click_on_the_button_counts_as_a_press() {
        assert_eq!(
            Pointer::from_message(WM_NCLBUTTONDBLCLK, ON_BUTTON),
            Some(Pointer::Pressed)
        );
    }

    #[test]
    fn double_click_on_the_button_toggles_the_window_twice() {
        // Windows sends the second press of a double-click as WM_NCLBUTTONDBLCLK.
        let mut button = MaximizeButton::default();
        let toggles = [
            WM_NCLBUTTONDOWN,
            WM_NCLBUTTONUP,
            WM_NCLBUTTONDBLCLK,
            WM_NCLBUTTONUP,
        ]
        .into_iter()
        .filter_map(|message| Pointer::from_message(message, ON_BUTTON))
        .filter(|&pointer| button.on(pointer).toggle)
        .count();
        assert_eq!(toggles, 2);
    }

    #[test]
    fn mouse_beside_the_button_is_not_the_buttons() {
        let beside = HTCAPTION as WPARAM;
        for message in [
            WM_NCMOUSEMOVE,
            WM_NCLBUTTONDOWN,
            WM_NCLBUTTONDBLCLK,
            WM_NCLBUTTONUP,
        ] {
            assert_eq!(Pointer::from_message(message, beside), None, "{message}");
        }
        assert_eq!(Pointer::from_message(WM_NCHITTEST, ON_BUTTON), None);
    }

    #[test]
    fn leaving_is_the_buttons_wherever_windows_says_the_pointer_is() {
        assert_eq!(
            Pointer::from_message(WM_NCMOUSELEAVE, 0),
            Some(Pointer::Left)
        );
    }

    #[test]
    fn moving_and_leaving_are_left_to_windows_but_pressing_and_releasing_are_not() {
        assert!(Pointer::Moved.is_left_to_windows());
        assert!(Pointer::Left.is_left_to_windows());
        assert!(!Pointer::Pressed.is_left_to_windows());
        assert!(!Pointer::Released.is_left_to_windows());
    }

    #[test]
    fn mouse_message_point_keeps_negative_coordinates() {
        // A second monitor left of the first has negative x.
        let lparam = (0x0010_FFF6_u32).cast_signed() as LPARAM;
        let point = screen_point(lparam);
        assert_eq!((point.x, point.y), (-10, 16));
    }
}
