//! The tests' adapter of both of the documents module's ports, [`Pages`] and [`Windows`]: it keeps
//! every message a page is sent and everything done to a window, in order, so a whole show is
//! tested without a webview (ticket 28).

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::{Documents, NewWindow, WindowError, Windows};
use crate::message::{Message, Pages};

/// A message as a [`Recorder`] keeps it: the window it went to, and the message.
pub type Sent = (String, Message);

/// Something done to a window, as a [`Recorder`] keeps it with the window's label ([`Done`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToWindow {
    /// The window opened, as this says.
    Opened(NewWindow),
    /// The window was titled this.
    Titled(String),
    BroughtToFront,
}

/// Something done to a window as a [`Recorder`] keeps it: the window's label, and what was done.
pub type Done = (String, ToWindow);

/// The documents module with no window open yet, whose pages and windows `Recorder` keeps.
pub fn recorded() -> (Documents, Recorder) {
    let recorder = Recorder::default();
    (Documents::new(recorder.clone(), recorder.clone()), recorder)
}

/// Keeps every message sent and everything done to a window, in order, for a test to check. Its
/// clones share what they keep.
#[derive(Clone, Default)]
pub struct Recorder {
    sent: Arc<(Mutex<Vec<Sent>>, Condvar)>,
    done: Arc<Mutex<Vec<Done>>>,
    /// Windows whose pages can't be given a message, as a webview that fails to run an event.
    failing_pages: Arc<Mutex<Vec<String>>>,
    /// Windows that can't be opened, titled or brought to the front.
    failing_windows: Arc<Mutex<Vec<String>>>,
    /// What runs the next time a window is titled, before the title is set.
    before_next_title: Arc<Mutex<Option<BeforeTitle>>>,
}

/// What another thread does while a window is being titled, for [`Recorder::before_next_title`].
type BeforeTitle = Box<dyn FnOnce() -> Result<(), WindowError> + Send>;

impl Recorder {
    /// The next time any window is titled, `meanwhile` runs first, as what another thread does
    /// while the title is being set, and the title fails, and isn't kept, if `meanwhile` fails.
    /// So a test puts a second show into the middle of a first one at a moment of its choosing.
    pub fn before_next_title(
        &self,
        meanwhile: impl FnOnce() -> Result<(), WindowError> + Send + 'static,
    ) {
        *lock(&self.before_next_title) = Some(Box::new(meanwhile));
    }

    /// From now on, every message to `window`'s page fails to send, and isn't kept.
    pub fn fail_sends_to(&self, window: &str) {
        lock(&self.failing_pages).push(window.to_owned());
    }

    /// From now on, opening, titling or bringing `window` to the front fails, and isn't kept.
    /// Labels are `document-0`, `document-1` and so on, so a test can name a window before it
    /// opens.
    pub fn fail_window(&self, window: &str) {
        lock(&self.failing_windows).push(window.to_owned());
    }

    /// Every message sent so far.
    pub fn sent(&self) -> Vec<Sent> {
        lock(&self.sent.0).clone()
    }

    /// Everything done to a window so far.
    pub fn done(&self) -> Vec<Done> {
        lock(&self.done).clone()
    }

    /// Waits until the messages sent so far are `done`, or `timeout` has passed, and returns
    /// every message sent by then. Messages sent from a watch's thread arrive in their own time.
    pub fn wait_until(&self, timeout: Duration, done: impl Fn(&[Sent]) -> bool) -> Vec<Sent> {
        let (sent, arrived) = &*self.sent;
        let (sent, _) = arrived
            .wait_timeout_while(lock(sent), timeout, |sent| !done(sent))
            .unwrap_or_else(PoisonError::into_inner);
        sent.clone()
    }

    fn do_to(&self, window: &str, what: ToWindow) -> Result<(), WindowError> {
        if fails(&self.failing_windows, window) {
            return Err(WindowError::new("the window can't be changed"));
        }
        lock(&self.done).push((window.to_owned(), what));
        Ok(())
    }
}

impl Pages for Recorder {
    fn send(&self, window: &str, message: Message) -> Result<(), WindowError> {
        if fails(&self.failing_pages, window) {
            return Err(WindowError::new("the page can't be given the message"));
        }
        lock(&self.sent.0).push((window.to_owned(), message));
        self.sent.1.notify_all();
        Ok(())
    }
}

impl Windows for Recorder {
    fn open(&self, window: &str, new_window: &NewWindow) -> Result<(), WindowError> {
        self.do_to(window, ToWindow::Opened(new_window.clone()))
    }

    fn set_title(&self, window: &str, title: &str) -> Result<(), WindowError> {
        // Taken out of its lock before it runs, since what it does may title a window too.
        let meanwhile = lock(&self.before_next_title).take();
        if let Some(meanwhile) = meanwhile {
            meanwhile()?;
        }
        self.do_to(window, ToWindow::Titled(title.to_owned()))
    }

    fn bring_to_front(&self, window: &str) -> Result<(), WindowError> {
        self.do_to(window, ToWindow::BroughtToFront)
    }
}

fn fails(failing: &Mutex<Vec<String>>, window: &str) -> bool {
    lock(failing).iter().any(|failing| failing == window)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
