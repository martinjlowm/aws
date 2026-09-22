//! What the form remembers between visits.
//!
//! The browser's own storage rather than a server, for the reason everything
//! else here is: there is no server. A reload is a new page and a new set of
//! workers, and the numbers somebody settled on last week are not something to
//! make them settle on again.
//!
//! Only a deviation is kept. Nothing under a key means the defaults, so a form
//! put back to them stores nothing, and a later revision of the analysis whose
//! defaults have moved moves this form with them rather than being pinned to a
//! copy of the old ones taken on somebody's first visit. It is also what makes
//! the reset button a flush: it puts the form back to the defaults, and a form
//! at the defaults has nothing to store.
//!
//! Every call here fails quietly. Storage is refused outright in some private
//! windows and full in others, and a preference that cannot be written is not a
//! reason for a page that measures audio to stop.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The analysis settings, as the form last had them.
pub const ANALYSIS: &str = "dubplate.analysis";

/// What goes on the stick, as the form last had it.
pub const DEVICE: &str = "dubplate.device";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// What was stored under a key, if it still reads.
///
/// A value written by a version whose shape has since changed comes back as
/// absent rather than as an error. The defaults are always a valid answer, and a
/// page that will not load because of a stale preference is worse than one that
/// forgets a preference.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    let raw = storage()?.get_item(key).ok()??;
    serde_json::from_str(&raw).ok()
}

pub fn save<T: Serialize>(key: &str, value: &T) {
    let Some(storage) = storage() else {
        return;
    };
    if let Ok(raw) = serde_json::to_string(value) {
        let _ = storage.set_item(key, &raw);
    }
}

pub fn forget(key: &str) {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(key);
    }
}
