//! The page.

use crate::bridge::Pool;
use crate::components::charts::{CamelotWheel, TempoKeyPlot};
use crate::components::drop_zone::DropZone;
use crate::components::section::SectionHeading;
use crate::components::settings::Settings;
use crate::components::sources::Sources;
use crate::components::tracks::Tracks;
use crate::run::{
    Phase, Run, append_all, build_image, human_bytes, read_capabilities, read_defaults,
};
use leptos::prelude::*;
use leptos_shadcn_ui::{
    Badge, BadgeVariant, Button, ButtonSize, Card, CardContent, CardDescription, CardHeader,
    Progress,
};

#[component]
pub fn App() -> impl IntoView {
    let run = Run::new();

    // The pool holds `Worker` handles, which are neither `Send` nor `Sync`, and
    // every shadcn callback is. A thread-local stored value is the join between
    // them: the handle it hands out is `Copy` and `Send`, and the value it holds
    // never leaves this thread. There is only ever one thread here anyway.
    let pool = StoredValue::new_local(Pool::new().ok());
    let started = pool.with_value(|pool| pool.is_some());

    // What this build can write is asked once, before anything is dropped, so
    // the settings form is right the first time it is opened.
    if let Some(ready) = pool.with_value(Clone::clone) {
        leptos::task::spawn_local(read_capabilities(run, ready.clone()));
        leptos::task::spawn_local(read_defaults(run, ready));
    }

    // An image describes the settings it was built from, and a finished one is
    // still behind a download link when they change. Watched as a whole rather
    // than control by control, so a setting added later is covered by having
    // been added.
    Effect::new(move |_| {
        run.device.track();
        run.invalidate();
    });

    let busy = Signal::derive(move || {
        matches!(
            run.phase.get(),
            Phase::Reading | Phase::Analysing | Phase::Building { .. }
        )
    });

    let on_files = Callback::new(move |files: Vec<web_sys::File>| {
        let Some(pool) = pool.with_value(Clone::clone) else {
            return;
        };
        leptos::task::spawn_local(append_all(run, pool, files));
    });

    let on_build = Callback::new(move |()| {
        let Some(pool) = pool.with_value(Clone::clone) else {
            return;
        };
        leptos::task::spawn_local(build_image(run, pool));
    });

    view! {
        <div class="mx-auto w-full max-w-[var(--container-lg)] px-6 py-12 md:px-10 md:py-16">
            <Header />

            <Show
                when=move || started
                fallback=|| {
                    view! {
                        <Card class="mt-12">
                            // Top padding is `pt-` and never `py-`. CardContent
                            // carries `p-6 pt-0`, and Tailwind emits `pt-0`
                            // after every `py-` rule, so a `py-` here loses the
                            // cascade on padding-top alone and the first line
                            // sits against the card edge.
                            <CardContent class="px-7 pt-6 pb-6">
                                <p style="color: var(--status-danger)">
                                    "This browser would not start a worker, so there is nowhere to
                                     run the analysis. A recent Chrome, Firefox or Safari will."
                                </p>
                            </CardContent>
                        </Card>
                    }
                }
            >
                <main class="mt-12 space-y-8">
                    <DropZone
                        on_files=on_files
                        busy=busy
                        appending=Signal::derive(move || !run.sources.get().is_empty())
                    />

                    <Show when=move || { !run.sources.get().is_empty() }>
                        <Sources run=run busy=busy />
                    </Show>

                    <Settings
                        analysis=run.analysis
                        defaults=Signal::derive(move || run.defaults.get())
                        device=run.device
                        formats=Signal::derive(move || run.formats.get())
                    />

                    <Show when=move || { !run.tracks.with(|tracks| tracks.is_empty()) }>
                        <Runway run=run />
                        <Tracks run=run />
                    </Show>

                    <Show when=move || { run.measured() > 1 }>
                        <Collection run=run />
                    </Show>

                    <Show when=move || {
                        matches!(
                            run.phase.get(),
                            Phase::Measured | Phase::Building { .. } | Phase::Ready { .. },
                        ) && run.measured() > 0
                    }>
                        <Download run=run on_build=on_build />
                    </Show>

                    {move || match run.phase.get() {
                        Phase::Failed(why) => {
                            view! {
                                <Card>
                                    <CardContent class="px-7 pt-6 pb-6">
                                        <p style="color: var(--status-danger)">{why}</p>
                                    </CardContent>
                                </Card>
                            }
                                .into_any()
                        }
                        _ => ().into_any(),
                    }}
                </main>
            </Show>

            <Footer />
        </div>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header>
            <h1 class="text-[length:var(--text-display-md)]">"Dubplate"</h1>
            <p
                class="mt-4 text-[length:var(--text-body-lg)]"
                style="color: var(--text-secondary)"
            >
                "Drop in the zips from your orders. Every track comes back with its BPM and
                 its key, renamed so the stick sorts itself by tempo, and the whole lot
                 writes to a USB a Pioneer CDJ or a Denon player reads the moment you plug
                 it in."
            </p>
            <p
                class="mt-3 text-[length:var(--text-body-md)]"
                style="color: var(--text-muted)"
            >
                "Both libraries go on the same stick, rekordbox for the Pioneers and Engine
                 DJ for the Denons, so it works on whatever is in the booth. Nothing is
                 uploaded. Every track is measured here, in this tab."
            </p>
        </header>
    }
}

