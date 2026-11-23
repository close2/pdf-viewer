# RFC 0015 — The out-of-scope rows, planned: what playing and rendering clause 13 would take, done safely

Status: **proposed** — six questions in section 12; `doc/questions/Q946` asks them. Nothing in this
document lifts an exclusion, changes a ledger status, a `Cargo.toml` or a line of code that ships.
`CLAUDE.md`'s clause 13 exclusion is the owner's, ratified in `doc/scope-ratifications.md` with a
reopening trigger for each half; lifting one is the owner's decision on the answers below.
Round: 2124 — the planning step the owner set on 2026-10-28, verbatim: "when all other rows are
moved to final states, we start with the out-of-scope rows. We will start planing, what it would
actually mean, to implement this in a safe way." The owner confirmed that roadmap on 2026-11-19. The
ledger has no `partial` row since round 2117, and its `inapplicable` rows were re-read in round 2119.
Builds on: RFC 0009 section 3 (clause 13's four tiers A to D, the libraries, the sandbox argument —
not repeated here), RFC 0012 (the two decoders audited and the presentation surface sized), RFC
0008 sections 6.2 and 6.6 (the confined-worker pattern and the launch-path ordering, built since as
`pdf-script-worker`), ADRs 0014, 1946, 2382, 2400 and 2428, `doc/questions/A254`, `A632`, `A649`,
`deny.toml`, `doc/stack.md`.
Asks: **Q946** (section 12).

`§N` in this document is a clause of ISO 32000-2 and nothing else. *ECMA-363* is the Universal 3D
file format; *ISO 14739-1* is PRC; *ISO 21757-1* is ECMAScript for PDF. The tree holds none of the
three texts. ECMA-363 is freely published; ISO 14739-1 is a paid standard; and the owner decided on
2026-08-28 not to buy ISO 21757-1, with Adobe's *JavaScript for Acrobat API Reference* as the working
source for scripts. What this document says of those three formats is therefore read from ISO
32000-2's own references to them and from the candidates' published sources, and it says so where it
matters. Figures marked *measured* were run on 2026-11-19 (section 9); every round count and line
count is an *estimate*, by analogy to the nearest thing the tree has built, and is labelled so.

---

## 0. In one paragraph

The 51 `out-of-scope` rows are not one thing but six, and they are unequal. **27 are §13.6's 3D
artwork, 18 are §13.7's rich media, four are clause 13's general rows and two are clause 12's
actions into them**; audio and video playing, which the exclusion is usually spoken of as, holds
only §13.1, §13.2.1 and §13.2.7.5 — the rest of §13.2, §13.3 and §13.4 are already `implemented` as
far as the hand-off admits. Read against the tree, **the hand-off the exclusion admits is not
finished either**: every window receives `Event::Media` and says "this window starts no player yet,
so nothing plays" — the `Media` level A254 owes, and the start of this machine's own player, are
unbuilt, and they come before any part (section 3). Done safely, every part has one shape: **a
parser or decoder of the document's bytes in a confined worker of its own, spawned on the first
activation the reader's level lets run and never at open, which hands the host only what the host
can present** — samples, frames, or a bounded and validated triangle scene — with the host's
graphics device drawing a scene under the shared-device lock it already takes. Audio is the
smallest part (about nine rounds, estimated) and its trigger needs only the owner's word; video is
about fourteen more on top of audio; U3D is about twenty-four and its trigger is unmet — and, read
against the standard, names the wrong edition: ISO 32000-2 cites ECMA-363's **third** edition and
says it does not cite the fourth; PRC is not plannable to the round until the project holds ISO
14739-1, and has no trigger at all; rich media and the two actions are mostly the other parts'
sum plus a scripting runtime the script exclusion keeps out. The recommendation (section 11) is to
build the owed hand-off now, to open audio on the owner's word, to keep video, U3D and PRC closed
with their triggers corrected, and to leave rich media and the two actions to follow the parts they
are made of.

## 1. What this round was asked, and what it does not do

The brief asks, for each of six parts — playing audio, playing video, rendering U3D, rendering
PRC, rich media, and the two clause 12 rows — five things: what a reader must do clause by clause
and what already ships; the safety shape; the cost in rounds, on the launch path and in the
binaries; whether the reopening trigger is met and what would meet it; and a recommendation, as
one question per part. Sections 4 to 8 answer them part by part after the shape every part shares
(section 4); section 10 reads the triggers; section 12 is the question.

What it does not do: build, lift, or move a row. RFC 0009 section 3.7 warned that "anyone arguing
this on coverage is overselling it", and the counts here bear that out differently from how it was
written: clause 13's own subclauses state 151 obligations on a reader (section 2's count), 102 of
them in §13.6 alone.

## 2. The 51 rows, sorted into the parts

Counted from `doc/conformance/ledger.toml` with `tomllib` on 2026-11-19: 788 `implemented`, 51
`out-of-scope`, 31 `inapplicable`, 11 `departed`, 2 `writer-side`. Every out-of-scope row names the
exclusion `clause-13-multimedia`.

