//! What a collection looks like, rather than what one track measured.
//!
//! Two pictures, because they answer the two questions a set is built on: what
//! tempos are in here, and what keys. Both are drawn as SVG by hand, which is
//! what the analyser's own figures are and for the same reasons: the axis labels
//! stay selectable, the colours are the page's own tokens rather than a
//! library's, and there is no chart engine in the bundle.

use crate::report::{Report, format_bpm};
use leptos::prelude::*;
use std::sync::Arc;

/// Every track's tempo against its key, with a marker per track.
///
/// A scatter rather than a histogram: a histogram of eighteen tracks is six bars
/// and tells you less than the eighteen points do. The vertical axis is the
/// Camelot number, so tracks that mix sit on the same row or one either side of
/// it, and a cluster is a set.
#[component]
pub fn TempoKeyPlot(reports: Signal<Vec<Arc<Report>>>) -> impl IntoView {
    const WIDTH: f64 = 720.0;
    const HEIGHT: f64 = 300.0;
    const LEFT: f64 = 44.0;
    const BOTTOM: f64 = 28.0;
    const TOP: f64 = 12.0;

    let points = Memo::new(move |_| {
        let reports = reports.get();
        if reports.is_empty() {
            return Vec::new();
        }

        // The axis spans the tempos present with a bar of air either side,
        // rather than the search range: a set of 126 to 132 BPM tracks plotted
        // across 60 to 220 is one dot.
        let tempos: Vec<f64> = reports.iter().map(|report| report.tempo.bpm).collect();
        let low = tempos.iter().cloned().fold(f64::INFINITY, f64::min);
        let high = tempos.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (low, high) = if (high - low) < 8.0 {
            let middle = (low + high) / 2.0;
            (middle - 6.0, middle + 6.0)
        } else {
            (low - 2.0, high + 2.0)
        };

        reports
            .iter()
            .enumerate()
            .filter_map(|(index, report)| {
                let number = report.camelot_number()?;
                let x = LEFT + (report.tempo.bpm - low) / (high - low) * (WIDTH - LEFT - 16.0);
                let y = TOP + (12.0 - f64::from(number)) / 11.0 * (HEIGHT - TOP - BOTTOM);
                Some(Point {
                    index,
                    x,
                    y,
                    minor: report.camelot_is_minor(),
                    label: format!(
                        "{} — {} BPM, {}",
                        report.source.path,
                        format_bpm(report.tempo.bpm),
                        report.key.camelot
                    ),
                    // A track whose reported tempo is not the tempo that won is
                    // the one worth looking at, so it is drawn hollow.
                    doubtful: report.warnings() > 0,
                })
            })
            .collect::<Vec<_>>()
    });

    let ticks = Memo::new(move |_| {
        let reports = reports.get();
        if reports.is_empty() {
            return Vec::new();
        }
        let tempos: Vec<f64> = reports.iter().map(|report| report.tempo.bpm).collect();
        let low = tempos.iter().cloned().fold(f64::INFINITY, f64::min);
        let high = tempos.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (low, high) = if (high - low) < 8.0 {
            let middle = (low + high) / 2.0;
            (middle - 6.0, middle + 6.0)
        } else {
            (low - 2.0, high + 2.0)
        };
        (0..=4)
            .map(|step| {
                let bpm = low + (high - low) * f64::from(step) / 4.0;
                (
                    LEFT + f64::from(step) / 4.0 * (WIDTH - LEFT - 16.0),
                    format!("{bpm:.0}"),
                )
            })
            .collect::<Vec<_>>()
    });

    view! {
        <figure class="m-0">
            <svg
                viewBox=format!("0 0 {WIDTH} {HEIGHT}")
                class="w-full h-auto"
                role="img"
                aria-label="Tempo against Camelot key, one marker per track"
            >
                // The twelve rows of the wheel, so a marker's height is readable
                // without counting.
                {(1..=12)
                    .map(|number| {
                        let y = TOP + (12.0 - f64::from(number)) / 11.0 * (HEIGHT - TOP - BOTTOM);
                        view! {
                            <line
                                x1=LEFT
                                y1=y
                                x2=WIDTH - 16.0
                                y2=y
                                stroke="var(--border-subtle)"
                                stroke-width="1"
                            />
                            <text
                                x=LEFT - 10.0
                                y=y + 3.0
                                text-anchor="end"
                                class="figure"
                                font-size="9"
                                fill="var(--text-muted)"
                            >
                                {number.to_string()}
                            </text>
                        }
                    })
                    .collect_view()}

                {move || {
                    ticks
                        .get()
                        .into_iter()
                        .map(|(x, label)| {
                            view! {
                                <text
                                    x=x
                                    y=HEIGHT - 8.0
                                    text-anchor="middle"
                                    class="figure"
                                    font-size="9"
                                    fill="var(--text-muted)"
                                >
                                    {label}
                                </text>
                            }
                        })
                        .collect_view()
                }}

                {move || {
                    points
                        .get()
                        .into_iter()
                        .map(|point| {
                            // Minor keys filled, major hollow-ringed: the inner
                            // and outer rings of the wheel, kept consistent with
                            // the wheel below.
                            let fill = if point.doubtful {
                                "none".to_string()
                            } else if point.minor {
                                "var(--accent-primary)".to_string()
                            } else {
                                "var(--accent-secondary)".to_string()
                            };
                            let stroke = if point.minor {
                                "var(--accent-primary)"
                            } else {
                                "var(--accent-secondary)"
                            };
                            view! {
                                <circle
                                    cx=point.x
                                    cy=point.y
                                    r="5"
                                    fill=fill
                                    stroke=stroke
                                    stroke-width="1.5"
                                    class="animate-bloom"
                                    style=format!(
                                        "animation-delay:{}ms",
                                        (point.index as u32).min(40) * 30,
                                    )
                                >
                                    <title>{point.label}</title>
                                </circle>
                            }
                        })
                        .collect_view()
                }}
            </svg>
            <figcaption class="label mt-3">
                "Tempo across, Camelot number up. Filled is minor, ringed is major, \
                 hollow is a track that raised a finding."
            </figcaption>
        </figure>
    }
}

