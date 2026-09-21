//! The analysis, off the main thread.
//!
//! A second binary in this crate rather than a separate build. It links
//! `dubplate-wasm`, so the measurement here is dubplate's own code compiled from
//! the same crates the command-line tool links, and cargo pins which commit that
//! is. Trunk builds it as a worker and names the wrapper after this binary, which
//! is what lets the page reach it at a fixed path while everything else it emits
//! carries a content hash.
//!
//! Nothing large crosses this boundary. A file arrives as a `Blob`, which is a
//! reference to bytes the browser already holds, and leaves the same way: the
//! archive is read out of one, an extracted track is written into a file in the
//! origin-private filesystem, and the image is written into another. A Beatport
//! month is a gigabyte and the address space is four, so the rule is that the
//! only bytes in wasm memory are the ones a decoder is working on.
//!
//! Two things are held rather than passed, and both live in the worker the page
//! reserves for them. `Archives` holds every zip dropped so far, because opening
//! one is a read of its central directory and doing that once per extraction
//! would be a read per track. The device under construction holds a reference to
//! each track's file, which it reads when the image is asked for and not before.
//!
//! Answering is asynchronous because the origin-private filesystem is: every
//! handle comes back through a promise. Messages are answered one at a time in
//! the order they arrive, so a state that is taken out and put back cannot be
//! taken twice.

use dubplate_wasm::{Archives, Device, Source, SyncHandle, analyze, default_options};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

/// The audio formats this build can write onto an image.
///
/// The one line to change when the analysis module grows encoders. Both the
/// answer to `capabilities` and the guard on `device-add` read it, so the page's
/// form and the worker's refusal cannot drift apart: a format named here is
/// offered and accepted, and one left out is not drawn at all.
///
/// `source` is the file as it arrived, copied rather than re-encoded, and it is
/// the only one today. `dubplate-wasm` links symphonia and hound to decode AIFF,
/// FLAC and MP3, and neither of them encodes anything.
const WRITABLE: &[&str] = &["source"];

fn main() {
    console_error_panic_hook::set_once();

    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let worker = Rc::new(Worker {
        scope: scope.clone(),
        state: RefCell::new(State::default()),
        waiting: RefCell::new(VecDeque::new()),
        answering: Cell::new(false),
    });

    let handler = {
        let worker = worker.clone();
        Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            worker.clone().accept(event.data());
        })
    };
    scope.set_onmessage(Some(handler.as_ref().unchecked_ref()));
    // The closure outlives this function, which returns immediately while the
    // worker stays alive waiting for messages.
    handler.forget();

    // Say so, now that there is something to say it to.
    //
    // The loader Trunk generates is `import init from './analysis.js'; await init();`,
    // and that await yields. A message arriving before it returns is dispatched
    // to a worker with no handler and is dropped, silently and for good. Several
    // workers each compiling a couple of megabytes of wasm makes that window
    // seconds wide, so the page waits for this rather than for a guess about how
    // long starting takes.
    let _ = scope.post_message(&object(&[("ready", JsValue::TRUE)]));
}

#[derive(Default)]
struct State {
    archives: Archives,
    device: Option<Device>,
}

/// One worker, and the requests it has not answered yet.
struct Worker {
    scope: DedicatedWorkerGlobalScope,
    state: RefCell<State>,
    waiting: RefCell<VecDeque<JsValue>>,
    answering: Cell<bool>,
}