| part | rows | count | headings among them |
|---|---|---|---|
| general — the engine's preamble | §13.1, §13.2.1 | 2 | 0 |
| playing video's screen | §13.2.7.5 (monitor specifier) | 1 | 0 |
| alternate presentations | §13.5 | 1 | 0 |
| 3D artwork (U3D and PRC alike) | §13.6, §13.6.1, §13.6.3 and its 3, §13.6.4 and its 7, §13.6.5, §13.6.6, §13.6.7 and its 1, 2, 3 (with 3.1 to 3.6) and 4 | 27 | 5 |
| rich media | §13.7, §13.7.1, §13.7.2, §13.7.2.2 and its 6, §13.7.2.3, §13.7.2.3.3 to .5, §13.7.2.3.6 and its 3 | 18 | 5 |
| clause 12's actions into them | §12.6.4.16 Go-To-3D-View, §12.6.4.18 Rich-Media-Execute | 2 | 0 |

The obligations, extracted verbatim from `doc/md/` and tagged by whom they bind (a reader, or the
file), are in round 2124's working file; the counts of those binding a *reader*:

| subclauses | binding a reader | binding the file |
|---|---|---|
| §12.6.4.16 | 2 | 3 |
| §12.6.4.18 | 0 (one more binds "the annotation's instance of the 3D ECMAScript engine") | 7 |
| §13.1, §13.2.1, §13.2.7.5 | 1, and 3 that could be read either way | 2 |
| §13.5 | 3 | 6 |
| §13.6 (all) | 102 | 232 |
| §13.7 (all) | 40 | 37 |

Two things follow from the sort. **Audio and video playing is three rows**, because RFC 0009's tier
A and ADR 1946's reading moved the rest of §13.2, §13.3 and §13.4 to `implemented` for every
requirement the amended entry leaves in scope; opening Part 1 or 2 would *add* the playing to
those `implemented` rows (each note says the playing rests on the exclusion) as much as it would
move the three. And **3D is over half the population and two thirds of the obligations**, so a
plan judged by rows moved is a plan for §13.6.

## 3. What ships today, read off the tree — and what the admitted half still owes

Read on 2026-11-19 in `crates/pdf-model`, `crates/viewer-core`, `crates/viewer-host` and the
windows:

- **The still page, whole.** Every `3D`, `RichMedia`, `Movie`, `Screen` and `Sound` annotation's
  `/AP` is placed under §12.5.5; a 3D or rich media annotation without one is refused by §13.6.2's
  and §13.7.2.1's own requirement (§13.6.2 and §13.7.2.1 `implemented`, ADR 1946). Every 3D
  annotation stays in §13.6.2's inactive state, which is "the default initial state".
- **The poster.** Table 306's stream-form `/Poster` is drawn (§13.4 `implemented`); the boolean
  form, "retrieved from the movie file", is the decoder the exclusion keeps out.
- **The judgement.** `pdf_model::media::judge` runs §13.2.2's viability over the rendition tree for
  the one player this program has, the hand-off (§13.2.2 `implemented`, ADR 1960); §13.3's sound and
  §13.4's movie files cross the same hand-off (ADR 2382), a sound's own samples written as RIFF
  WAVE.
- **Rich media's assets.** Table 341's `/Assets` is read, listed and saved, "never handed to a
  player" (§13.7.2.3.1 and .2 `implemented`, ADRs 2399, 2410).
- **The actions.** `GoTo3DView` and `RichMediaExecute` are refused by name in `action.rs`.

**What the admitted half still owes.** The exclusion admits "the media file saved or handed to this
machine's own player under *A document's restrictions are the reader's to set*, `ask` by default".
The model side is built: `viewer_core::Event::Media` reaches every window once §13.2.2 calls a clip
viable and Table 286's `/TF` (`pdf_model::restriction::Operation::HandOverMedia`) allows it. **No
window starts a player.** `viewer_host::handed_media` — called by `quorra`'s `dispatch.rs`,
`quorra-confined`'s `events.rs` and `viewer_host::native_window`, which the GTK and Qt windows
share — says, in every window, "this window starts no player yet, so nothing plays", and its doc
comment names why: the level for starting that player, "RFC 0009 section 11's question 3,
`doc/questions/A254` — is not built". A254's `Owes:` line lists it: "the `Media` level in
`viewer_host::policy` and the three windows' menus, the hand-off through the portal". This is not
an out-of-scope row and needs no lifting; it is **owed under the current rules**, it is the step
every part's activation would ask through, and section 11 puts it first. Its cost, estimated by
the `Links` level it copies (ADR 1155, `may_open_uri`): **two rounds** — the level and its four
answers in `viewer_host::policy` with the portal's `OpenURI` (a temporary file for an embedded
clip, under Table 286's `/TF`), then the menus in `quorra`, GTK, Qt and the C ABI.

## 4. The shape every part shares

RFC 0009 section 3.6 stated the sandbox argument once — "A decoder for untrusted media is clause
13's attack surface and goes where the JBIG2 and JPEG 2000 decoders went" — and RFC 0008 section
6.2 built its own instance since: `pdf-script-worker`, confined by
`pdf_sandbox::lockdown::Profile::Script`, spawned on the first trigger, killed and named on a missed
deadline. Every part below is that pattern with a different payload, so it is written once here.

### 4.1 One worker per payload, never the view worker

`pdf-view-worker` holds the page, and RFC 0008 section 6.2's reason applies unchanged: "the budget
must bound the hostile thing without bounding the work the user is waiting for". A codec or a 3D
parser that exhausts its ceiling must take a process that held nothing but itself. Two new workers,
each its own binary so the host never links a decoder or a parser:

| worker | payload | lockdown profile, new | ceiling (estimate) | deadline |
|---|---|---|---|---|
| `pdf-media-worker` | a clip's bytes in; PCM samples or video frames out | `Profile::Media`: one decode thread, or two with a demuxer thread if measurement asks for it; the clip arrives as frames on the wire, no descriptor received except the shared-memory ring of section 4.3; no executable memory | audio: tens of MiB; video: 1 GiB (RFC 0012 section 6: a 1080p RGBA frame is 8.3 MB, a decoder's reference frames many times over) | per chunk, sized from the stream's own rate: a decoder that falls behind real time by more than a bound is stopped and the clip handed off, said out loud |
| `pdf-scene-worker` | a 3D stream's bytes in; a validated scene out | `Profile::Scene`: one thread, no descriptor, no executable memory — `Profile::Script`'s list | measured on a held corpus before it is fixed; a decompression ratio cap and the scene bounds of section 4.4 hold it independently | per stream, as `pdf-sandbox-worker`'s per-image deadline |

What crosses into either worker is one object's bytes, already decrypted and unfiltered by the view
worker — never the document. Landlock leaves each with no filesystem, seccomp-BPF with no network
and no `execve`, `RLIMIT_NOFILE` at zero where no descriptor is passed. Each is spawned on the first
activation the reader's level lets run (section 4.6), on `confined-transport`'s wire, which "carries
a kind byte and a length and does not know what either means".

### 4.2 The levels

Every part's activation is a program-like act on the document's word, so each asks one policy, in
the place a host can supply, under the four levels `CLAUDE.md` names — `off`, `on`, `ask`, `warn`.
The `Media` level of section 3 is the first; playing in this program rather than handing off is a
second answer of the *same* level (*hand off*, *play here*, *ask*, *refuse*) rather than a new
switch, so the reader keeps the one question A254 settled. 3D activation is a level of its own,
`ask` by default, because Table 310's `PO` and `PV` activate on the document's word with no click.

### 4.3 The wire

RFC 0012 section 5.1 found the wire one reply per request, copying a raster, and video a stream at
93 MB/s of 4:2:0 at 1080p30. Audio fits the present shape: 0.4 MB/s of PCM in chunks a request
each. Video needs a **streaming arm** in `confined-transport` and a **shared-memory ring** the host
creates as a `memfd` and passes once over the socket's `SCM_RIGHTS` — the worker maps it and writes
YUV frames into it; the host converts to RGB on the device. That adds `mmap` of one passed
descriptor to `Profile::Media`'s list, narrowed by argument as `Profile::Interpreter`'s `fcntl` is
(`MAP_SHARED`, no `PROT_EXEC`). A scene crosses once, as frames of the ordinary wire.

### 4.4 The scene: what a 3D worker may hand the host

The host never parses U3D or PRC. It receives a **scene in a format this tree defines**, every
field bounded and checked again on arrival as a compromised worker's output would have to be
(`pdf-script-worker`'s `form_wire.rs` holds the same rule: "a compromised worker does not keep its
budget"): node tree with 4×4 transforms and the names §13.6.4.7's node dictionaries address;
per-node triangle, line and point lists — positions, normals, texture coordinates, `u32` indices
checked in range; materials (§13.6.4.4's render modes need ambient, diffuse, specular, emissive,
opacity); textures as RGBA8 under the image budget `viewer_core` already holds; lights; the
artwork's own default view and animation keyframes. Bounds: a triangle count, a node depth and
count, a texture pixel total, each named in the refusal that hits it. U3D's continuous-level-of-
detail mesh is resolved in the worker to the resolution the bounds admit; the host never sees
ECMA-363's bit-stream.

### 4.5 The graphics device

3D is drawn on the host's `wgpu` device, the one page one is drawn on, so it takes nothing new from
the shared card's rules: `raster_gpu`'s section hook holds `/run/lock/gpu.lock` for the device's
life (ADR 2675), and a 3D annotation's frames are that device's work. The pipeline is a depth-tested
mesh pipeline, near RFC 0013's hardware lane (a page's fills as triangles under a render bundle,
built for the moving view on A662). Its shaders are compiled **on first activation, never in
bring-up's set** — principle 2's "nothing on the launch path waits for warmth". `render-cpu` draws
the same scene as the **oracle**: neither pdf.js nor poppler renders 3D (ADR 2400 section 3), so the
cross-backend comparison is this tree's two backends and nothing else, and the CPU mesh rasteriser
(a depth buffer, perspective-correct interpolation, the fifteen render modes) is a cost of the part,
not optional. Acrobat, the only other reader that draws 3D, is evidence about a reading and never
the definition of one.

### 4.6 The launch path

Nothing in any part is on it. Every worker is spawned on first activation; a document with a 3D
annotation draws its normal appearance at open exactly as today. Table 310's `PO` ("as soon as the
page containing the annotation is opened") and `PV` are reconciled as RFC 0008 section 6.6
reconciled §12.6.4.17's open scripts: **page one is presented first** from the appearance streams
the file carries, and activation runs after the first present, the scene replacing the normal
appearance when it is ready. §13.6.2's "Inactive (the default initial state): the annotation
displays the annotation's normal appearance" makes the order the clause's own. Cost to
time-to-first-page: zero for every document; `open_cost` reports activation as its own line.

### 4.7 Fuzzing from the first parser commit

Each part's first commit that parses a byte adds a `fuzz/` target over the worker's entry point and
a seed corpus (principle 3), a crasher becomes a regression test, and the scene and media wire
formats get a target each on the host's receiving side — the worker is assumed compromised there.
`fuzz/` is its own workspace: `cd fuzz && cargo check --bins` is owed by every public-type change.

### 4.8 Dependencies, and what `deny.toml` and principle 3 ask of each

