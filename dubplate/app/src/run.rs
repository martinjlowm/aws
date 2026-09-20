//! Any number of archives and loose tracks, from dropped to downloadable.
//!
//! The whole of the application's behaviour is here: list what was dropped,
//! measure everything in it, and then, when the image is asked for, read the
//! sources a second time for the tracks that are still ticked. The components
//! read the signals this writes and never drive anything themselves, which is
//! what keeps the state machine in one file rather than spread across a dozen
//! event handlers.
//!
//! Nothing here holds audio. A track is a `File`, which is a reference to bytes
//! the browser already has, whether the person dropped it or the worker wrote it
//! into the origin-private filesystem. It is handed to `analyze` to be measured
//! and handed to the device builder if it is kept, and the builder reads it when
//! the image is asked for. So a run costs one read of each archive and one write
//! of each track it holds, and unticking a row costs nothing at all.

use crate::bridge::{Pool, get, object};
use crate::options::{Analysis, Device, Format};
use crate::report::Report;
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Poll, Waker};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::File;

/// What was dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A zip, listed and extracted through the worker.
    Archive,
    /// One track, dropped on its own. Nothing lists it and nothing extracts it:
    /// the file is the audio.
    Loose,
}

/// One dropped file, archive or track.
///
/// The handle, not the bytes. A Beatport month is a gigabyte, and a person who
/// drops four of them has dropped four gigabytes that the browser is already
/// holding on disk. Reading one at the moment it is needed and letting it go
/// afterwards keeps the tab at one file however many are listed here.
#[derive(Clone)]
pub struct Source {
    /// Stable across removals, which is what the tracks below refer to.
    pub id: u32,
    pub name: String,
    pub kind: Kind,
}

impl Source {
    /// What the tracks table calls this source.
    ///
    /// A loose track's file name is already the row's name, so repeating it in
    /// the archive column says nothing. Every loose track answers the same way
    /// instead, which is the fact the column is there to carry.
    pub fn label(&self) -> String {
        match self.kind {
            Kind::Archive => self.name.clone(),
            Kind::Loose => "Added directly".to_string(),
        }
    }
}

/// Where a single track is in the run.
#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Waiting,
    Extracting,
    Measuring,
    /// Measured, with the report of measuring it.
    Done(Arc<Report>),
    /// A track that could not be measured, and why. One failure is one track:
    /// an archive is not abandoned because a shop shipped a torn MP3 in it.
    Failed(String),
}

impl Stage {
    pub fn is_running(&self) -> bool {
        matches!(self, Stage::Extracting | Stage::Measuring)
    }
}

/// The seven plots, as the analyser drew them.
///
/// Held rather than rendered on demand because they are drawn once, during the
/// run, from intermediate values the report does not carry: the novelty curve
/// and the spectrogram exist for the length of one `pipeline::run` and nowhere
/// after it.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct Figures {
    /// SVG source by file name. Inlined into the page rather than loaded as an
    /// image, so the plots inherit the page's own colours and their axis labels
    /// stay selectable.
    pub svg: BTreeMap<String, String>,
    /// The spectrogram, as a `data:` URL. The one figure that is a pixel grid.
    pub png: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct Track {
    /// What it came out of, by [`Source::id`].
    pub source: u32,
    /// The path inside that archive, which is what identifies it to the worker.
    /// For a loose track it is the file's own name.
    pub name: String,
    /// What the analyser called it, `126_05A_Artist-Title.flac`. The format the
    /// image is written in may still rename it, so the name a row shows comes
    /// from [`Track::image_name`] rather than from here.
    pub export_name: Option<String>,
    pub size: u64,
    pub stage: Stage,
    pub figures: Option<Arc<Figures>>,
    /// Whether the person wants it on the image. Their choice alone. A track
    /// that is ticked can still be left off for claiming a name an earlier one
    /// already has, and [`selection`] is where the two meet.
    pub included: bool,
    /// The report as the analyser wrote it.
    ///
    /// Held as text because that is what the device builder parses. [`Report`]
    /// is a narrow view of a wide file, so re-serialising the parsed form would
    /// hand the builder a report with most of its fields missing.
    pub report_json: Option<String>,
    /// The file this track's audio is in.
    ///
    /// What the person dropped, or what `archive-extract` wrote. A reference
    /// either way, so holding one per track for the length of a run costs the
    /// page nothing: it is handed to `analyze` to measure and to `device-add`
    /// to be written, and it is read on the far side of the boundary both times.
    pub audio: File,
}

impl Track {
    /// The part of the name a person reads, without the directories the shop
    /// zipped it under.
    pub fn display_name(&self) -> &str {
        self.name.rsplit('/').next().unwrap_or(&self.name)
    }

    /// How long it plays, once it has been measured.
    pub fn duration_seconds(&self) -> f64 {
        match &self.stage {
            Stage::Done(report) => report.source.duration_seconds,
            _ => 0.0,
        }
    }

