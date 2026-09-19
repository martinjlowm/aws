//! What a run was built from.
//!
//! One row per zip, saying what it contributed rather than what it is. A name
//! and a size say nothing a person can act on, and "nine tracks, seven on the
//! image" says where the other two went. Dropping a second archive that repeats
//! the first is the case this card exists for, and it is the case where the two
//! numbers differ.
//!
//! Loose tracks are one row between them rather than one row each. A row here
//! earns its place by being something you would remove as a unit, and a track
//! dropped on its own is already a line in the table with a tick of its own.

use crate::components::section::SectionHeading;
use crate::run::{Kind, Run, Standing};
use leptos::prelude::*;
use leptos_shadcn_ui::{Button, ButtonSize, ButtonVariant, Card, CardContent, CardHeader};

#[component]
pub fn Sources(run: Run, #[prop(into)] busy: Signal<bool>) -> impl IntoView {
    view! {
        <Card>
            <CardHeader class="px-7 pt-7 pb-0">
                <SectionHeading eyebrow="The sources" title="What this image is made of" />
            </CardHeader>

            <CardContent class="px-7 pt-5 pb-6">
                <ul class="space-y-3">
                    <For
                        each=move || {
                            run.sources
                                .get()
                                .into_iter()
                                .filter(|source| source.kind == Kind::Archive)
                                .collect::<Vec<_>>()
                        }
                        key=|source| source.id
                        let(source)
                    >
                        {
                            let id = source.id;
                            view! {
                                <Row
                                    run=run
                                    busy=busy
                                    name=source.name.clone()
                                    // Counted against this archive rather than
                                    // the run, which is the number that answers
                                    // "what did the second zip actually add".
                                    counts=Signal::derive(move || count(run, move |at| at == id))
                                    on_remove=Callback::new(move |()| run.remove_source(id))
                                />
                            }
                        }
                    </For>

                    <Show when=move || { run.loose() > 0 }>
                        <Row
                            run=run
                            busy=busy
                            name=Signal::derive(move || {
                                let loose = run.loose();
                                if loose == 1 {
                                    "One track added directly".to_string()
                                } else {
                                    format!("{loose} tracks added directly")
                                }
                            })
                            counts=Signal::derive(move || {
                                count(run, move |at| is_loose(run, at))
                            })
                            on_remove=Callback::new(move |()| run.remove_loose())
                        />
                    </Show>
                </ul>
            </CardContent>
        </Card>
    }
}

#[component]
fn Row(
    run: Run,
    #[prop(into)] busy: Signal<bool>,
    #[prop(into)] name: Signal<String>,
    /// Tracks contributed, and how many of them reach the image.
    #[prop(into)]
    counts: Signal<(usize, usize)>,
    #[prop(into)] on_remove: Callback<()>,
) -> impl IntoView {
    let _ = run;
    view! {
        <li class="flex items-baseline justify-between gap-6">
            <span class="min-w-0">
                <span class="block truncate">{move || name.get()}</span>
                <span class="label mt-1 block">
                    {move || {
                        let (total, kept) = counts.get();
                        if total == kept {
                            format!("{total} tracks, all on the image")
                        } else {
                            format!("{total} tracks, {kept} on the image")
                        }
                    }}
                </span>
            </span>
            <Button
                variant=ButtonVariant::Ghost
                size=ButtonSize::Sm
                class="label shrink-0"
                disabled=busy
                on_click=on_remove
            >
                "Remove"
            </Button>
        </li>
    }
}

/// Tracks from the sources a predicate accepts, and how many reach the image.
fn count(run: Run, mine: impl Fn(u32) -> bool) -> (usize, usize) {
    let tracks = run.tracks.get();
    let counted: Vec<Standing> = tracks
        .iter()
        .zip(run.standing())
        .filter(|(track, _)| mine(track.source))
        .map(|(_, standing)| standing)
        .collect();
    (
        counted.len(),
        counted
            .iter()
            .filter(|standing| **standing == Standing::Kept)
            .count(),
    )
}

fn is_loose(run: Run, id: u32) -> bool {
    run.sources
        .get()
        .iter()
        .any(|source| source.id == id && source.kind == Kind::Loose)
}
