//! The tracks, as they are measured and once they are.
//!
//! One table that changes character rather than two views: a row starts as a
//! name with a needle sweeping under it and ends as a name with a tempo, a key
//! and whatever the analyser wanted to say about them. The row does not move,
//! so an archive of thirty settles rather than reflows.
//!
//! The leading column is what the image is built from. A row is ticked by
//! default and untickable once it has been measured, and a row the tick cannot
//! reach is one whose name an earlier archive already claimed: the table says
//! which, because "seven of nine" with no explanation is the report of a bug.

use crate::report::{Report, Severity, format_bpm};
use crate::run::{Figures, Run, Stage, Standing, Track, human_bytes};
use leptos::prelude::*;
use leptos_shadcn_ui::{Badge, BadgeVariant, Card};
use std::sync::Arc;

#[component]
pub fn Tracks(run: Run) -> impl IntoView {
    // Shown only when the rows have different answers. Counted over the labels
    // rather than the sources, because every loose track answers "Added
    // directly" and a column of one repeated word is a ruler down the middle of
    // the table.
    let many = Signal::derive(move || {
        let labels: std::collections::HashSet<String> = run
            .sources
            .get()
            .iter()
            .map(crate::run::Source::label)
            .collect();
        labels.len() > 1
    });
    let columns = Signal::derive(move || if many.get() { 7 } else { 6 });

    let all_on = Signal::derive(move || {
        let tracks = run.tracks.get();
        !tracks.is_empty() && tracks.iter().all(|track| track.included)
    });

    view! {
        <Card class="overflow-hidden">
            // Both lines hold their place from the first row, empty until there
            // is something to say. Said only once a row can answer it, but
            // arriving then as well would push the whole table down at the exact
            // moment somebody is reading the first answer off it.
            <div class="flex flex-wrap items-baseline justify-between gap-4 px-5 pt-5">
                // Said once: the marker in each row is what carries this
                // afterwards.
                <p class="label">
                    {move || {
                        if run.measured() > 0 {
                            "Open a track for the evidence behind its answer"
                        } else {
                            "\u{a0}"
                        }
                    }}
                </p>

                <p class="label">
                    {move || {
                        if run.measured() == 0 {
                            return "\u{a0}".to_string();
                        }
                        let counted = format!(
                            "{} of {} on the image",
                            run.selected(),
                            run.measured(),
                        );
                        let estimate = run.estimate();
                        if estimate.total > 0 {
                            format!("{counted}, about {}", human_bytes(estimate.total))
                        } else {
                            counted
                        }
                    }}
                </p>
            </div>

            <div class="overflow-x-auto pt-2">
                <table class="w-full border-collapse text-[length:var(--text-body-sm)]">
                    <thead>
                        <tr style="border-bottom: 1px solid var(--border-subtle)">
                            <th class="label px-5 py-3 text-left font-normal">
                                <input
                                    type="checkbox"
                                    aria-label="Put every measured track on the image"
                                    prop:checked=move || all_on.get()
                                    on:change=move |_| run.include_all(!all_on.get_untracked())
                                />
                            </th>
                            <Th class="text-left">"Track"</Th>
                            <Show when=move || many.get()>
                                <Th class="text-left">"Source"</Th>
                            </Show>
                            <Th class="text-right">"BPM"</Th>
                            <Th class="text-right">"Key"</Th>
                            <Th class="text-right">"Grid"</Th>
                            <Th class="text-right">"Length"</Th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || {
                                run.tracks
                                    .get()
                                    .into_iter()
                                    .zip(run.standing())
                                    .enumerate()
                                    .map(|(index, (track, standing))| (index, track, standing))
                                    .collect::<Vec<_>>()
                            }
                            key=|(index, track, standing)| {
                                (*index, track.name.clone(), stage_key(&track.stage), *standing)
                            }
                            let((index, track, standing))
                        >
                            <Row
                                run=run
                                index=index
                                track=track
                                standing=standing
                                columns=columns
                                many=many
                            />
                        </For>
                    </tbody>
                </table>
            </div>
        </Card>
    }
}

/// A key that changes exactly when a row's appearance should.
fn stage_key(stage: &Stage) -> u8 {
    match stage {
        Stage::Waiting => 0,
        Stage::Extracting => 1,
        Stage::Measuring => 2,
        Stage::Done(_) => 3,
        Stage::Failed(_) => 4,
    }
}