    /// What this track will occupy on the image.
    ///
    /// Exact for the format that copies the file across, which is its size here.
    /// For a re-encode it is arithmetic over what the report measured: the rate
    /// and the channel count are in it, so the WAV is exact and the other two
    /// are the ratios those encoders hold to it.
    pub fn image_bytes(&self, format: Format) -> u64 {
        let Stage::Done(report) = &self.stage else {
            return self.size;
        };
        let seconds = report.source.duration_seconds;
        let uncompressed = seconds
            * f64::from(report.source.sample_rate)
            * f64::from(report.source.channels)
            * 2.0;
        match format {
            Format::Source => self.size,
            Format::Wav => uncompressed as u64,
            // A little over half, on the music this is pointed at. Dance music
            // compresses worse than the quiet end of a catalogue does.
            Format::Flac => (uncompressed * 0.6) as u64,
            // 320 kbit/s, which is the only bitrate worth writing to a stick.
            Format::Mp3 => (seconds * 40_000.0) as u64,
        }
    }

    /// What this track lands on the image as, once the format has renamed it.
    pub fn image_name(&self, format: Format) -> Option<String> {
        self.export_name
            .as_deref()
            .map(|export_name| format.rename(export_name))
    }
}

/// What the page is doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Nothing dropped yet.
    Idle,
    Reading,
    Analysing,
    /// Everything dropped so far has been measured. The image has not been asked
    /// for, and more may still arrive.
    Measured,
    /// Reading the sources again for the tracks that were kept. `gathered` of
    /// `total`; at the end of it the filesystem itself is being written.
    Building {
        gathered: usize,
        total: usize,
    },
    /// An image is ready, as an object URL and its size in bytes.
    Ready {
        url: String,
        bytes: usize,
    },
    Failed(String),
}

#[derive(Clone, Copy)]
pub struct Run {
    pub phase: RwSignal<Phase>,
    /// What was dropped, in the order it was dropped.
    ///
    /// `new_local` because a `File` is neither `Send` nor `Sync` and every
    /// shadcn callback is. The handle this hands out is `Copy` and `Send`; the
    /// files it holds never leave this thread, and there is only ever one.
    pub sources: RwSignal<Vec<Source>, LocalStorage>,
    /// Every track from every source, in the order they were listed.
    ///
    /// `new_local` for the same reason `sources` is: a track carries the
    /// `File` its audio is in, and a `File` is neither `Send` nor `Sync`.
    pub tracks: RwSignal<Vec<Track>, LocalStorage>,
    pub analysis: RwSignal<Analysis>,
    pub device: RwSignal<Device>,
    /// The track whose figures are open, by index.
    pub inspecting: RwSignal<Option<usize>>,
    /// The audio formats this build's worker can write, by wire name.
    ///
    /// Asked for once on load rather than assumed, because what a module can
    /// encode is a property of the crates it links and this page is pinned to a
    /// revision of them. Empty until the answer arrives.
    pub formats: RwSignal<Vec<String>>,
    next_source: RwSignal<u32>,
}

impl Run {
    pub fn new() -> Run {
        Run {
            phase: RwSignal::new(Phase::Idle),
            sources: RwSignal::new_local(Vec::new()),
            tracks: RwSignal::new_local(Vec::new()),
            analysis: RwSignal::new(Analysis::default()),
            device: RwSignal::new(Device::default()),
            inspecting: RwSignal::new(None),
            formats: RwSignal::new(Vec::new()),
            next_source: RwSignal::new(0),
        }
    }

    pub fn measured(&self) -> usize {
        self.tracks.with(|tracks| {
            tracks
                .iter()
                .filter(|track| matches!(track.stage, Stage::Done(_)))
                .count()
        })
    }

    pub fn failed(&self) -> usize {
        self.tracks.with(|tracks| {
            tracks
                .iter()
                .filter(|track| matches!(track.stage, Stage::Failed(_)))
                .count()
        })
    }

    /// Every report, in the order the sources listed them.
    pub fn reports(&self) -> Vec<Arc<Report>> {
        self.tracks.with(|tracks| {
            tracks
                .iter()
                .filter_map(|track| match &track.stage {
                    Stage::Done(report) => Some(report.clone()),
                    _ => None,
                })
                .collect()
        })
    }

    /// Where every track stands with respect to the image, one per row.
    pub fn standing(&self) -> Vec<Standing> {
        let format = self.device.get().format;
        self.tracks.with(|tracks| standing(tracks, format))
    }

    pub fn selected(&self) -> usize {
        self.standing()
            .into_iter()
            .filter(|standing| *standing == Standing::Kept)
            .count()
    }

    /// Throw away an image the page is no longer describing.
    ///
    /// Ticking a row, dropping another file or changing a device setting all
    /// change what an image would hold, and the finished one is still sitting
    /// behind a download link saying otherwise. Revoked rather than forgotten,
    /// because the blob behind that URL is the whole library.
    pub fn invalidate(&self) {
        let stale = match self.phase.get_untracked() {
            Phase::Ready { url, .. } => url,
            _ => return,
        };
        let _ = web_sys::Url::revoke_object_url(&stale);
        self.phase.set(Phase::Measured);
    }

    /// Tick or untick one row.
    pub fn include(&self, index: usize, included: bool) {
        self.tracks.update(|tracks| {
            if let Some(track) = tracks.get_mut(index) {
                track.included = included;
            }
        });
        self.invalidate();
    }

    /// Tick or untick everything that was measured.
    pub fn include_all(&self, included: bool) {
        self.tracks.update(|tracks| {
            for track in tracks.iter_mut() {
                track.included = included;
            }
        });
        self.invalidate();
    }

    /// Forget one source and everything it contributed.
    pub fn remove_source(&self, id: u32) {
        self.forget(|source| source.id != id);
    }

