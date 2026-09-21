//! Every knob the analysis has, and the four the device has.
//!
//! Grouped the way the signal chain is: what the transform sees, what the tempo
//! search does with it, what the key stage does. The order inside each group is
//! the order in `AnalysisOptions`, which is the order `--help` prints, so a
//! person who knows the CLI finds a field where they expect it.
//!
//! Every control carries the sentence from the flag's own documentation. A
//! number here changes an answer, and a form that lets you change one without
//! saying what it does is a form that produces answers nobody can defend.

use crate::components::section::SectionTitle;
use crate::options::{Analysis, Device, Format, Profile, Target};
use leptos::prelude::*;
use leptos_shadcn_ui::{
    Button, ButtonSize, ButtonVariant, Card, CardContent, Input, Label, Separator, Switch, Tabs,
    TabsContent, TabsList, TabsTrigger,
};

#[component]
pub fn Settings(
    analysis: RwSignal<Analysis>,
    /// The settings as the analysis module defaults them, which is what
    /// "Defaults" means here and what the reset returns to. Read from the
    /// module rather than from this crate, so a default that moves upstream
    /// moves here.
    defaults: Signal<Analysis>,
    device: RwSignal<Device>,
    /// What the worker said it can write, by wire name. Every other format is
    /// drawn and disabled rather than hidden, because the question a person has
    /// is whether this tool writes FLAC, and a missing button does not answer
    /// it.
    #[prop(into)]
    formats: Signal<Vec<String>>,
) -> impl IntoView {
    let open = RwSignal::new(false);

    view! {
        <Card>
            // The heading wraps the button rather than sitting inside it: a
            // button holds phrasing content, and an h2 is not that. The pair is
            // the accordion the rest of the page's cards are, so the eyebrow and
            // the title come from the same component they do.
            <h2 class="section-heading">
                <button
                    class="flex w-full items-center justify-between px-7 pt-7 pb-7 text-left"
                    aria-expanded=move || if open.get() { "true" } else { "false" }
                    on:click=move |_| open.update(|open| *open = !*open)
                >
                    <SectionTitle eyebrow="Step two, optional" title="Settings" />
                    <span class="flex items-center gap-4">
                        <span class="label">
                            {move || {
                                if analysis.get() == defaults.get() {
                                    "Defaults".to_string()
                                } else {
                                    "Changed".to_string()
                                }
                            }}
                        </span>
                        <span
                            class="figure text-[length:var(--text-body-lg)] transition-transform"
                            style=move || {
                                if open.get() { "transform: rotate(45deg)" } else { "" }
                            }
                        >
                            "+"
                        </span>
                    </span>
                </button>
            </h2>

            <Show when=move || open.get()>
                <div class="animate-rise">
                    <Separator />
                    <CardContent class="px-7 pt-7 pb-7">
                    <Tabs default_value="device">
                        <TabsList>
                            <TabsTrigger value="device">"Device"</TabsTrigger>
                            <TabsTrigger value="transform">"Transform"</TabsTrigger>
                            <TabsTrigger value="tempo">"Tempo"</TabsTrigger>
                            <TabsTrigger value="key">"Key"</TabsTrigger>
                            <TabsTrigger value="cues">"Cues"</TabsTrigger>
                        </TabsList>
                    <TabsContent value="device">
                    <Group
                        title="Device"
                        note="What goes on the stick, rather than what is measured."
                    >
                        <Text
                            label="Volume label"
                            note="What a player shows in its source list. FAT32 allows eleven characters."
                            value=Signal::derive(move || device.get().label)
                            set=Callback::new(move |next: String| {
                                device.update(|device| device.label = next.chars().take(11).collect())
                            })
                        />
                        <Text
                            label="Playlist"
                            note="The one playlist both databases carry, holding every track."
                            value=Signal::derive(move || device.get().playlist)
                            set=Callback::new(move |next: String| {
                                device.update(|device| device.playlist = next)
                            })
                        />
                        <Text
                            label="Date"
                            note="Recorded against every track, as YYYY-MM-DD. Fixed rather than read \
                                  from the clock, so one archive builds to one image every time."
                            value=Signal::derive(move || device.get().date)
                            set=Callback::new(move |next: String| {
                                device.update(|device| device.date = next)
                            })
                        />

                        <Field
                            label="Databases"
                            note=Signal::derive(move || device.get().target.detail().to_string())
                        >
                            <div class="flex gap-2">
                                {[Target::Rekordbox, Target::Engine, Target::Both]
                                    .into_iter()
                                    .map(|target| {
                                        view! {
                                            <Button
                                                variant=Signal::derive(move || {
                                                    if device.get().target == target {
                                                        ButtonVariant::Default
                                                    } else {
                                                        ButtonVariant::Outline
                                                    }
                                                })
                                                on_click=Callback::new(move |()| {
                                                    device.update(|device| device.target = target)
                                                })
                                            >
                                                {target.label()}
                                            </Button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </Field>

                        // Only the formats this build can write, and only when
                        // there is more than one of them. A row of buttons where
                        // every option but the chosen one is unavailable is a
                        // control that cannot be operated, and drawing it greyed
                        // out asks the person to work out why. When the module
                        // grows encoders they appear here, and until then the
                        // settings say nothing about a choice that does not
                        // exist.
                        <Show when=move || { offered(formats).len() > 1 }>
                            <Field
                                label="Audio format"
                                note=Signal::derive(move || {
                                    device.get().format.detail().to_string()
                                })
                            >
                                <div class="flex flex-wrap gap-2">
                                    {move || {
                                        offered(formats)
                                            .into_iter()
                                            .map(|format| {
                                                view! {
                                                    <Button
                                                        variant=Signal::derive(move || {
                                                            if device.get().format == format {
                                                                ButtonVariant::Default
                                                            } else {
                                                                ButtonVariant::Outline
                                                            }
                                                        })
                                                        on_click=Callback::new(move |()| {
                                                            device
                                                                .update(|device| {
                                                                    device.format = format
                                                                })
                                                        })
                                                    >
                                                        {format.label()}
                                                    </Button>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </div>
                            </Field>
                        </Show>

                        <Field
                            label="Figures"
                            note=Signal::derive(|| {
                                "Draw the seven plots per track. Roughly doubles the time a run takes, \
                                 and they are what turns an answer into something you can argue with."
                                    .to_string()
                            })
                        >
                            <Switch
                                checked=Signal::derive(move || device.get().figures)
                                on_change=Callback::new(move |next: bool| {
                                    device.update(|device| device.figures = next)
                                })
                            />
                        </Field>
                    </Group>

                    </TabsContent>

                    <TabsContent value="transform">
                    <Group
                        title="Transform"
                        note="One pass of the short-time Fourier transform feeds every stage below. \
                              Onsets need time resolution and key needs frequency resolution, which is \
                              the tension the window sits in the middle of."
                    >
                        <Number
                            label="Window"
                            unit="samples"
                            note="Larger resolves frequency, smaller resolves time."
                            step=256.0
                            value=Signal::derive(move || analysis.get().window as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.window = next.max(256.0) as usize)
                            })
                        />
                        <Number
                            label="Hop"
                            unit="samples"
                            note="Sets the frame rate of every novelty curve, and so the finest tempo \
                                  difference that can be resolved."
                            step=64.0
                            value=Signal::derive(move || analysis.get().hop as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.hop = next.max(64.0) as usize)
                            })
                        />
                        <Number
                            label="Onset bands"
                            unit=""
                            note="Frequency bands the onset detector splits the spectrum into. Narrow \
                                  this to the band that agrees with your ears when a kick and the hats \
                                  disagree."
                            step=1.0
                            value=Signal::derive(move || analysis.get().onset_bands as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.onset_bands = next.max(1.0) as usize)
                            })
                        />
                        <Number
                            label="Compression"
                            unit="gamma"
                            note="Gamma of the logarithmic compression applied before the flux."
                            step=100.0
                            value=Signal::derive(move || f64::from(analysis.get().compression))
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.compression = next as f32)
                            })
                        />
                        <Number
                            label="Local mean"
                            unit="seconds"
                            note="Width of the moving average subtracted from the flux. Stays wider \
                                  than a beat period or it removes the pulse being measured."
                            step=0.1
                            value=Signal::derive(move || analysis.get().local_mean)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.local_mean = next)
                            })
                        />
                    </Group>

                    </TabsContent>

                    <TabsContent value="tempo">
                    <Group
                        title="Tempo"
                        note="The search over that curve, and the rules that decide which metrical \
                              level the answer is reported at. Nothing here names or shapes a tempo: \
                              what weights the candidates is measured from the track, not set."
                    >
                        <Number
                            label="Minimum BPM"
                            unit=""
                            note="Wide enough that an octave error stays inside the range and visible, \
                                  rather than being clipped out of it."
                            step=1.0
                            value=Signal::derive(move || analysis.get().min_bpm)
                            set=Callback::new(move |next: f64| analysis.update(|a| a.min_bpm = next))
                        />
                        <Number
                            label="Maximum BPM"
                            unit=""
                            note="Narrow the pair to force a metrical level rather than arguing with \
                                  the salience."
                            step=1.0
                            value=Signal::derive(move || analysis.get().max_bpm)
                            set=Callback::new(move |next: f64| analysis.update(|a| a.max_bpm = next))
                        />
                        <Number
                            label="Resolution"
                            unit="BPM"
                            note="Spacing of the tempo grid. The answer is refined between grid points, \
                                  so this sets the cost of the search rather than the precision."
                            step=0.05
                            value=Signal::derive(move || analysis.get().bpm_resolution)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.bpm_resolution = next)
                            })
                        />
                        <Number
                            label="Pulses"
                            unit="comb teeth"
                            note="Comb teeth used by the tempo salience."
                            step=1.0
                            value=Signal::derive(move || analysis.get().pulses as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.pulses = next.max(1.0) as usize)
                            })
                        />
                        <Number
                            label="Comb penalty"
                            unit="0 to 1"
                            note="At 0 the salience is a plain harmonic sum; at 1 it argues hardest \
                                  against slow metrical levels, and against any tempo whose offbeats \
                                  carry weight. Off by default for the second reason."
                            step=0.05
                            value=Signal::derive(move || analysis.get().comb_penalty)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.comb_penalty = next.clamp(0.0, 1.0))
                            })
                        />
                        <Number
                            label="Metrical floor"
                            unit="BPM"
                            note="Slowest level the answer may be reported at. At 0 the salience \
                                  curve's own answer stands, subharmonic and all."
                            step=5.0
                            value=Signal::derive(move || analysis.get().metrical_floor)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.metrical_floor = next)
                            })
                        />
                        <Number
                            label="Floor ratio"
                            unit=""
                            note="How strong the doubled candidate must be, relative to the original, \
                                  for the floor to double it."
                            step=0.05
                            value=Signal::derive(move || analysis.get().metrical_floor_ratio)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.metrical_floor_ratio = next)
                            })
                        />
                        <Number
                            label="Integer snap"
                            unit="BPM"
                            note="Largest gap the answer may be moved by to reach a whole number. \
                                  Produced music is written on integers, so the default closes the \
                                  tool's own error and nothing wider. 0 reports what was measured."
                            step=0.05
                            value=Signal::derive(move || analysis.get().integer_snap)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.integer_snap = next)
                            })
                        />
                    </Group>

                    </TabsContent>

                    <TabsContent value="key">
                    <Group
                        title="Key"
                        note="Chroma against a profile. The two profiles disagree on tracks built from \
                              a repeating loop, which is most of them."
                    >
                        <Field
                            label="Profile"
                            note=Signal::derive(|| {
                                "Krumhansl is fitted to listener ratings and reads a natural-minor loop \
                                 as its relative major more often. Temperley is fitted to note counts \
                                 and is the better default."
                                    .to_string()
                            })
                        >
                            <div class="flex gap-2">
                                {[Profile::Temperley, Profile::Krumhansl]
                                    .into_iter()
                                    .map(|profile| {
                                        view! {
                                            <Button
                                                variant=Signal::derive(move || {
                                                    if analysis.get().key_profile == profile {
                                                        ButtonVariant::Default
                                                    } else {
                                                        ButtonVariant::Outline
                                                    }
                                                })
                                                on_click=Callback::new(move |()| {
                                                    analysis.update(|a| a.key_profile = profile)
                                                })
                                            >
                                                {profile.label()}
                                            </Button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </Field>
                        <Optional
                            label="Tuning"
                            unit="cents"
                            note="Override the measured offset from A = 440 Hz. Leave it off unless you \
                                  know the track is not at concert pitch and the estimate disagrees."
                            fallback=0.0
                            step=1.0
                            value=Signal::derive(move || analysis.get().tuning_cents)
                            set=Callback::new(move |next: Option<f64>| {
                                analysis.update(|a| a.tuning_cents = next)
                            })
                        />
                    </Group>

                    </TabsContent>

                    <TabsContent value="cues">
                    <Group
                        title="Cues"
                        note="Where the eight pads land. The sections are measured from the band \
                              energies the onset detector already computed, and the pads are fixed \
                              to roles rather than to tracks, because the hands learn the pad."
                    >
                        <Field
                            label="Trim the lead-in"
                            note=Signal::derive(|| {
                                "Skip the near-silence at the head of the file before measuring \
                                 anything. On, because a grid laid from sample zero on a track that \
                                 opens with two seconds of black carries that offset into every \
                                 bar, and a first beat at 2.1 seconds is one nothing can cue to."
                                    .to_string()
                            })
                        >
                            <Switch
                                checked=Signal::derive(move || analysis.get().trim_lead_in)
                                on_change=Callback::new(move |next: bool| {
                                    analysis.update(|a| a.trim_lead_in = next)
                                })
                            />
                        </Field>
                        <Number
                            label="Memory offset"
                            unit="bars"
                            note="How far ahead of its hot cue a memory cue sits. Sixteen bars is \
                                  where a mix starts rather than where the section does."
                            step=1.0
                            value=Signal::derive(move || analysis.get().memory_offset_bars as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.memory_offset_bars = next.max(0.0) as usize)
                            })
                        />
                        <Number
                            label="Loop length"
                            unit="bars"
                            note="What the two loop pads mark out."
                            step=1.0
                            value=Signal::derive(move || analysis.get().loop_bars as f64)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.loop_bars = next.max(1.0) as usize)
                            })
                        />
                        <Number
                            label="Drop after"
                            unit="of the track"
                            note="How far in a drop has to be to count as the drop rather than a \
                                  taste of the hook. When every drop is earlier than this the \
                                  earliest one takes the pad, and the run says the rule bent."
                            step=0.05
                            value=Signal::derive(move || analysis.get().drop_after_fraction)
                            set=Callback::new(move |next: f64| {
                                analysis.update(|a| a.drop_after_fraction = next.clamp(0.0, 1.0))
                            })
                        />
                    </Group>

                    </TabsContent>
                    </Tabs>

                    <Separator class="my-7" />

                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Sm
                        class="label"
                        on_click=Callback::new(move |()| analysis.set(defaults.get()))
                    >
                        "Reset to the defaults the CLI prints"
                    </Button>
                    </CardContent>
                </div>
            </Show>
        </Card>
    }
}