`deny.toml` admits Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, MIT, Unicode-3.0, Zlib and CC0-1.0.
Principle 3: "Any C dependency … must be isolated in the sandboxed process and justified in
writing". Section 9 holds the measured table; the readings per part are in each part's section.

## 5. Part 1 — playing audio

**What a reader must do.** §13.3: "Sound players shall convert between formats, downsample rates,
and combine channels as necessary", over the formats it says processors "should support"; §13.2.5's
play parameters (volume, duration, repeat count, `/A` auto-play) and Table 307's `Synchronous` for a
movie's sound; §13.2.1's "Embedded media … may be played with a variety of player software", which
is the row that moves. §13.2.2's viability is already computed; it would gain this program as a
second player, so `MH` options a player is told become honourable here rather than failing.

**What ships.** The judgement, the sound's own samples as RIFF WAVE, the hand-off event — and, per
section 3, no window starts anything yet.

**Safety.** `pdf-media-worker` decodes; the host plays PCM. §13.3's own formats are a loop (raw,
signed, µ-law, A-law). Compressed audio in a rendition is overwhelmingly AAC in MP4: `rusty_aac`
with `default-features = false` (the scalar build), recorded by ADR 2428; an MP4 demuxer audited as
RFC 0012 section 3 audited the decoders before it is taken. The new edge is the **audio sink in each
window**: `cpal` for `quorra` and GTK, which links `libasound` through `alsa-sys` — C, but in the
host on the *output* side, receiving PCM this program decoded and parsing nothing the document
wrote, the same standing as the toolkits' own C; `Qt6::Multimedia`'s `QAudioSink` for Qt; WinUI's
audio graph for the fourth window. The justification principle 3 asks for is that sentence, written
in the part's first ADR. GTK's `MediaFile` and any system framework fed the document's bytes stay
refused, RFC 0009 section 3.6's rule.

**Cost (estimated, in order).** (1) `pdf-media-worker` and `Profile::Media`, with its fuzz target,
decoding §13.3's PCM and µ-law; (2) the audio sink in `quorra`, behind the `Media` level's *play
here*; (3) GTK; (4) Qt; (5) the C ABI and WinUI; (6) a media clock in `viewer-host` — the audio
device's sample clock as master, which video needs next; (7) §13.2.5's play parameters and Table
307's `Synchronous`; (8) the MP4 demuxer's audit; (9) AAC through `rusty_aac`, with a conformance
gate over a held list of RFC 0012's fetched streams. **Nine rounds.** Launch path: none — the sink
is opened on first play. Binaries: the worker is a new binary; each window gains its sink — `cpal` adds 105 KiB
to a stripped release binary (section 9, measured).

**Trigger.** "Decoding or presenting audio and video" reopens on "the owner's decision to build the
player, on an H.264 and an AAC decoder admissible under `deny.toml`". The decoder half is met (RFC
0012, A649). The decision is not given. Audio alone is not separately triggered: the exclusion
names "audio and video" together, so opening Part 1 alone is a narrower decision than the trigger
contemplates, and the question says so.

## 6. Part 2 — playing video

**What a reader must do.** Everything of Part 1, and §13.2.6's screen parameters — the floating
window's size, position, title and resize rules, `/O` the offset dictionary, §13.2.6.3's timespans;
§13.2.7.5's monitor specifier, "an integer that shall identify a physical monitor attached to a
system" and Table 304's seven values, value 2's "If no monitor is considered primary, shall treat as
case 0"; Table 307's `ShowControls`, "whether to display a movie controller bar while playing the
movie", and its `Mode`, `Rate`, `Start`, `Duration`, `FWScale`, `FWPosition`; Table 306's boolean
`/Poster`, "retrieved from the movie file".

**Safety.** The media worker as Part 1, with `rusty_h264-decoder` under `default-features = false,
features = ["std"]` — ADR 2428's choice, which keeps out the crate's `#[global_allocator]` and its
SIMD crate (RFC 0012 section 2.2). No candidate decodes interlaced H.264 (15 of the suite's 66
streams), so interlaced video stays on the hand-off, said by name, never a silent region. The ring
of section 4.3 is the one widening of a profile this plan asks for.

**Cost (estimated, in order, after Part 1).** (1) the streaming arm in `confined-transport`; (2)
the shared-memory ring and `Profile::Media`'s narrowed `mmap`; (3) H.264 in the worker, its fuzz
target and the conformance gate; (4) YUV to RGB on the device and in `render-cpu` as the oracle;
(5) to (8) the frame sink per window — `quorra`, GTK, Qt, WinUI (60 to 120 lines each by RFC 0012
section 5's analogy); (9) A/V synchronisation on Part 1's clock; (10) §13.2.6's screen parameters
and the floating window; (11) §13.2.7.5's monitor choice, which needs each toolkit's monitor list
and a colour depth not every toolkit reports; (12) Table 307's controller bar and modes; (13) the
boolean `/Poster`, one decoded frame; (14) §13.4's movie and §12.5.6.18's screen annotation playing
in place. **Fourteen rounds after Part 1's nine.** Launch path: none. Binaries: the decoder lives in
the worker binary only.

**Trigger.** The same as Part 1's, and unmet on the same ground.

## 7. Part 3 — rendering U3D

**What a reader must do.** §13.6.2's active state, "the annotation displays a rendering of the 3D
artwork", with Table 310's activation (`A`, `AIS`, `D`, `DIS`, `TB`, `NP`, `Style`, `Window`,
`Transparent`) and Table 309's `/3DV`, `/3DB`, `/3DU`; §13.6.3.1's default-view precedence, "in the
annotation dictionary, in the 3D stream dictionary, or in the 3D artwork contained in the 3D
stream"; §13.6.3.2's keyframe animation; §13.6.3.3's shared artwork; §13.6.4's views — the camera
(`C2W`, `U3DPath`, `CO`), Table 316's projections and their `OB` scaling, Table 317's background,
Table 319's fifteen render modes, Table 321's lighting schemes, Table 322's cross sections with
their caps, transparency and intersection lines, Table 323's per-node opacity and visibility;
§13.6.5's coordinate systems; §13.6.6's markup "only printed and displayed when the specified view is
the current view"; §13.6.7's measurements, each shown with its view and, on a changed geometry, "a
warning shall be issued to the user". 102 reader obligations in all (section 2).