    /// Forget every loose track at once.
    ///
    /// They are listed as one line rather than one line each, so they are
    /// removed the same way. A single loose track is still unticked in the
    /// table like any other row.
    pub fn remove_loose(&self) {
        self.forget(|source| source.kind != Kind::Loose);
    }

    fn forget(&self, keep: impl Fn(&Source) -> bool) {
        let gone: Vec<u32> = self
            .sources
            .get_untracked()
            .iter()
            .filter(|source| !keep(source))
            .map(|source| source.id)
            .collect();
        self.sources.update(|sources| sources.retain(&keep));
        self.tracks
            .update(|tracks| tracks.retain(|track| !gone.contains(&track.source)));
        self.inspecting.set(None);
        self.invalidate();
        if self.tracks.with_untracked(|tracks| tracks.is_empty()) {
            self.phase.set(Phase::Idle);
        }
    }

    /// How many loose tracks were dropped.
    pub fn loose(&self) -> usize {
        self.sources
            .get()
            .iter()
            .filter(|source| source.kind == Kind::Loose)
            .count()
    }

    pub fn source_label(&self, id: u32) -> String {
        self.sources
            .get()
            .iter()
            .find(|source| source.id == id)
            .map(Source::label)
            .unwrap_or_default()
    }

    fn set_stage(&self, index: usize, stage: Stage) {
        self.tracks.update(|tracks| {
            if let Some(track) = tracks.get_mut(index) {
                track.stage = stage;
            }
        });
    }

    fn claim(&self, name: String, kind: Kind) -> u32 {
        let id = self.next_source.get_untracked();
        self.next_source.set(id + 1);
        self.sources
            .update(|sources| sources.push(Source { id, name, kind }));
        self.invalidate();
        id
    }
}

/// Where one measured track stands with respect to the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standing {
    /// On the image.
    Kept,
    /// Not measured, or measured and refused.
    Unmeasured,
    /// Unticked by hand.
    Unticked,
    /// An earlier source already writes the name this one wants.
    Duplicate,
    /// Measured, but neither exporter can write a track of this format.
    Unwritable,
}

impl Standing {
    /// What the row says about being off, or nothing when it is on.
    pub fn reason(self) -> Option<&'static str> {
        match self {
            Standing::Duplicate => Some("An earlier source already writes this name"),
            Standing::Unwritable => Some(
                "Neither database can hold this format, so it cannot go on the image. \
                 Measured all the same, and re-encoding is what will fix it.",
            ),
            _ => None,
        }
    }
}

/// Where every track stands, in order.
///
/// A track is on the image when it was measured, when it was not unticked, when
/// nothing before it already claimed the name it would be written under, and
/// when a player can read what that name ends in.
///
/// Two archives that share a release are the ordinary case for the third rule.
/// Both copies measure the same, so both want the same name, and one FAT32
/// directory holds one of them. The earlier source wins, which is the order they
/// were dropped in.
///
/// The name judged is the one the format produces rather than the one the
/// analyser returned. Writing everything as WAV is itself a way for a FLAC and
/// an MP3 of one track to collide, and it is also what turns a file no player
/// reads into one they do.
pub fn standing(tracks: &[Track], format: Format) -> Vec<Standing> {
    let mut claimed: HashSet<String> = HashSet::new();
    tracks
        .iter()
        .map(|track| {
            let Some(name) = track.image_name(format) else {
                return Standing::Unmeasured;
            };
            if track.report_json.is_none() {
                return Standing::Unmeasured;
            }
            if !track.included {
                return Standing::Unticked;
            }
            if !writable(&name) {
                return Standing::Unwritable;
            }
            if claimed.insert(name) {
                Standing::Kept
            } else {
                Standing::Duplicate
            }
        })
        .collect()
}

/// Whether the device can write a track under this name.
///
/// These five spellings are `collection::Format::from_extension`, which is what
/// both exporters call to decide what a track is. Nothing else reaches the
/// databases, whatever a player would have read off a stick: a name outside this
/// list fails the export for the whole image rather than for one track, so the
/// page holds those rows off instead of offering them.
///
/// It is a copy of another crate's list and it drifts if that one grows. What
/// that costs is visible rather than silent, which is the reason it is safe to
/// copy: a format added there and not here is a track this page declines to
/// write, and one added here and not there is a build that stops and names it.
fn writable(name: &str) -> bool {
    matches!(
        name.rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .as_deref(),
        Some("wav" | "aiff" | "aif" | "flac" | "mp3")
    )
}

/// What the image will come to, before anything is built.
///
/// The point of it is the stick. A person with a 32 GB stick and forty gigabytes
/// of archive needs to know which it is before spending ten minutes building
/// something that will not fit, and the number moves as rows are unticked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Estimate {
    /// What the `.img` will be.
    pub total: u64,
    /// The audio itself, which is exact.
    pub audio: u64,
    /// The two databases, which is the part that is guessed.
    pub databases: u64,
    /// Whether FAT32's own floor is what sets the size rather than the content.
    pub at_minimum: bool,
}

/// Slack over the payload, and the floor under the whole thing.
///
/// These four are `image::build`'s own constants rather than an impression of
/// them, which is what makes this an arithmetic answer instead of a guess: the
/// only estimated term is the database size below.
const SLACK_PERCENT: u64 = 12;
const SLACK_BYTES: u64 = 64 * 1024 * 1024;
const MINIMUM_BYTES: u64 = 128 * 1024 * 1024;
const SECTOR_BYTES: u64 = 512;