#[derive(Clone, PartialEq)]
struct Point {
    index: usize,
    x: f64,
    y: f64,
    minor: bool,
    label: String,
    doubtful: bool,
}

/// How the collection sits on the Camelot wheel.
///
/// Twenty-four segments, two rings, shaded by how many tracks landed in each.
/// This is the picture a set is built from: neighbours on the wheel mix, and a
/// library that is all one colour is a library with one hour in it.
#[component]
pub fn CamelotWheel(reports: Signal<Vec<Arc<Report>>>) -> impl IntoView {
    const SIZE: f64 = 300.0;
    let centre = SIZE / 2.0;

    let counts = Memo::new(move |_| {
        let mut counts = [[0usize; 2]; 12];
        for report in reports.get() {
            if let Some(number) = report.camelot_number() {
                let ring = usize::from(report.camelot_is_minor());
                counts[(number as usize - 1) % 12][ring] += 1;
            }
        }
        counts
    });

    view! {
        <figure class="m-0">
            <svg
                viewBox=format!("0 0 {SIZE} {SIZE}")
                class="w-full h-auto max-w-[300px] mx-auto"
                role="img"
                aria-label="Camelot wheel, shaded by how many tracks are in each key"
            >
                {move || {
                    let counts = counts.get();
                    let most = counts.iter().flatten().copied().max().unwrap_or(0).max(1);
                    (0..12)
                        .flat_map(|slot| {
                            // 1 at the top, clockwise, which is how the wheel is
                            // drawn everywhere else a DJ meets it.
                            let start = -90.0 + f64::from(slot as u32) * 30.0 - 15.0;
                            let end = start + 30.0;
                            [0usize, 1]
                                .into_iter()
                                .map(move |ring| {
                                    let (inner, outer) = if ring == 1 {
                                        (52.0, 92.0)
                                    } else {
                                        (94.0, 134.0)
                                    };
                                    let count = counts[slot][ring];
                                    let weight = f64::from(count as u32) / f64::from(most as u32);
                                    let letter = if ring == 1 { "A" } else { "B" };
                                    let hue = if ring == 1 {
                                        "var(--accent-primary)"
                                    } else {
                                        "var(--accent-secondary)"
                                    };
                                    let label = format!("{}{letter}", slot + 1);
                                    let text_radius = (inner + outer) / 2.0;
                                    let middle = ((start + end) / 2.0).to_radians();
                                    view! {
                                        <path
                                            d=segment(centre, inner, outer, start, end)
                                            fill=hue
                                            fill-opacity=format!("{:.3}", 0.06 + weight * 0.74)
                                            stroke="var(--border-subtle)"
                                            stroke-width="1"
                                            class="animate-bloom"
                                            style=format!("animation-delay:{}ms", slot * 25)
                                        >
                                            <title>
                                                {format!(
                                                    "{label}: {count} track{}",
                                                    if count == 1 { "" } else { "s" },
                                                )}
                                            </title>
                                        </path>
                                        <text
                                            x=centre + text_radius * middle.cos()
                                            y=centre + text_radius * middle.sin() + 3.0
                                            text-anchor="middle"
                                            class="figure"
                                            font-size="9"
                                            fill=move || {
                                                if weight > 0.55 {
                                                    "var(--accent-primary-text)"
                                                } else {
                                                    "var(--text-secondary)"
                                                }
                                            }
                                        >
                                            {label}
                                        </text>
                                    }
                                })
                        })
                        .collect_view()
                }}
            </svg>
            <figcaption class="label mt-3 text-center">
                "Inner ring minor, outer major. Darker is more tracks."
            </figcaption>
        </figure>
    }
}

/// One annular segment, as an SVG path.
fn segment(centre: f64, inner: f64, outer: f64, start_deg: f64, end_deg: f64) -> String {
    let (start, end) = (start_deg.to_radians(), end_deg.to_radians());
    let point =
        |radius: f64, angle: f64| (centre + radius * angle.cos(), centre + radius * angle.sin());
    let (x1, y1) = point(outer, start);
    let (x2, y2) = point(outer, end);
    let (x3, y3) = point(inner, end);
    let (x4, y4) = point(inner, start);
    format!(
        "M {x1:.2} {y1:.2} A {outer} {outer} 0 0 1 {x2:.2} {y2:.2} \
         L {x3:.2} {y3:.2} A {inner} {inner} 0 0 0 {x4:.2} {y4:.2} Z"
    )
}
