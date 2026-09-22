//! The page.

use crate::bridge::Pool;
use crate::components::charts::{CamelotWheel, TempoKeyPlot};
use crate::components::drop_zone::{DragVeil, DropZone, watch_window_drops};
use crate::components::section::SectionHeading;
use crate::components::settings::Settings;
use crate::components::sources::Sources;
use crate::components::tracks::Tracks;
use crate::options::Device;
use crate::run::{
    Phase, Run, append_all, build_image, human_bytes, read_capabilities, read_defaults,
};
use crate::store;
use leptos::prelude::*;
use leptos_shadcn_ui::{
    Badge, BadgeVariant, Button, ButtonSize, Card, CardContent, CardDescription, Progress,
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

    // What the form was left at last visit, read before the worker is asked
    // anything so the first paint is already somebody's own numbers rather than
    // a set of defaults that flick over to them a moment later.
    if let Some(analysis) = store::load(store::ANALYSIS) {
        run.analysis.set(analysis);
    }
    if let Some(device) = store::load(store::DEVICE) {
        run.device.set(device);
    }

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

    // Kept for the next visit, and only what differs from the defaults. A form
    // back at the defaults stores nothing, which is what the reset button at the
    // foot of the panel does and why it needs no storage of its own.
    Effect::new(move |_| {
        let analysis = run.analysis.get();
        if !run.defaults_seen.get() {
            return;
        }
        if analysis == run.defaults.get() {
            store::forget(store::ANALYSIS);
        } else {
            store::save(store::ANALYSIS, &analysis);
        }
    });

    Effect::new(move |_| {
        let device = run.device.get();
        if device == Device::default() {
            store::forget(store::DEVICE);
        } else {
            store::save(store::DEVICE, &device);
        }
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

    // The window is the drop target, so this is installed once for the page
    // rather than once per card. What it hands back is whether a file is
    // currently overhead, which is what the veil and the card are drawn from.
    let dragging = watch_window_drops(on_files, busy);

    view! {
        // Two shapes, and the breakpoint between them is where the third column
        // stops fitting. Below 1536px this is a page: it is as tall as what is
        // on it, the window scrolls, and the sections follow one another down in
        // the order the work happens. At 1536 and up it is an application: the
        // window is the frame, the header stays on it, and each of the three
        // columns scrolls inside its own height.
        //
        // 1536 rather than 1280, because the settings rail is 480px wide: the
        // five tabs sit on one line at that width and not at any less, and two
        // rails of it leave a table worth reading only once the window is this
        // wide.
        //
        // The page half is the one a phone gets, and it is the original: a
        // column of cards at a readable measure, which is what a narrow screen
        // can show and what a thumb can scroll.
        <div class="mx-auto flex w-full max-w-[var(--container-lg)] flex-col px-6 py-12 md:px-10 md:py-16 2xl:h-full 2xl:max-w-none 2xl:px-0 2xl:py-0">
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
                <DragVeil dragging=dragging />

                // All three columns are here from the first paint, whether or
                // not they have anything in them yet, so the run in the middle
                // is laid out once: an image finishing is a column filling and
                // never the table beside it narrowing under the pointer.
                //
                // Written in the order they are read in, which is also the order
                // they are used in: the settings decide what a drop measures, so
                // they come before the drop on a page and to the left of it in
                // the application. Nothing here is reordered by CSS, so what a
                // keyboard walks through is what the eye walks through.
                <main class="mt-12 grid items-start gap-8 2xl:mt-0 2xl:min-h-0 2xl:flex-1 2xl:grid-cols-[480px_minmax(0,1fr)_480px] 2xl:items-stretch 2xl:gap-0">
                    <aside
                        class="2xl:col-start-1 2xl:row-start-1 2xl:flex 2xl:h-full 2xl:min-h-0 2xl:flex-col 2xl:gap-6 2xl:overflow-y-auto 2xl:border-r 2xl:p-8"
                        style="border-color: var(--border-subtle)"
                    >
                        <Settings
                            analysis=run.analysis
                            defaults=Signal::derive(move || run.defaults.get())
                            device=run.device
                            formats=Signal::derive(move || run.formats.get())
                        />
                    </aside>

                    <div class="space-y-8 2xl:col-start-2 2xl:row-start-1 2xl:h-full 2xl:min-h-0 2xl:space-y-6 2xl:overflow-y-auto 2xl:p-8">
                        <DropZone
                            on_files=on_files
                            busy=busy
                            dragging=dragging
                            appending=Signal::derive(move || !run.sources.get().is_empty())
                        />

                        <Show when=move || { !run.sources.get().is_empty() }>
                            <Sources run=run busy=busy />
                        </Show>

                        <Show when=move || { !run.tracks.with(|tracks| tracks.is_empty()) }>
                            <Runway run=run />
                            <Tracks run=run />
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
                    </div>

                    // The rails carry the border between the columns rather
                    // than a gap, because a gap wide enough to separate them at
                    // this width is a stripe of page down the middle of an
                    // application.
                    <aside
                        class="flex flex-col gap-8 2xl:col-start-3 2xl:row-start-1 2xl:h-full 2xl:min-h-0 2xl:gap-6 2xl:overflow-y-auto 2xl:border-l 2xl:p-8"
                        style="border-color: var(--border-subtle)"
                    >
                        <Results run=run on_build=on_build />

                        // The colophon, in the corner the work ends in. It sits
                        // under the image and the collection because it is about
                        // where the numbers above it come from, and it is the
                        // last thing on the page for the same reason.
                        //
                        // `mt-auto` is what holds it against the bottom, which
                        // is why this column is a flex column with a gap rather
                        // than `space-y`: that sets a top margin on every child
                        // but the first, through a selector that beats it.
                        <Colophon class="mt-auto hidden 2xl:block" />
                    </aside>

                </main>
            </Show>

            <Colophon class="mt-16 border-t pt-8 2xl:hidden" />
        </div>
    }
}

/// The title, and nothing else.
///
/// What this is used to be said here and is said on the drop target instead,
/// which is where somebody who has not dropped anything is looking. What is
/// left is a name: a line on a page, and the bar across the top of the
/// application, where every pixel it does not take is one the three columns
/// get.
#[component]
fn Header() -> impl IntoView {
    view! {
        <header
            class="flex shrink-0 items-center justify-between gap-6 2xl:border-b 2xl:px-8 2xl:py-5"
            style="border-color: var(--border-subtle)"
        >
            <h1 class="text-[length:var(--text-display-md)] 2xl:text-[length:var(--text-display-sm)]">
                "Dubplate"
            </h1>
            <Source />
        </header>
    }
}

/// Where the code is.
///
/// The mark and nothing else. A page that measures a track and writes a
/// filesystem in the tab it is read in invites the question of what it is doing,
/// and the answer that settles it is the source.
#[component]
fn Source() -> impl IntoView {
    view! {
        <a
            class="shrink-0 transition-colors"
            style="color: var(--text-muted)"
            href="https://github.com/martinjlowm/dubplate"
            target="_blank"
            rel="noreferrer"
            title="Dubplate on GitHub"
            aria-label="Dubplate on GitHub"
            on:mouseenter=move |event| set_colour(&event, "var(--text-primary)")
            on:mouseleave=move |event| set_colour(&event, "var(--text-muted)")
        >
            <svg
                viewBox="0 0 16 16"
                width="22"
                height="22"
                fill="currentColor"
                aria-hidden="true"
                class="block"
            >
                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
            </svg>
        </a>
    }
}

/// The hover, set on the element rather than through a class: the colour is a
/// token and Tailwind's hover variants are written against its own palette.
fn set_colour(event: &leptos::ev::MouseEvent, colour: &str) {
    use wasm_bindgen::JsCast;
    if let Some(target) = event.current_target() {
        if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
            let _ = element.style().set_property("color", colour);
        }
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
            <div class="px-7 pt-7">
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
            </div>

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

/// The right-hand rail: what the run came to, and the image built from it.
///
/// The export leads and the collection follows, rather than the other way
/// round. Both arrive as tracks are measured, and the one that arrives second is
/// the wheel: under the export it grows downwards into empty column, and above
/// it, it would push the button a person is reaching for out from under their
/// pointer.
#[component]
fn Results(run: Run, #[prop(into)] on_build: Callback<()>) -> impl IntoView {
    let exporting = Signal::derive(move || {
        matches!(
            run.phase.get(),
            Phase::Measured | Phase::Building { .. } | Phase::Ready { .. },
        ) && run.measured() > 0
    });

    view! {
        <Show when=move || exporting.get() fallback=|| view! { <Awaiting /> }>
            <Download run=run on_build=on_build />
        </Show>

        <Show when=move || { run.measured() > 1 }>
            <Collection run=run />
        </Show>
    }
}

/// The rail before there is anything to put in it.
///
/// Here so the column is not an empty stripe down the side of a page that has
/// not been used yet, and so the two things that land here are announced before
/// they do.
#[component]
fn Awaiting() -> impl IntoView {
    view! {
        <Card>
            <div class="px-7 pt-7 pb-7">
                <SectionHeading eyebrow="The image" title="Cut the image" />
                <CardDescription class="mt-3 max-w-prose text-[length:var(--text-body-md)]">
                    "The image the ticked tracks are written to is built from here, and under it
                     the tempos and keys you have dropped are drawn as they are measured. Both
                     wait on the first answer."
                </CardDescription>
            </div>
        </Card>
    }
}

#[component]
fn Collection(run: Run) -> impl IntoView {
    let reports = Signal::derive(move || run.reports());
    view! {
        <Card>
            <div class="px-7 pt-7 pb-6">
                <SectionHeading eyebrow="The collection" title="What you have dropped" />
                <CardDescription class="mt-3 max-w-prose text-[length:var(--text-body-md)]">
                    "Tempo and key are what a set is built from, so they are what the collection
                     is drawn as. Neighbours on the wheel mix; a cluster on the left is an hour
                     that holds together."
                </CardDescription>
            </div>
            <CardContent class="px-7 pb-7">
                // Side by side while this card has the page's width, stacked
                // once it is in the rail: two figures in 360px are two
                // thumbnails.
                <div class="mt-4 grid gap-10 lg:grid-cols-[2fr_1fr] lg:items-center 2xl:grid-cols-1 2xl:items-stretch">
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
            <div class="px-7 pt-7 pb-6">
                <SectionHeading eyebrow="The image" title="Cut the image" />
                <CardDescription class="mt-3 max-w-prose text-[length:var(--text-body-md)]">
                    "One FAT32 filesystem holding the audio named after what was measured in it,
                     and the databases a player browses it through. Write it to a stick with dd,
                     and read the disk number twice."
                </CardDescription>
                <CardDescription class="mt-2 max-w-prose text-[length:var(--text-body-sm)]">
                    "A stick already partitioned FAT32 does not need dd. Mount the image and copy
                     everything at its root to the root of the stick, since both databases store
                     their paths from the device root. The stick keeps its own volume name, which
                     is the one a player shows in its source list."
                </CardDescription>
                <CardDescription class="mt-2 max-w-prose text-[length:var(--text-body-sm)]">
                    "Only the ticked tracks are written. Each one is already a file of its own, so
                     nothing is unpacked twice and a track left off costs nothing."
                </CardDescription>
            </div>

            // One slot, given its height before anything is in it. What sits
            // here is an estimate and a button, then a progress line, then a
            // link, and the three are not the same size: measured as they come,
            // the card would shrink when a build starts and grow when it
            // finishes, and the collection under it would step up and down the
            // rail each time.
            <CardContent class="export-slot px-7 pb-7">
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

/// Where the analysis comes from.
///
/// Placed twice rather than moved, because the two shapes want it in two places:
/// last on the page, and at the foot of the settings rail in the application,
/// where it sits under everything else nobody needs while they are working. The
/// trimmings differ and the sentence does not, so the sentence is here once.
#[component]
fn Colophon(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <footer
            class=format!("text-[length:var(--text-body-sm)] {class}")
            style="border-color: var(--border-subtle); color: var(--text-muted)"
        >
            <p class="max-w-prose">
                "The analysis here is dubplate's own, compiled to WebAssembly from the crates the
                 command-line tool links. The same file measures the same way in both, which is
                 the only reason a page is worth having."
            </p>
        </footer>
    }
}
