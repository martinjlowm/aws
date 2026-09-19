//! Where the music comes in.
//!
//! A drop target and a file picker over the same handler, because half of the
//! people who use this will drag the zip out of Downloads and the other half
//! will not think to. Several at once either way. A release bought in three
//! orders arrives as three zips, and they are one image.
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
use leptos_shadcn_ui::{Button, ButtonSize, Card, CardContent};
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, FileList, HtmlInputElement};

#[component]
pub fn DropZone(
    /// Called with everything dropped at once, in the order it was given.
    #[prop(into)]
    on_files: Callback<Vec<File>>,
    #[prop(into)] busy: Signal<bool>,
    /// Whether anything has been dropped yet. The copy is different once it has:
    /// the next archive adds to the run rather than starting one.
    #[prop(into)]
    appending: Signal<bool>,
) -> impl IntoView {
    let over = RwSignal::new(false);
    let input: NodeRef<html::Input> = NodeRef::new();

    let on_drop = move |event: DragEvent| {
        event.prevent_default();
        over.set(false);
        if busy.get() {
            return;
        }
        if let Some(files) = event.data_transfer().and_then(|transfer| transfer.files()) {
            let files = collect(&files);
            if !files.is_empty() {
                on_files.run(files);
            }
        }
    };

    view! {
        <Card
            class="transition-colors"
            style=Signal::derive(move || {
                if over.get() {
                    "border-color: var(--accent-primary); background: var(--surface-accent-soft)"
                        .to_string()
                } else {
                    String::new()
                }
                    .into()
            })
        >
            <div
                on:dragover=move |event: DragEvent| {
                    event.prevent_default();
                    over.set(true);
                }
                on:dragleave=move |_| over.set(false)
                on:drop=on_drop
            >
                <CardContent class="px-8 pt-12 pb-12 text-center">
                    <input
                        node_ref=input
                        type="file"
                        accept=".zip,.wav,.aiff,.aif,.flac,.mp3,.mp4,.m4a,.m4b,.mov,.webm,.mka,.ogg,.oga,application/zip,audio/*,video/mp4,video/webm"
                        multiple=true
                        class="hidden"
                        on:change=move |event: ev::Event| {
                            let element: HtmlInputElement = event
                                .target()
                                .unwrap()
                                .unchecked_into();
                            if let Some(files) = element.files() {
                                let files = collect(&files);
                                if !files.is_empty() {
                                    on_files.run(files);
                                }
                            }
                            // Cleared, so choosing the same archive twice fires
                            // again: that is what a rerun after a settings
                            // change is.
                            element.set_value("");
                        }
                    />

                    <Show
                        when=move || appending.get()
                        fallback=|| {
                            view! {
                                <SectionHeading eyebrow="Step one" title="Drop your tracks" />
                            }
                        }
                    >
                        <SectionHeading eyebrow="Step one" title="Drop some more" />
                    </Show>
                    <p
                        class="mx-auto mt-4 max-w-prose text-[length:var(--text-body-md)]"
                        style="color: var(--text-secondary)"
                    >
                        {move || {
                            if appending.get() {
                                "Zips or loose tracks. They are measured and added to the ones
                                 below, and one image is built from all of them, minus whatever
                                 you untick."
                            } else {
                                "A zip straight from Beatport, Bandcamp, Juno or Traxsource, or
                                 loose tracks from anywhere. WAV, AIFF, FLAC and MP3, and the sound
                                 out of an MP4 or an M4A when the only copy you have is a video. As
                                 many at once as you like. Everything is measured in this tab. No
                                 byte of it reaches a server, and there is no server to reach."
                            }
                        }}
                    </p>

                    <div class="mt-7">
                        <Button
                            size=ButtonSize::Lg
                            disabled=busy
                            on_click=Callback::new(move |()| {
                                if let Some(element) = input.get() {
                                    element.click();
                                }
                            })
                        >
                            {move || {
                                if appending.get() {
                                    "Add more"
                                } else {
                                    "Choose files"
                                }
                            }}
                        </Button>
                    </div>
                </CardContent>
            </div>
        </Card>
    }
}

/// A `FileList` as a `Vec`, which it is not: it is a live index with a length.
fn collect(files: &FileList) -> Vec<File> {
    (0..files.length())
        .filter_map(|index| files.get(index))
        .collect()
}