/// What the two databases come to, measured rather than reasoned about.
///
/// Built here from eight two-minute tracks and again from four of them, and
/// solved: 385 KB that is there whatever the collection is, and 64 KB per
/// two-minute track on top. The fixed part is the Engine schema and the
/// rekordbox skeleton; the rest is the beat grid and the waveform each player
/// draws, which is why it is charged per second rather than per track.
///
/// On anything stick-sized this is well under a percent of the total, so the
/// estimate is as good as its exact term.
const DATABASE_FIXED: u64 = 385 * 1000;
const DATABASE_PER_SECOND: u64 = 536;

impl Run {
    /// What the image would come to if it were built right now.
    pub fn estimate(&self) -> Estimate {
        let format = self.device.get().format;
        let (audio, seconds) = self.tracks.with(|tracks| {
            let mut audio = 0u64;
            let mut seconds = 0f64;
            for (track, standing) in tracks.iter().zip(standing(tracks, format)) {
                if standing != Standing::Kept {
                    continue;
                }
                audio += track.image_bytes(format);
                seconds += track.duration_seconds();
            }
            (audio, seconds)
        });

        if audio == 0 {
            return Estimate {
                total: 0,
                audio: 0,
                databases: 0,
                at_minimum: false,
            };
        }

        let databases = DATABASE_FIXED + (seconds * DATABASE_PER_SECOND as f64) as u64;
        let payload = audio + databases;
        let sized = payload * (100 + SLACK_PERCENT) / 100 + SLACK_BYTES;
        Estimate {
            total: sized.max(MINIMUM_BYTES).next_multiple_of(SECTOR_BYTES),
            audio,
            databases,
            at_minimum: sized < MINIMUM_BYTES,
        }
    }
}

/// A byte count as a stick is labelled.
///
/// Powers of ten, because that is what is printed on the stick somebody is
/// comparing this against: a 32 GB stick holds 32 billion bytes, not 34.4.
pub fn human_bytes(bytes: u64) -> String {
    const GB: f64 = 1_000_000_000.0;
    const MB: f64 = 1_000_000.0;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else {
        format!("{:.0} MB", bytes / MB)
    }
}

/// What a file's first bytes say it is.
///
/// An extension is a claim and a header is evidence. A track pulled off a video
/// site arrives named whatever the downloader felt like, an archive saved from a
/// mail client arrives as `.zip.bin`, and a file renamed by hand arrives lying.
/// All three are ordinary, and all three are decided here instead of by the
/// characters after the last dot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sniff {
    Zip,
    Wav,
    Aiff,
    Flac,
    Mp3,
    /// MP4, M4A, MOV: anything whose fourth word is `ftyp`.
    Iso,
    /// WebM and Matroska, which share their magic.
    Matroska,
    Ogg,
    /// Nothing the analyser has a reader for.
    Unknown,
}

impl Sniff {
    /// Whether this is something to hand to `analyze` rather than to open.
    fn is_audio(self) -> bool {
        !matches!(self, Sniff::Zip | Sniff::Unknown)
    }

    /// What a file holding these bytes is called when nothing is wrong.
    ///
    /// Only the four the device can write. Renaming an Ogg to `.ogg` would be
    /// true and would change nothing: the databases have no entry for it, so it
    /// is held off the image under either name, and a rename that buys nothing
    /// is a file the person no longer recognises.
    fn preferred(self) -> Option<&'static str> {
        match self {
            Sniff::Wav => Some("wav"),
            Sniff::Aiff => Some("aiff"),
            Sniff::Flac => Some("flac"),
            Sniff::Mp3 => Some("mp3"),
            Sniff::Ogg | Sniff::Iso | Sniff::Matroska | Sniff::Zip | Sniff::Unknown => None,
        }
    }

    /// The names already right for these bytes, which are left alone.
    ///
    /// A rename that changes nothing is still a rename: it moves a file the
    /// person recognises to one they do not. `.aif` and `.aiff` are the same
    /// file to everything that reads either, so an archive written in the first
    /// spelling stays in it.
    fn acceptable(self) -> &'static [&'static str] {
        match self {
            Sniff::Wav => &["wav"],
            Sniff::Aiff => &["aiff", "aif"],
            Sniff::Flac => &["flac"],
            Sniff::Mp3 => &["mp3"],
            Sniff::Ogg | Sniff::Iso | Sniff::Matroska | Sniff::Zip | Sniff::Unknown => &[],
        }
    }
}

/// What a track is called on the image.
///
/// The name a file arrives under is a claim and its bytes are the fact, and the
/// two disagree often enough to matter: a shop ships a FLAC inside a zip under
/// an `.mp3` name, a download arrives with no extension at all. Both databases
/// pick a track's format out of its name and neither looks at the file, so the
/// claim is what decides whether a player finds the track. This is where it is
/// made to match.
///
/// Nothing is transcoded here and nothing needs to be. Every rename is one where
/// the bytes already are what the new name says, so what lands on the stick is a
/// valid file of the format it now claims to be.
///
/// A format the device cannot write is left under whatever name it came with.
/// Renaming it would not make it writable, and `standing` holds it off the image
/// and says so.
async fn name_on_the_image(name: &str, file: &File) -> String {
    let sniffed = sniff(file).await;
    let Some(preferred) = sniffed.preferred() else {
        return name.to_string();
    };

    let (stem, extension) = split_name(name);
    if extension.is_some_and(|extension| {
        sniffed
            .acceptable()
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    }) {
        return name.to_string();
    }
    format!("{stem}.{preferred}")
}