impl Worker {
    /// Take a request, and start answering if nothing else is.
    ///
    /// Queued rather than answered where it arrives, because answering yields at
    /// every filesystem handle and two answers in flight would each take the
    /// state out of the cell and find the other's work missing.
    fn accept(self: Rc<Self>, request: JsValue) {
        self.waiting.borrow_mut().push_back(request);
        if self.answering.get() {
            return;
        }
        self.answering.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                let next = self.waiting.borrow_mut().pop_front();
                let Some(request) = next else { break };
                self.answer(request).await;
            }
            self.answering.set(false);
        });
    }

    async fn answer(&self, request: JsValue) {
        let id = get(&request, "id");
        let kind = get(&request, "kind").as_string().unwrap_or_default();
        let payload = get(&request, "payload");

        let reply = js_sys::Object::new();
        set(&reply, "id", id);
        match self.handle(&kind, &payload).await {
            Ok(value) => {
                set(&reply, "ok", JsValue::TRUE);
                set(&reply, "payload", value);
            }
            Err(error) => {
                set(&reply, "ok", JsValue::FALSE);
                set(&reply, "error", JsValue::from_str(&error));
            }
        }
        let _ = self.scope.post_message(&reply);
    }

    async fn handle(&self, kind: &str, payload: &JsValue) -> Result<JsValue, String> {
        match kind {
            // What this build can write onto an image, which is a property of
            // the crates it links rather than of the page that draws the form.
            "capabilities" => {
                let formats = js_sys::Array::new();
                for format in WRITABLE {
                    formats.push(&JsValue::from_str(format));
                }
                Ok(object(&[("formats", formats.into())]))
            }

            // Every analysis setting, as the module itself defaults them.
            //
            // The form draws its own numbers first, because it exists before
            // this worker answers anything, and replaces them with these. A
            // default changed in `pipeline` therefore reaches the page without
            // anyone editing the page, which is what keeps a form from going on
            // sending a value the analysis has moved away from.
            "default-options" => default_options().map_err(describe),

            // One more zip, kept open beside the ones already given.
            //
            // `Archives::add` answers with how many tracks the archive
            // contributed, not with where it went. Where it went is the count
            // before it was added, which is what `entries` tags each entry with
            // and what `extract` takes back.
            "archive-add" => {
                let name = get(payload, "name").as_string().unwrap_or_default();
                let file = source(payload, "file")?;
                let mut state = self.state.borrow_mut();
                let index = state.archives.count();
                let tracks = state.archives.add(&name, &file).map_err(describe)?;
                Ok(object(&[
                    ("index", JsValue::from_f64(index as f64)),
                    ("tracks", JsValue::from_f64(tracks as f64)),
                ]))
            }

            // Every audio file in every archive, tagged with which one holds it.
            "archive-entries" => {
                let entries = self
                    .state
                    .borrow_mut()
                    .archives
                    .entries()
                    .map_err(describe)?;
                Ok(object(&[("entries", entries)]))
            }

            // One entry out of one archive and into a file of its own.
            //
            // The decompressor writes straight into the filesystem, so the track
            // is never in wasm memory whole. What goes back is a `File` over what
            // was written, which is a reference the page can hand to `analyze`
            // and later to `device-add` without reading it either time.
            "archive-extract" => {
                let archive = get(payload, "archive").as_f64().unwrap_or_default() as usize;
                let name = get(payload, "name").as_string().unwrap_or_default();
                let slot = get(payload, "slot").as_f64().unwrap_or_default() as usize;

                let scratch = Scratch::open(&format!("track-{slot}")).await?;
                let started = now();
                let written = self
                    .state
                    .borrow_mut()
                    .archives
                    .extract(archive, &name, scratch.handle())
                    .map_err(describe);
                if let Ok(bytes) = written {
                    timed("unpacked", bytes, now() - started);
                }
                // Closed before the file is read: a sync handle is exclusive, and
                // `getFile` on a file still held open fails.
                let file = scratch.finish().await?;
                written?;
                Ok(object(&[("file", file)]))
            }

            "analyze" => {
                let name = get(payload, "name").as_string().unwrap_or_default();
                let file = source(payload, "file")?;
                let figures = get(payload, "figures").as_bool().unwrap_or(false);
                analyze(&name, &file, get(payload, "options"), figures).map_err(describe)
            }

            "device-open" => {
                self.state.borrow_mut().device = Some(Device::new());
                Ok(object(&[("count", JsValue::from_f64(0.0))]))
            }

            "device-add" => {
                let format = get(payload, "format")
                    .as_string()
                    .unwrap_or_else(|| "source".into());
                // Refused rather than quietly written under the wrong name. The
                // page asks `capabilities` first and offers nothing else, so
                // reaching this means the page and the module disagree, which is
                // worth a stopped build.
                if !WRITABLE.contains(&format.as_str()) {
                    return Err(format!(
                        "this build cannot write {format}: the analysis module it links \
                         decodes audio and encodes none."
                    ));
                }
                let export_name = get(payload, "exportName").as_string().unwrap_or_default();
                let file = source(payload, "file")?;
                let report = get(payload, "report").as_string().unwrap_or_default();

                let mut state = self.state.borrow_mut();
                let device = state
                    .device
                    .as_mut()
                    .ok_or("no device is being assembled")?;
                device.add(&export_name, &file, &report).map_err(describe)?;
                Ok(object(&[(
                    "count",
                    JsValue::from_f64(device.count() as f64),
                )]))
            }

            // The finished image, written into a file and handed back as a
            // reference to it. Three gigabytes of FAT32 never enters this module.
            "device-image" => {
                let label = get(payload, "label").as_string().unwrap_or_default();
                let playlist = get(payload, "playlist").as_string().unwrap_or_default();
                let date = get(payload, "date").as_string().unwrap_or_default();
                let target = get(payload, "target").as_string().unwrap_or_default();

                let scratch = Scratch::open("image.img").await?;
                let started = now();
                let written = {
                    let mut state = self.state.borrow_mut();
                    let device = state
                        .device
                        .as_mut()
                        .ok_or("no device is being assembled")?;
                    device
                        .image(scratch.handle(), &label, &playlist, &date, &target)
                        .map_err(describe)
                };
                let file = scratch.finish().await?;
                let written = written?;
                let milliseconds = now() - started;
                timed("wrote the filesystem", written, milliseconds);

                // Dropped here rather than left for the next run: it holds a
                // reference to every track's file.
                self.state.borrow_mut().device = None;
                Ok(object(&[
                    ("file", file),
                    ("bytes", JsValue::from_f64(written)),
                    ("ms", JsValue::from_f64(milliseconds)),
                ]))
            }

            other => Err(format!("unknown request: {other}")),
        }
    }
}

