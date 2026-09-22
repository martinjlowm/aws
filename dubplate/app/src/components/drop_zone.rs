//! Where the music comes in.
//!
//! The whole window is the target. A zip dragged out of Downloads lands wherever
//! it is let go, including over the table of tracks already measured and over
//! the settings, because by the time somebody drops a second archive the card
//! that invited the first one is off the top of the screen.
//!
//! The file picker is still here, behind a quiet button. Dragging is what a
//! pointer does and there is nothing else a keyboard can do, so removing it
//! would close the door on anybody not holding a mouse.
//!
//! Zips and loose tracks come through the same door. Sorting one from the other
//! is a question about a file name, and run.rs asks it.
//!
//! The `File` is handed on rather than its bytes. Reading a gigabyte here would
//! hold it for as long as the page does, and the handle the browser gives is
//! backed by the file on disk; `run.rs` reads it when it needs it and lets it go
//! afterwards.

use crate::components::section::SectionHeading;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_shadcn_ui::{Button, ButtonSize, ButtonVariant, Card, CardContent};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DragEvent, File, FileList, HtmlInputElement};

/// Take a drop anywhere on the page, and say while one is being offered.
///
/// Listened for on the window rather than on the card, so there is one handler
/// and one place a dropped file can enter the run. A card that listened as well
/// would take the same drop a second time on the way past, and the archive would
/// be added twice.
///
/// The signal it hands back is what the page draws the invitation from.
pub fn watch_window_drops(on_files: Callback<Vec<File>>, busy: Signal<bool>) -> Signal<bool> {
    // Counted rather than set. Both events fire on every element the pointer
    // crosses and both bubble to the window, so dragging from a card onto the
    // page behind it is a leave followed by an enter, and a flag cleared by the
    // leave alone goes false with the file still overhead.
    let depth = RwSignal::new(0i32);

    let _ = window_event_listener(ev::dragenter, move |event: DragEvent| {
        if !carries_files(&event) {
            return;
        }
        event.prevent_default();
        depth.update(|depth| *depth += 1);
    });

    // A default not prevented on every single dragover is the browser taking the
    // drop as a navigation: the page is replaced by the file, and the run with
    // it.
    let _ = window_event_listener(ev::dragover, move |event: DragEvent| {
        if carries_files(&event) {
            event.prevent_default();
        }
    });

    let _ = window_event_listener(ev::dragleave, move |event: DragEvent| {
        if !carries_files(&event) {
            return;
        }
        depth.update(|depth| *depth = (*depth - 1).max(0));
    });

    let _ = window_event_listener(ev::drop, move |event: DragEvent| {
        if !carries_files(&event) {
            return;
        }
        event.prevent_default();
        depth.set(0);
        if busy.get_untracked() {
            return;
        }
        let Some(files) = event
            .data_transfer()
            .and_then(|transfer| transfer.files())
            .map(|files| collect(&files))
        else {
            return;
        };
        if !files.is_empty() {
            on_files.run(files);
        }
    });

    Signal::derive(move || depth.get() > 0)
}

/// Whether what is being dragged is files at all.
///
/// A drag carries what it is before it carries what it holds: `types` is
/// readable while the pointer is moving and the files themselves are not, which
/// is what lets text selected in another tab pass over the page without the
/// invitation appearing.
fn carries_files(event: &DragEvent) -> bool {
    event
        .data_transfer()
        .is_some_and(|transfer| transfer.types().includes(&JsValue::from_str("Files"), 0))
}

/// Said over the whole page, while a file is over the whole page.
///
/// `pointer-events: none`, so this is a thing to look at and never a thing in
/// the way: the drop is taken by the window underneath it.
#[component]
pub fn DragVeil(#[prop(into)] dragging: Signal<bool>) -> impl IntoView {
    view! {
        <Show when=move || dragging.get()>
            <div
                class="animate-rise fixed inset-0 z-50 flex items-center justify-center p-10 text-center"
                style="background: var(--overlay-scrim); pointer-events: none"
                aria-hidden="true"
            >
                <p
                    class="section-title"
                    style="color: var(--sand-050); font-family: var(--font-display)"
                >
                    "Let go anywhere"
                </p>
            </div>
        </Show>
    }
}