/// A name as a stem and the extension it claims, if it claims one.
///
/// A dot near the end is an extension; the dot in `Artist - Title Vol. 2` is
/// not. Four characters is where the one stops looking like the other.
fn split_name(name: &str) -> (&str, Option<&str>) {
    match name.rsplit_once('.') {
        Some((stem, tail)) if !stem.is_empty() && !tail.is_empty() && tail.len() <= 4 => {
            (stem, Some(tail))
        }
        _ => (name, None),
    }
}

/// The longest prefix any of the checks below needs.
///
/// `ftyp` sits at byte four and the AIFF and WAVE tags at byte eight, so twelve
/// covers every one of them. Read off the front of the file rather than out of
/// it: a `File` is backed by the disk, and slicing sixteen bytes off a gigabyte
/// costs what reading sixteen bytes costs.
const HEAD_BYTES: i32 = 16;

/// Read the first bytes of a file and say what they are.
async fn sniff(file: &File) -> Sniff {
    let Ok(head) = file.slice_with_i32_and_i32(0, HEAD_BYTES) else {
        return Sniff::Unknown;
    };
    let Ok(buffer) = JsFuture::from(head.array_buffer()).await else {
        return Sniff::Unknown;
    };
    classify(&js_sys::Uint8Array::new(&buffer).to_vec())
}

/// What a file's first bytes are, by the magic each format opens with.
fn classify(head: &[u8]) -> Sniff {
    let at = |from: usize, tag: &[u8]| {
        head.len() >= from + tag.len() && &head[from..from + tag.len()] == tag
    };

    // Every zip starts `PK`, and the three that follow separate a local entry
    // from an empty archive from a spanned one.
    if at(0, b"PK\x03\x04") || at(0, b"PK\x05\x06") || at(0, b"PK\x07\x08") {
        return Sniff::Zip;
    }
    // RIFF and FORM are container tags shared with formats this cannot read, so
    // both are confirmed by the type that follows the size word.
    if at(0, b"RIFF") && at(8, b"WAVE") {
        return Sniff::Wav;
    }
    if at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")) {
        return Sniff::Aiff;
    }
    if at(0, b"fLaC") {
        return Sniff::Flac;
    }
    if at(0, b"OggS") {
        return Sniff::Ogg;
    }
    if at(0, &[0x1A, 0x45, 0xDF, 0xA3]) {
        return Sniff::Matroska;
    }
    // An ISO base media file names its brand at byte four, which is what MP4,
    // M4A and MOV all are underneath.
    if at(4, b"ftyp") {
        return Sniff::Iso;
    }
    // An MP3 is either tagged or it starts at a frame, and a frame starts with
    // eleven set bits.
    if at(0, b"ID3") {
        return Sniff::Mp3;
    }
    if head.len() >= 2 && head[0] == 0xFF && head[1] & 0xE0 == 0xE0 {
        return Sniff::Mp3;
    }
    Sniff::Unknown
}

/// Whether a name claims to be a WAV.
///
/// The one extension taken at its word, so the common case costs no read at all.
/// A WAV that is really something else still fails, and says so from the
/// analyser rather than from here.
fn claims_wav(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("wav"))
}

/// Measure one dropped file, whatever it turns out to be.
///
/// The name decides nothing except for a `.wav`. Everything else is routed on
/// its first bytes, so an archive saved under the wrong extension still opens
/// and a track named `download` still measures.
async fn append(run: Run, pool: Rc<Pool>, file: File, permits: &Rc<Permits>, settings: &Settings) {
    let name = file.name();
    let sniffed = if claims_wav(&name) {
        Sniff::Wav
    } else {
        sniff(&file).await
    };

    if sniffed == Sniff::Zip {
        append_archive(run, pool, file, permits, settings).await;
    } else if sniffed.is_audio() {
        append_track(run, pool, file, permits, settings).await;
    } else {
        run.phase.set(Phase::Failed(format!(
            "{name} is neither a zip nor a track this reads. Its first bytes match no \
             format the analyser has a reader for: WAV, AIFF, FLAC, MP3, and the audio \
             inside an MP4, M4A, WebM or Ogg."
        )));
    }
}

/// Ask the worker what this build can write.
pub async fn read_capabilities(run: Run, pool: Rc<Pool>) {
    let Ok(reply) = pool.call_device("capabilities", JsValue::UNDEFINED).await else {
        // A worker that cannot answer this cannot analyse anything either, and
        // the first dropped file says so in a place a person is looking.
        return;
    };
    if let Ok(formats) = serde_wasm_bindgen::from_value::<Vec<String>>(get(&reply, "formats")) {
        run.formats.set(formats);
    }
}