/// A file in the origin-private filesystem, open for writing.
///
/// Held as the pair it takes to use one: the handle the crates write through,
/// and the file handle it came from, which is what answers for the bytes
/// afterwards.
struct Scratch {
    file: JsValue,
    sync: SyncHandle,
}

impl Scratch {
    /// Open one by name, making it if it is not there.
    ///
    /// The same name twice is the same file twice, which is deliberate: a slot
    /// is reused across runs and `extract` truncates what it finds.
    async fn open(name: &str) -> Result<Scratch, String> {
        let directory = root().await?;
        let file = resolve(call(
            &directory,
            "getFileHandle",
            &[
                JsValue::from_str(name),
                object(&[("create", JsValue::TRUE)]),
            ],
        )?)
        .await?;
        let sync = resolve(call(&file, "createSyncAccessHandle", &[])?).await?;
        Ok(Scratch {
            file,
            sync: sync.unchecked_into(),
        })
    }

    fn handle(&self) -> &SyncHandle {
        &self.sync
    }

    /// Close the handle and hand back a `File` over what was written.
    async fn finish(self) -> Result<JsValue, String> {
        let _ = call(self.sync.as_ref(), "close", &[]);
        resolve(call(&self.file, "getFile", &[])?).await
    }
}

/// How long the worker has been running, in milliseconds.
///
/// A worker has no `Window`, so the clock comes off the global scope's own
/// `performance` rather than the one a page reaches through `window`.
fn now() -> f64 {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("performance"))
        .ok()
        .and_then(|performance| {
            js_sys::Reflect::get(&performance, &JsValue::from_str("now"))
                .ok()?
                .dyn_into::<js_sys::Function>()
                .ok()?
                .call0(&performance)
                .ok()?
                .as_f64()
        })
        .unwrap_or(0.0)
}

/// Say what a stage cost, where a person looking for it will find it.
///
/// Writing a filesystem is one call into the analysis module, so there is
/// nowhere inside it to report from and nothing to report until it returns.
/// What can be said is what it was asked to do and how long it took, which is
/// what turns a wait nobody can see into a number somebody can compare.
fn timed(stage: &str, bytes: f64, milliseconds: f64) {
    let rate = if milliseconds > 0.0 {
        format!(
            ", {:.0} MB/s",
            bytes / 1_000_000.0 / (milliseconds / 1000.0)
        )
    } else {
        String::new()
    };
    let megabytes = bytes / 1_000_000.0;
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "dubplate: {stage} {megabytes:.1} MB in {milliseconds:.0} ms{rate}"
    )));
}