/// Whether this build can write a format.
fn available(formats: Signal<Vec<String>>, format: Format) -> bool {
    formats.get().iter().any(|name| name == format.wire())
}

/// The formats this build can write, in the order the form lists them.
///
/// Empty until the worker has answered, which is why the field is drawn on a
/// count rather than on a flag: a form that flickers a control into existence a
/// second after the page loads is worse than one that never had it.
fn offered(formats: Signal<Vec<String>>) -> Vec<Format> {
    Format::ALL
        .into_iter()
        .filter(|format| available(formats, *format))
        .collect()
}

#[component]
fn Group(title: &'static str, note: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="pt-7">
            <h3 class="text-[length:var(--text-body-xl)]">{title}</h3>
            <p
                class="mt-2 max-w-prose text-[length:var(--text-body-sm)]"
                style="color: var(--text-secondary)"
            >
                {note}
            </p>
            <div class="mt-6 grid gap-6 md:grid-cols-2">{children()}</div>
        </div>
    }
}

#[component]
fn Field(
    label: &'static str,
    #[prop(into)] note: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div>
            <Label class="label">{label}</Label>
            <div class="mt-2 flex min-h-10 items-center">{children()}</div>
            <p
                class="mt-2 text-[length:var(--text-body-sm)]"
                style="color: var(--text-muted); line-height: var(--leading-body-tight)"
            >
                {move || note.get()}
            </p>
        </div>
    }
}

