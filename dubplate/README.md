# Dubplate on AWS

[dubplate](https://github.com/martinjlowm/dubplate) in a browser tab: drop the
zips Beatport or Bandcamp sent you, or the tracks themselves, watch every one
get measured,
leave off the ones you do not want, download a FAT32 image a CDJ or a Denon deck
will read. Served at **dubplate.martinjlowm.dk**.

Two things live here. `deployment/` is the CDK app: a delegated hosted zone and
a certificate in us-east-1, then a private bucket and a CloudFront distribution
in the `dubplate-web` account. `app/` is the application itself, a Leptos
interface compiled to WebAssembly.

There is no compute in this stack and no endpoint to upload to. The analysis runs
in the visitor's browser, which is a decision about where somebody's music
library goes rather than a way to save money on Lambda.

---

## Tutorial

A first deployment, from a clean checkout to a page you can open. You need the
management account assumed and a checkout of dubplate beside this repository.

1. **Create the account.** The `dubplate-web` account is declared in
   `organization/`, in the Music unit beside `music-storage`.

   ```sh
   just deploy organization
   ```

2. **Reload the organization.** `AWS_ORG` is how every stack here learns the
   account it belongs in, and it does not know about `dubplate-web` until the
   account exists.

   ```sh
   direnv reload
   ```

   The reload fails until step 1 has finished creating the account, and says so.

3. **Build it.** The analysis is a cargo dependency, so there is nothing to
   clone and nothing to build first. `Cargo.toml` at the repository root says
   which checkout of dubplate it resolves to.

   ```sh
   just build-app
   ```

   That writes `app/dist`, which is the whole of what gets uploaded.

5. **Look at it before anybody else does.**

   ```sh
   just serve-app
   ```

   Open `localhost:8080` and drop an archive on it.

6. **Deploy.** The parent zone has to exist first, because this project writes
   its own delegation into it. `domains/README.md` covers that half.

   ```sh
   just deploy domains          # in the domains account, once
   assume aws-mus-dub
   just deploy dubplate         # both stacks, in order
   ```

   `just deploy` passes `*` when given no stack, because this project is the one
   with more than one stack and cdk refuses to guess. `just deploy dubplate
   dubplate-dns` narrows it.

   `dubplate-dns` deploys first: it creates the zone, writes the NS record into
   the parent, and waits for ACM to validate the certificate against it. Then
   `dubplate-site` puts the distribution behind that certificate and points
   `dubplate.martinjlowm.dk` at it.

   The first deployment blocks on validation, which takes a few minutes. It only
   completes once `martinjlowm.dk` is answering from Route 53, so it fails until
   the nameservers have moved.

---

## How-to guides

### Move the analysis to a different commit of dubplate

`[workspace.dependencies]` in the repository's root `Cargo.toml` pins a rev.
Change it and `cargo update -p dubplate-wasm`; `Cargo.lock` records what the
interface was built against either way.

To work on both repositories at once, swap the same line for a path:

```toml
dubplate-wasm = { path = "../music-analyze/tools/rust/dubplate-wasm" }
```

### Build one image from several sources

Drop archives and loose tracks in any order, or choose several at once. Each zip
is listed under "What this image is made of" with what it contributed, and its
tracks are appended to the table rather than replacing it. Loose tracks are one
line between them, because a row there earns its place by being something you
would remove as a unit, and a single track already has a tick of its own.

A track that only exists as the audio of a video is the other thing that lands
here. Drop the `.mp4` or the `.m4a`: the container is read, its audio track is
picked out, and the picture is never decoded. "What the analyser will read" says
which of them decode and which do not.

A release bought twice is the case this is for, and the second copy is left off:
two files cannot share a name in one FAT32 directory, so the earlier archive
wins and the later row says which. Remove an archive and the rows it contributed
go with it, along with any exclusion its arrival caused.

### Leave a track off the image, and fit a stick

Untick it. The count above the table says how many of how many are going on and
what the image will come to; the button under it says the same. A row that is off
is dimmed rather than hidden, because it was still measured and its answer is
still worth reading.

The size is the number to untick against. It is the formula `image::build` uses,
over bytes the page already knows, so it is arithmetic rather than an
impression: built at five tracks and again at eight, the estimate and the file
agreed to the byte. A 32 GB stick holds 32 billion bytes, which is the unit the
estimate is written in for exactly that comparison.

Three things hold a row off the image on their own, and each says so where the
row is.

| It says | What happened |
|---|---|
| nothing | It is on the image. |
| An earlier source already writes this name | Two sources carry the same release. One FAT32 directory holds one of them, and the earlier source wins. |
| Neither database can hold this format | It measured, but no exporter can write a track under that name. An MP4 is the usual case, and re-encoding is what will fix it. |

Unticking after an image has been built revokes the download link, because that
image no longer describes the page. So does changing any device setting. Build
again to get a new one.

### Change the palette

`app/style/tokens.css` is the design system, copied from the personal-site kit
and unmodified. `app/style/shadcn.css` maps it onto the role variables the
shadcn components read. Change a colour in the first and the second follows;
change a mapping in the second and only the components move.

Every triple in `shadcn.css` names the token it came from in a comment, because
the components want HSL and the system is written in hex.

### Find out why a component renders unstyled

Tailwind purges what it cannot see, and the shadcn components' class names are in
the cargo registry rather than in this tree. `app/tailwind.config.js` lists
`../../target/**/leptos-shadcn-*/**/*.rs` for exactly that reason. A component
that renders as unstyled markup usually means it was added to `Cargo.toml`
without a build having pulled its source down yet.

### Serve the page from another hostname

Both stacks name it. `DOMAIN` in `deployment/stacks/dns.ts` is the one place it
is written; the certificate, the distribution's alternate name and the alias
records all read it.

A second hostname on the same distribution needs it in `domainNames` and on the
certificate as a subject alternative name, and a second pair of alias records.

---

## Reference

### Layout

| Path | What it is |
|---|---|
| `deployment/stacks/dns.ts` | The delegated zone, its NS record in the parent, and the certificate. us-east-1. |
| `deployment/stacks/site.ts` | The bucket, the distribution, the response headers policy and the two deployments. |
| `app/index.html` | Trunk's entry point. Names the wasm, the stylesheet and the copied assets. |
| `../Cargo.toml` | The workspace this crate is a member of. The build directory, and the profile that puts every dependency at `opt-level = 3` while this crate stays at `z`. |
| `app/Cargo.toml` | The dependencies, and the two that are named only to turn features on in crates further down: symphonia's MP4 reader and AAC decoder. |
| `app/Trunk.toml` | The build. The `pre_build` hook is what runs Tailwind. |
| `app/tailwind.config.js` | The bridge from the shadcn role classes to the design tokens. |
| `app/style/fonts.css` | The three faces, served from this origin. Generated from the kit bundle. |
| `app/style/tokens.css` | The design system: colour, type, space, radius, shadow, motion. |
| `app/style/shadcn.css` | Those tokens as the HSL triples shadcn components read. |
| `app/style/main.css` | Tailwind's entry, the base layer, and the animations. |
| `app/src/bin/analysis.rs` | The worker: dubplate's crates, built by Trunk as a second binary. |
| `app/src/bridge.rs` | The pool: several workers, one request id per call. |
| `app/src/run.rs` | The whole state machine, from dropped archives to downloadable image. |
| `app/src/options.rs` | Every analysis setting, mirroring `pipeline::AnalysisOptions`, and what the device is written as. |
| `app/src/report.rs` | The fields of `report.json` this page draws. |
| `app/src/components/sources.rs` | The archives a run was built from, and what each contributed. |
| `app/src/components/` | The rest of the interface, on shadcn primitives. |

### What the page sends the worker

| Request | Payload | Reply |
|---|---|---|
| `capabilities` | | `{formats: [...]}`, the audio formats this build can write |
| `archive-open` | the archive's bytes | `{entries: [{name, size}]}` for every audio file in it |
| `archive-extract` | one name | `{bytes}` |
| `archive-close` | | `{}` |
| `analyze` | a name, its bytes, the settings, whether to draw figures | the report, the export name, the figures |
| `device-open` | | `{count: 0}` |
| `device-add` | export name, audio bytes, report JSON, format | `{count}` |
| `device-image` | label, playlist, date, target | `{image}`, the `.img` bytes |

`archive-*` and `device-*` always go to one worker of their own, and `analyze`
fans out over the rest. That worker holds two things too large to send twice: a
Beatport month is a gigabyte, and the device builder holds every track's audio
until the image is asked for. It is not one of the measurers, because extraction
feeds the measurements, and an extraction queued behind a measurement on the same
worker starves every other worker waiting for the next track.

One archive is open at a time, and the page opens each one twice: once to measure
what is in it, and once more for the tracks that were kept.

The name in `device-add` is the one the image holds, extension and all, which
is not always the one `analyze` returned: choosing a format renames every track
to it. The page applies that rule so that the row and the file agree.

### What the analyser will read

Dropped on its own, a file goes straight to `analyze`, which probes the bytes
rather than trusting the name, so the page accepts more than the archive listing
inside the module does. Accepting is not a promise that it decodes.

The page reads the first sixteen bytes before it decides anything, and the name
settles nothing except for a `.wav`, which is taken at its word and costs no read
at all. A zip saved as `.mp3` is opened as an archive; a FLAC called `download`
is measured; a text file called `.mp3` is refused by name before a worker sees
it. Sixteen bytes covers every check, because `ftyp` sits at byte four and the
`WAVE` and `AIFF` tags at byte eight.

| First bytes | Taken as |
|---|---|
| `PK\x03\x04`, `PK\x05\x06`, `PK\x07\x08` | a zip |
| `RIFF` … `WAVE` | WAV |
| `FORM` … `AIFF` or `AIFC` | AIFF |
| `fLaC` | FLAC |
| `ID3`, or eleven set bits | MP3 |
| `OggS` | Ogg |
| `1A 45 DF A3` | WebM or Matroska |
| `ftyp` at byte four | MP4, M4A or MOV |
| anything else | refused |

The name is then corrected to match the bytes, for a zip entry and a dropped file
alike, because both exporters read a track's format out of its name and neither
opens the file. A FLAC that a shop zipped under an `.mp3` name goes on the stick
as a `.flac` and both databases record it as one. Nothing is transcoded and
nothing needs to be: every rename is one where the bytes already are what the new
name says, so what lands there is a valid file of the format it now claims.

A name that is already right is left alone, `.aif` included. A rename that
changes nothing still moves a file the person recognises to one they do not.

A format no exporter can write keeps whatever name it arrived under. Renaming it
would not make it writable, and the row is held off the image with the reason
said against it.

| Container | Codec | Reads | Goes on the image |
|---|---|---|---|
| WAV, AIFF | PCM | yes | yes |
| FLAC | FLAC | yes | yes |
| MP3 | MP3 | yes | yes |
| MP4, M4A, MOV | AAC, ALAC | yes | no |
| WebM, MKA, OGG | Vorbis | yes | no |
| WebM, MKA | Opus | no | no |

The two right-hand columns differ because measuring and writing are different
questions, and the second one is not about what a player can read. Both exporters
pick a track's format out of its name through `collection::Format::from_extension`,
which knows `wav`, `aiff`, `aif`, `flac` and `mp3` and nothing else. A CDJ-3000
will happily play an M4A off a stick, and neither database has anywhere to record
that the track is one, so the page holds those rows off rather than offering a
build that stops halfway.

`run.rs` carries that list of five as its own copy, and that copy is load-bearing.
A name outside it fails the export for the whole image rather than for one track,
so a format added to the exporter and not here is a track this page declines to
write, and one added here and not there is a build that stops and names the file.

Opus is the one gap in the reading column. Symphonia decodes it only through a C
libopus adapter, which does not build for this target, so a WebM whose audio is
Opus is read as far as its codec and refused by name there.

The formats come from `Cargo.toml` rather than from any code in this tree.
`audio` takes symphonia with `mp3` and `aiff`, and `from_encoded_bytes` probes
through `symphonia::default::get_probe()`, which offers whatever symphonia was
compiled with. Naming `isomp4`, `aac` and `alac` in this crate's manifest adds
them to that probe by feature unification, so the terminal and the page decode
the same file the same way with no patch to dubplate's crates.

### What the worker says it can write

`capabilities` answers with the audio formats `device-add` will accept, and the
settings form draws those and nothing else.

Today the answer is `source` alone: the file as the archive held it, copied
rather than re-encoded. `dubplate-wasm` links symphonia and hound, which decode
AIFF, FLAC and MP3 and encode nothing.

So the Audio format field is not in the form at all. A control offering one
option is a question with one answer, and drawing the other three greyed out
asks the reader to work out why a thing they cannot use is there. The field
appears when there is a choice to make, which is when the count of writable
formats passes one.

Both the reply and the guard on `device-add` read one list, `WRITABLE` in
`app/src/bin/analysis.rs`, so the form and the refusal cannot drift apart.
Adding `"wav"` there is what brings the field back, and it is a lie until
`Device::add` can act on the format it is handed.

### What lands in the bucket

Two deployments, split by whether a name changes when its content does.

| Deployment | Holds | Cache |
|---|---|---|
| `hashed` | what Trunk compiled, plus `fonts/` | one year, immutable |
| `verbatim` | `index.html`, `favicon.svg`, and the three files Trunk emits for the worker | `no-cache`, invalidated on deploy |

Trunk hashes what it compiles, so a new build of the interface is a new file
name. The worker is the exception: Trunk names it after its binary rather than
hashing it, which is what lets the page reach it at a fixed path, and a
year-cached `analysis_loader.js` is a visitor running last month's analysis
against this month's page. The fonts sit in the first group because they are the
one verbatim asset that never changes.

### Response headers

The distribution sets a content security policy whose `connect-src` is `'self'`
and whose `default-src` is `'none'`. The tool reads files and talks to nothing,
and that policy is where the claim is enforced rather than asserted.
`'wasm-unsafe-eval'` is in `script-src` because instantiating a wasm module is
what a browser calls eval.

Cross-origin isolation is deliberately **not** set. It is what a threaded wasm
build would need for `SharedArrayBuffer`, nothing here is threaded, and turning
it on would break every subresource that is not served with CORP.

---

## Explanation

### Why the analysis is not in the interface's binary

The measurement is dubplate's own code, compiled to WebAssembly from the crates
the command-line tool links. It could have been a dependency of the Leptos
binary. It is a separate module loaded by a worker instead, for two reasons.

The first is the one that decided it: measuring a track is a second of solid
arithmetic, and a second of arithmetic on the thread that draws the page is a
page that stops drawing. In a worker it is a message, and the pool runs as many
as the machine has cores to spare.

The second is size. The analysis module carries an FFT, two exporters, SQLite and
a FAT32 writer. A visitor who reads the front page and leaves never downloads any
of it.

### Why the measurements run several at a time, and the FFT does not run on the GPU

The pool is as wide as the machine has cores to spare, and a permit is taken
before a track leaves its source and given back when its report arrives. That is
the whole of the parallelism: as many tracks are in flight as there are workers
to measure them, and the extraction loop cannot run ahead and decode the library.

Two other levers were measured on the same six-track archive, in release builds,
three runs each.

| Build | Six tracks |
|---|---|
| everything at `opt-level = "z"` | 5.0 s |
| `"z"` plus rustfft's `wasm_simd` | 5.5 s |
| dependencies at `opt-level = 3` | 0.5 s |
| `opt-level = 3` plus `wasm_simd` | 0.8 s |

The optimization level is the lever and SIMD is not. rustfft's WebAssembly
kernels lost to its scalar ones in both builds and cost 0.3 MB of module, so the
feature is off. The reason is not that SIMD cannot help an FFT; it is that the
FFT is not where the time goes. The tempo comb search, the chroma, and drawing
seven figures per track are ordinary scalar Rust, and `opt-level = "z"` was
crushing all of it.

A GPU would be a worse version of the same mistake, and an expensive one. The
work that a compute shader would win is the transform, which is the part already
fastest; everything downstream of it is branchy search over small arrays, which
is what GPUs are bad at. It would also cost the property the whole project is
built on. The measurement here is dubplate's own crates compiled to WebAssembly,
so the same file measures the same way in a terminal and in a tab. Kernels
written in WGSL would be a second implementation of the analysis, answering to
nothing, and the first time it disagreed with the command line there would be no
saying which was right.

If a run ever does need to be faster than the table above, the order is: draw
fewer figures, then raise the worker clamp, then push the arithmetic into the
crates where both callers get it. The GPU is not on that list.

### Why an archive is read twice

Measuring hands nothing to the device builder. The audio is extracted, measured
and dropped, and what is kept is the report and the name; the archives are read
again when the image is asked for, for the tracks that are still ticked.

That is what an exclusion costs, and it is the whole reason for the second pass.
The single-pass version has to add every track to the builder as it is measured,
because the alternative is holding a decoded library in the tab, and
`Device::add` is one way: once a track is in, nothing takes it back out. So the
choice was between paying for the tick box in memory and paying for it in a
second extraction, and a second extraction is the cheaper of the two by an order
of magnitude.

The archive itself is a `File` handle between the passes rather than a gigabyte
of it. The browser already has the zip on disk, and reading it at the moment it
is needed is what keeps four dropped archives from being four gigabytes in the
tab.

### Why the size is estimated rather than measured

An image is built once and it takes as long as it takes, so finding out it is
two gigabytes over the stick is a thing to find out before the build rather than
after. The estimate is `image::build`'s own formula over what the page already
holds, and only one of its terms is inexact.

The audio is exact: a zip entry carries its uncompressed size, a loose file
carries its own, and that is what lands under `/Contents`. The slack is exact,
because twelve percent and 64 MB and a 128 MB floor are constants copied from the
builder. The databases are the estimate, and they were measured rather than
reasoned about: images built here from eight two-minute tracks and from four came
to 385 KB fixed plus 64 KB per track, which is the Engine schema and the
rekordbox skeleton, and then the beat grid and the waveform each player draws.
Charged per second rather than per track, because that is what the second half
scales with.

Copying four constants out of another crate is a duplication that a version bump
can silently break. It is the right trade anyway: the alternative is building the
image to find out how big the image is.

### Why the settings form states what every knob does

A number here changes an answer. The metrical floor is the difference between
reporting 63 BPM and reporting 126 for the same track, and neither is a lie: one
of them applies a rule about how dance music is counted. A form that lets you
change that without saying so produces answers nobody can defend, so every field
carries the sentence from its own flag's documentation.

### Why the charts are hand-written SVG

The analyser draws its own figures as SVG for reasons it states: the axis labels
stay selectable and no chart engine is in the crate graph. The two
collection-level pictures here follow it. They also inherit the page's colours,
which a canvas-based library would not, and which is what makes them work in both
themes without a second palette.

### Why nothing is uploaded

The natural shape of this application is an upload endpoint and a queue. That
version is easier to build, easier to parallelise and can encode formats this one
cannot. It also means somebody's music library sits on somebody else's disk, and
a stack that has no server is a claim that can be checked rather than a promise
in a privacy policy. The content security policy above is where it is checked.