/// Measure everything dropped at once.
///
/// Archives are registered one at a time, because each is read where it is
/// registered and the worker that holds them holds one set. The measurements
/// are not: a permit is taken before a track leaves its archive and given back
/// when its report arrives, so as many tracks are in flight as there are workers
/// to measure them, whichever archive they came out of.
pub async fn append_all(run: Run, pool: Rc<Pool>, files: Vec<File>) {
    let permits = Permits::new(pool.analysers());
    let settings = Settings::read(run);

    // A file that fails as a whole fails here rather than in a row, and the next
    // file's first act is to say it is analysing. Without this the second drop
    // erases the first one's reason, and a zip that was refused looks like a zip
    // that was ignored.
    let mut refused: Vec<String> = Vec::new();
    for file in files {
        append(run, pool.clone(), file, &permits, &settings).await;
        if let Phase::Failed(why) = run.phase.get_untracked() {
            refused.push(why);
        }
    }

    // The loop is done handing tracks out; the last of them are still being
    // measured.
    permits.drain().await;

    match refused.as_slice() {
        [] => run.phase.set(Phase::Measured),
        [only] => run.phase.set(Phase::Failed(only.clone())),
        many => run.phase.set(Phase::Failed(many.join(" "))),
    }
}

/// Register one archive and hand everything in it to the pool.
///
/// The archive stays open in the worker for the length of the run. Opening one
/// is a read of its central directory, and closing it after the listing would
/// mean doing that again for every extraction.
async fn append_archive(
    run: Run,
    pool: Rc<Pool>,
    file: File,
    permits: &Rc<Permits>,
    settings: &Settings,
) {
    let name = file.name();
    run.phase.set(Phase::Reading);

    let added = pool
        .call_device(
            "archive-add",
            object(&[
                ("name", JsValue::from_str(&name)),
                ("file", file.clone().into()),
            ]),
        )
        .await;
    let index = match added {
        Ok(reply) => get(&reply, "index").as_f64().unwrap_or_default() as usize,
        Err(error) => return run.phase.set(Phase::Failed(error)),
    };

    let listed = match pool
        .call_device("archive-entries", JsValue::UNDEFINED)
        .await
    {
        Ok(listed) => listed,
        Err(error) => return run.phase.set(Phase::Failed(error)),
    };
    let entries: Vec<Entry> = match serde_wasm_bindgen::from_value(get(&listed, "entries")) {
        Ok(entries) => entries,
        Err(error) => {
            return run.phase.set(Phase::Failed(format!(
                "the listing could not be read: {error}"
            )));
        }
    };
    // `entries` answers for every archive at once, and the ones wanted here are
    // the ones the archive just added holds.
    let mut entries: Vec<Entry> = entries
        .into_iter()
        .filter(|entry| entry.archive == index)
        .collect();
    for entry in &mut entries {
        entry.repair();
    }

    if entries.is_empty() {
        return run.phase.set(Phase::Failed(format!(
            "no WAV, AIFF, FLAC or MP3 files in {name}"
        )));
    }

    let id = run.claim(name, Kind::Archive);
    run.phase.set(Phase::Analysing);

    for entry in entries {
        // Before the extraction rather than after it, so a track is never
        // written to disk with nowhere to measure it.
        let permit = permits.take().await;

        // The row goes up before the extraction and says so. A track inside a
        // zip is decompressed before anything can measure it, and on a long WAV
        // that is a second where the person is owed a line that moves.
        let slot = run.tracks.with_untracked(|tracks| tracks.len());
        run.tracks.update(|tracks| {
            tracks.push(Track {
                source: id,
                name: entry.name.clone(),
                export_name: None,
                size: entry.size,
                stage: Stage::Extracting,
                figures: None,
                included: true,
                report_json: None,
                audio: nothing_yet(),
            })
        });

        let extracted = pool
            .call_device(
                "archive-extract",
                object(&[
                    ("archive", JsValue::from_f64(index as f64)),
                    ("name", JsValue::from_str(&entry.name)),
                    ("slot", JsValue::from_f64(slot as f64)),
                ]),
            )
            .await;

        match extracted {
            Ok(reply) => {
                let audio: File = get(&reply, "file").unchecked_into();
                run.tracks.update(|tracks| {
                    if let Some(track) = tracks.get_mut(slot) {
                        track.audio = audio;
                    }
                });
            }
            Err(error) => {
                run.set_stage(slot, Stage::Failed(error));
                continue;
            }
        }

        measure_soon(
            run,
            pool.clone(),
            slot,
            entry.file_name.clone(),
            settings.clone(),
            permit,
        );
    }
}

/// Hand one track dropped on its own to the pool.
///
/// No listing and no extraction: the file is the audio, so it goes straight to
/// `analyze`. It never touches the worker that holds archives.
async fn append_track(
    run: Run,
    pool: Rc<Pool>,
    file: File,
    permits: &Rc<Permits>,
    settings: &Settings,
) {
    let name = file.name();
    let id = run.claim(name.clone(), Kind::Loose);

    let index = run.tracks.with_untracked(|tracks| tracks.len());
    run.tracks.update(|tracks| {
        tracks.push(Track {
            source: id,
            name: name.clone(),
            export_name: None,
            size: file.size() as u64,
            stage: Stage::Waiting,
            figures: None,
            included: true,
            report_json: None,
            audio: file,
        })
    });
    run.phase.set(Phase::Analysing);

    let permit = permits.take().await;
    measure_soon(run, pool, index, name, settings.clone(), permit);
}

/// Start measuring one track and return without waiting for it.
///
/// The permit rides along and is dropped when the report lands, which is what
/// lets the next track out of its archive.
fn measure_soon(
    run: Run,
    pool: Rc<Pool>,
    index: usize,
    display_name: String,
    settings: Settings,
    permit: Permit,
) {
    leptos::task::spawn_local(async move {
        measure(run, &pool, index, &display_name, &settings).await;
        drop(permit);
    });
}