/// A free-text field: a name rather than a measurement.
#[component]
fn Text(
    label: &'static str,
    note: &'static str,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] set: Callback<String>,
) -> impl IntoView {
    view! {
        <Field label=label note=Signal::derive(move || note.to_string())>
            <Input
                value=Signal::derive(move || value.get())
                on_change=Callback::new(move |next: String| set.run(next))
            />
        </Field>
    }
}

#[component]
fn Number(
    label: &'static str,
    unit: &'static str,
    note: &'static str,
    step: f64,
    #[prop(into)] value: Signal<f64>,
    #[prop(into)] set: Callback<f64>,
) -> impl IntoView {
    // `step` is not a prop the component exposes, and a number field without one
    // steps by 1, which is useless for a 0.05 penalty. It is set on the element
    // the component rendered, by id, once.
    let id = field_id(label);
    apply_step(id.clone(), step);

    view! {
        <Field label=label note=Signal::derive(move || note.to_string())>
            <div class="flex items-center gap-3">
                <Input
                    id=id
                    input_type="number"
                    class="figure"
                    value=Signal::derive(move || value.get().to_string())
                    on_change=Callback::new(move |next: String| {
                        if let Ok(parsed) = next.parse::<f64>() {
                            set.run(parsed);
                        }
                    })
                />
                <span class="label whitespace-nowrap">{unit}</span>
            </div>
        </Field>
    }
}