**One sentence makes U3D separable from PRC.** §13.6.3.1: "PDF processors shall be prepared to
encounter unknown values for Subtype and recover appropriately, which usually means leaving the
annotation in its inactive state, displaying its normal appearance." A processor that reads U3D and
not PRC is conforming on a PRC stream by drawing the normal appearance, which is what every 3D
stream gets today.

**Two things the part does not own.** Table 311's `OnInstantiate`, "an ECMAScript script that shall
be executed by applications that support ECMAScript", and Table 310's `AIS` scripts address
ISO 21757-1's 3D API, which the *script* exclusion keeps out by name ("ISO 21757-1's 3D API under
clause 13"). This program does support ECMAScript, so a stream with `OnInstantiate` is one Part 3
would draw without the script that was to adjust it — reported by name, the clause's NOTE ("falling
back to the normal appearance") being the alternative a level could choose. And **printing**:
Table 310's `/A` says that in non-interactive applications "PO and PV indicate that the annotation
shall be activated when the page is printed or placed", so once Part 3 exists the print path owes the
rendered artwork for `PO` and `PV` annotations, and `render-cpu`'s oracle is what it draws with.

**Safety.** `pdf-scene-worker` (section 4.1) with an in-tree ECMA-363 reader; the scene of section
4.4; the device of section 4.5. ECMA-363's textures are image formats (JPEG and PNG among them), and
the worker decodes them with the decoders `pdf-sandbox-worker` already confines rather than a second
copy. **Dependencies**: section 9's measurement of the one crate, `u3d`; the reader is written
in-tree or taken from it, and either way is `#![forbid(unsafe_code)]` and C-free. `glam`-class
matrix arithmetic is a few hundred lines this tree can own.

**The edition.** The trigger says "ECMA-363's fourth edition whole". ISO 32000-2's introduction says
"This document makes explicit reference to ECMA-363 U3D 3 rd  edition and not the latest U3D 4 th 
edition", its bibliography cites "Standard ECMA-363, Universal 3D File Format, 3rd Edition (U3D) ,
(June 2006)", and §3.67 defines U3D as a "means of reference to ECMA-363, Universal 3D File Format,
3rd Edition (U3D)". The specification is the only source of truth: a U3D stream in a PDF is the third
edition's, and a reader is measured against that. Whether the fourth edition reads every third-
edition stream unchanged is a reading of the two ECMA texts nobody here has done; the trigger should
name the edition the standard names, and the question asks for that correction.

**Cost (estimated, in order).** (1) `pdf-scene-worker`, `Profile::Scene`, the scene wire format
and both fuzz targets; (2) ECMA-363's block structure, file header, declaration and continuation
blocks, and the bit-stream's context-adaptive arithmetic decoder; (3) the node blocks and the
modifier chain; (4) to (6) the continuous-level-of-detail mesh — base mesh, then the progressive
continuation that `u3d`'s README leaves unticked, then its resolution update; (7) point and line
sets; (8) textures, materials, lights and shaders; (9) motions and the animation modifier; (10) a
held corpus and its gate; (11) the device mesh pipeline; (12) `render-cpu`'s mesh oracle; (13) the
comparison gate between them; (14) activation, Table 310 and the new level; (15) views, camera,
projections, `OB` and the default-view precedence; (16) background and the fifteen render modes;
(17) lighting schemes; (18) cross sections; (19) node dictionaries; (20) §13.6.3.2's animation
style; (21) §13.6.3.3's shared artwork; (22) §13.6.6's markup association; (23) and (24) §13.6.7's
measurements. **Twenty-four rounds**, the reader's half the least certain: ECMA-363's third edition
runs to about 200 pages and no tree holds it yet. Launch path: none. Binaries: the reader lives in
the scene worker; the host gains the scene type, the mesh pipeline's shaders and the oracle's
rasteriser.

**Trigger.** "A U3D reader in Rust that reads ECMA-363's fourth edition whole and is fuzzed, or the
owner's decision to write one in-tree" (ADR 2400). Not met: `u3d` 0.1.0 is unchanged since ADR 2400 (section 9). What
would meet it is either the owner's word or a reader of the *third* edition, whole and fuzzed, once
the trigger names it.

## 8. Part 4 — rendering PRC

**What a reader must do.** The same §13.6 obligations as Part 3 — they bind "3D artwork" whatever
its `/Subtype` — over a PRC stream, plus Table 323's `/N`, whose interpretation the standard makes
depend on the subtype.