#[component]
fn Th(class: &'static str, children: Children) -> impl IntoView {
    view! {
        <th class=format!("label px-5 py-3 font-normal {class}")>{children()}</th>
    }
}

#[component]
fn Row(
    run: Run,
    index: usize,
    track: Track,
    /// Where this row stands with respect to the image.
    standing: Standing,
    #[prop(into)] columns: Signal<usize>,
    #[prop(into)] many: Signal<bool>,
) -> impl IntoView {
    let open = move || run.inspecting.get() == Some(index);
    let report = match &track.stage {
        Stage::Done(report) => Some(report.clone()),
        _ => None,
    };
    let failure = match &track.stage {
        Stage::Failed(why) => Some(why.clone()),
        _ => None,
    };
    let running = track.stage.is_running();
    let has_report = report.is_some();
    // What a row is doing, for the line under its name.
    let status = match &track.stage {
        Stage::Waiting => "Waiting",
        Stage::Extracting => "Unpacking",
        Stage::Measuring => "Measuring",
        Stage::Done(_) | Stage::Failed(_) => "",
    };
    let included = track.included;
    let name = track.display_name().to_string();
    let display_name = name.clone();
    let figures = track.figures.clone();
    let source = track.source;
    // What it lands on the image as, which is the export name after the format
    // has renamed it. The row shows that rather than what the analyser returned,
    // because the extension is the visible half of choosing FLAC.
    let image_name = {
        let track = track.clone();
        move || track.image_name(run.device.get().format)
    };
    // One line under the name, whatever the row is doing, because a table that
    // grows a line per track as the answers arrive resettles under the pointer
    // of somebody reading it. The slot holds whichever of these matters most:
    // why the track failed, why it is not going on the image, what it was called
    // before it was renamed, or what is happening to it.
    //
    // The failure is truncated rather than wrapped, for the same reason, and
    // carries the whole of itself in its title.
    let (sub_line, sub_is_bad) = if let Some(why) = failure.clone() {
        (why, true)
    } else if let Some(reason) = standing.reason() {
        (reason.to_string(), true)
    } else if has_report {
        (display_name.clone(), false)
    } else {
        (status.to_string(), false)
    };
    let sub_line = if sub_line.is_empty() {
        "\u{a0}".to_string()
    } else {
        sub_line
    };

    // Ties the button to the row it opens, so a screen reader reaches the
    // evidence from the name rather than hunting for it.
    let evidence_id = format!("evidence-{index}");

    view! {
        <tr
            class="animate-rise align-top"
            style=move || {
                let border = "border-bottom: 1px solid var(--border-subtle)";
                // Dimmed rather than hidden: a row that is not on the image was
                // still measured, and its answer is still the reason to have
                // dropped the archive.
                if has_report && standing != Standing::Kept {
                    format!("{border}; opacity: 0.55")
                } else {
                    border.to_string()
                }
            }
        >
            <td class="px-5 py-4">
                <input
                    type="checkbox"
                    disabled=!has_report
                    aria-label=format!("Put {display_name} on the image")
                    prop:checked=move || included
                    on:change=move |_| run.include(index, !included)
                />
            </td>

            <td class="px-5 py-4">
                <button
                    class="disclosure"
                    disabled=!has_report
                    aria-expanded=move || if open() { "true" } else { "false" }
                    aria-controls=evidence_id.clone()
                    title=move || {
                        if !has_report {
                            String::new()
                        } else if open() {
                            "Hide the evidence".to_string()
                        } else {
                            "Show the evidence behind this answer".to_string()
                        }
                    }
                    on:click=move |_| {
                        run.inspecting
                            .update(|open| {
                                *open = if *open == Some(index) { None } else { Some(index) }
                            })
                    }
                >
                    <span class="disclosure-marker" aria-hidden="true">"+"</span>
                    <span>
                        <span class="disclosure-name block">
                            {
                                let name = name.clone();
                                move || image_name().unwrap_or_else(|| name.clone())
                            }
                        </span>
                        <span
                            class="label mt-1 block truncate"
                            title=sub_line.clone()
                            style=if sub_is_bad { "color: var(--status-danger)" } else { "" }
                        >
                            {sub_line.clone()}
                        </span>
                    </span>
                </button>

                // The needle: a tempo search is a sweep across the BPM range,
                // and this is that at a speed a person can watch. Transform
                // only, so it keeps moving whatever the main thread is doing.
                //
                // Its track is always here and only the sweep comes and goes,
                // so a row does not gain two pixels and a margin the moment it
                // starts being measured.
                <span
                    class="mt-3 block h-[2px] w-full overflow-hidden"
                    style=move || {
                        if running {
                            "background: var(--surface-sunken)"
                        } else {
                            "background: transparent"
                        }
                    }
                >
                    <Show when=move || running>
                        <span
                            class="animate-sweep block h-full w-1/3"
                            style="background: var(--accent-primary)"
                        />
                    </Show>
                </span>
            </td>

            <Show when=move || many.get()>
                <td class="label px-5 py-4">{move || run.source_label(source)}</td>
            </Show>

            {match report.clone() {
                Some(report) => {
                    view! {
                        <>
                            <td class="figure px-5 py-4 text-right">
                                <span class="text-[length:var(--text-body-lg)]">
                                    {format_bpm(report.tempo.bpm)}
                                </span>
                                // The pulse, at the tempo just measured. A 174
                                // BPM track and a 126 one do not look the same,
                                // which is the point of animating it at all.
                                <span
                                    class="animate-beat ml-3 inline-block h-[14px] w-[3px] align-middle"
                                    style=format!(
                                        "background: var(--accent-primary); --beat-period: {:.0}ms",
                                        60_000.0 / report.tempo.bpm.max(1.0),
                                    )
                                />
                                <span class="label mt-1 block">
                                    {if report.tempo_was_snapped() {
                                        format!("measured {:.2}", report.tempo.bpm_measured)
                                    } else {
                                        "\u{a0}".to_string()
                                    }}
                                </span>
                            </td>
                            <td class="figure px-5 py-4 text-right">
                                <span class="text-[length:var(--text-body-lg)]">
                                    {report.key.camelot.clone()}
                                </span>
                                <span class="label mt-1 block">{report.key.name.clone()}</span>
                            </td>
                            <td class="figure px-5 py-4 text-right">
                                {format!("{:.0}%", report.tempo.grid.matched_fraction * 100.0)}
                                <span class="label mt-1 block">
                                    {format!("{:.1}x pulse", report.tempo.grid.pulse_ratio)}
                                </span>
                            </td>
                            <td class="figure px-5 py-4 text-right">
                                {duration(report.source.duration_seconds)}
                                <span class="label mt-1 block">"\u{a0}"</span>
                            </td>
                        </>
                    }
                        .into_any()
                }
                None => {
                    view! {
                        <>
                            <Waiting />
                            <Waiting />
                            <Waiting />
                            <Waiting />
                        </>
                    }
                        .into_any()
                }
            }}
        </tr>

        <Show when=move || { open() && has_report }>
            {
                let report = report.clone().expect("guarded by the Show above");
                view! {
                    <tr
                        id=evidence_id.clone()
                        style="border-bottom: 1px solid var(--border-subtle)"
                    >
                        <td
                            colspan=move || columns.get()
                            class="px-5 py-6"
                            style="background: var(--surface-sunken)"
                        >
                            <Evidence report=report.clone() figures=figures.clone() />
                        </td>
                    </tr>
                }
            }
        </Show>
    }
}