/// How far along the run is.
#[component]
fn Runway(run: Run) -> impl IntoView {
    let total = move || run.tracks.with(|tracks| tracks.len());
    let settled = move || run.measured() + run.failed();

    view! {
        <Card>
            // A header and a body, which is the pairing CardContent is built
            // for: its own class is `p-6 pt-0`, so used alone it has no top
            // padding at all and the first line sits against the card's edge.
            <CardHeader class="px-7 pt-7 pb-0">
                <div class="flex items-baseline justify-between gap-6">
                    <span class="label">
                        {move || match run.phase.get() {
                            Phase::Reading => "Reading the archive",
                            Phase::Analysing => "Measuring",
                            Phase::Building { .. } => "Building the image",
                            _ => "Measured",
                        }}
                    </span>
                    <span class="figure text-[length:var(--text-body-sm)]">
                        {move || format!("{} of {}", settled(), total())}
                        {move || {
                            let failed = run.failed();
                            (failed > 0).then(|| format!(", {failed} refused"))
                        }}
                    </span>
                </div>
            </CardHeader>

            <CardContent class="px-7 pb-6">
                <div class="mt-4">
                    // Given as a percentage, with the maximum left at its
                    // default. The component reads `max` once in its own body
                    // rather than inside a closure, so a maximum that grows as
                    // rows arrive is read when the first row does and never
                    // again: the bar would fill against a denominator of one.
                    // Only `value` is tracked, so only `value` may move.
                    <Progress
                        value=Signal::derive(move || {
                            settled() as f64 / total().max(1) as f64 * 100.0
                        })
                        animated=Signal::derive(move || {
                            matches!(
                                run.phase.get(),
                                Phase::Analysing | Phase::Reading | Phase::Building { .. },
                            )
                        })
                    />
                </div>

                <p
                    class="mt-4 text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted)"
                >
                    {move || {
                        format!(
                            "{} at a time, one per core this machine will spare.",
                            crate::bridge::worker_count(),
                        )
                    }}
                </p>
            </CardContent>
        </Card>
    }
}

#[component]
fn Collection(run: Run) -> impl IntoView {
    let reports = Signal::derive(move || run.reports());
    view! {
        <Card>
            <CardHeader class="px-7 pt-7">
                <SectionHeading eyebrow="The collection" title="What you have dropped" />
                <CardDescription class="mt-3 max-w-prose text-[length:var(--text-body-md)]">
                    "Tempo and key are what a set is built from, so they are what the collection
                     is drawn as. Neighbours on the wheel mix; a cluster on the left is an hour
                     that holds together."
                </CardDescription>
            </CardHeader>
            <CardContent class="px-7 pb-7">
                <div class="mt-4 grid gap-10 lg:grid-cols-[2fr_1fr] lg:items-center">
                    <TempoKeyPlot reports=reports />
                    <CamelotWheel reports=reports />
                </div>
            </CardContent>
        </Card>
    }
}