**What it would take, honestly.** ISO 14739-1 is a CAD exchange format with exact boundary
representation — surfaces and curves that must be tessellated before a triangle is drawn — besides
its own tessellated form. The project does not hold the standard; principle 5 forbids writing a
reader from anything else, and no reading of another implementation would be admissible as the
definition. **It cannot be planned to the round before the standard is held.** With it, the
estimate by analogy to Part 3 is the scene worker reused, a reader at least twice ECMA-363's, and a
tessellator — **thirty to forty rounds, estimated with low confidence**.

**Dependencies.** Two Rust PRC readers now exist, `prc` 0.3.0 (56 `unsafe`, a C API of its own) and
`pdfcer-3d` 0.163.0 (no `unsafe`); section 9 has them, and why neither can be judged without the
standard. Taking either would still put it in the scene worker, and the scene of section 4.4 would
still be all the host sees.

**Trigger.** **None exists.** ADR 2400 section 3: "the trigger names only U3D … PRC needs an argument
of its own. This is recorded here so that no round reads a U3D reader as reopening PRC." The
exclusion therefore stands with no stated way out, which `CLAUDE.md`'s "each until the trigger
written beside its ratification is met" does not contemplate. The question proposes one.

## 9. The measured table

Measured on 2026-11-19 with cargo-deny 0.20.2 and cargo 1.100.0, each crate in a scratch package of
its own (its own `[workspace]`, one dependency at the newest version, default features unless
named), against a copy of the tree's `deny.toml`, checks `licenses bans advisories sources`.
*Lines* and *`unsafe`* are `grep -rw unsafe` counts over the crate's own `src/`; the graph's totals
include build and procedural-macro crates and overstate what links. *C* is a `-sys` crate, a
`links =` key or a `cc`, `cmake`, `bindgen` or `pkg-config` build dependency anywhere in the graph.
The working file is `~/scratch/agent/r2124/crates.md`.

| crate | version, released | licence | `cargo deny` | own lines / `unsafe` | C | part |
|---|---|---|---|---|---|---|
| `u3d` | 0.1.0, 2026-10-09 | MIT | ok | 2 153 / 0 | none | 3 |
| `prc` | 0.3.0, 2026-10-31 | MIT | ok | 35 330 / 56 | none | 4 |
| `pdfcer-3d` | 0.163.0, 2026-11-19 | MIT | ok | 48 423 / 0 | none | 4 |
| `glam` | 0.34.3 | MIT OR Apache-2.0 | ok | (SIMD; 924 `unsafe`) | none | 3, 4 |
| `cpal` | 0.18.2 | Apache-2.0 | ok | 22 488 / 203 | `alsa-sys` (`links = "alsa"`, `pkg-config`, `bindgen`) | 1 |
| `rodio`, no default features | 0.23.0 | MIT OR Apache-2.0 | ok | 16 828 / 1 | none; playback needs `cpal` | 1 |
| `mp4` | 0.14.0, 2023-08-01 | MIT | ok | 10 402 / 1 | none | 1, 2 |
| `re_mp4` | 0.5.1, 2026-07-08 | MIT | ok | 7 912 / 1 | none | 1, 2 |
| `matroska` | 0.30.1 | MIT/Apache-2.0 | ok | 2 221 / 0 | none | 2 |
| `mp4parse` | 0.17.0, 2023-05-29 | MPL-2.0 | **fails** (licences) | 8 429 / 0 | none | — |
| `symphonia` | 0.6.1 | MPL-2.0 | **fails** (licences, 11) | — | none | — |
| `rusty_h264-decoder`, `std` only | 0.17.3, 2026-11-19 | BSD-2-Clause | ok | 18 870 / 1 | none | 2 |
| `rusty_aac`, no default features | 2.1.1, 2026-11-19 | Apache-2.0 | ok | 22 752 / 41 | none | 1 |
| `openh264` | 0.9.10 | BSD-2-Clause | ok | 4 682 / 107 | `openh264-sys2` compiles Cisco's vendored C++ with `cc` | — |

What the figures say, part by part:

- **Part 1.** `cpal` is the one audio output that is not a system framework, and its C is
  `libasound`, linked dynamically (`ldd` names `libasound.so.2`; the development files, 1.2.14, are
  present on this machine). A release binary with LTO and stripping grows from 288 120 to 395 376
  bytes with it: **+105 KiB**, and the probe played an output stream on the default host. `rusty_aac`
  has a release newer than RFC 0012's audit (2.1.1 against 2.0.1); the audit's conformance figures
  are for 2.0.1 and would be re-run against the version taken. Of the MP4 demuxers, `mp4` has had no
  release since 2023 and `re_mp4` is maintained; `mp4parse` is out on its licence.
- **Part 2.** `rusty_h264-decoder` is at 0.17.3 against the 0.16.1 RFC 0012 measured; the same
  re-run applies. `openh264` stays the C alternative ADR 0014's precedent would have to justify.
- **Part 3.** `u3d` is unchanged since ADR 2400: one release, 29 downloads, no `unsafe`, its README
  claiming the compression context, the modifier chain and the base-mesh continuation and leaving
  the progressive-mesh continuation, Adobe's mesh resource and fuzzing unticked. Which edition of
  ECMA-363 it reads, it does not say. It does not meet the trigger.
- **Part 4.** Two PRC readers exist that ADR 2400 did not see in this form. `prc` 0.3.0 calls itself a
  schema-driven parser of the 2014 edition of ISO 14739-1, with 56 `unsafe` and its own C API, and
  leaves the compressed tessellation's interpretation and B-rep tessellation undone. `pdfcer-3d`
  0.163.0, released today with an API it calls unstable, claims PRC reading and writing, glTF, a CPU
  rasteriser and shaders, without `unsafe`, and does not mention U3D. **Neither claim can be judged
  here**: principle 5 measures a reader against the standard, and the standard is the one text the
  project does not hold. They are evidence that a PRC reader in Rust is possible, not that one exists.