/// A number that can also be off, which is what `Option<f64>` is on the CLI.
#[component]
fn Optional(
    label: &'static str,
    unit: &'static str,
    note: &'static str,
    fallback: f64,
    step: f64,
    #[prop(into)] value: Signal<Option<f64>>,
    #[prop(into)] set: Callback<Option<f64>>,
) -> impl IntoView {
    let id = field_id(label);
    apply_step(id.clone(), step);

    view! {
        <Field label=label note=Signal::derive(move || note.to_string())>
            <div class="flex items-center gap-3">
                <Switch
                    checked=Signal::derive(move || value.get().is_some())
                    on_change=Callback::new(move |on: bool| {
                        set.run(if on { Some(fallback) } else { None })
                    })
                />
                <Input
                    id=id
                    input_type="number"
                    class="figure"
                    disabled=Signal::derive(move || value.get().is_none())
                    value=Signal::derive(move || {
                        value.get().map(|value| value.to_string()).unwrap_or_default()
                    })
                    on_change=Callback::new(move |next: String| {
                        if let Ok(parsed) = next.parse::<f64>() {
                            set.run(Some(parsed));
                        }
                    })
                />
                <span class="label whitespace-nowrap">{unit}</span>
            </div>
        </Field>
    }
}

/// A stable id for one field, from its label.
fn field_id(label: &str) -> String {
    format!(
        "field-{}",
        label
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            })
            .collect::<String>()
    )
}

/// Put a `step` on a rendered number field.
///
/// The component has no prop for it and a number input without one steps by 1,
/// which cannot reach a 0.05 comb penalty from the keyboard. An effect rather
/// than a wrapper element, so the component stays the shadcn one.
fn apply_step(id: String, step: f64) {
    Effect::new(move |_| {
        if let Some(element) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(&id))
        {
            let _ = element.set_attribute("step", &step.to_string());
        }
    });
}