#[component]
pub fn DropZone(
    /// Called with everything chosen through the picker, in the order it was
    /// given. A drop reaches the run through the window listener instead.
    #[prop(into)]
    on_files: Callback<Vec<File>>,
    #[prop(into)] busy: Signal<bool>,
    /// Whether a file is over the page. The card is the invitation, so it is
    /// what lights up, wherever the pointer happens to be.
    #[prop(into)]
    dragging: Signal<bool>,
    /// Whether anything has been dropped yet. The copy is different once it has:
    /// the next archive adds to the run rather than starting one.
    #[prop(into)]
    appending: Signal<bool>,
) -> impl IntoView {
    let input: NodeRef<html::Input> = NodeRef::new();

    view! {
        <Card
            class="transition-colors"
            style=Signal::derive(move || {
                if dragging.get() {
                    "border-color: var(--accent-primary); background: var(--surface-accent-soft)"
                        .to_string()
                } else {
                    String::new()
                }
                    .into()
            })
        >
            <CardContent class="px-8 pt-12 pb-12 text-center">
                <input
                    node_ref=input
                    type="file"
                    accept=".zip,.wav,.aiff,.aif,.flac,.mp3,.mp4,.m4a,.m4b,.mov,.webm,.mka,.ogg,.oga,application/zip,audio/*,video/mp4,video/webm"
                    multiple=true
                    class="hidden"
                    on:change=move |event: ev::Event| {
                        let element: HtmlInputElement = event.target().unwrap().unchecked_into();
                        if let Some(files) = element.files() {
                            let files = collect(&files);
                            if !files.is_empty() {
                                on_files.run(files);
                            }
                        }
                        // Cleared, so choosing the same archive twice fires
                        // again: that is what a rerun after a settings change
                        // is.
                        element.set_value("");
                    }
                />

                // What this is, said where a person is looking. It was the
                // page's subtitle until the header became a bar, and a heading
                // over the drop target is the last place a promise about the
                // drop can go.
                //
                // Here whether or not anything has been dropped. It is the only
                // place the page says what it does, and a second visitor
                // reading over a shoulder arrives after the first drop rather
                // than before it. Only the heading changes.
                <Show
                    when=move || appending.get()
                    fallback=|| {
                        view! { <SectionHeading eyebrow="The music" title="Drop your tracks" /> }
                    }
                >
                    <SectionHeading eyebrow="The music" title="Drop some more" />
                </Show>

                <p
                    class="mx-auto mt-4 max-w-prose text-[length:var(--text-body-md)]"
                    style="color: var(--text-secondary)"
                >
                    "Drag a zip straight from Beatport, Bandcamp, Juno or Traxsource onto this
                     page, or loose tracks from anywhere. Every track comes back with its BPM and
                     its key, renamed so the stick sorts itself by tempo, and the whole lot writes
                     to a USB a Pioneer CDJ or a Denon player reads the moment you plug it in."
                </p>
                <p
                    class="mx-auto mt-3 max-w-prose text-[length:var(--text-body-md)]"
                    style="color: var(--text-secondary)"
                >
                    "WAV, AIFF, FLAC and MP3, and the sound out of an MP4 or an M4A when the only
                     copy you have is a video. As many at once as you like, and a second drop adds
                     to the first rather than replacing it. Both libraries go on the same stick,
                     rekordbox for the Pioneers and Engine DJ for the Denons, so it works on
                     whatever is in the booth."
                </p>
                <p
                    class="mx-auto mt-3 max-w-prose text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted)"
                >
                    "Everything is measured in this tab. No byte of it reaches a server, and there
                     is no server to reach."
                </p>

                // The picker, kept quiet. Dragging is the way in and this is the
                // way in for anybody whose hands are on the keyboard.
                <div class="mt-7">
                    <Button
                        variant=ButtonVariant::Outline
                        size=ButtonSize::Sm
                        disabled=busy
                        on_click=Callback::new(move |()| {
                            if let Some(element) = input.get() {
                                element.click();
                            }
                        })
                    >
                        "Or choose files"
                    </Button>
                </div>
            </CardContent>
        </Card>
    }
}

/// A `FileList` as a `Vec`, which it is not: it is a live index with a length.
fn collect(files: &FileList) -> Vec<File> {
    (0..files.length())
        .filter_map(|index| files.get(index))
        .collect()
}
