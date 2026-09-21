//! The settings a run uses, as a form fills them in.
//!
//! Every field is one of `pipeline::AnalysisOptions`, and the names are the ones
//! serde writes on the other side.
//!
//! The numbers below are a starting point and not the answer. A page has to draw
//! a form before it has asked the worker anything, so it draws these, and then
//! `default-options` replaces them with what the module itself reports. That
//! round trip is the whole reason they cannot drift: a default changed in
//! `pipeline` reaches this form without anybody editing this file, and a version
//! that moves one is not a version that quietly keeps the old value.
//!
//! They did drift, once, which is why the round trip is here. The metrical floor
//! sat at 90 in both places until dubplate moved it to 80, and a page still
//! sending 90 would have gone on reporting a track measured at 89.99 as 179.96,
//! which is the bug that move exists to fix.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Analysis {
    pub window: usize,
    pub hop: usize,
    pub onset_bands: usize,
    pub compression: f32,
    pub local_mean: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub bpm_resolution: f64,
    pub pulses: usize,
    pub comb_penalty: f64,
    pub metrical_floor: f64,
    pub metrical_floor_ratio: f64,
    pub integer_snap: f64,
    /// Whether a track's measured energy chooses a tempo prior for it.
    pub energy_bands: bool,
    pub tempo_prior_width: f64,
    pub key_profile: Profile,
    pub tuning_cents: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    Krumhansl,
    Temperley,
}

impl Profile {
    pub fn label(self) -> &'static str {
        match self {
            Profile::Krumhansl => "Krumhansl",
            Profile::Temperley => "Temperley",
        }
    }
}

impl Default for Analysis {
    fn default() -> Self {
        Analysis {
            window: 2048,
            hop: 512,
            onset_bands: 8,
            compression: 1000.0,
            local_mean: 0.5,
            min_bpm: 60.0,
            max_bpm: 220.0,
            bpm_resolution: 0.1,
            pulses: 4,
            comb_penalty: 0.0,
            metrical_floor: 80.0,
            metrical_floor_ratio: 0.5,
            integer_snap: 0.25,
            energy_bands: true,
            tempo_prior_width: 0.7,
            key_profile: Profile::Temperley,
            tuning_cents: None,
        }
    }
}

/// What goes on the stick, as opposed to what is measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    /// Volume name, which is what a player shows in its source list. FAT32
    /// allows eleven characters.
    pub label: String,
    pub playlist: String,
    /// Recorded against every track. A field rather than a clock reading, so the
    /// same archive built twice is the same image twice.
    pub date: String,
    pub target: Target,
    /// Whether to draw the seven figures per track. Roughly doubles the work, so
    /// it is a choice rather than a default.
    pub figures: bool,
    /// What the audio is written as.
    pub format: Format,
}

/// The audio on the image, as opposed to the audio in the archive.
///
/// `Source` copies the file across untouched. The other three are re-encodes,
/// and whether a build can perform one is a property of the analysis module it
/// links rather than of this form: the page asks the worker which formats it can
/// write and draws the rest disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Source,
    Wav,
    Flac,
    Mp3,
}

impl Format {
    /// Every format the form draws, available or not.
    pub const ALL: [Format; 4] = [Format::Source, Format::Wav, Format::Flac, Format::Mp3];

    pub fn wire(self) -> &'static str {
        match self {
            Format::Source => "source",
            Format::Wav => "wav",
            Format::Flac => "flac",
            Format::Mp3 => "mp3",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Source => "As supplied",
            Format::Wav => "WAV",
            Format::Flac => "FLAC",
            Format::Mp3 => "MP3",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Format::Source => {
                "The file as the archive held it, copied rather than re-encoded. The only one \
                 that cannot lose anything, and the only one that needs no encoder."
            }
            Format::Wav => {
                "Uncompressed, and what every player reads without argument. Around twice the \
                 FLAC, so an image that held a release holds half of one."
            }
            Format::Flac => {
                "Lossless and about half the WAV. A CDJ-3000 and Engine DJ read it; a CDJ-2000 \
                 does not."
            }
            Format::Mp3 => {
                "Lossy, and a re-encode of whatever the archive held rather than of the master. \
                 A second generation of loss on anything that arrived as an MP3."
            }
        }
    }

    /// The extension a track lands under once this format has had its say.
    ///
    /// `None` for `Source`, which keeps whatever the archive called it.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Format::Source => None,
            Format::Wav => Some("wav"),
            Format::Flac => Some("flac"),
            Format::Mp3 => Some("mp3"),
        }
    }

    /// What the device writes a track under: the name both databases store, and
    /// the name the page shows against the row.
    pub fn rename(self, export_name: &str) -> String {
        let Some(extension) = self.extension() else {
            return export_name.to_string();
        };
        match export_name.rsplit_once('.') {
            Some((stem, _)) => format!("{stem}.{extension}"),
            None => format!("{export_name}.{extension}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Rekordbox,
    Engine,
    Both,
}

impl Target {
    pub fn wire(self) -> &'static str {
        match self {
            Target::Rekordbox => "rekordbox",
            Target::Engine => "engine",
            Target::Both => "both",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Target::Rekordbox => "Pioneer",
            Target::Engine => "Denon",
            Target::Both => "Both",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Target::Rekordbox => "PIONEER/ only: export.pdb and the ANLZ files a CDJ reads.",
            Target::Engine => "Engine Library/ only: schema 2.21.2, which Engine DJ 2 and 3 read.",
            Target::Both => {
                "Both databases on one image. They live in separate directories, \
                 neither player reads the other's, and the audio is shared."
            }
        }
    }
}

impl Default for Device {
    fn default() -> Self {
        Device {
            label: "MUSIC".into(),
            playlist: "All tracks".into(),
            date: "2026-01-01".into(),
            target: Target::Both,
            figures: true,
            format: Format::Source,
        }
    }
}
