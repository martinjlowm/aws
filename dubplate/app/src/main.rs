// A `view!` is one nested tuple per element, and the trait solver walks the whole
// nest. The default limit of 128 is reached somewhere inside `tachys`, and the
// error names its own generated types rather than anything here, so this is set
// once at the crate root instead of by breaking components up to appease it.
#![recursion_limit = "1024"]

//! Dubplate, in a tab.

mod bridge;
mod components;
mod options;
mod report;
mod run;

mod app;

fn main() {
    // Without this a Rust panic reaches the console as `unreachable executed`
    // and nothing else.
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
