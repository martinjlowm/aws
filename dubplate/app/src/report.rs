//! The parts of `report.json` this page draws.
//!
//! A narrow view of a wide file, declared here rather than shared with the
//! analyser: the report is the contract between the two, and naming the fields
//! that are read is what makes a change to it a parse error instead of a blank
//! chart. Everything not listed is still in the JSON and still reachable, it is
//! simply not something this page plots. Field names are the report's own, which
//! is why nothing here is renamed.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Report {
    pub source: Source,
    pub tempo: Tempo,
    pub key: Key,
    #[serde(default)]
    pub bands: Vec<Band>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Source {
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Tempo {
    /// The answer, which is a whole number on anything that was sequenced.
    pub bpm: f64,
    /// What was measured, before the snap. The gap between the two is the
    /// analyser's own error and the page says so when it is not zero.
    pub bpm_measured: f64,
    pub salience: f64,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    pub grid: Grid,
    #[serde(default)]
    pub diagnostics: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Candidate {
    pub bpm: f64,
    /// Comb salience, the primary ranking score.
    pub salience: f64,
    /// What the Fourier tempogram gives this tempo. The column that settles an
    /// octave, so the page shows it beside the salience rather than behind a
    /// disclosure.
    #[serde(default)]
    pub fourier_salience: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Grid {
    /// Beats landing within the tolerance of an onset, in `0..1`.
    #[serde(default)]
    pub matched_fraction: f64,
    /// Mean novelty on the grid over mean novelty everywhere. At 1.0 the grid is
    /// no better than an arbitrary one.
    #[serde(default)]
    pub pulse_ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Key {
    /// Camelot, e.g. `5A`. What the file name, both databases and this page use.
    pub camelot: String,
    /// The same key as notes, for when that is what you want.
    pub name: String,
    /// Winner over runner-up. Under about 0.05 the two are indistinguishable,
    /// and they are usually a key and its relative.
    #[serde(default)]
    pub margin: f64,
    /// Deviation from A = 440 Hz the chroma mapping compensated for. Beyond
    /// about 15 cents the track is not at concert pitch.
    #[serde(default)]
    pub tuning_cents: f64,
    #[serde(default)]
    pub ranked: Vec<KeyScore>,
    #[serde(default)]
    pub diagnostics: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct KeyScore {
    pub camelot: String,
    pub name: String,
    pub correlation: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Band {
    pub low_hz: f64,
    pub high_hz: f64,
    pub bpm: f64,
    pub salience: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Warning,
    Info,
}

impl Report {
    /// Every finding from every stage, warnings first, the way the terminal
    /// summary orders them.
    pub fn findings(&self) -> Vec<&Finding> {
        let mut all: Vec<&Finding> = self
            .tempo
            .diagnostics
            .iter()
            .chain(self.key.diagnostics.iter())
            .collect();
        all.sort_by_key(|finding| match finding.severity {
            Severity::Warning => 0,
            Severity::Info => 1,
        });
        all
    }

    pub fn warnings(&self) -> usize {
        self.findings()
            .iter()
            .filter(|finding| finding.severity == Severity::Warning)
            .count()
    }

    /// Whether the answer moved off the measurement on its way to being
    /// reported. The interesting case, and the one the findings explain.
    pub fn tempo_was_snapped(&self) -> bool {
        (self.tempo.bpm - self.tempo.bpm_measured).abs() > 0.005
    }

    /// The Camelot number, `1..=12`, for placing a track on the wheel.
    pub fn camelot_number(&self) -> Option<u32> {
        let camelot = &self.key.camelot;
        camelot.get(..camelot.len().checked_sub(1)?)?.parse().ok()
    }

    /// `true` for a minor key, which is the inner ring of the wheel.
    pub fn camelot_is_minor(&self) -> bool {
        self.key.camelot.ends_with('A')
    }
}

/// A tempo as it is written for a reader: `138` when it is a whole number,
/// `137.62` when it is not.
///
/// The same rule the analyser prints by. Two decimals on a whole number claim a
/// precision the measurement does not have.
pub fn format_bpm(bpm: f64) -> String {
    if bpm.is_finite() && (bpm - bpm.round()).abs() < 1e-9 {
        format!("{bpm:.0}")
    } else {
        format!("{bpm:.2}")
    }
}