#[component]
fn Download(run: Run, #[prop(into)] on_build: Callback<()>) -> impl IntoView {
    view! {
        <Card>
            <CardHeader class="px-7 pt-7">
                <SectionHeading eyebrow="Step three" title="Cut the image" />
                <CardDescription class="mt-3 max-w-prose text-[length:var(--text-body-md)]">
                    "One FAT32 filesystem holding the audio named after what was measured in it,
                     and the databases a player browses it through. Write it to a stick with dd,
                     and read the disk number twice."
                </CardDescription>
                <CardDescription class="mt-2 max-w-prose text-[length:var(--text-body-sm)]">
                    "Only the ticked tracks are written. Each one is already a file of its own, so
                     nothing is unpacked twice and a track left off costs nothing."
                </CardDescription>
            </CardHeader>

            <CardContent class="px-7 pb-7">
                {move || match run.phase.get() {
                    Phase::Ready { url, bytes } => {
                        view! {
                            <div class="animate-rise flex flex-wrap items-center gap-5">
                                <a
                                    class="inline-flex h-11 items-center justify-center rounded-md bg-primary px-8 text-[length:var(--text-body-lg)] text-primary-foreground transition-colors hover:bg-primary/90"
                                    href=url
                                    download=move || {
                                        format!("{}.img", run.device.get().label.to_lowercase())
                                    }
                                >
                                    "Download the image"
                                </a>
                                <Badge variant=BadgeVariant::Secondary class="figure">
                                    {human_bytes(bytes as u64)}
                                </Badge>
                            </div>
                        }
                            .into_any()
                    }
                    Phase::Building { gathered, total } => {
                        view! {
                            <div class="flex items-center gap-4">
                                // The bar the build fills. Before the first
                                // update there is nothing to fill it with, so
                                // it sweeps to say the work has started and
                                // stops sweeping the moment it can say how far
                                // along it is.
                                <span
                                    class="block h-[3px] w-40 overflow-hidden"
                                    style="background: var(--surface-sunken)"
                                >
                                    {move || match run.writing.get() {
                                        Some((written, whole)) => {
                                            let filled = (written / whole.max(1.0) * 100.0)
                                                .clamp(0.0, 100.0);
                                            view! {
                                                <span
                                                    class="block h-full"
                                                    style=format!(
                                                        "width: {filled:.1}%; background: var(--accent-primary); transition: width var(--duration-fast) var(--ease-standard)",
                                                    )
                                                />
                                            }
                                                .into_any()
                                        }
                                        None => {
                                            view! {
                                                <span
                                                    class="animate-sweep block h-full w-1/3"
                                                    style="background: var(--accent-primary)"
                                                />
                                            }
                                                .into_any()
                                        }
                                    }}
                                </span>
                                <span class="label">
                                    {move || {
                                        let size = run.estimate().total;
                                        let seconds = run.elapsed.get();
                                        if gathered < total {
                                            format!("Collecting {gathered} of {total}")
                                        } else if let Some((written, whole)) = run.writing.get() {
                                            // A share rather than the two byte
                                            // counts behind it. The build
                                            // measures itself against the
                                            // payload, which is smaller than
                                            // the image by the slack nothing
                                            // writes, and a line reading
                                            // "16 MB of 19 MB" under a card
                                            // promising 134 MB invites a
                                            // question with a long answer.
                                            format!(
                                                "Writing the filesystem, {:.0}%, {seconds}s",
                                                written / whole.max(1.0) * 100.0,
                                            )
                                        } else if size > 0 {
                                            format!(
                                                "Writing {} of filesystem, {seconds}s",
                                                human_bytes(size),
                                            )
                                        } else {
                                            format!("Writing the filesystem, {seconds}s")
                                        }
                                    }}
                                </span>
                            </div>
                        }
                            .into_any()
                    }
                    _ => {
                        view! {
                            <div class="space-y-5">
                                <SizeEstimate run=run />
                                <Button
                                    size=ButtonSize::Lg
                                    disabled=Signal::derive(move || run.selected() == 0)
                                    on_click=on_build
                                >
                                    {move || {
                                        let selected = run.selected();
                                        let measured = run.measured();
                                        if selected == measured {
                                            format!("Build from {measured} tracks")
                                        } else {
                                            format!("Build from {selected} of {measured} tracks")
                                        }
                                    }}
                                </Button>
                            </div>
                        }
                            .into_any()
                    }
                }}
            </CardContent>
        </Card>
    }
}

/// What the image will come to, and what it is made of.
///
/// Above the button rather than after the download, because the number is only
/// useful while there is still something to do about it: a stick is a fixed
/// size, and the answer to an image that will not fit is unticking rows.
#[component]
fn SizeEstimate(run: Run) -> impl IntoView {
    let estimate = Signal::derive(move || run.estimate());

    view! {
        <Show when=move || { estimate.get().total > 0 }>
            <div>
                <p class="label">
                    "Estimated image "
                    <span class="figure">{move || human_bytes(estimate.get().total)}</span>
                </p>
                <p
                    class="mt-2 max-w-prose text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted); line-height: var(--leading-body-tight)"
                >
                    {move || {
                        let estimate = estimate.get();
                        if estimate.at_minimum {
                            format!(
                                "{} of audio. FAT32 is not FAT32 under 128 MB, so that is what \
                                 this comes to whatever is on it.",
                                human_bytes(estimate.audio),
                            )
                        } else {
                            format!(
                                "{} of audio, {} of databases, and the twelve percent plus 64 MB \
                                 of room a filesystem is built with. Untick rows to bring it under \
                                 the stick you have.",
                                human_bytes(estimate.audio),
                                human_bytes(estimate.databases),
                            )
                        }
                    }}
                </p>
            </div>
        </Show>
    }
}

#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer
            class="mt-16 border-t pt-8 text-[length:var(--text-body-sm)]"
            style="border-color: var(--border-subtle); color: var(--text-muted)"
        >
            <p>
                "The analysis here is dubplate's own, compiled to WebAssembly from the crates the
                 command-line tool links. The same file measures the same way in both, which is
                 the only reason a page is worth having."
            </p>
        </footer>
    }
}