/// The directory this worker unpacks into, emptied once per visit.
///
/// Everything written here is scratch: a track pulled out of a zip so the
/// analyser has a file to read, and the image built from the tracks that were
/// kept. None of it is worth a second visit, and all of it is the size of the
/// music, so leaving it behind means a page that quietly fills somebody's disk
/// with gigabytes nothing will ever reclaim.
///
/// One directory holds all of it, which is what makes clearing it one call
/// rather than a walk over names this worker would have to guess. It is removed
/// and remade the first time anything asks for it, so a visit starts empty and
/// last visit's files go with it. Within a visit the files stay, because the
/// page holds a `File` over each one and the device reads them when the image is
/// written.
///
/// Only the worker the page reserves for archives and the device ever asks. If
/// the measuring workers did, one would empty the directory the other was
/// reading out of.
async fn root() -> Result<JsValue, String> {
    thread_local! {
        static ROOT: RefCell<Option<JsValue>> = const { RefCell::new(None) };
    }

    if let Some(directory) = ROOT.with(|root| root.borrow().clone()) {
        return Ok(directory);
    }

    let origin = resolve(call(&storage()?, "getDirectory", &[])?).await?;

    // Last visit's, if the browser kept it. A first visit has nothing to remove
    // and says so by rejecting, which is not a failure worth reporting.
    if let Ok(removing) = call(
        &origin,
        "removeEntry",
        &[
            JsValue::from_str(SCRATCH),
            object(&[("recursive", JsValue::TRUE)]),
        ],
    ) {
        let _ = resolve(removing).await;
    }

    let directory = resolve(call(
        &origin,
        "getDirectoryHandle",
        &[
            JsValue::from_str(SCRATCH),
            object(&[("create", JsValue::TRUE)]),
        ],
    )?)
    .await?;

    ROOT.with(|root| *root.borrow_mut() = Some(directory.clone()));
    Ok(directory)
}

/// Where the unpacked tracks and the image go.
///
/// A directory of its own rather than the root, so clearing it cannot touch
/// anything another page on this origin put there.
const SCRATCH: &str = "dubplate-scratch";

/// The origin-private filesystem, as this worker reaches it.
fn storage() -> Result<JsValue, String> {
    let navigator = get(&js_sys::global(), "navigator");
    let storage = get(&navigator, "storage");
    if storage.is_undefined() {
        return Err(
            "this browser has no origin-private filesystem, which is where \
                    an archive is unpacked and an image is written"
                .into(),
        );
    }
    Ok(storage)
}

/// Call a method on a JavaScript object and hand back the promise it returned.
fn call(target: &JsValue, method: &str, arguments: &[JsValue]) -> Result<js_sys::Promise, String> {
    let function: js_sys::Function = get(target, method)
        .dyn_into()
        .map_err(|_| format!("the filesystem has no {method}"))?;
    let list = js_sys::Array::new();
    for argument in arguments {
        list.push(argument);
    }
    js_sys::Reflect::apply(&function, target, &list)
        .map_err(|error| format!("{method} failed: {}", describe_value(&error)))?
        .dyn_into()
        .map_err(|_| format!("{method} did not return a promise"))
}

async fn resolve(promise: js_sys::Promise) -> Result<JsValue, String> {
    JsFuture::from(promise)
        .await
        .map_err(|error| describe_value(&error))
}

/// One field of a payload, as the file it is.
///
/// Unchecked, because the only thing on the other side of this boundary that
/// answers to a `Blob` is one: the page sends what a person dropped or what
/// `archive-extract` wrote, and a value that is neither fails in the decoder
/// with the file's name on it.
fn source(payload: &JsValue, field: &str) -> Result<Source, String> {
    let value = get(payload, field);
    if value.is_undefined() || value.is_null() {
        return Err(format!("the request carried no {field}"));
    }
    Ok(value.unchecked_into())
}

/// A `JsError` carries its message on the JS side, so reading it means going
/// through the `Error` object it converts into.
fn describe(error: JsError) -> String {
    describe_value(&error.into())
}

fn describe_value(value: &JsValue) -> String {
    get(value, "message")
        .as_string()
        .or_else(|| value.as_string())
        .unwrap_or_else(|| "the analysis failed and said nothing".into())
}

fn get(target: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(target, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED)
}

fn set(target: &js_sys::Object, name: &str, value: JsValue) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(name), &value);
}

fn object(fields: &[(&str, JsValue)]) -> JsValue {
    let object = js_sys::Object::new();
    for (name, value) in fields {
        set(&object, name, value.clone());
    }
    object.into()
}
