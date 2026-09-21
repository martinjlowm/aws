//! Talking to the workers that hold the analysis.
//!
//! The analysis is the `analysis` binary in this crate, which links dubplate's
//! crates and is built by Trunk as a worker. The page runs several of them and
//! reaches each by message, because measuring a track is a second of solid
//! arithmetic and a second of arithmetic on the thread that draws the page is a
//! page that stops drawing.
//!
//! Each request carries an id and each reply carries it back, so a pool of
//! workers needs no queue discipline: whichever is free takes the next request,
//! and the reply finds its own caller.

use js_sys::{Object, Reflect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

/// How many tracks to measure at once.
///
/// One fewer than the cores, so the thread drawing the page keeps one. Clamped
/// at the top because past a handful the decoded audio, not the arithmetic, is
/// what runs out: a track is held from the moment it is extracted until its
/// report comes back, and this is how many are held at once.
pub fn worker_count() -> usize {
    let cores = web_sys::window()
        .map(|window| window.navigator().hardware_concurrency() as usize)
        .unwrap_or(2);
    cores.saturating_sub(1).clamp(1, 8)
}

type Pending = Rc<RefCell<HashMap<u32, Box<dyn FnOnce(Result<JsValue, String>)>>>>;

/// Where an unsolicited message goes.
///
/// A worker writing an image posts as it goes, so those arrive with no request
/// id and answer nobody. They are handed here instead.
type Watching = Rc<RefCell<Option<Box<dyn Fn(JsValue)>>>>;

/// One worker and the requests it has not answered yet.
struct Slot {
    worker: Worker,
    /// Requests sent and not yet answered. Depth rather than a boolean, because
    /// the reply carries its own id and nothing here has to be in order.
    outstanding: Rc<RefCell<usize>>,
    /// Whether the worker has said it can be spoken to.
    ///
    /// A worker is not listening the moment it is constructed. Its module is
    /// still evaluating, and a message that arrives first is dispatched to a
    /// worker with no handler and dropped for good. So nothing is sent until it
    /// says otherwise, and anything asked for before then waits below.
    ready: Rc<RefCell<bool>>,
    queued: Rc<RefCell<Vec<JsValue>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::Event)>,
}

/// The workers that measure, and the one that holds the archive and the device.
pub struct Pool {
    /// The measurers. `analyze` fans out over these and nothing else does.
    slots: Vec<Slot>,
    /// Told about progress a worker reports without being asked.
    watching: Watching,
    /// The archive and the device under construction both live here.
    ///
    /// Its own worker rather than the first of the pool. Extraction feeds the
    /// measurements, so an extraction queued behind a measurement on the same
    /// worker starves the other workers waiting for the next track.
    device: Slot,
    pending: Pending,
    next_id: RefCell<u32>,
}