### 9.1 Audio in use — the command and its readings

Part 1 shipped (rounds 2148–2183), and A946 asks for its clock and sinks measured in use before
Part 2 is asked about. The command is `tools/audio-in-use.sh` (ADR 3441): it drives a window's
`viewer_host::sound::Player`, the confined worker and the sound helper onto a stand-in device —
ALSA's `file` plugin over `null` into a one-page pipe read at 48 kHz on the monotonic clock, since
this machine's user may open no sound card — with sounds whose frames carry their own index.
Every row is a line of its `readings.txt`. The readings are four runs of 2026-11-23, under load
averages from 10 to 358 on 32 processors; `doc/reviews/2185-audio-measured-in-use.md`
has the method, every reading, and what a stand-in device cannot measure (a card's buffer and a
crystal's drift).

| figure | its line in `readings.txt` | reading, 2026-11-23 | bound |
|---|---|---|---|
| activation → first frame at the device, cold / warm (2 s sound) | `cold start`, `warm start` | medians 42–144 ms / 25–70 ms | — |
| activation → first frame, a 30 s sound (raw, WAVE, AAC LC, HE-AAC) | `activation -> the player says Playing` | 0.3–3.1 s: the sound is decoded, converted and sent whole first | — |
| pause → silence, resume → sound, stop → silence | `pause/resume/stop` | 19–25 ms, 9–24 ms, 16–25 ms; a resume repeats 10–40 ms | — |
| media clock − device's frames over 643–659 s (160 s × 4) | `media clock` | −1.8 to 9.4 ppm by least squares; offset within −37 to +26 ms; no frame lost or repeated in a pass | — |
| worker peak address space / resident | `worker:` | ≤ 7.2 / ≤ 5.4 MiB | 64 MiB (`Profile::Media`) |
| worker processor per second of sound | `worker:` | ≤ 8.2 ms (HE-AAC ≈ 12 ms a 1.37 s request at most) | 1 s per request |
| helper peak resident, 30 s / 160 s sound | `helper:`, `long sound's helper` | 15 / 120 MiB (two passes held at once); 60 ms to 2.6 s of silence between passes | — |
| underruns in the long sound | `in the long sound`, `underruns by minute` | 841 to 2 414 (0.6–2.5 % of its time); 8–56 a minute at load 10–21, 300–457 at 91–262; no paging | — |

## 10. Part 5 — rich media, and Part 6 — the two actions

### 10.1 Rich media

**What a reader must do.** §13.7.2.2's settings: activation and deactivation conditions (Tables
335, 336), Table 337's animation (`PlayCount`, `Speed`), Table 338's presentation — "A interactive
PDF processor shall support the state Embedded", `Transparent`, `PassContextClick` — and Table 339's
windowed style, constrained "within the presentation area"; §13.7.2.3's configurations and instances
(Table 342: "Valid values are 3D , Sound , and Video"; Table 343's instance `/Subtype` "shall match
the asset file type"), Table 344's `Snapshot`, and §13.7.2.3.6's state data, "the run time shall
iterate through all active instances for the current configuration and query them for state data".
And Table 334's `/Scripts`: "each script in the array shall be executed in order in a common
environment per annotation by the rich media run time ECMAScript interpreter".

**What it is made of.** A `3D` configuration is Part 3 or 4, a `Sound` configuration Part 1, a
`Video` configuration Part 2; the presentation, windowing and activation are a few rounds of their
own (estimated at **five**: settings and activation; presentation, embedded and windowed; views and
snapshots; configurations and instances; animation). Two halves are not any part's:

- **The run time.** §13.7's scripts and its state save and load address "the rich media run time",
  whose interpreter and whose instances' state protocol the standard leaves to ISO 21757-1 and to
  the asset's own handler; it states neither. For a 3D instance that handler is ISO 21757-1's 3D API,
  which the script exclusion names.
- **The formats.** Clause 13.7 names no asset format for `Sound` or `Video`; its one example asset is
  `3D.prc`. Which file types "match" `Sound` and `Video` is therefore a documented choice, and the
  natural one is the set Parts 1 and 2 decode, everything else handed off.

**Trigger.** Rich media has no trigger of its own; it follows its configurations' parts, and its
script half follows the script exclusion.

### 10.2 The two clause 12 rows

- **§12.6.4.16, Go-To-3D-View.** "A go-to-3D-view action ( PDF 1.6 ) identifies a 3D annotation and
  specifies a view for the annotation to use", widened by Errata Collection 3's Issue #282 to a
  rich media annotation's `Views`. Once Part 3's views exist it is **one round**: resolve `/V` by
  index, name, `F`/`L`/`D`/`P`/`N`, or dictionary, and set the camera; activating an inactive
  annotation first under the 3D level.
- **§12.6.4.18, Rich-Media-Execute.** Table 223's `/C` is "the script command (a primitive
  ECMAScript function name)" sent to the annotation's handler, and the call "shall be made in the
  global context of the annotation's instance of the 3D ECMAScript engine" for 3D content. There is
  no handler without the run time of section 10.1, and for 3D the handler *is* ISO 21757-1's API. It
  stays with the script exclusion whatever Parts 1 to 4 do.

### 10.3 The general rows and §13.5