/// One measurement that has not arrived.
///
/// The same two lines the answer will occupy, so a row is the height it will
/// always be from the moment it appears. The dot is there to be seen as a
/// column that is waiting rather than a column that is empty.
#[component]
fn Waiting() -> impl IntoView {
    view! {
        <td class="figure px-5 py-4 text-right" style="color: var(--text-muted)">
            <span class="text-[length:var(--text-body-lg)]">"·"</span>
            <span class="label mt-1 block">"\u{a0}"</span>
        </td>
    }
}

#[component]
fn Findings(report: Arc<Report>) -> impl IntoView {
    // Owned: `findings()` hands back references into the report, and the
    // closures below outlive this borrow.
    let findings: Vec<crate::report::Finding> = report.findings().into_iter().cloned().collect();
    if findings.is_empty() {
        return view! { <span class="label">"None"</span> }.into_any();
    }
    view! {
        <ul class="space-y-2">
            {findings
                .into_iter()
                .map(|finding| {
                    // A finding is a measured disagreement, not a confidence
                    // score, so a warning is drawn as one and an observation is
                    // not.
                    let variant = match finding.severity {
                        Severity::Warning => BadgeVariant::Destructive,
                        Severity::Info => BadgeVariant::Secondary,
                    };
                    view! {
                        <li>
                            <Badge variant=variant class="label">
                                {finding.code.clone()}
                            </Badge>
                            <span
                                class="mt-1 block max-w-prose text-[length:var(--text-body-sm)]"
                                style="color: var(--text-secondary); line-height: var(--leading-body-tight)"
                            >
                                {finding.message.clone()}
                            </span>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
        .into_any()
}

/// The findings, and then the tables behind the headline: the tempo ranking the
/// answer came out of, and the keys it beat.
///
/// The findings lead, because a finding is the reason to have opened the row at
/// all: it says which of the numbers above is the one to argue with. The
/// candidate table follows, and its Fourier column is what settles an octave.
/// Printing it beside the salience is the whole reason a disagreement is legible
/// rather than a shrug.
#[component]
fn Evidence(report: Arc<Report>, figures: Option<Arc<Figures>>) -> impl IntoView {
    let candidates = report.tempo.candidates.clone();
    let keys = report.key.ranked.clone();
    let bands = report.bands.clone();
    let margin = report.key.margin;
    let tuning = report.key.tuning_cents;

    view! {
        <div class="mb-8">
            <h4 class="label">"Findings"</h4>
            <div class="mt-3">
                <Findings report=report.clone() />
            </div>
        </div>

        <div class="grid gap-8 lg:grid-cols-3">
            <div>
                <h4 class="label">"Tempo candidates"</h4>
                <p
                    class="mt-2 max-w-prose text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted)"
                >
                    "Comb salience ranks; the Fourier column is what separates a tempo from
                     half of it, because there is no energy at a beat frequency that is not
                     the beat."
                </p>
                <table class="figure mt-4 w-full text-[length:var(--text-body-sm)]">
                    {candidates
                        .into_iter()
                        .take(6)
                        .map(|candidate| {
                            view! {
                                <tr>
                                    <td class="py-1">{format_bpm(candidate.bpm)}</td>
                                    <td class="py-1 text-right">
                                        {format!("{:.3}", candidate.salience)}
                                    </td>
                                    <td
                                        class="py-1 text-right"
                                        style="color: var(--text-muted)"
                                    >
                                        {format!("{:.3}", candidate.fourier_salience)}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </table>
            </div>

            <div>
                <h4 class="label">"Keys"</h4>
                <p
                    class="mt-2 max-w-prose text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted)"
                >
                    {format!(
                        "Margin {margin:.3} over the runner-up; under 0.05 the two are \
                         indistinguishable and usually a key and its relative. Tuning {tuning:+.1} cents.",
                    )}
                </p>
                <div class="mt-4 space-y-1">
                    {
                        let best = keys.first().map(|k| k.correlation).unwrap_or(1.0).max(1e-9);
                        keys.into_iter()
                            .take(8)
                            .map(|key| {
                                let width = (key.correlation / best * 100.0).clamp(0.0, 100.0);
                                view! {
                                    <div class="flex items-center gap-3">
                                        <span class="figure w-10 text-[length:var(--text-body-sm)]">
                                            {key.camelot}
                                        </span>
                                        <span
                                            class="h-[8px] flex-1"
                                            style="background: var(--surface-card)"
                                        >
                                            <span
                                                class="block h-full"
                                                style=format!(
                                                    "width: {width:.1}%; background: var(--accent-primary)",
                                                )
                                            />
                                        </span>
                                        <span
                                            class="figure w-12 text-right text-[length:var(--text-mono-sm)]"
                                            style="color: var(--text-muted)"
                                        >
                                            {format!("{:.3}", key.correlation)}
                                        </span>
                                    </div>
                                }
                            })
                            .collect_view()
                    }
                </div>
            </div>

            <div>
                <h4 class="label">"Tempo per onset band"</h4>
                <p
                    class="mt-2 max-w-prose text-[length:var(--text-body-sm)]"
                    style="color: var(--text-muted)"
                >
                    "A kick at 150 and hats at 100 is a shuffle or a polyrhythm, and the
                     broadband estimate lands between them."
                </p>
                <table class="figure mt-4 w-full text-[length:var(--text-body-sm)]">
                    {bands
                        .into_iter()
                        .map(|band| {
                            view! {
                                <tr>
                                    <td class="py-1" style="color: var(--text-muted)">
                                        {format!("{:.0}-{:.0} Hz", band.low_hz, band.high_hz)}
                                    </td>
                                    <td class="py-1 text-right">{format_bpm(band.bpm)}</td>
                                    <td
                                        class="py-1 text-right"
                                        style="color: var(--text-muted)"
                                    >
                                        {format!("{:.3}", band.salience)}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </table>
            </div>
        </div>

        // `Plots` carries the space above it. What used to sit here was a
        // `Separator`, which draws an empty div and no line.
        {figures.map(|figures| view! { <Plots figures=figures /> })}
    }
}

/// The seven plots, each with the caption that says what it would look like if
/// the stage above it had gone wrong.
///
/// The SVG is inlined rather than loaded as an image because it was drawn
/// without colours of its own: inline, it inherits the page's, and its axis
/// labels stay selectable and searchable.
#[component]
fn Plots(figures: Arc<Figures>) -> impl IntoView {
    const CAPTIONS: [(&str, &str, &str); 6] = [
        (
            "novelty.svg",
            "The grid over the onsets",
            "Lines on the peaks mean the tempo is right. Lines drifting off them across the \
             window mean it is close and wrong, and peaks with no line between them mean the \
             grid sits at half the tempo of the track.",
        ),
        (
            "tempo-salience.svg",
            "Where the octave was decided",
            "Both estimators across the range, with the tempi an octave error lands on marked. \
             Autocorrelation peaking an octave below the Fourier tempogram is the disagreement \
             the metrical floor resolves.",
        ),
        (
            "tempo-over-time.svg",
            "One estimate per twenty-second window",
            "A flat line is a sequenced track. A line that collapses is the same track measured \
             as two different things depending on where you look, which is what separates a \
             tempo from half of it when the grid cannot.",
        ),
        (
            "key-correlations.svg",
            "The keys it beat",
            "Two bars of nearly equal height are a tie the correlation cannot break, and the \
             pair is usually a key and its relative, which share every note.",
        ),
        (
            "chroma.svg",
            "Pitch-class energy",
            "Nearly uniform here means the key ranking above it is arbitrary however good its \
             correlations look.",
        ),
        (
            "spectrum.svg",
            "Long-term average spectrum",
            "A hard ceiling at 16 or 19 kHz is a transcode, whatever the file extension says.",
        ),
    ];

    view! {
        <div class="mt-8 grid gap-8 md:grid-cols-2">
            {CAPTIONS
                .into_iter()
                .filter_map(|(name, title, caption)| {
                    let svg = figures.svg.get(name)?.clone();
                    Some(
                        view! {
                            <figure class="m-0">
                                <h4 class="label">{title}</h4>
                                <div
                                    class="mt-3 overflow-x-auto"
                                    style="color: var(--text-primary)"
                                    inner_html=svg
                                />
                                <figcaption
                                    class="mt-3 max-w-prose text-[length:var(--text-body-sm)]"
                                    style="color: var(--text-muted); line-height: var(--leading-body-tight)"
                                >
                                    {caption}
                                </figcaption>
                            </figure>
                        },
                    )
                })
                .collect_view()}

            {figures
                .png
                .get("spectrogram.png")
                .cloned()
                .map(|source| {
                    view! {
                        <figure class="m-0 md:col-span-2">
                            <h4 class="label">"Time against log frequency"</h4>
                            <img src=source alt="" class="mt-3 w-full" />
                            <figcaption
                                class="mt-3 max-w-prose text-[length:var(--text-body-sm)]"
                                style="color: var(--text-muted); line-height: var(--leading-body-tight)"
                            >
                                "A flat black band across the top is the same transcode the \
                                 spectrum shows as a cliff. A drop is an edge."
                            </figcaption>
                        </figure>
                    }
                })}
        </div>
    }
}

fn duration(seconds: f64) -> String {
    let total = seconds.round().max(0.0) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}