impl Pool {
    /// Start `worker_count()` workers.
    ///
    /// Module workers, because the loader shim Trunk emits uses `import`. Every browser that
    /// runs wasm at all supports them.
    pub fn new() -> Result<Rc<Pool>, JsValue> {
        let pending: Pending = Rc::new(RefCell::new(HashMap::new()));
        let watching: Watching = Rc::new(RefCell::new(None));
        let mut slots = Vec::new();

        // One more than there are measurers: the extra holds the archive and
        // the device, and spends most of a run idle.
        for _ in 0..worker_count() + 1 {
            let options = WorkerOptions::new();
            options.set_type(WorkerType::Module);
            // Trunk names a worker wrapper after its binary rather than hashing
            // it, so this path is fixed while everything else it emits is not.
            let worker = Worker::new_with_options("/analysis_loader.js", &options)?;

            let outstanding = Rc::new(RefCell::new(0usize));
            let ready = Rc::new(RefCell::new(false));
            let queued: Rc<RefCell<Vec<JsValue>>> = Rc::new(RefCell::new(Vec::new()));
            let on_message = {
                let pending = pending.clone();
                let watching = watching.clone();
                let outstanding = outstanding.clone();
                let ready = ready.clone();
                let queued = queued.clone();
                let worker = worker.clone();
                Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                    let data = event.data();

                    // The worker's first message says it is listening. Whatever
                    // was asked for before that goes now, in the order it was
                    // asked.
                    if get(&data, "ready").as_bool().unwrap_or(false) {
                        *ready.borrow_mut() = true;
                        for message in queued.borrow_mut().drain(..) {
                            let _ = worker.post_message(&message);
                        }
                        return;
                    }

                    // Progress, which nobody asked for and nothing is waiting
                    // on. Handled before the bookkeeping below, because that
                    // bookkeeping is about replies and this is not one: counted
                    // as a reply it would report a request answered that is
                    // still running.
                    if get(&data, "progress").as_bool().unwrap_or(false) {
                        if let Some(watcher) = watching.borrow().as_ref() {
                            watcher(get(&data, "update"));
                        }
                        return;
                    }

                    // One borrow, not two. `*x.borrow_mut() = x.borrow()...`
                    // holds the shared guard until the end of the statement and
                    // panics taking the exclusive one, which in a wasm build
                    // compiled to abort is a bare `unreachable` with no message.
                    {
                        let mut outstanding = outstanding.borrow_mut();
                        *outstanding = outstanding.saturating_sub(1);
                    }
                    let Some(id) = get(&data, "id").as_f64() else {
                        return;
                    };
                    // Taken out of the map before it is called: a reply that
                    // starts another request must not find its own entry still
                    // there.
                    let Some(reply) = pending.borrow_mut().remove(&(id as u32)) else {
                        return;
                    };
                    if get(&data, "ok").as_bool().unwrap_or(false) {
                        reply(Ok(get(&data, "payload")));
                    } else {
                        reply(Err(get(&data, "error").as_string().unwrap_or_else(|| {
                            "the worker failed and said nothing".into()
                        })));
                    }
                })
            };
            worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            // A worker that dies takes its outstanding requests with it, and
            // without this the page waits for a reply that is never coming.
            let on_error = {
                let pending = pending.clone();
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                    let waiting: Vec<u32> = pending.borrow().keys().copied().collect();
                    for id in waiting {
                        if let Some(reply) = pending.borrow_mut().remove(&id) {
                            reply(Err("the analysis worker stopped".into()));
                        }
                    }
                })
            };
            worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            slots.push(Slot {
                worker,
                outstanding,
                ready,
                queued,
                _on_message: on_message,
                _on_error: on_error,
            });
        }

        // The last one built becomes the archive worker, so the measurers are
        // the contiguous front of the list.
        let device = slots.pop().ok_or_else(|| JsValue::from_str("no workers"))?;

        Ok(Rc::new(Pool {
            slots,
            device,
            watching,
            pending,
            next_id: RefCell::new(0),
        }))
    }

    /// Hear what a worker says while it is busy.
    ///
    /// One watcher at a time, because one thing at a time reports: the image
    /// being written is the only work here long enough to be worth watching.
    pub fn watch(&self, watcher: impl Fn(JsValue) + 'static) {
        *self.watching.borrow_mut() = Some(Box::new(watcher));
    }

    /// Stop listening.
    pub fn unwatch(&self) {
        *self.watching.borrow_mut() = None;
    }

    /// How many tracks can be measured at once.
    pub fn analysers(&self) -> usize {
        self.slots.len()
    }

    /// Send a request to the least busy measurer and await its reply.
    pub async fn call(&self, kind: &str, payload: JsValue) -> Result<JsValue, String> {
        let slot = self
            .slots
            .iter()
            .min_by_key(|slot| *slot.outstanding.borrow())
            .ok_or_else(|| "no analysis workers were started".to_string())?;
        *slot.outstanding.borrow_mut() += 1;
        self.send(slot, kind, payload).await
    }

    /// Send a request to the worker that holds the archive and the device.
    ///
    /// Always the same one, and never one that measures. The builder holds every
    /// track's audio, and spreading it over the pool would mean sending each
    /// track twice and holding the library in several places at once.
    pub async fn call_device(&self, kind: &str, payload: JsValue) -> Result<JsValue, String> {
        let slot = &self.device;
        *slot.outstanding.borrow_mut() += 1;
        self.send(slot, kind, payload).await
    }

    async fn send(&self, slot: &Slot, kind: &str, payload: JsValue) -> Result<JsValue, String> {
        let id = {
            let mut next = self.next_id.borrow_mut();
            *next = next.wrapping_add(1);
            *next
        };

        let message = Object::new();
        set(&message, "id", JsValue::from_f64(f64::from(id)));
        set(&message, "kind", JsValue::from_str(kind));
        set(&message, "payload", payload);

        let (sender, receiver) = futures_channel();
        self.pending.borrow_mut().insert(id, sender);

        if *slot.ready.borrow() {
            if let Err(error) = slot.worker.post_message(&message) {
                self.pending.borrow_mut().remove(&id);
                return Err(format!("could not reach the analysis worker: {error:?}"));
            }
        } else {
            slot.queued.borrow_mut().push(message.into());
        }
        receiver.await
    }
}

/// A one-shot channel over a `oneshot`-shaped closure, without the dependency.
///
/// The pool stores the sending half as a boxed `FnOnce`; this is the awaiting
/// half. Small enough that a channel crate would be a dependency to move one
/// value across one await.
fn futures_channel() -> (
    Box<dyn FnOnce(Result<JsValue, String>)>,
    impl std::future::Future<Output = Result<JsValue, String>>,
) {
    let state: Rc<RefCell<(Option<Result<JsValue, String>>, Option<std::task::Waker>)>> =
        Rc::new(RefCell::new((None, None)));

    let sender = {
        let state = state.clone();
        Box::new(move |value: Result<JsValue, String>| {
            let waker = {
                let mut state = state.borrow_mut();
                state.0 = Some(value);
                state.1.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }) as Box<dyn FnOnce(Result<JsValue, String>)>
    };

    let receiver = std::future::poll_fn(move |context| {
        let mut state = state.borrow_mut();
        match state.0.take() {
            Some(value) => std::task::Poll::Ready(value),
            None => {
                state.1 = Some(context.waker().clone());
                std::task::Poll::Pending
            }
        }
    });

    (sender, receiver)
}

pub fn object(fields: &[(&str, JsValue)]) -> JsValue {
    let object = Object::new();
    for (name, value) in fields {
        set(&object, name, value.clone());
    }
    object.into()
}

fn set(target: &Object, name: &str, value: JsValue) {
    let _ = Reflect::set(target, &JsValue::from_str(name), &value);
}

pub fn get(target: &JsValue, name: &str) -> JsValue {
    Reflect::get(target, &JsValue::from_str(name)).unwrap_or(JsValue::UNDEFINED)
}