/// One permit per worker that measures.
///
/// The extraction loop runs ahead of the measurements. Without this it would
/// pull every track out of the archive as fast as the zip decompresses and hand
/// them all to the pool at once, which holds the whole library decoded. A permit
/// is taken before a track is extracted and given back when its report arrives,
/// so the number of tracks in flight is the number of workers measuring them.
struct Permits {
    total: usize,
    free: Cell<usize>,
    waiting: RefCell<Vec<Waker>>,
}

impl Permits {
    fn new(total: usize) -> Rc<Permits> {
        Rc::new(Permits {
            total: total.max(1),
            free: Cell::new(total.max(1)),
            waiting: RefCell::new(Vec::new()),
        })
    }

    /// Wait for a free worker and take it.
    async fn take(self: &Rc<Permits>) -> Permit {
        let permits = self.clone();
        std::future::poll_fn(move |context| {
            if permits.free.get() > 0 {
                permits.free.set(permits.free.get() - 1);
                Poll::Ready(Permit {
                    permits: permits.clone(),
                })
            } else {
                permits.waiting.borrow_mut().push(context.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    /// Wait for every measurement still running to finish.
    async fn drain(self: &Rc<Permits>) {
        let permits = self.clone();
        std::future::poll_fn(move |context| {
            if permits.free.get() == permits.total {
                Poll::Ready(())
            } else {
                permits.waiting.borrow_mut().push(context.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    fn give_back(&self) {
        self.free.set(self.free.get() + 1);
        // Everything waiting, not one of them: a waiter may be `drain`, which
        // wants the last permit back rather than the next one.
        for waker in self.waiting.borrow_mut().drain(..) {
            waker.wake();
        }
    }
}

/// A worker held for as long as one track is being measured.
struct Permit {
    permits: Rc<Permits>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.permits.give_back();
    }
}

/// What a run measures with, read once rather than per track.
#[derive(Clone)]
struct Settings {
    analysis: JsValue,
    figures: bool,
}

impl Settings {
    fn read(run: Run) -> Settings {
        let analysis = run.analysis.get_untracked();
        Settings {
            analysis: serde_wasm_bindgen::to_value(&analysis).unwrap_or(JsValue::UNDEFINED),
            figures: run.device.get_untracked().figures,
        }
    }
}

/// Measure one track and record the answer against its row.
///
/// The file crosses as a reference, so nothing here is large and nothing is
/// copied. What comes back is the report, the name and the figures if they were
/// drawn, which is all that is kept.
async fn measure(run: Run, pool: &Pool, index: usize, display_name: &str, settings: &Settings) {
    let Some(audio) = run
        .tracks
        .with_untracked(|tracks| tracks.get(index).map(|track| track.audio.clone()))
    else {
        return;
    };

    // The analyser derives the name a track lands under from the name it is
    // given, so this is where a file that arrived misnamed is put right, for an
    // archive entry and a dropped file alike.
    let measured_as = name_on_the_image(display_name, &audio).await;

    run.set_stage(index, Stage::Measuring);
    let analysed = match pool
        .call(
            "analyze",
            object(&[
                ("name", JsValue::from_str(&measured_as)),
                ("file", audio.into()),
                ("options", settings.analysis.clone()),
                ("figures", JsValue::from_bool(settings.figures)),
            ]),
        )
        .await
    {
        Ok(analysed) => analysed,
        Err(error) => return run.set_stage(index, Stage::Failed(error)),
    };

    let report_value = get(&analysed, "report");
    let report: Report = match serde_wasm_bindgen::from_value(report_value.clone()) {
        Ok(report) => report,
        Err(error) => {
            return run.set_stage(
                index,
                Stage::Failed(format!("the report could not be read: {error}")),
            );
        }
    };

    // Both are what the build works from, and a track missing either can never
    // reach an image. Failing here rather than leaving a row that is ticked,
    // dimmed and never written.
    let (Some(export_name), Some(report_json)) = (
        get(&analysed, "exportName").as_string(),
        js_sys::JSON::stringify(&report_value)
            .ok()
            .and_then(|json| json.as_string()),
    ) else {
        return run.set_stage(
            index,
            Stage::Failed("the analyser returned a report with no name".into()),
        );
    };

    let drawn: Option<Arc<Figures>> =
        serde_wasm_bindgen::from_value::<Figures>(get(&analysed, "figures"))
            .ok()
            .map(Arc::new);
    run.tracks.update(|tracks| {
        if let Some(track) = tracks.get_mut(index) {
            track.export_name = Some(export_name.clone());
            track.report_json = Some(report_json.clone());
            track.figures = drawn.clone();
        }
    });
    run.set_stage(index, Stage::Done(Arc::new(report)));
}

/// Hand the kept tracks to the device builder and ask for the image.
///
/// One pass, because the builder takes a reference to each track's file and
/// reads it when the image is written. Nothing is extracted twice and nothing
/// is held: a track left off costs the run nothing but the row it sits in.
pub async fn build_image(run: Run, pool: Rc<Pool>) {
    let device = run.device.get_untracked();
    let tracks = run.tracks.get_untracked();
    let kept: Vec<bool> = standing(&tracks, device.format)
        .into_iter()
        .map(|standing| standing == Standing::Kept)
        .collect();
    let total = kept.iter().filter(|kept| **kept).count();

    if total == 0 {
        return run
            .phase
            .set(Phase::Failed("no tracks are ticked for the image".into()));
    }

    run.phase.set(Phase::Building { gathered: 0, total });

    if let Err(error) = pool.call_device("device-open", JsValue::UNDEFINED).await {
        return run.phase.set(Phase::Failed(error));
    }

    let mut gathered = 0usize;
    for (track, _) in tracks.iter().zip(&kept).filter(|(_, kept)| **kept) {
        // The name the format produces rather than the one the analyser
        // returned, so the rule for what a track is called lives in one place
        // and the row shows what the image holds. Both are `Some` here: a track
        // with either missing is not in `kept`.
        let export_name = track.image_name(device.format).unwrap_or_default();
        let report_json = track.report_json.clone().unwrap_or_default();
        if let Err(error) = pool
            .call_device(
                "device-add",
                object(&[
                    ("exportName", JsValue::from_str(&export_name)),
                    ("file", track.audio.clone().into()),
                    ("report", JsValue::from_str(&report_json)),
                    ("format", JsValue::from_str(device.format.wire())),
                ]),
            )
            .await
        {
            return run.phase.set(Phase::Failed(error));
        }

        gathered += 1;
        run.phase.set(Phase::Building { gathered, total });
    }

    let reply = pool
        .call_device(
            "device-image",
            object(&[
                ("label", JsValue::from_str(&device.label)),
                ("playlist", JsValue::from_str(&device.playlist)),
                ("date", JsValue::from_str(&device.date)),
                ("target", JsValue::from_str(device.target.wire())),
            ]),
        )
        .await;

    match reply {
        Ok(reply) => {
            let image: web_sys::Blob = get(&reply, "file").unchecked_into();
            let bytes = get(&reply, "bytes").as_f64().unwrap_or_default() as usize;
            match object_url(&image) {
                Ok(url) => run.phase.set(Phase::Ready { url, bytes }),
                Err(error) => run.phase.set(Phase::Failed(error)),
            }
        }
        Err(error) => run.phase.set(Phase::Failed(error)),
    }
}

/// A URL for a file that never left the browser.
///
/// The image is a file in the origin-private filesystem and this is a reference
/// to it, so clicking the link streams it off disk rather than out of a copy in
/// the tab.
fn object_url(image: &web_sys::Blob) -> Result<String, String> {
    web_sys::Url::create_object_url_with_blob(image)
        .map_err(|_| "the image could not be given a download link".to_string())
}

/// One entry of one archive, as the worker lists it.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    /// Which archive holds it, by the index `archive-add` gave back.
    archive: usize,
    /// The entry as the archive keys it, directories and all, which is what
    /// `archive-extract` takes back.
    name: String,
    /// The same without the directories, which is the name to measure under.
    file_name: String,
    size: u64,
}

impl Entry {
    /// Put right a name the archive wrote as UTF-8 and the reader took for
    /// code page 437.
    ///
    /// Both fields, because one is what the track is called and the other is
    /// what the archive is keyed by, and a half-repaired entry is one that
    /// either reads wrong or cannot be found.
    fn repair(&mut self) {
        if let Some(name) = undo_cp437(&self.name) {
            self.name = name;
        }
        if let Some(file_name) = undo_cp437(&self.file_name) {
            self.file_name = file_name;
        }
    }
}

/// Recover a name the archive stored as UTF-8 and the reader decoded as
/// code page 437.
///
/// A zip says whether its names are UTF-8 in one flag of one header, and the
/// tools people actually zip with leave that flag clear while writing UTF-8
/// anyway. A reader that believes the flag decodes those bytes as code page
/// 437, so `Beyoncé` arrives as `Beyonc├⌐`: the two bytes of the `é` read as
/// two symbols from a character set nobody has used since DOS.
///
/// The bytes under the mistake are still the UTF-8 they always were, and they
/// are also the key the archive indexes that entry by, so undoing it repairs
/// the name on the stick and the lookup that extracts it in one move.
///
/// `None` when there is nothing to undo. An ASCII name encodes back to itself,
/// and a name genuinely written in code page 437 gives bytes that are not
/// UTF-8, which is what distinguishes the two cases without guessing.
fn undo_cp437(name: &str) -> Option<String> {
    let mut raw = Vec::with_capacity(name.len());
    for character in name.chars() {
        raw.push(cp437_byte(character)?);
    }
    let recovered = String::from_utf8(raw).ok()?;
    (recovered != name).then_some(recovered)
}

/// The byte code page 437 writes a character as.
fn cp437_byte(character: char) -> Option<u8> {
    if character.is_ascii() {
        return Some(character as u8);
    }
    CP437_HIGH
        .iter()
        .position(|known| *known == character)
        .map(|at| 0x80 + at as u8)
}

/// What code page 437 puts in `0x80..=0xFF`.
///
/// The half that differs from ASCII, which is the half a mistaken decode draws
/// from. Written out rather than reached for through a crate: it is a constant
/// from 1981 and it is not going to change.
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', //
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', //
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', //
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', //
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', //
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀', //
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', //
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// A file for a row whose own file has not arrived.
///
/// A row exists from the moment its entry is listed, which is before the
/// extraction that gives it bytes and possibly before a failure that never
/// does. `Track::audio` is not an `Option` for the sake of that window: nothing
/// reads it while the row is `Extracting`, and a row that fails never leaves it.
fn nothing_yet() -> File {
    let parts = js_sys::Array::new();
    File::new_with_u8_array_sequence(&parts, "").expect("an empty file is always constructible")
}