§13.1 and §13.2.1 move with Part 1. §13.2.7.5 moves with Part 2. **§13.5 belongs to no part.**
Its slideshow is a root resource such as `mysvg.svg` with `mysvg.js` and `mymusic.mp3` beside it
(the clause's own example): an SVG renderer and a script engine over a virtual file system, which is
a web runtime. The clause opens "The features described in this clause are deprecated with PDF 2.0"
and releases the reader in its own words: "Since PDF processors are not required to support
alternate presentations, authors of documents containing alternate presentations should define the
files such that something useful and meaningful can be displayed and printed." No part plans it.

## 11. Cost, order and recommendation

| order | step | rounds (est.) | trigger | recommendation |
|---|---|---|---|---|
| 0 | the `Media` level and the hand-off to this machine's player (section 3) | 2 | none — in scope now, owed by A254 | **build now** |
| 1 | Part 1, audio | 9 | the owner's decision to build the player | **open on the owner's word**, audio alone, with the sink measured before the next step |
| 2 | Part 2, video | 14 after Part 1 | the same | **stay closed** until Part 1 ships and its sinks are measured in use |
| 3 | Part 3, U3D | 24 | a third-edition reader, whole and fuzzed, or the owner's word — trigger corrected | **stay closed**; correct the edition |
| 4 | Part 4, PRC | 30–40, low confidence | none today | **stay closed**; write a trigger naming the standard's purchase |
| 5 | rich media | 5 plus its parts | follows its parts | follows Parts 1–4 |
| 6 | Go-To-3D-View | 1 after Part 3 | follows Part 3 | with Part 3 |
| 6 | Rich-Media-Execute | — | the script exclusion | stays with the script exclusion |
| — | §13.5 | — | — | stays; the clause releases the reader |

**What the whole would add to the launch path: nothing**, by section 4.6's ordering. **To the
binaries**: two worker binaries; in the windows, an audio sink each (Part 1) and the scene type, mesh
pipeline and oracle rasteriser (Part 3). The shipped viewer's size grows by section 9's `cpal` figure,
105 KiB, for audio and by an estimated few thousand lines of host code for 3D; decoders and parsers never
enter the host.

## 12. Questions for the owner — Q946

One per part, each with its recommendation; `doc/questions/Q946` carries them.

1. **The owed hand-off.** Build the `Media` level and the start of this machine's own player now,
   as A254 owes (two rounds)? *Recommendation*: yes, before anything below.
2. **Audio.** Open "decoding or presenting audio" alone — §13.3's PCM and AAC in `pdf-media-worker`,
   an audio sink in each window — leaving video closed? *Recommendation*: yes, in section 5's order,
   if the owner wants this program to be the player at all; the hand-off of question 1 is the
   alternative the standard itself names first.
3. **Video.** Keep it closed until Part 1 has shipped and its clock and sinks are measured in use?
   *Recommendation*: yes; then reopen on the owner's word in section 6's order.
4. **U3D.** Restate the trigger to the edition ISO 32000-2 names — "a U3D reader in Rust that reads
   ECMA-363's third edition whole and is fuzzed, or the owner's decision to write one in-tree" — and
   keep it closed? *Recommendation*: yes; and if the owner decides to write one, section 7's
   twenty-four rounds in that order, the scene worker first.
5. **PRC.** Add the trigger PRC lacks: "the project holds ISO 14739-1, and a PRC reader in Rust
   reads it whole and is fuzzed, or the owner's decision to write one in-tree"? *Recommendation*:
   yes; a purchase of the standard is the owner's, and nothing about PRC is plannable before it.
6. **Rich media and the two actions.** Let rich media and Go-To-3D-View follow the parts they are
   made of, with no trigger of their own, and leave Rich-Media-Execute and §13.7's run time with the
   script exclusion's ISO 21757-1 clause? *Recommendation*: yes, and record §13.5 as planned by no
   part on the clause's own release.

## 13. The current restrictions, each with its rationale, and the unconstrained design

| restriction | rationale | unconstrained |
|---|---|---|
| clause 13's playing and 3D excluded | a media engine and a 3D renderer are not a rendering question, and each is a large attack surface | every part built in section 11's order |
| no decoder or parser of document bytes in the host | principle 3 | unchanged — this is the sandbox, not negotiable |
| `deny.toml`'s licence list | the project's licence | `symphonia` (MPL-2.0) would give the one pure-Rust audio stack |
| ISO 21757-1 not bought | the owner's 2026-08-28 decision | the 3D API behind `OnInstantiate`, `AIS` and Rich-Media-Execute |
| ISO 14739-1 not held | principle 5 admits no other source | a PRC reader |
| nothing on the launch path | principle 2 | unchanged |

## 14. Sources

`doc/md/ISO_32000-2_sponsored_EC3.md`, clause 13 and §12.6.4.16, §12.6.4.18, its introduction,
bibliography and §3.67; `doc/conformance/ledger.toml`; RFCs 0008, 0009, 0012, 0013; ADRs 1155, 1946,
1960, 1961, 2382, 2400, 2428; `doc/questions/A254`, `A632`, `A649`, `A662`;
`crates/viewer-host/src/policy.rs` (`handed_media`), `crates/viewer-ui/src/bin/quorra/dispatch.rs`,
`crates/viewer-host/src/native_window.rs`, `crates/pdf-sandbox/src/lockdown.rs`,
`crates/pdf-script-worker/src/lib.rs`, `crates/pdf-model/src/action.rs`; crates.io and `cargo deny`
on 2026-11-19 (section 9).
