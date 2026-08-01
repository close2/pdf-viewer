# Handover

Written 2026-07-26, rewritten and halved 2026-08-01 at the end of the **hundred-and-thirtieth**
session, and kept current since; the **hundred-and-forty-fifth** is the last one in it. Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed
exclusion list. **Principle 5 is the one that changes how you work**: the specification is the
only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it
right, never the definition of right.

`doc/PLAN.md` holds the phases and the ledger's design; `doc/adr/` holds every decision's
argument; `doc/conformance/ledger.toml` holds one row per subclause. **This file is the state of
play, the traps, the habits and what to do next** — where something is written elsewhere, this is
a pointer.

**A lesson lives here exactly once**: in a trap if it changes how you write code, in Habits if it
changes how you work, in the numbers if it is a fact about today. A session's narrative belongs
in its ADR and nowhere else. This file was halved in the fifty-ninth session and again in the
hundred-and-thirtieth; if you find yourself retelling a session here, you are undoing that.

---

## Where we are

A PDF **viewer**, and until the hundred-and-thirty-first session that word would have been a
claim rather than a description.

It **draws** what a page says: geometry, colour, images, shadings, patterns, embedded text,
transparency groups, soft masks, and annotations both from stored appearance streams and
constructed where the standard states one — including §12.5.6.4's seven icons, whose artwork is
this processor's own because the clause requires one and draws none. Two backends (CPU and GPU)
that agree to the channel — over `test-scenes`' fixtures **and, since the hundred-and-forty-third
session, over real pages at a real window's resolution**, which is where they did not (ADR 0127).
The GPU backend **bands a target the device cannot draw in one pass**, because Vello's working
buffers are fixed constants with no knob and a page of small text at a laptop's resolution can
exceed them.
JBIG2 and JPEG 2000 in a confined worker. Encryption at every revision
and method §7.6 states, and since the hundred-and-forty-fourth session in both directions. §12.3.2's destinations, §12.3.3's outline, §12.4.2's page labels,
§12.5.6.5's links performing **eleven of §12.6's actions**, §14.9's accessibility entries,
§12.4.4's whole presentation read for a caller that has one to play, and everything a document
says *about itself*: §14.7's logical structure, §14.8's tagged-PDF vocabulary, §7.11.4's embedded
files, §14.13's associated files, §12.2's viewer preferences, §12.11's requirements, §7.12's
extensions.

It is **used**, which is what the nine sessions from the hundred-and-thirty-first added. A
locked document asks for its password (§7.6.4.1, owed since the twenty-second session); the page
zooms and scrolls; the cursor knows what it is over and §12.5.5's appearances follow it, as does
§12.5.6.19's `/H`; a drag **selects text**, whose shapes cross to the host as geometry so that it
draws them in its own colour; a person can **fill in a form field**, undo it and redo it; and the
result can be **saved** — the file it was opened from, unchanged, with §7.5.6's incremental update
appended, which is the one kind of writing `CLAUDE.md` permits.

All of it sits behind **`viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API. Two consumers —
`viewer-ui`'s winit window and a headless test harness — and §0 is the whole story. ADRs 0116 to
0121.

**What it still does not do**: speak a page (AccessKit has six sessions of §14.7 and §14.9
reading waiting for it, and one `Query` to go), run a slide show (§12.4.4 is read and nothing
plays it), search, or draw a panel for the outline, the layers and the attachments that
`Query` already answers with.

### The four gates, today

| gate | number | where |
|---|---|---|
| tests | **944**, `clippy` silent under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`, `fmt` clean, `cargo deny` clean on all four, **five fuzz targets clean at 50 000 runs** | tests, `clippy`, `fmt` and `cargo deny` re-run in session 145; the five fuzzers last in 139 |
| corpus (974 pdf.js documents, page one) | 964 open, 959 reach page one, **868 draw with nothing reported**, **91 report something**, 0 slower than 30 s | `tests/corpus.rs`, ~2 s |
| oracle (1794 pages vs poppler, mupdf, ghostscript) | of **1665** we call complete: **839 agree**, **65 contradicted**, 750 ambiguous, 10 not comparable | `tests/oracle.rs`, ~30 s |
| text (vs `pdftotext`, same 974) | **97.9%** of the reference's words, **42** named below the 0.90 floor | `tests/text_extraction.rs`, ~30 s |
| dates | 1545 date strings | `tests/dates.rs` |
| conformance | 3067 citations, 318 quotations, 181 tables, **823 ledger rows** | `-p conformance` |

Counts are **ratcheted**: they may only improve, except where a rise is a new report and is
written down as one (trap 5). The 14 specification PDFs in `doc/` — including ISO 32000-2 itself,
1023 pages, 101 318 objects — all parse, draw page one with nothing reported, and extract 100% of
`pdftotext`'s words.

**Read the oracle's 45% ambiguous with care.** 372 of those pages are two long books set in fonts
nobody embedded, so each renderer substitutes differently. That row means "reported nothing", not
"drew it right".

**Both moving numbers move in both directions on purpose.** Contradicted pages: 174 → 65 over
sessions 6 to 61, steady at 65 since. Corpus documents drawing incompletely: 291 → 89 over
sessions 6 to 122, then **91** in the hundred-and-twenty-seventh, where two documents that had
been drawing the wrong font in silence started saying so.

### The ledger

All **823** subclauses of the eight technical clauses have been read against this code, since the
fifty-sixth session. Counts come from `cargo run -p conformance --bin ledger`, which prints them
— **not** from arithmetic in this file, which has been wrong about them twice.

| status | rows | |
|---|---|---|
| `implemented` | 366 | every normative requirement in the clause is executed |
| `partial` | 243 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so |
| `inapplicable` | 88 | a marking device, a layout engine, a production workflow |
| `out-of-scope` | 87 | principle 5's closed exclusions, which the row names |
| `reported` | 33 | not implemented, detected and named at runtime |
| `writer-side` | 6 | addresses a PDF *generator* |

**`silent` is zero** — there is no requirement in the eight technical clauses that this program
fails without saying so. That is a narrow claim: `partial` and `reported` are 275 rows between
them and each names what it owes.

**The ledger has been wrong four times and this file's arithmetic about it once.** A row that
names a rasteriser's behaviour has recorded that rasteriser (§8.4.3.2); a row written during a
review describes what the code *should* do (ADRs 0056, 0057, 0060). The defences are reading the
*family* rather than the row, and `FILE_ONLY_EVIDENCE_CEILING`, which is zero and asserted with
`==`.

**`writer-side`'s 7 rows were re-read in the hundred-and-thirty-seventh session** against the
amended definition — `CLAUDE.md` excludes *authoring*, not writing — and `ledger.toml`'s header
now carries that definition rather than "we do not create files". **Six stay and one moved**:
§7.2.2's "Representation" binds this tree now that it writes, and all three of its requirements
are met by construction. ADR 0122.

### What is not implemented

Every one is *reported* at runtime rather than silently skipped. The count is how many of the 974
documents' first pages it affects.

| Missing | Corpus | Notes |
|---|---|---|
| A `/DA` font `/DR` does not define | 7 | A malformed file, not a clause gap: §12.7.4.3 requires the name to match a `/DR` entry. Since ADR 0112 the value is laid out in a stand-in **where the stand-in can draw all of it** and the missing font is named; four Arabic-valued documents decline, because a Latin stand-in drawing their punctuation is worse than a blank. |
| A composite `/DA` font, a list box, `/DS`, `/RV` | 0 | The rest of §12.7.4.3's edges, none reached. A composite font needs §9.7.6.2's codespace ranges inverted; §12.7.5.4 states which items are selected and nothing about how that *looks*; `/DS` and `/RV` are XFA. |
| Public-key handlers (§7.6.5) | 0 | CMS enveloped data, X.509, the user's private keys — an infrastructure and a threat model, not a cipher. |
| `/R` 5, and a non-ASCII revision-4 password | 1 | Table 21 says `/R` 5 "shall not be used" and states no algorithm. The password refusal is now cheap to close: `pdf-syntax` holds Table D.3, so inverting it would do it. |
| Icons for `Stamp`, `FileAttachment`, `Sound` | 1 | Their clauses say a reader **should** provide predefined icons; §12.5.6.4 says **shall**, which is why its seven are drawn and these three are not (ADR 0109). |
| Predefined `CMap`s (§9.7.5.2) | 13 | 13 fonts name one of Table 116's registered files. **The licence question is answered — see §1** — and §9.7.5.2 states it as a `shall`. |
| A substitute that cannot be addressed | 41 | Counting fonts: 27 composite with no `/ToUnicode`, 23 whose substitute draws none of the declared codes. Honest refusals; closing them means better substitution, and some the `-UCS2` `CMap`s would answer. |
| Transparency departures (§11.4, §11.5.3, §11.6.6) | 19 | Each reported where it can change a pixel: a knockout element whose shape is not its coverage (5), a non-isolated group NOTE 5 cannot flatten whose elements blend (6), a blending space that is not the device's three components (4, all `/DeviceCMYK`), a soft-mask group with such a space (7). |
| Optional content's interactive half | — | §8.11 is honoured wherever it decides what is drawn. **The layer panel's data model is read** since session 67 — `/Order`, `/ListMode`, `/Locked`, `/RBGroups`, `/Name` — and since 131 it is `Query::Layers` with `Command::SetGroup` as the switch. What is missing is a panel to draw it in. Still unread: alternate `/Configs`. |
| `NoZoom`, `NoRotate`, `/FixedPrint` | — | Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. **Measured**: 90 corpus annotations set `NoZoom` — 78 popups this tree draws nothing for, 11 `Text`, 1 `FileAttachment`. |
| Grid-fitting a stroke's coordinates (`/SA`) | — | A documented departure: the non-uniformity it removes is an artefact of the binary scan conversion §10.7.4 requires and this tree already departs from. |
| A filled degenerate subpath's device pixel (§8.5.3.3.1) | — | The clause calls the result "device-dependent and not generally useful" in the same breath. Recorded, not reported. |
| A mask at a grid the bound refuses | 1 | `issue16263.pdf`: a 2×2 image with a 34862×4332 mask, 604 MB. The clause's answer is compositing at *device* resolution, which needs the display list to carry image and mask separately. |
| JPEG 2000 at reduced resolution | 1 | `issue19517.pdf`, 212 megapixels. The format's answer needs the intended scale to reach the decoder. |
| A stream whose data is in an external file (§7.3.8.1) | 0 | The renderer has no filesystem (principle 3). Refused by name rather than drawn from the bytes the clause says to ignore — which is what it did, silently, for the project's whole life. |
| `/ColorTransform` (Table 13) | — | Its one corpus witness contradicts the clause. |
| Sampled shadings on the GPU | 2 | Type 1 only; the CPU backend draws them. |
| A page whose scene overflows Vello's buffers | — | **Closed in session 143, by banding.** Vello sizes its GPU working buffers from constants "hand picked to accommodate the vello test scenes", offers no way to enlarge them, and draws *nothing* when a scene needs more — page 6 of ISO 32000-2 at 1132×1600 needs 4% more tile records than the buffer holds. `render_checked` sees the flag and halves the target until it fits, which is Vello's own issue 366 remedy; 38.1 ms against 24.6 ms unbanded and 98 ms on the processor. The CPU fallback remains for a scene no band can hold. ADR 0127. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Read and recorded; `A2B0` not yet selected for `Perceptual`. |
| §12.7.6.2's submit, §12.6.4's remote/launch/sound/movie | — | A network, a second file, a media engine. Refused by name and printed on a click. |
| Signature *validation* (§12.8.3) | — | 17 ledger rows. Needs a trust store and a network; what a program without one can say is said (ADRs 0088, 0089). |
| Sandboxing the interpreter and rasteriser | — | Spike D confines the image codecs (ADR 0014). §0 makes this a transport change rather than a design question. |

---

## What to do next

**Two tracks, and take from both in every session.** *Demand-driven* is what the corpus and the
oracle name; *spec-driven* is the ledger's 33 `reported` rows and the notes on its 242 `partial`
ones. A project running only the first finishes when the corpus goes quiet, which can happen with
much of the standard unimplemented and nothing able to say which parts; one running only the
second ships features no file exercises. This is a principle-5 rule, not a suggestion.

**But the ten sessions from the hundred-and-twentieth found that the map is not the territory.**
Four of their six findings were on no list: a `shall` hiding behind a silence about artwork (ADR
0109), a clause with two populations where the row named one (ADR 0110), a malformed optional
entry that erased a font (ADR 0111), and a font cache keyed by a name that drew wrong glyphs in
silence for thirty-one sessions (ADR 0115). None was `silent`, none was `reported`, and no gate
could see the last. Three were found by reading the clause beside the code; the fourth by
measuring something else.

### 0. The UI boundary — built, with two consumers on it

**Everything a viewer still owes was blocked on one missing interface.** Since the
hundred-and-thirty-first session that interface is code — `crates/viewer-core`, ADRs 0116 to
0121 — with `viewer-ui` on it as a tier-2 host and `tests/headless.rs` driving it with no display
at all. This section is now half description and half instruction: read the first half to know
what is there, the second to know what is next.

#### Why it was the headline

Five owed items were the same item, and **three of them are done**: a password prompt, which this
file called "the missing piece, not the clause" for twenty sessions and which session 132 landed
in eleven lines of host code; an editable field (sessions 135 and 136); and the layer panel's
data, which `Query::Layers` answers with and which now wants only a panel. The other two —
presentation mode and AccessKit — are features rather than architecture: one `Command::Tick` and
one `Query` respectively.

#### The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. Later it must support **text selection** and **annotation
editing**, possibly form-field text editing. `CLAUDE.md`'s exclusion list was amended in the
hundred-and-thirtieth session to permit the writing that implies.

#### What exists

Two consumers: `viewer-ui`'s `pdf-viewer.rs` (winit + vello, tier 2) and
`viewer-core/tests/headless.rs` (no display at all, tier 1). Neither can prove the interface
alone — one is a toolkit, the other is not a program — and together they are why the vocabulary
is worth trusting.

```
host toolkit  ──Command──▶  viewer-core (no threads, no I/O, no clock)  ──Event──▶  host
 Win32/AppKit/Qt/GTK/winit  ◀──Answer──   query(&self, Query)   ◀──Query──
                                    │                                  ▲
                                    └──NeedsRender──▶ worker ──RenderReady──┘
```

- `Viewer::handle(&mut self, Command) -> impl Iterator<Item = Event>`, and
  `Viewer::query(&self, Query) -> Answer` beside it. **Selection cannot wait for a render round
  trip**, which is why the second channel is not a command.
- `Command`: `Open { id, bytes, password }`, `Close`, `Focus`, `Resize { width, height, scale }`,
  `GoTo(PageTarget)`, `Zoom`, `Scroll`, `SetGroup`, `Pointer { at, action }`, `Select`,
  `Edit(Edit)`, `Undo`, `Redo`, `Save`, `Supply { purpose, bytes }`,
  `RenderReady { token, rendered }`.
- `Event`: `Opened`, `OpenFailed`, **`PasswordRequired`**, `Closed`, `PageChanged`,
  `NeedsRender(RenderRequest)`, `Damage(Rect)`, `OpenUri`, `NeedsFile`, `Transition`, `Dirty`,
  `Saved { bytes }`,
  `Reported { document, page: Option<usize>, notes }` — the `None` page is what the *document*
  says about itself (§12.11, §12.8, §7.11.4), said before any page is drawn.
- `Query` → `Answer`: `PageCount`, `CurrentPage`, `PageGeometry`, `LinkAt`, `FieldAt`,
  `Selection`, `Dirty`, `Outline`, `Layers`, `Attachments`, `Frame`, `Reports`. **`Selection` answers in device pixels
  and produces no events**: a drag emits `Damage` and never `NeedsRender`, which is what keeps
  chrome off the rendering path.
- **Nothing is `#[non_exhaustive]`**, deliberately: it forces a catch-all arm on every host, and
  a catch-all arm is where a message added later goes to be ignored in silence. A new `Event`
  should fail to compile in every consumer.
- **This crate interprets; the host rasterises.** `NeedsRender` carries an `Arc<DisplayList>` and
  a `TargetSpec`, so a zoom or a scroll re-rasterises *without re-interpreting* — asserted by
  pointer equality of the list in `zooming_rasterises_again_without_interpreting_again`.
- **A stale token is dropped**, so a page turned mid-render cannot be overwritten by the frame
  the previous page produced.
- `Rendered::{Raster, Presented, Failed}` is where tier 1 and tier 2 differ and the only place
  they do: a host drawing onto its own surface has no raster to hand back and has not failed.

#### What is still owed, in the order to do it

1. **Both costs §7.5.6's writer recorded are closed.** Encryption on the way out landed in the
   hundred-and-forty-fourth session (ADR 0129), so every one of the corpus's 26 encrypted
   documents can be saved; §12.7.4.3's appearance stream is *written into the file* since the
   hundred-and-forty-fifth (ADR 0130), so a reader that ignores Table 224's `/NeedAppearances`
   still sees the new value. **What the writer still owes is an `Edit` variant that carries a new
   object rather than a field's value** — the markup and free-text annotation authoring under
   "Near, and far" below. `Update` in `view.rs` already allocates object numbers for it.
2. **The rest of the vocabulary, as its feature arrives.** `Command::Tick { millis }` for
   §12.4.4's `/Dur` and transitions (rule 3, and `Event::Transition` already leaves), and
   `Query::AccessibilityTree`, which is what AccessKit needs and the last of §0's five owed
   items with nothing behind it. Selection, editing and saving all landed in sessions 134 to
   136 and their commands are in the table above.
3. **Then one native host.** GTK4 via `gtk4-rs` first — Rust-safe, no C++ bridge, and it is the
   development platform. Qt/KDE second via `cxx-qt`, because that costs a C++ bridge and should
   not be the experiment that shapes the API.
4. **`viewer-ffi` last**, and it is the **only** crate permitted `unsafe`. Every crate touching
   PDF bytes keeps `#![forbid(unsafe_code)]`; the FFI crate touches messages, not documents, so
   the compiler-enforced rule survives. **Do not freeze a C ABI until two Rust consumers have
   shaken the API out** — the vocabulary roughly doubles with selection and editing.

**The panels are now cheap and nobody has drawn one.** `Query::Layers`, `Query::Outline` and
`Query::Attachments` answer with everything a panel needs and no consumer asks any of them. That
is a UI job, not a clause job, and it is the first thing a native host should show.

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + vello
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.

#### Crates

- `viewer-core` — the state machine. **Exists**; depends on `pdf-model`, `pdf-render` and
  `pdf-syntax` and nothing else. Owns the open-document set, page/zoom/scroll, links and
  §12.6's actions, the selection, the edit log and the render scheduler's *bookkeeping* (not its
  threads). Still owes search and a navigation history.
- `viewer-render` (new, optional) — a default worker a host may use instead of writing one.
- `viewer-gpu` (new, later) — tier 2. The only crate that may name `raw-window-handle`, `wgpu` or
  `vello` in its API.
- `viewer-ffi` (new, last) — the C ABI, and the only crate in the tree permitted `unsafe`.
- `viewer-ui` — consumer #1 since session 132, and a tier-2 host.
- `pdf-model` — has the text layer (ADR 0118). The edit log lives in `viewer-core` and reaches
  interpretation through `ViewState`, which was already the log §12.6.4's actions write to — so
  `interpret` did not need a third input after all, and rule 1 holds without one.

#### Five rules, and each has a reason that already exists in the tree

1. **`pdf_syntax::Document` is immutable, forever.** An edit is a log beside it, not a change to
   it — the pattern `view.rs` already uses for §12.6.4's actions, and which the edit log joined
   in the hundred-and-thirty-fifth session rather than displacing. `interpret` stays a pure
   function of the document and the view state, which is what keeps the oracle's comparison of
   1665 pages meaning anything. Stated in `CLAUDE.md`, and held.
2. **No filesystem in the core.** The host supplies bytes; the core produces bytes. Not new
   policy — `Request::Import` and `Request::Resolve` already do exactly this, argued in ADRs 0090
   and 0104: "a document naming a file is a document asking this machine for something, and
   whether to give it is not a rendering decision."
3. **No clock.** §12.4.4's transitions and `/Dur` auto-advance arrive as `Command::Tick { millis }`.
4. **No threads the core was not handed**, and no blocking.
5. **No toolkit or graphics type in `viewer-core`'s public API.** Tier 2 (below) lives in a
   separate crate.

#### Pixels: three tiers, and interactive chrome is not pixels

| tier | what crosses | hosts | cost |
|---|---|---|---|
| **1** | a CPU `Raster` | everything, today, no unsafe | one copy per frame |
| **2** | a raw window handle; we drive wgpu/vello | anything producing `raw-window-handle` | a graphics dependency in the *binding* |
| **3** | the host's own GPU device/texture | one toolkit at a time | interop per platform |

**Define the interface at tier 1**, because that is what makes the core toolkit-free by
construction rather than by discipline — and it is not a compromise here: `CLAUDE.md` already
makes the CPU backend both the correctness oracle *and* the startup path, so a tier-1 host gets
the startup behaviour the project already wants. Cost, with a number: 1920×1080 RGBA is 8.3 MB,
so full-window repaint at 60 fps is ~500 MB/s of memcpy — a few percent of a core, and only
during smooth scroll. `TargetSpec::transform` already carries "any tile offset", so tiled repaint
is the first lever if it matters.

**Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
annotation rubber-band, resize handles, a caret — these change at pointer speed and must not
force a page re-render. Emitting them as quads and points lets a native host draw selection in
**macOS's selection colour, KDE's accent, the Windows highlight brush**, with its own caret blink
and focus ring. That is most of what makes an embedded view feel native and is unreachable if we
hand over finished pixels. It also means a slow render never blocks feedback.

| | crosses as | changes at | drawn by |
|---|---|---|---|
| page content | `Raster` | page, zoom, edit | us |
| interactive chrome | geometry | pointer speed | the host |

#### Two artefacts, both of which now exist

**A text layer — done in the hundred-and-thirty-third session (ADR 0118), and selection on it in
the hundred-and-thirty-fourth (ADR 0119).**
`Interpretation::text_layer` is one `Placed` per character code: the range of the readback it
produced and the quadrilateral its glyph occupies, in the display list's coordinates. The box is
the glyph's advance by Table 122's `/Ascent` and `/Descent`, and it is built for rendering modes
3 and 7 too, because an OCR layer under a scanned page is exactly the text a person selects.
Measured at **+1.69%** of interpretation by an A/B in one sitting, and kept unconditional with the
cost written down.

**Search is the layer's third consumer** since the hundred-and-fortieth session: `Query::Find`
answers with the same shapes `Query::Selection` does, case-insensitively, and cost one function
because the geometry was already there. **What is still not built on it**: a caret, word and
paragraph selection, and — the one with a clause behind it — §14.8.2.5's *logical* order. A selection is taken in content order, so a page
whose producer wrote its columns out of order gives its text in that order.
`Interpretation::marked` already carries the `/MCID` spans and `Tree::logical_text` already
produces the logical string; what is missing is the map between the two orders' offsets.

**An edit log — done in the hundred-and-thirty-fifth session (ADR 0120).** `Open::log` is what a
person did, with a cursor; undo moves the cursor and the surviving prefix is *replayed* rather
than inverted, because an inverse would have to remember what each edit replaced and would drift
the moment two edits touched one field. `ViewState::set_field` is the fourth statement about a
field's value beside Table 226's `/V`, §12.7.6.3's `/DV` and §12.7.8's imported one, and the last
one made stands.

**And it is saved** since the hundred-and-thirty-sixth session (ADR 0121): `ViewState::save`
produces the file with §7.5.6's incremental update appended, the host writes the bytes, and
`pdftotext` and `mutool` both read the value back out of what it wrote. The producer's bytes are
still there underneath, which is the clause's whole point. **Both costs written down are closed.**
§12.7.4.3's appearance stream is *written* since the hundred-and-forty-fifth session (ADR 0130)
rather than owed to the next reader behind Table 224's `/NeedAppearances` — the bytes are the ones
this program draws, so writing them is not a new opinion about the file — and the flag is now set
only for a widget whose stream this program could not produce or could produce only part of. A
widget that had no `/AP` gets an object *added*, which is the half of §7.5.6's "changed, replaced,
or deleted" the writer did not do; the number it starts from is the larger of `/Size` and the
highest the cross-reference table holds, because 68 corpus documents understate the first. **An
encrypted document is written since the hundred-and-forty-fourth session** (ADR 0129): §7.6.2's
ciphers run on the way out through `decrypt_object`'s mirror, so the clause's exceptions are
stated once rather than twice, and the six corpus documents covering every revision and method
§7.6 states take a string and give it back — `mutool` and `pdftotext` read all six. The cost of
*that* is one testing habit: §7.6.3.2 requires a fresh random initialisation vector per AES
string, so an encrypted document's save is no longer byte-identical from one run to the next, and
its tests read the file back rather than compare it.

#### The prize: one boundary, not two

Principle 3 wants the interpreter and rasteriser confined, and this file has recorded the open
question as "the protocol would have to carry a display list rather than an image, which is a real
design question". **If the boundary is `Command`/`Event` with `Raster` payloads, that question
dissolves**: the confined process owns document, interpretation and rasterisation, and the host
receives pixels and events. Simpler than shipping display lists, and one protocol instead of two.
Design with that in mind from the first line even if the sandbox lands much later.

#### Near, and far

- **Form-field editing landed in the hundred-and-thirty-fifth session** and saving in the
  hundred-and-thirty-sixth. What is left of it is a *caret*: a host sends whole values, and
  nothing lays out a cursor between two characters. The text layer has the geometry for one.
- **Markup and free-text annotation editing is next, and is the same log and the same writer.**
  The constructions exist (§12.5.6.6, §12.5.6.10) and `pdf_syntax::write` puts an object into a
  file; what is new is authoring `/QuadPoints` and `/Rect` from a drag, and an `Edit` variant
  that carries a new object rather than a field's value.
- **Editing the page's own text is far** and deliberately out of scope until the first two exist:
  it means re-laying-out content streams whose producer's intent is recorded nowhere.

### 1. Third-party data: cleared to ship, in this order

**This project is MIT** as of the hundred-and-thirtieth session (relicensed from MPL-2.0; one
author in the whole history, so nobody else's consent was needed). `deny.toml`'s allow-list
dropped MPL with it.

**None of the data is blocked by a licence.** Read off copies on this machine, not recalled:

| data | source examined | terms |
|---|---|---|
| Adobe predefined `CMap`s | `poppler-data`'s `COPYING.adobe` (1990–2019), `doc/pdf.js/external/bcmaps/LICENSE` (2009), `hayro-cmap`'s `assets/LICENSE.txt` (2023) | **BSD-3-Clause** |
| Foxit standard-14 programs | `doc/pdf.js/external/standard_fonts/LICENSE_FOXIT`, from PDFium | **BSD-3-Clause** |
| Liberation Sans | `LICENSE_LIBERATION` | **SIL OFL 1.1** (reserved font name: ship and use freely, do not modify and keep the name) |
| poppler's `cidToUnicode`, `nameToUnicode`, `unicodeMap` | `poppler-data`'s `COPYING` | **GPL-2 or GPL-3** — Glyph & Cog's, *not* Adobe's |

`BSD-3-Clause` was already allowed. Its cost is three obligations: reproduce the notice and
disclaimer "in the documentation and/or other materials provided with the distribution", keep
them in source, never use Adobe's or Google's name to endorse this.

**The trap is the last row.** `poppler-data` is two data sets under two licences and says so. A
`CMap` gets code → CID; getting a CID to a glyph in a **non-embedded** CJK font needs CID →
Unicode, which is the GPL half. The permissive equivalent is Adobe's own `Adobe-Japan1-UCS2`,
`Adobe-GB1-UCS2`, `Adobe-CNS1-UCS2`, `Adobe-KR-UCS2` — BSD files inside the `cMap` directory,
counted. For an *embedded* CIDFont none of it is needed: the font's own charset or `/CIDToGIDMap`
answers.

| set | files | raw | compressed |
|---|---|---|---|
| the 14 standard fonts (10 Foxit `.pfb`, 4 Liberation `.ttf`) | 14 | **804 KB** | 177 KB gzipped (`.pfb`s alone) |
| Adobe `CMap`s, poppler's copy | 248 | **13 MB** | ~1.6 MB, one `zstd -19` archive |
| Adobe `CMap`s, `hayro`'s compacted blob | 1 | — | **250 KB** brotli, `include_bytes!` |
| Adobe `CMap`s, pdf.js's per-file binary form | 169 | 1.7 MB | — |

How four projects carry it: **poppler** a separate optional package read from a path;
**ghostscript** a symlink to poppler's, plus a package dependency; **pdf.js** pre-compiled binary
`CMap`s fetched lazily; **`hayro`** — the Rust one this tree benchmarks against — a crate of its
own, MIT/Apache for the code, one brotli blob behind a default-on `embed-cmaps` feature, with the
notice and a pointer to the fork that generated it. **`hayro`'s is the closest precedent.**

**Order, and it is not negotiable:**

1. **The attribution surface, before any byte of data lands.** Both licences oblige a *binary*
   distribution to carry the notices and this program has nowhere to put them. Three pieces: a
   `NOTICE` at the root; `pdf-viewer --licences`, which prints it and is what the licence text
   literally asks for, is testable and works headless; and the **menu with an About panel** the
   owner asked for. There is no UI toolkit here — draw the panel with what the tree has,
   `pdf-font`'s glyph outlines and `pdf-render`'s display list, which becomes possible as soon as
   step 2 lands. `cargo deny` reads Cargo metadata and **cannot see vendored data**, so the check
   that the notices are present must be a test of this tree's own.
2. **The 14 standard fonts — the bigger win and the smaller download.** 804 KB ends
   `substitute.rs`'s standing description of itself as "the only machine-dependent code in the
   tree". Downstream of that machine dependency: the oracle's 14 substituted-font pages,
   `text_render_modes.rs`'s "install a font or run this where one exists" panic, and session 125
   being unable to *see* its own fix because `checkbox-bad-appearance.pdf`'s tick is
   `/ZapfDingbats` and this machine has no such face — `FoxitDingbats.pfb` is exactly it. The
   integration point is `pdf_font::substitute::find`. **Recommendation: the compiled-in set wins
   for the standard 14 and the machine's fonts serve everything else** — those fourteen are the
   only faces a *document* names without supplying, so they are the only ones where the file's
   intent is known and a substitute is not a guess. Every render then reproduces on any machine.
3. **The predefined `CMap`s.** §9.7.5.2 states it as a `shall`: "A PDF processor shall support
   Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9 character collections." Two
   constraints, neither legal. **Principle 2**: 13 MB decompressed at first use is not "nothing
   eager", so the form must be per-`CMap` — a compiled-in index of name → (offset, length) into
   individually-compressed entries, which is pdf.js's shape and `hayro`'s, arrived at
   independently. **The parser already exists**: `pdf_font::cmap::CMap::parse` reads the same
   PostScript syntax these files are written in, because §9.7.5.4's embedded `CMap` streams use
   it — so ship the files as they are and measure that against a compacted form before building
   one. Whatever subset ships must be transitively closed under `usecmap` and should include the
   four `Adobe-*-UCS2` files.
4. **What none of it fixes**: the 27 documents whose composite fonts name an `Identity` ordering,
   where the codes are indices into a font nobody supplied.

**Provenance is a principle-4 question.** The tree has one precedent — `pdf-spec`'s Arlington
tables, built by `build.rs` from a pinned submodule. Vendored data arrives the same way: a
checked-in tool, a pinned upstream revision recorded beside the bytes, the licence file verbatim
next to what it covers.

### 2. The ledger, and where a false claim can still hide

- **Keep `REVIEW_OWED` empty.** A clause the code cites and nobody has read is the cheapest debt
  this project can accrue, and the list fails the build the moment one appears.
- **`FILE_ONLY_EVIDENCE_CEILING` is zero, asserted with `==`.** 58 → 0 over four sessions of
  auditing (ADRs 0098, 0100, 0101, 0102), **every one of which found a false or unheld claim**.
  It does *not* say the right test was named: three of the four false claims it hid were caught
  by the oracle rather than by a row.
- **The 33 `reported` rows have two known failure classes and both were found by reading.** A row
  whose reason is a true observation about the *wrong half of the sentence* (§12.5.6.4's icon was
  refused for stating no artwork, which it does, while requiring one anyway with a `shall` — ADR
  0109); and a row that names one of a clause's *two* populations (§11.7.4.4 governs `B`, `B*`,
  `b`, `b*` **and** glyphs in text mode 2 or 6 — ADR 0110). **All 33 that remain were read in the
  hundred-and-twenty-first and -second and none is of either class**: 17 are cryptographic
  validation needing a trust store or network, 5 need a second file, a media engine or a network,
  3 are icon clauses whose own verb is *should*, and the rest name a device or a user control this
  program does not have. That population is worked out.
- **The 242 `partial` rows are the population with no gate**, and reading them has paid four
  sessions running. What to look for, in the order the findings came: a note that *understates*
  the code (five in session 115); a note whose **reason** has expired — "while §X does not
  exist", "needs §Y" — which no gate can watch (117, 118); a note claiming an entry is *unread*
  where the tree reads it (three in 122). ~190 rows remain unread against the code. **The class
  that resists a grep is a note whose "what IS done" half is wrong**, because the name being
  present is what the grep would look for.
- **A gate cannot see a cache.** ADR 0115's defect drew wrong glyphs on two documents in silence
  for thirty-one sessions: no report, no contradicted page, and one of them sat on the text gate's
  "undiagnosed" list at 83%. **Where a lookup is memoised, ask what the key claims.** Every cache
  in the tree keys on object identity, checked one by one in session 128.
- **A silence is not a gap**, and the first move on one is neither a report nor a feature: work
  out what the clause asks *of this device*. §10.7.5's `/SA` was implemented in the half a display
  can state and recorded as a departure in the half it cannot; §11.7.4's overprinting was six rows
  a reading of Table 146 removed altogether.

### 3. What the corpus still names

**The oracle's 65 contradicted pages**, grouped and ratcheted in both directions in `oracle.rs`:
4 page rounding, 2 our own anti-aliasing at a shape's edge (§10.7.4's first departure, measured),
**9 glyph edges** whose ink matches the consensus to half a level (measured, session 75), 7 a
shared JBIG2 decoder, 1 a shared *gap*, 3 a link border, 1 a sub-pixel image, 1 a `CalRGB`
alternate, 1 an eight-bit mask value, 4 a `DeviceCMYK` conversion, 2 a reference that drew
nothing, 1 a CID width, 1 a negative line width, 14 substituted fonts, **14 unexplained**.

**Not one of the 14 is above its bound** — the list starts at 0.85, `issue7696.pdf` having left
it in the hundred-and-fortieth session. Rank before opening anything,
**by our worst measurement over the bound it is held to**; that has chosen the next item five
times, twice finding something that was not one page's problem at all (a rule nobody had
implemented, at 25.7×; the device transform, at 1.81, worth 11 pages). `issue7891_bc1.pdf` at
1.78 was measured in session 61 and is not a defect (trap 12). The 8 glyph-edge pages are one
population, measured in session 75.

Three entries that used to be here are the argument for spending the hour: `issue20504.pdf` was
worth 15 of 81 and was a whole subclause; `close-path-bug.pdf` was every dashed line in every
document; `issue11279.pdf` was §8.10.1 step c) on every form since the first. **The only way to
find out which kind an entry is, is to open `<target>/tmp/oracle/<stem>/p<n>/` and look at the
side-by-side first.** Two cautions: a page may be contradicted for a reason other than the one its
group names (seven for seven on the group being wrong), and "make it match mupdf" is the failure
principle 5 forbids.

**`colors.pdf` pages 1 and 2 left this list in session 68 and are not fixed.** All five renderers
agree about the swatch interiors to the byte and sit on a spectrum of edge softness; §10.7.4 asks
for the hard edge, this tree's anti-aliasing is that subclause's first documented departure, and
the pair the gate votes with is the pair nearest the clause. **A page can be contradicted by a
departure this project decided on purpose**, and `CONTRADICTED_ANTIALIASED_EDGES` says so.

**The 91 incomplete documents**: 41 fonts, 19 transparency, 10 annotations over 9 documents, 8
malformed images, 8 operator soup (`BT` without `ET`, `BDC` without `EMC`, fuzzed streams), 1
content. Session 59's reading of the corpus's own issue trackers says most of the font half is
glyph rasterisation on files chosen for having hard fonts, which session 68 then measured on one.

**The text gate's 42 below the floor**: ~31 are fonts where all three of §9.10.2's methods fail
*and* the program names nothing, 7 are right-to-left text read back in painting order (**not**
§14.8.2.5.3's `ReversedChars`, which no corpus document writes — measured in session 83), 1 is a
Symbol font naming Greek glyphs, 4 undiagnosed. **All of the undiagnosed agree with the reference
consensus on pixels**, so none of what is left is a drawing defect. Two left this list in session
127 and neither was diagnosed here — both were the font cache. **A text shortfall nobody can
diagnose may be a font nobody has looked up.**

### 4. Performance

**One fair comparison exists.** Every other renderer here is C; `hayro` is Rust, forbids unsafe,
and rasterises on the CPU single-threaded as we do.

| | 139th | 125th | 119th | 106th | 99th | 73rd | 65th | 58th |
|---|---|---|---|---|---|---|---|---|
| total, ours | **6.91 s** / 863 complete pages | 6.04 / 865 | 7.08 / 864 | 6.99 / 862 | 7.08 / 859 | 6.91 / 858 | 6.20 / 852 | 7.13 / 852 |
| total, `hayro` | 47.89 s | 41.02 | 41.28 | 39.59 | 49.03 | 41.87 | 34.93 | 39.03 |
| **median page** | **2.15×** slower | 2.16 | 2.13 | 2.14 | 2.15 | 2.14 | 2.15 | 2.29 |

**Quote a total against a total taken the same afternoon.** Both totals rose ~15% between the
hundred-and-twenty-fifth session and the hundred-and-thirty-ninth — ours 6.04 → 6.91, `hayro`'s
41.02 → 47.89 — while the median ratio moved 2.16 → 2.15. Two independent programs do not slow
down together; the machine did. **The ratio is the measurement and the totals are the machine**,
which is also why the 6.04 s was never an improvement on the 7.08 s beside it. In aggregate we are 6.9× faster than `hayro` on the complete pages and 14× over every page both
render, because their distribution has a long tail and ours no longer does; the totals and the
median answer different questions and only quoting both is honest. We are faster on 103 of the
863.

**Interpretation, by callgrind on `examples/callgrind_interpret`**: **2 150.8 M** today, of which
the text layer is 35.8 M (session 133's A/B, below). The six sessions from the hundred-and-thirty-
third cost **140 097 instructions, +0.0065%** — selection, editing, saving and §12.5.6.19's `/H`
are all paths a page render does not take. Session
124 rebuilt `0723cda` in the same sitting and got **2 119 519 869** against the 2 119.5 M session
119 recorded for the same commit — a *repeat*, not a drift. **So the 0.42% "drift floor" this file
used to quote is drift between machines and builds; an A/B in one sitting resolves far below it.**
Four sessions of change cost 3.3 M (+0.16%), of which §11.7.4.4's per-glyph bookkeeping is 1.1 M
(0.05%), measured by stubbing exactly that out.

Per-feature interpretation costs, measured the same way and kept because they are the only honest
scale for "what a feature costs": text rendering modes +0.46%, composite fonts +0.44%,
constructed appearances +0.34%, variable text +0.31%, **the text layer +1.69%** (2 114.8 M →
2 150.7 M, an A/B in one sitting), masking +0.12%, soft masks +0.05%, §14.7's
parent tree **+4.5%** (object streams the drawing path never touches; 885 of 974 documents pay one
dictionary lookup), and §8.4/§8.5's path rules **−0.21%** — collapsing consecutive `m` operators
leaves fewer commands to build than the rules cost to apply.

**Where interpretation goes on the median page** (session 58, the specification's own page):
`zlib_rs::inflate` **28.0%**, `Interpreter::show_text` 6.5%, `Lexer::next_token` 5.1%,
`inflate_table` 4.0%, AGL name lookup 3.2% — the last was ours and avoidable, and a cache took
interpretation 2 013.8 M → 1 989.1 M.

**The decompression item is priced and it is small** (session 128). Over one interpretation of
every corpus page: 6220 inflations of 38.08 MB; among the streams above 4 KB — 722 calls, 35.0 MB,
92% of the bytes — **35 are repeats costing 925 KB, 2.6%**, so a decoded-data cache is worth about
**0.7% of interpretation**, against a real memory cost, a bound to argue for and a liveness
invariant to write down. Below 4 KB the count is worthless: an address freed with one document is
handed to the next. **The benchmark page is not representative** — one 88 KB font program inflated
twice is 58% of *that* page's inflation and 2.6% of the corpus's. **Price an item on the corpus,
not on the page the profiler happens to open.**

**The worst page**, `bug1721218_reduced.pdf`: 144.05 G instructions → 54.05 G when a ramp stopped
carrying 256 stops for a linear function (ADR 0068) → **43.13 G** when the built shading was
cached per object (ADR 0069), re-measured unchanged in session 113. What is left, in order:
`tiny_skia::pipeline::lowp::gradient` 36.6%, `Mask::intersect_path` 8.1%, `build_soft_mask` 8.0%,
`fill_path_impl` 6.4%, `calloc` 4.5%. **The two mask lines are one item**: `MaskCache::get` is
24.34% of the page, 3608 chains, no eviction and no duplication worth removing (ADR 0103). The
shortcut nobody has taken is in `MaskCache::build`'s own comment — a child's band is inside its
parent's, so a chain could be one crop and one intersect instead of a fill and three; it needs the
intermediate clips cached and the page is at 87% of `MASK_BUDGET`, so it starts with a
measurement of what those cost.

**A page turn on the largest document was 380 ms and is 9 ms** (session 141, ADR 0124). §12.3.3's
`section_at` resolved every outline item's destination with `Pages::index_of`, which is a *search*
of the page tree — 988 items over 1023 pages, `O(items × pages)`, on the path of an arrow key.
`Pages::indices` gathers the whole map in one walk. Two more walks went with it: `Query::LinkAt`
6.05 ms → 52 µs and `Query::PageGeometry` 3.06 ms → 832 ns, both asked at pointer speed and both
`Pages::get` looking up the page already on the screen. **The gates cannot see any of this** — no
gate turns a page in a viewer and the specification's own PDF is in none of them — and the two
regression tests are *ratios* against a walk the test performs itself.

**The GPU backend's own question is open and has a plan** (ADR 0128, session 143). Page 6 is 5933
fills of **107 distinct outlines**, and Vello re-flattens all 5933 every frame; a glyph atlas is
therefore the largest single optimisation available to this program, and it is not reachable from
outside Vello. The plan is deliberately ordered so each step prices the next: stale-frame zoom
(perceived latency, host-side, judged *ugly but acceptable for now* by the owner), then **a glyph
coverage cache in `render-cpu`** — the same insight in the backend that is both oracle and startup
path, which prices the atlas before anyone writes a shader — then a moving window of interpreted
pages, then a spike of our own backend against Vello and `vello_hybrid`. A whole document cannot be
resident: 70 MB of draw records is affordable, the **4.0 s** to interpret 1023 pages is not, and
the startup rule decides it.

**Still open, priced or unpriced**: colour-managing an image in parallel (`issue19971.pdf`'s
3.4-megapixel photograph went 30 ms → 120 ms when `ICCBased` images began converting through their
profile; the loop is embarrassingly parallel apart from its memo and rayon is already here, and
nobody has tried it). Carrying an image *and its sampling intent* to the backends, which is one
`pdf-render` change unblocking three items — reduction happens at decode resolution, a mask of a
very different size is bounded rather than composited at device resolution, and the JPEG 2000
decoder cannot be told a target resolution.

Two fixes worth carrying as patterns: unpacking JPEG output cost 6.89 G until two paired
`chunks_exact` iterators took it to 1.25 G — **the safety habits this project enforces everywhere
are expensive in a loop that runs per pixel** — and `Triangle::is_subpixel` took
`personwithdog.pdf` from 17.3 s to 1.06 s *while* moving every mesh page closer to the references.

---

## Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

`--page N` opens at a page, `--cpu` draws with `render-cpu` instead of the graphics device, and
**`--trace` prints every window event, command, event and frame with its duration** — the last
line printed is the step that did not finish. `--trace` also installs a receiver for what `wgpu`,
`vello` and `naga` say about themselves, at `PDFVIEWER_LOG`'s level (default `warn`): those three
write to the `log` facade and a facade with nothing behind it drops every record, which is how a
page that would not draw produced no output at all.

**Rebuild the release binary before saying anything about speed, and at the end of any session
that touches the viewer.** The agent's builds go to `/home/AI/cargo-target/pdf-viewer/`, so
`release/pdf-viewer` there is what a person runs — and `cargo test` only ever builds the *debug*
one. The hundred-and-forty-second session was reported as "still lags" against a binary three
hours and six commits old, one of which was the 40× page-turn fix. A stale executable is a
measurement of the past.

Arrows / Page Up / Down / Space turn pages, Home and End jump, `+`/`-`/`0` zoom, the up and down
arrows scroll a page larger than the window, Escape quits. The title bar names how many things on
the page could not be drawn and the things themselves are printed. A click follows §12.5.6.5's
links and performs the eleven §12.6 actions this program can, printing every refusal — including
§12.7.6.4's import, which reads an FDF file **beside the open document** and nowhere else. A
locked document is asked for its password at the terminal (§7.6.4.1), three times, with an empty
line to give up. `--no-sandbox` decodes
JBIG2 and JPEG 2000 in-process — faster by a spawn and a pipe round trip, appropriate for trusted
documents, and it prints what it gave up.

## Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
cargo test -p conformance -- --nocapture   # 3067 citations, 318 quotations, 181 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild another
# package's binaries. Build it first or the numbers below are somebody else's (trap 10).
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus          -- --ignored --nocapture  # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle          -- --ignored --nocapture  # 1794 pages, ~30 s
cargo test --release -p pdf-model --test text_extraction -- --ignored --nocapture  # ~30 s
cargo test --release -p pdf-model --test dates           -- --ignored --nocapture  # 1545 dates
cargo deny check                           # from the workspace root: fuzz/ is its own workspace
cargo bench -p pdf-model
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret            # stops at the display list
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo build --release -p hayro-compare --bins && \
  cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf   # ~45 min
cd fuzz && cargo +nightly fuzz run lexer         -- -runs=50000   # needs nightly
cd fuzz && cargo +nightly fuzz run cmap          -- -runs=50000   # §9.7's CMap parser
cd fuzz && cargo +nightly fuzz run crypt         -- -runs=50000   # §7.6's algorithms
cd fuzz && cargo +nightly fuzz run variable_text -- -runs=50000   # §12.7.4.3's /DA and layout
cd fuzz && cargo +nightly fuzz run forms_data    -- -runs=50000   # §12.7.8's FDF, §7.9.4's dates
```

**Incomplete pages are compared and printed by the oracle but cannot fail it** — a page we
already say we cannot draw is expected to differ. **The denominator moves in both directions on
purpose**: it grows when reports stop firing (46 pages in session 21) and shrinks when a silence
ends (8 in session 17, 43 in session 8). A report must never be reached for as a way of making a
contradiction go away (trap 5).

Two classification counts are deliberately throwaway — scratch diagnostics do not belong in a
tree held to `clippy::pedantic`. **Whether a page's fonts are embedded** walks each `/Font`
resource and its `/DescendantFonts` for `/FontFile`, `/FontFile2`, `/FontFile3`. **The annotation
subtype breakdown** comes free from the corpus gate's own output:
`grep -o 'Annotation { detail: "[^"]*"' | sort | uniq -c`.

The oracle's first run on a fresh build directory is ~95 s and writes 319 MB of remembered
reference renders; every run after it is ~30 s. **Read the printed hit rate rather than the
clock.** Of that ~30 s, ~23 s is the three external renderers at a 99.7% hit rate; the rest is
ours — roughly 600 s of processor time over 24 cores on our own render, the comparison and the
artefacts. **If 30 s ever becomes the constraint, that is where to look and not at the
subprocesses.** `PDFREF_CACHE=off` asks the three renderers again — how "the cache changes no verdict" is
re-checked; `PDFVIEWER_ORACLE_ONLY=a,b` compares only matching pages in 0.2 s and refuses to check
the ratchets, saying so. `PDFVIEWER_CORPUS_TRACE=1` names each document as it starts, which is how
a hang is identified from a killed run.

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It arrives
through `iai-callgrind`, a dev-dependency reaching no shipped binary, and `deny.toml` records the
exception. Nothing to chase.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) holding the 974 PDFs. Optional to
clone — every test using it reports being skipped — but the ratchets mean nothing without it, so
CI must have it. **The time budget reports; it cannot enforce**: a Rust thread cannot be
cancelled, so a document that never returns hangs the suite rather than failing it.

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document`, decryption, §7.5.6's writer | Touches untrusted bytes first. `crypt.rs` is §7.6's standard security handler, every algorithm against its own subclause; `document.rs` decides *what* is decrypted, because that is where an object's identity is known (ADR 0031). `xref.rs` is §7.5 whole, and the thing to know is that an entry is an `Option<Location>`: a free entry and an unknown entry type both *record* that the number names nothing, because §7.5.6 makes a deletion the most recent copy of an object (ADR 0100). `tree.rs` is §7.9.6's name trees and §7.9.7's number trees in one module, because the second clause defines itself as the first with integer keys (ADR 0053). `text_string.rs` is §7.9.2.2 and Annex D's Table D.3, which is *not* ISO Latin 1. `date.rs` is §7.9.4, beside it because NOTE 1 makes a date a text string that happens to spell one (ADR 0092). `write.rs` is the *only* writing in the tree: clause 7's syntax on the way out and §7.5.6's incremental update, which appends and never rewrites (ADRs 0121, 0129) — and `Document::encrypt_for_update` beside `decrypt_object` is §7.6.2 in the other direction, so an encrypted document can be saved. `filter.rs` is §7.4's ten filters — four decoded here, one a pass-through for §7.6.6, four image codecs deliberately `None` so a *content* stream naming one is visibly unsupported |
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3, image decode | Where PDF semantics live. `annotation.rs` is selection and placement (§12.5.5) and knows no subtype; `appearance.rs` constructs what a subtype's clause states, splices under `/NeedAppearances`, and argues the refusals (ADRs 0030, 0032); `icon.rs` beside it is the one module that is pure invention and says so (ADR 0109). `view.rs` is the `ViewState` §12.6.4's actions change — **the precedent the edit log will follow**. `variable_text.rs` is §12.7.4.3 and the one place this tree *writes* a content stream. `image.rs` owns §8.9.6's and §11.6.5.2's masking, with `combine_on_the_finer_grid` the one place two rasters of different sizes are combined rather than refused, its `Decode` one table per component and its `Conversion` an *exact* per-image memo (ADRs 0034, 0035). `page.rs` is §7.7.3 and §14.11.2's five boundaries. `accessibility.rs`, `uri.rs` and `file_spec.rs` hold no PDF at all. Then one module per clause family: `action.rs`, `forms_data.rs`, `named_page.rs`, `structure.rs`, `article.rs`, `collection.rs`, `measurement.rs`, `thumbnail.rs`, `signature.rs`, `attachment.rs`, `page_label.rs`, `navigation.rs`, `requirements.rs`, `document_part.rs`, `viewer_preferences.rs` |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both simple-font encoding algorithms (§9.6.5.2, §9.6.5.4 — ADR 0015). `name_keyed.rs` is what a name-keyed program offers a code, and `cff.rs` and `type1.rs` each produce one because §9.6.2.1's NOTE 1 makes them one format's two spellings (ADR 0040). `type1.rs` is the one program kept *parsed*, measured: re-parsing per glyph put 11 ms on `tracemonkey.pdf`. `cmap.rs` is §9.7, where `Code` carries a value *and* a length. `substitute.rs` is **the only machine-dependent code in the tree** and ranks three sources of a request with an argument: the name, then §9.8.3.2's PANOSE, then Table 121's flags, which producers set carelessly (ADR 0086) |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. Three device decisions live here so the two backends cannot differ: `Image::is_smoothed`, `Image::area_averaged` (a departure from §10.7.4, ADR 0025) and `Stroke::device_width` (§8.4.3.2 with §10.7.5, ADR 0028). `Command::Group` is the one nested command; `MeshRaster` is §8.7.4.5.5 shared by both backends because neither rasteriser has the primitive and a second copy would drift (ADR 0051). `Transform::max_stretch` is *not* `determinant().abs().sqrt()`: a shear separates the singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path. `blend.rs` is §11.3.5.3's four non-separable modes written here rather than shared, on purpose: sharing them would make the cross-backend scene compare one implementation with itself (ADR 0047) |
| `render-gpu` | Vello/wgpu backend | Headless by construction. Its own soft-mask readback, because Vello's luminance mask is the SVG formula and no blend mode is a `/TR` |
| `viewer-core` | Toolkit-independent application logic | `Command` in, `Event` out, `Query` → `Answer` beside them (ADRs 0116, 0117). `select.rs` is every choice a selection needs and the standard does not state (ADR 0119); `interact.rs` is what a click does — §12.5.6.5's links and the eleven §12.6 actions; `notes.rs` is what a document says about itself when it opens. `viewer.rs` is the state machine and the one place a render is scheduled; `open.rs` is one document's page, zoom and scroll, and `fitted` there is why a page fitted to a window is not one pixel taller than it; `report.rs` words an `Unsupported` for a person, which is a presentation decision and so not `pdf-model`'s. `tests/headless.rs` is consumer #2 and the proof the crate's first sentence is true |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs`: a window, a keyboard, a GPU with `render-cpu` behind it for a page the device refuses (ADR 0125), and the two decisions a host owns — which files a document may name (§12.7.6.4) and what to do when one asks for a password (§7.6.4.1). Everything else is `viewer-core`'s |
| `pdf-sandbox` | Confined worker + three image filters | Its `decode.rs` is the only place a JBIG2, JPX or CCITT codestream is looked at |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here; `cache.rs` keys on the invocation itself (ADR 0020) |
| `tools/conformance` | Citation checker and ledger | Depends on nothing but `thiserror`. The one crate the citation scan skips — its own comments cite clauses that do not exist, deliberately |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it |

---

## Traps — read these before writing code

### 1. The metrics lie. Look at the page.

**The most important thing in this file.** `Interpretation::is_complete()` says what the
interpreter *knows* it skipped. It cannot say a font loaded and produced garbage, a page is upside
down, or a gradient came out opaque. The archetype: wiring bare-CFF support in made every affected
document report `unsupported: []` and render **almost no text**.

`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable` writes PNGs;
the oracle's artefacts are better. Two automated checks catch a wrong mapping, both in
`pdf-font/src/lib.rs`: `the_pdf_widths_agree_with_the_font_programs_own_advances` — the `/Widths`
and the charstring's own advance are independent statements of one fact — and
`an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`. Neither replaces looking.

**Every page a new feature makes drawable is a page nobody has ever looked at**, and the habit has
paid every session since the tenth: dashed squares that should not have been solid; a fax page
**upside down** because `/Rotate` 90 and 270 had been exchanged since the first page tree; a
gradient painted opaque because one `return` dropped §11.6.4.4's alpha; a `0 w` line invisible on
the GPU; `issue7901.pdf` drawing `üãÍ†Ë` because Table 115's presence condition was read as a
condition on meaning. **A page a feature makes drawable can be one that never rendered at all** —
a `no render` count is a to-do list of pages nobody has looked at, and it is 19.

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect. `issue6621.pdf` and `issue7901.pdf`
were both code that was right about the clause it cited.

**A contradicted page's group names a hypothesis, not a diagnosis — seven for seven on being
wrong.** Open the artefact before believing the label — **and measure it, because a label this
project wrote is still a label**. Twice the instrument that settled one was the font's own `cmap`,
`loca` and `post` tables read directly: ten minutes, exact.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape, so
composing the page-to-device transform into it yourself applies it twice. Both backends did, and it
shipped: every gradient mirrored about the page's centre line, and `issue19971.pdf`'s photograph
came out as one flat rectangle. Three things about how it survived: **no metric saw it**; **the
CPU-vs-GPU comparison could not**, because both had it; and **every scene compared them with a
gradient running along x**, where a y mirror is invisible.

Guards: `render-cpu/tests/shading_placement.rs` and `image_placement.rs` at three scales, plus
`headless_gpu.rs`'s vertical-gradient and image scenes. All confirmed to fail when the defects are
reintroduced.

**Five instances now, and each teaches a different edge:**

- **A convention that agrees with the clause is worse than one that does not.** `tiny-skia` draws
  a zero-width stroke as one device pixel, which is exactly §8.4.3.2 — so the rule was never
  written down and every `0 w` line was invisible on the GPU for fifteen sessions. **Where two
  backends are the oracle, a decision either can make alone is a decision neither has made**,
  which is why the device decisions live in `pdf-render`.
- **Three libraries, three answers, none the standard's.** §8.5.3.2's zero-length stroke:
  `tiny-skia` paints a projecting cap where the clause asks for none, `kurbo` drops the contour,
  and a one-`m` path is an error on one and silence on the other. `pdf-render`'s `degenerate.rs`
  states it once.
- **What a library cannot say at all.** §11.4.6's knockout is Porter-Duff Source modulated by
  coverage; Vello's layers compose over the layer's whole *bounding box*, so `Compose::Copy`
  erased a row outside the shape. **Where one backend states a clause directly and the other has
  to build it, the built one needs a scene at the magnitude *and* the fractional coverage where
  the two constructions differ** — the knockout scene has a diagonal edge for that reason.
- **A scene set is worth what its scenes can express.** Fourteen cross-backend scenes existed and
  every `Command` in all of them carried `BlendMode::Normal`, so sixteen blend functions had never
  been compared — and three disagree by 113 of 255. **Ask what parameter every scene leaves at its
  default.** ADR 0046.
- **A scene must fail at the defect's *magnitude* as well as in its axis.** The first reduced-image
  scene was in the right axis and **passed with the GPU's filter removed altogether**. Deleting the
  code a scene guards is one command, and it is the only thing that establishes the scene guards
  it.

### 3. An oracle is only as good as how it invokes the other renderers

The first run reported 54 documents whose page *size* we disagreed about. `pdftoppm` and `gs`
default to the **media box**; `mutool` and we use the **crop box**. Every invocation is now
explicit about the page box, *including* `mutool`'s, whose default was already right: a default
that silently changes is a comparison that silently changes. One level up: `gs` renders for a
**printer**, so Table 167's Print flag decides what it draws, and four link borders disagreed for
that reason alone. **Check what question each reference is being asked before reading its answer
as a verdict.**

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them was "the
caller's responsibility" and then did not, so every modern PDF failed with a misleading `/Root is
not a dictionary`. `pdf-syntax/tests/real_documents.rs` and `pdf-model/tests/render_real_pdf.rs`
run over everything in `doc/`. The converse is trap 8.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle: `Unsupported`, `FontError`, `ImageError`,
`CpuRasterError::UnsupportedCommand`. Do not "helpfully" fall back to a default that renders
something plausible. **A rise in the incomplete count is not a regression when it is a new
report.**

The rule is easiest to lose *inside* a partly-implemented feature, because the operator is handled
and the code path exists: `Tr` was parsed with four of its eight modes silently absent; Table 57's
`/LC`, `/LJ` and `/ML` read nothing while `J`, `j` and `M` set the same parameters. **Where a
clause gives a parameter two routes, implementing one of them is the failure mode that reports
nothing.**

**Six places report *while* drawing, each deliberate**, and the test for adding a seventh is that
suppressing either statement loses information: `/NeedAppearances` (stale appearances drawn
because they are all the file offers); §11.6.5.2's `/Matte` where pre-blending cannot be undone
(refusing would draw a rectangle of pure matte colour); a constructed appearance drawing what its
clause states while naming what it does not (ADR 0030); §8.11.4.4's `/User` and `/Language`
(switching a layer off would answer a question about this machine that nobody asked, ADR 0044);
§12.5.6.7's `/LE` and `/Cap`, which decorate a line the clause makes *required* — **so ask whether
the entry a refusal refuses is additive or substitutive**, and a cloudy `/BE` stays a whole
refusal because a different border is not an extra mark (ADR 0106); and a `/DA` font `/DR` lacks,
laid out in a stand-in **that declines where it cannot draw the whole value** (ADR 0112).

### 6. Colour: one conversion, and the specification often has no answer

Three separate `DeviceCMYK` → RGB conversions used to live here and they disagreed. Nothing about
a rendered page reveals that. `pdf-model/tests/colour_paths.rs` drives one value through all three
routes and demands they agree. **Add no fourth path**: `ColourSpace::to_rgb` is the only place a
colour becomes RGB, and `colour::xyz_d50_to_srgb` the only place an XYZ becomes a pixel — that
second rule exists because the same defect recurred one level down in a nine-constant matrix.

This file said for thirty-two sessions that ISO 32000-2 defines no `DeviceCMYK` conversion. **It
does — §10.4.2.5** — and what the standard actually does is *rank two answers*: §10.3's ICC route
for an ICC-enabled processor, which this is, and §10.4.2's "crude approximations" otherwise. The
three sources outranking the table are `/DefaultCMYK` (§8.6.5.6), an output intent's
`/DestOutputProfile` (§14.11.5), an `ICCBased` profile. Read ADRs 0009 and 0042 before touching
it. The same shape recurs for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point compensation
to the processor whenever `/UseBlackPtComp` is `Default`, which is every real document.

### 7. `#[expect]`, never `#[allow]`

Every lint exception is `#[expect(..., reason = "...")]`. It errors when it stops being necessary,
which has already removed several. A bare `allow` hides that forever.

### 8. A corpus finds what documents contain, not what the specification says

The mirror of trap 4. The ICC evaluator agreed with two other readers on every real profile; a
profile assembled *by hand* turned white into pure green. `calrgb.pdf` page 14 states
`BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, which Table 63 permits and no sane
producer writes.

**Three rules have been measured to be unreachable by all 974 documents, and the method is worth
as much as the finding.** §9.7.6.2's per-byte codespace test and §12.5.2's rule that a stored
appearance ignores `/CA` were each measured by breaking the rule and running both gates: all 1794
verdicts identical. §7.6.2's signature exception was measured more cheaply — eight documents carry
a signature dictionary, twenty-six an `/Encrypt`, and the two sets are disjoint, which is one
`grep`. **That turns "the corpus does not cover this" from a suspicion into a fact.**

**A fourth shape: a rule the corpus *does* exercise and cannot show you.** Three documents delete
an object in an incremental update and none still references what it deleted, so a reader that
resurrects a deleted object renders all three byte-identically.
`pdf-syntax/tests/cross_references.rs` pins §7.5's rules by hand for that reason, each as a *pair*
of files differing only in the rule.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle rests on ADR 0005: two implementations sharing no code agreeing about a page is
evidence. Four ways for that to fail.

- **A shared gap.** An unimplemented feature falls through to a *default*, so two unrelated
  programs that skipped the same clause produce the same picture. `visibility_expressions.pdf`:
  `mupdf` carries `/* FIXME: Calculate visibility from array */ return 0;` and `ghostscript`
  prints `WARNING: OCMD contains VE ... (ignoring)`, while `poppler` and pdf.js implement `/VE`
  and §8.11.2.2 is unambiguous. The page stays contradicted, with the source citations beside it.
- **Shared data.** `mupdf` and `ghostscript` disagree with us on four `DeviceCMYK` pages and agree
  with each other to under a level, because they run the same ICC profile. What settled it was
  *this tree's own* A2B evaluator pointed at `default_cmyk.icc`. **When two references agree
  suspiciously closely, ask what data they are both reading, and evaluate it yourself.** ADR 0048.
- **Shared code, wider than `jbig2dec`.** One `ldd`: `pdftoppm`, `mutool` and `gs` all link the
  same `libfreetype.so.6`, while we use `skrifa` and `tiny-skia`. On a page whose difference is a
  letter's edges the three references are one rasteriser. Recorded on `Reference::independence`
  and acted on nowhere — marking all three `Shared` for text would leave nothing to vote.
- **Two answers to two different questions.** `mupdf` constructs no link appearance while
  `ghostscript` renders for paper. Their agreement is a coincidence of two unrelated reasons.

The shape recurs with *us* in the minority: `mupdf` and `ghostscript` both refuse two files for
wanting a password, `poppler` and we open them, and §7.6.6 puts the refusal on the stream whose
key is missing. **Two against two is not a tie; it is a question with an answer, and the answer is
in the clause.** When a contradiction looks like "everyone disagrees with us", the cheap next step
is to search the other projects' source for the clause: a `FIXME` there is stronger evidence than
any number of agreeing pixels.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-sandbox's *library*, not its `pdf-sandbox-worker` binary —
Cargo never builds another package's binaries. So the tests run against whatever worker was last
compiled. Not hypothetical: the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed. `cargo test --workspace` or `cargo build -p pdf-sandbox --bins` builds
it. Both gates fail loudly if the worker is *missing* — and a missing worker and a stale one look
nothing alike.

### 10a. A cached reference render is a fourth thing that can be stale

The key is built from the invocation itself plus the renderer's version and the document's
SHA-256, so **a flag not in the key is a flag not passed to the renderer either**. What it cannot
see is a renderer whose output changes while its version string does not. **The variable names a
*directory* and only the literal `off` disables it** — `PDFREF_CACHE=on` silently starts a fresh
319 MB cache in a directory called `on`. **The hit rate is printed and it is the tell**: under 99%
on an unchanged tree means the corpus or a renderer moved. A remembered *timeout* is the one entry
whose truth decays, counted separately and expiring after a week.

### 11. A report is only as good as the condition it fires on

Trap 5's other edge. The reflex is to report whenever the unimplemented thing *could* be involved.
Four instances: §9.3.8's text knockout named 7 documents on one of the clause's two conditions and
took **three agreeing pages out of the gated set**; §11.6.2 named six, three of which set an alpha
to *zero* so there are no two portions to composite; §11.7.4's overprinting was 63 documents and
six `silent` rows and the honest condition has **no members** on this device; §12.5.6.19 fired
where the clause asks for nothing at all, naming 23 documents.

**Derive the condition from the clause, print what it matched before trusting the count, and cost
it in gated pages** — a page that reports is a page the oracle stops judging. **Both of §9.3.8's
conditions outlived the report**: they are what decides whether the implementation builds a group.
A condition worked out for a report is worth keeping when the feature lands. And the reverse worry
is real: **a report can hide another report** — `knockout_smask.pdf`'s knockout gap was covered by
its soft-mask report for four sessions.

### 12b. A test suite made of small scenes tests small scenes

Fourteen cross-backend fixtures — a gradient, a knockout group, sixteen blend modes — each a
handful of commands at one modest size. **The first real page at a real window's size came back
blank**, and nothing in the tree could see it: the corpus and the oracle rasterise with
`render-cpu`, so the GPU backend's only judge was those fixtures.

Vello sizes its GPU working buffers from constants "hand picked to accommodate the vello test
scenes"; a scene needing more overflows them *on the device*, which sets a flag, stops filling,
and returns `Ok(())` over a blank target. Page 6 of ISO 32000-2 at 1132×1600 is such a scene, and
1132×1600 is an A4 page fitted to a laptop window. ADR 0127.

Three rules out of it. **Ask what size every scene in a suite is**, not only which feature it uses
— ADR 0046 asked the feature question and this is the same question one axis over. **Where a
dependency returns success, ask what it does when it fails**: if the answer is "nothing visible",
that report is this project's to construct. And **a fix belongs on the path the person uses**: the
check first landed in `rasterize`, which is tier 1 and what the tests call, while the window draws
to its own surface through tier 2 — so the test went green while the black page stayed black.
`render_gpu::render_checked` is public for that reason and `Renderer::render_to_texture` is not
called anywhere in this tree.

And a fourth, from what enabling the feature cost: **a feature flag taken for one effect brings
its others.** `debug_layers` also makes vello hand wgpu a zero-length buffer slice whenever a
scene produces no lines — a blank page — and wgpu panics on it, which under `panic = "abort"`
kills the viewer. Two existing fixtures caught it; `keep_the_line_soup_non_empty` works around it
with one transparent rectangle. **Run the whole suite after turning a dependency's feature on**,
not only the test that motivated it.

### 12a. The display list's space is not the raster's, and a doc comment said it was

PDF's y axis points up from the bottom of the page; a raster's points down from its top row. The
flip lives in `TargetSpec::for_page` — "the page's top edge is raster row zero", ADR 0064 — and
**not** in `base_transform`, so a caller holding a pixel position must subtract it from the page's
height before asking `user_space_at` anything.

`user_space_at`'s own doc comment said it took a point in "the page's space — the display list's,
and the raster's" for seventy-five sessions, and **every click followed that sentence into the
mirror of the point it meant**. No gate clicks, so nothing saw it; the tests written for it took
their point from a grid scan of the broken mapping and asked whether *a* link was there, and on
the test document the mirror of a link is another link. ADR 0118.

Two rules out of it: **flip about the *page's* height, not the raster's** — the raster is rounded
up to contain the page and the spare fraction of a row is at the bottom — and **when a test needs
a point, take it from the document rather than from the code under test**.

### 12. A bound derived from two agreeing references is tighter than the arithmetic

`oracle.rs` judges us relative to how far the consensus references sit from one another. That is
right — it stops a page where every renderer differs from being called our defect — and **where
two references agree very closely the bound can be tighter than eight-bit arithmetic**.
`smask_luminosity_oob_transfer.pdf`: the closed form is `(223, 99, 80)`, `mupdf` `(222, 98, 79)`,
`ghostscript` `(223, 99, 79)`, we `(223, 100, 81)`. Everybody is within a level of the arithmetic;
the two references are within a level of *each other*, so the bound is 1.11 and ours is 2.02.
**Check the closed form** — write the clause's arithmetic down — then list the page with the
calculation beside it. Tightening our rounding until a reference's is matched is curve-fitting
with extra steps.

---

## Habits these sessions earned

Each was paid for once. Traps are about code; these are about how to work. Every one keeps the
anchor that makes it checkable.

### Reading the specification

- **A subclause is a checklist; check the code against it, not the code against itself.** §9.6.5.4
  names five routes from a code to a glyph; the code that stood in for it implemented one and a
  half — self-consistent, commented, and right about every document anyone had opened.
- **Read the whole subclause before believing the sentence that answered your question.**
  §12.7.4.3 opens by describing a processor *constructing* an appearance and closes by describing
  it *splicing* one.
- **Reading a silence is not reading the sentence it sits in — check the modal verb.** Table 175
  says a processor "**shall** provide predefined icon appearances"; three neighbouring tables say
  **should**. All four were read as one silence for a hundred and nineteen sessions. **"States no
  artwork" and "requires no artwork" are different claims**, and the question is not "may I fill
  this silence" but "does a sentence around it require me to". ADR 0109.
- **A clause can name more than one population in one sentence, and a summary will name one.**
  §11.7.4.4 governs "the B , B\* , b , and b\* operators … **and** the painting of glyphs with
  text rendering mode 2 or 6". **Where a rule lists the operators it applies to, count them against
  the code.** ADR 0110.
- **A claim that the standard is silent is a claim about the whole standard, and it is checkable.**
  Thirty-two sessions asserted no `DeviceCMYK` → RGB conversion exists; §10.4.2.5 is titled
  "Conversion from DeviceCMYK to DeviceRGB". Twice a recorded silence has been a clause four
  subclauses from one the tree cites constantly. `grep -n '^## '` the titles in `doc/md/` first.
- **"The clause says nothing" and "the clause says the opposite" are different findings, and only
  one is a licence.** Image reduction was recorded as unspecified from §8.9.5.3, which is about
  magnification; §10.7.4 says "there shall not be averaging over the pixel area". Only the second
  produces a *departure*, which must be argued and costed.
- **A departure is only honest once you have looked for the others.** One departure looks like a
  compromise; three in one subclause, all in the same direction, is a reading.
- **Where the standard defines nothing, refusing is a result.** `issue6621.pdf`'s `/Mask` is a
  one-bit greyscale image where Table 87 requires an image mask; both readings damage some file.
- **A clause read and dismissed is worth as much as one implemented**, and costs a minute against
  the 20 to 60 a review costs. **A cheap family review is where the expensive findings are** —
  clause 10 was picked because most of it was expected `inapplicable`; nineteen rows were, one was
  §10.4.2.5.
- **Where the standard defers to another document, the deferral is a citation.** §9.7.5.3 hands a
  `CMap`'s syntax to Adobe Technical Note #5014.
- **A default written in a table is not a suggestion**, and a comment arguing for a nicer one is a
  preference wearing a reason: `/MissingWidth` defaults to 0, and half an em cost `issue7439.pdf`
  six half-ems of invented space in one line.
- **A presence condition is not a restriction on meaning.** Table 115's `/CIDToGIDMap` is
  "Required for …" and then says what it *means*; reading the first as bounding the second drew a
  page as garbage. **Read what an entry's *value* means before branching on whether it is there** —
  the mirror: §12.5.6.7's `/LL` was refused on presence and its one corpus witness states `/LL 0`,
  Table 178's own "no leader lines".
- **A rule about how something is *encoded*, implemented as a rule about its value, is invisible
  forever.** §9.3.3 applies word spacing to "the single-byte character code 32", not to any code
  numerically 32.
- **Where two subclauses each condition a branch on one of two flags, the clause that defines the
  flags breaks the tie.** §9.6.5.4 cannot decide a font setting Symbolic *and* Nonsymbolic; §9.8.2
  calls the pair "a historical accident".
- **One dictionary, two clauses, and only the second says who wins.** §8.9.6 defines an image's
  `/Mask`; that an `/SMask` overrides it is in §11.6.4.3.
- **When two clauses disagree, ask which reading makes a file's own words mean nothing.** §12.5.2
  and §12.5.5 disagree about `/CA` beside an appearance stream; honouring both applies
  `highlight.pdf`'s 0.8 twice.
- **The clause can tell you two readers are one algorithm.** §9.6.2.1's NOTE 1 calls a CFF "an
  alternative, more compact but functionally equivalent representation of a Type 1 font program",
  which has now settled three design questions. **And a clause one analogy away is still the
  clause**: Table 124 forbids a `/FontFile` on a CIDFont, which answers what a *writer* may do.
- **Two callers of one clause can use disjoint halves of it.** §14.6.2 gives a property list two
  forms and §8.11 *cannot* use the inline one, so fifteen sessions proved nothing about it.
- **Ask what a feature looks like when its parameters are not their defaults.** Under `Identity-H`
  with `/CIDToGIDMap /Identity` both of §9.7's mappings collapse to nothing. **A parameter whose
  default is the unimplemented behaviour is a gap on every page in the world** (`Tk`), and **a
  default of `true` on an entry nobody implemented is a gap on every file that uses the feature**
  (Table 217's `/PreserveRB`).
- **A rule whose common case is the identity is a rule nobody tests, and the test written beside it
  will agree with it.** §7.6.4.3.2 step (a): for the *empty* password the wrong reading gives the
  same 32 bytes, so nineteen documents opened and every document with a password was refused.
- **A rule that changes nothing today can become load-bearing tomorrow.** Table 58's rule that one
  `m` overrides the previous changed no pixel until §8.5.3.2 made a single-point subpath a dot —
  then 205 unwanted dots on one page. **And a clause about the whole page can be invisible until
  one construction needs it**: §11.4.7 survived three reviews of clause 11's other families.
- **Ask what the clause requires of *this* device before deciding it is a gap.** Overprinting was
  63 documents and six `silent` rows until Table 146 was read against this device's colourants. **A
  gap sized by a corpus is a hypothesis about a clause.**
- **The standard sometimes states answers rather than rules, and those are the tests to write.**
  §12.4.2 gives no algorithm for Roman numerals — it gives nine labels beside a tree. **And a
  clause that states an algorithm can audit a corpus**: §12.3.3's `/Count` in three steps checked
  the reader against 146 producers at once.
- **§6.3.2.2 ranks what a corpus cannot.** Two gates taking the pdf.js corpus as their universe
  produce a demand curve, which cannot rank a requirement no file exercises.
- **A sentence inside a clause you have implemented can bind only a writer, and it becomes a
  requirement the day the program writes.** §7.6.3.2's "the initialization vector is a 16-byte
  random number" sat in an `implemented` row for a hundred and twenty sessions with no site in
  this tree, because a reader *reads* the vector. **After a session that gives the program a new
  verb** — writing, editing, pointing — **re-read the clauses it already claims, for the half
  addressed to the other side.** ADR 0129, and the same shape as ADR 0122's re-read of the rows
  whose reason began "this program has no".

### Judging against other implementations

- **Compare the references with each other before opening a page.** Four unexplained contradicted
  pages sorted themselves into one group from a table of pairwise means.
- **Rank the suspects by a ratio, not a distance** — our worst measurement over the bound it is
  held to. Five times it has chosen the next item before an artefact was opened.
- **Before believing "one pixel out" is rounding, compare the raster sizes.** One reference put
  type a row above ours from a raster *the same size as ours*, which no disagreement about row
  counts can explain. ADR 0064.
- **When a metric accuses you, find one that measures the same thing differently.** Eight text
  pages failed on mean absolute difference and passed every other bound; the page's *total ink*
  put us within half a level of both voting references. One number from artefacts already written
  turned eight questions into one population.
- **A page that draws the same glyph twice is an instrument, and it needs no reference at all.**
  `issue7696.pdf` is 200×50 and draws four glyphs twice, 80 pixels apart. `poppler`, `mupdf` and
  `ghostscript` draw the two halves *byte-identically*; ours differ by 2893 and `hayro`'s by 3541.
  That is grid-fitting measured from the inside — the three C renderers share `FreeType` and its
  hinting, the two Rust ones place a glyph where §9.4.4's matrix puts it — and it settles a
  contradicted page without comparing anything to anybody. **Ask what a page repeats.**
- **An inconsistency inside a reference's own output outranks any distance from it.** Two
  renderers spacing one line at two different widths cannot both be reading the document's `/W`.
- **Agreement with one reference is not evidence**, and **"both readers fail the same way" is
  agreement about a symptom** — `poppler` reporting the same broken flate stream was taken as
  proof a file was damaged; both readers were deriving the same wrong key.
- **Two references against two is not a tie and not a vote — it is a question with an answer.**
  `Type3WordSpacing.pdf` splits them over a `d1` glyph's stroke colour and Table 111 settles it.
- **An unimplemented feature has a default, and the default is usually "draw it".** That is a more
  common failure of the oracle's premise than shared code.
- **Point your own instrument at their data**, and **ask the reference the same question you asked
  yourself**.
- **A test corpus has a bibliography, and nobody had read it.** Every pdf.js file is named after an
  issue that says what is wrong with it. It corrected a written conclusion on the first afternoon.
- **A corpus document can be a conformance test, and then it outranks every renderer**
  (`issue14256.pdf` draws one picture eight ways) — **or check a decoder against itself** (an LZW
  image must decode to exactly `width × height` bytes; 96 documents encode one image ninety-six
  ways). Ask **what does this file already say about itself?**
- **Look at what a corpus file is *for* before filing it under a group.**

### Tests, gates and reports

- **A test asserted through the accessor that normalises the thing being tested is not a test.**
  §7.3.7's null-entry rule was checked through `Document::get_key`, which answers `Null` for an
  absent key. **And the accessor need not be one of ours**: `Object::as_dict` answers for a
  *stream* as well as a dictionary, so "the check box still has a dictionary of states" passed
  after the states had been replaced by a stream. `matches!(x, Object::Dictionary(_))` is the
  assertion; the way it was found is the next line. ADR 0130.
- **A discriminating test has to discriminate; check by breaking the thing.**
- **A constant that is right for the hand-built fixture is a landmine when a real file arrives.**
  `incremental_update.rs` replaced "object 1, the catalog", true of the file the test builds
  itself; in `bug900822.pdf` object 1 is the *encryption dictionary*, and the update wrote a
  catalog over it and produced a file no reader could open. Trap 12a's rule, one level up: take
  the identifier from the document, not from the fixture that happened to be first.
- **A test that skips silently is worse than no test.** A missing corpus is a skip; a present
  corpus that lacks what the test needs is a **panic**.
- **A gap measured on both sides is a fact; measured on one side it is an accusation.**
- **A gate cannot ratchet what has no consumer**, and **fixing an instrument can be worth a
  feature** — one line moved 25 pages into the judged set and showed one drawing nothing.
- **A page can leave the contradicted list without a pixel moving** (the tolerance class comes from
  what *we* drew, so anything improving extraction loosens a bound — take the raster's digest
  before writing "fixed") **and can leave with pixels moving and still be wrong** (`issue20232.pdf`
  agreed once the y flip was fixed and still draws `56` where three references draw `⌀56`).
- **A page can be visibly wrong inside a verdict the gate cannot fail on.** `issue7406.pdf` drew a
  JPEG cyan-on-black and its verdict was `ambiguous` before and after — 46% of the judged set lives
  there and nothing watches it.
- **A report has a price, paid in gated pages.** Print what a condition matched before trusting its
  count; **measure the corpus before choosing between reporting a gap and closing it** (every
  `/Decode` array in all 974 documents is Table 88's default or its exact reversal).
- **A "not implemented" count of zero can mean "nothing reports it".** `/FontFile` was recorded at
  zero while 57 documents embedded one and drew a substitute in silence.
- **A report that arrives with a fix is worth reading twice**, and neither is a regression however
  it looks in the count.
- **Build the strong gate, then let its own output tell you it is wrong.** A table-attribution
  checker failed fourteen of twenty-five references and all fourteen were correct writing; what
  shipped asserts the weaker true thing and *prints* every cited table's title.
- **A citation nothing checks is a citation that rots**, and **a gate that reads one file format
  checks one file format** — the ledger is 823 notes about ISO 32000-2 and the citation gate read
  Rust sources, so none of it was checked. **A `§` means one document**: `RFC 3986 §5.2` is right
  about the RFC and ISO 32000-2 has a §5.2 of its own.
- **A bucket that means "we failed" must not also come to mean "you have not told us the
  password".** When a ratchet fires on a change you believe in, ask whether the *category* is wrong
  before the number.
- **A gate's numerator moves when its denominator does, and only one of those is news.**
- **A count taken at one call site is not a count.** "Parsing was never the cost" was written after
  instrumenting the pattern path, which runs once where `sh` runs 3576 times. Instrument the
  *function you are accusing*.
- **A number in this file is a claim, and attributing it is a second claim.** `calloc` was 4.5% of
  a page and this file said it was the group's pixmap; `Pixmap::new` is 0.14%. Ask
  `callgrind_annotate --tree=caller`. ADR 0103.
- **Four plausible optimisations, four counts, four refusals — and counting was cheaper than
  any of them**: 0%, 1.3%, 2.5%, and a `Vec::reserve` per show string that *cost* 0.47%.
- **A profile ages past its conclusion, and the conclusion is what survives being read.** One
  profile was carried nineteen sessions; re-measured, its largest item was *four times* the share
  recorded and the sentence beside it had named the fix correctly the whole time.
- **A ratio has two ends, and this file has quoted the wrong one.** Quote the absolute number you
  control.

### The ledger, and claims about this tree

- **When two clauses describe one mechanism, reviewing one leaves the other lying.** Four instances
  in ten sessions; the check is one `grep` for the *other* clause a family cites.
- **A retired claim is a string, and strings are greppable.** When a session disproves a sentence
  this tree repeats, the work is done when the *sentence* is gone. "Vertical writing is refused"
  was true until session 36 and still written in four places in session 122 — a ledger row, a doc
  comment and two paragraphs of this file. ADRs 0101, 0111.
- **A prose claim about the code can be turned into a grep, and twice that has paid.** Session 118
  swept the notes for expired reasons ("while §X does not exist"); session 122 for sentences
  claiming an entry is *unread*. Twenty minutes apiece, three live findings apiece.
- **A comment that names a refusal outlives the refusal.** `appearance.rs`'s header listed
  §12.5.6.10's four text markups among things that "state no mark" for eighty sessions after the
  same file started drawing all four. A header is where a reader learns what a module refuses. ADR
  0105.
- **A stale row can understate as well as overstate, and only the overstatements have a gate.**
  Session 82 met six understating rows in one family. **A `silent` count is a *lower* bound on what
  exists.**
- **A row whose evidence is a file can be `implemented` for something the file never touches.**
  §8.7.4.5.2: fourteen tests in `shadings.rs` and not one a `/ShadingType 1`. That is what
  `FILE_ONLY_EVIDENCE_CEILING` counts.
- **A ledger note is a hypothesis the gates test, not a conclusion they inherit.** Three
  `implemented` rows claimed behaviour the code never had, each written from the clause during a
  review, each costing a visible defect, each found by the oracle.
- **A note that gives a reason gives a trigger, and nothing fires it.** "While §11.4.6 does not
  exist" expired forty-six sessions before anyone noticed. ADR 0107. **A row that names a
  *blocker* rather than a gap is the class no gate can watch** — one regular expression over the
  notes finds them in twenty minutes. ADR 0108.
- **A warning written into a ledger note before the code exists is a warning nobody reads when the
  code arrives.** §7.11.2.1's row named a defect three call sites had for as long as they existed.
  ADR 0104.
- **A feature can make a clause reachable, and nothing announces that.** Table 192's `/H`
  describes what happens when a mouse button is pressed, and until the hundred-and-thirty-second
  session nothing pressed one — implemented one session after it was noticed, ADR 0123. **After a
  session that adds a *capability* rather than a clause, re-read the rows whose notes give a
  reason beginning "this program has no".** ADR 0122.
- **An `inapplicable` row decays exactly as a `silent` one does.** §12.7.4.2's field names were
  `inapplicable` on sound reasoning until §12.6.4.11's hide action made a field name decide
  whether an annotation is drawn.
- **A ledger with a status per subclause can find a missing *component*, not only a missing
  feature.** Four rows in two clauses named one absent data structure — a name or number tree —
  which no clause review would have shown and no corpus document would have asked for.
- **A count taken over what you touched is not a count.** This file said clause 7 had no
  `unreviewed` row for six sessions, because the count was taken over the families a session had
  touched.
- **A ledger row is an entry, and an entry gets measured before it gets believed. Price the work
  before believing a reason not to do it.** `mesh_shading_empty.pdf`'s entry said for fifteen
  sessions that closing it needed a Gouraud rasteriser in both backends — true, and one shared
  raster satisfies that constraint *better*, in less code.
- **Read this project's own lists for the sentences that admit ignorance, not only the counts.**
- **Whatever this file asserts, run it once.** "Clippy clean" was claimed while eleven warnings sat
  in the tree.
- **A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have no
  memory-safe implementation" sat in `PLAN.md` as a reason, true when written and false for
  months. **Anything deferred on an external condition should carry the date it was last
  verified.**

### Measuring

- **Wall-clock benchmarks lie under load; count instructions instead.** One change measured as a
  24% regression and an 8.5% improvement twenty minutes apart. **A/B in one sitting**, and measure
  the baseline on this machine rather than trusting a number in this file.
- **Attribute a regression by removing the suspect, not by reading the profile.** The profile shows
  the *shape* of the extra work, not its cause; one stubbed field said 96 of 110 M.
- **When a page's error has a suspiciously round size, do the arithmetic.** Seven pixels of
  gradient where there should be an edge is 1800 ÷ 256.
- **Profile before believing an explanation, even one whose arithmetic matches.** A 48-second page
  was attributed to clip masks with `3576 × 485 kB = 1.7 GB`, exactly the memory held and silent
  about the time: callgrind put the masks under 4% and the gradient at 78.9%.
- **A suspiciously clean measurement is a reason to check the instrument.** Four callgrind numbers
  flat to four significant figures meant the benchmark was panicking and callgrind was faithfully
  counting the panic.
- **Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's 85
  seconds as the price of having an oracle; 95% was three programs re-answering a question.
- **Measure before optimising, and delete what does not measure.** A `FontRef` cache changed a dense
  page by less than noise and was removed with the reason recorded; the same session's real win was
  hoisting a string allocation, 1.37 ms → 18 µs.
- **An eager lookup on a cold path is a hot-path cost when the path runs per object.** Reading
  `/AcroForm` per constructed appearance was 2.7× the whole feature's cost.
- **A cost written down beside one call is not a cost anybody adds up.** `Pages::index_of`'s doc
  comment says it is a search that cannot skip a subtree and names the two callers it was written
  for; a third arrived, called it *in a loop over 988 outline items*, and inherited the comment's
  blessing without its argument — 344 ms of every page turn. **Ask of any function documented as
  expensive: who calls it in a loop.** One `grep`, and it found a second (`named_page`). ADR 0124.
- **A failed frame must not be reported as a drawn one.** `viewer-ui` answered
  `Rendered::Presented` when its GPU path refused a page, so the core recorded the page as shown,
  never asked again, and the window kept the *previous* page under a title bar naming the new one
  — a page a person cannot view and no reason given. It now answers `Rendered::Failed`, draws it
  on the CPU backend instead (which is what `CLAUDE.md` keeps that backend for), and says which.
  **And a refusal is recorded as an answer**: the scheduler must not re-ask a question whose
  answer cannot change, or the two spin. ADR 0125.
- **A performance defect on a path no gate walks is found by a person using the program.** The
  corpus interprets page one, the oracle renders pages it is handed by index, and neither turns a
  page. The largest document this project owns — ISO 32000-2, 1023 pages, committed in `doc/` —
  was in no gate at all until session 141 made it two tests.
- **Look at what a safe idiom compiles to in a loop that runs per pixel.** `.round()` on a clamped
  float is a library call — 205 M instructions on one page, 10.7%.
- **The exact fix is often available and is usually better than the approximate one.** A memo keyed
  on the input tuple beat an interpolated lookup grid: 3249 M → 1075 M, and simpler.
- **A change made for correctness that is also an order of magnitude faster means the old code was
  doing work that was worse than useless.** One mesh raster replaced 4096 flat pieces: 35.47 G →
  3.08 G, and closer to the references.
- **When the first design of a fix is the obviously safe one, still measure it.** Refusing to cache
  timeouts is unarguable in principle and left two pages accounting for 46 of 57 seconds.

### Code, bounds and dependencies

- **A gap inside a feature you have implemented does not announce itself.** Every missing
  *subsystem* reports, because whoever decided not to build it wrote the report. **A fast path
  inherits none of the rules of the path it skips.**
- **A "nothing here" is data, and dropping it is not the same as recording it.** §7.5's free
  entries and §7.5.8.3's unknown entry types both say an object number names nothing; both were
  *skipped*, so the question fell through to an older section and the reader resurrected objects
  its own file had deleted. **Ask what a `continue`, a dropped branch or an unmatched arm hands the
  question *to*.** ADR 0100.
- **The archetype is the `d` operator.** Every layer of dashing existed and one line read only the
  *empty* array, so not one dashed line in 974 documents. When a feature looks finished, check the
  operand path from the content stream to the state. **A feature switched off in one place is
  switched off everywhere it is not switched on**, and **a clause whose operators are implemented
  can still be unread** (`J`/`j`/`M` from the first commit; Table 57's `/LC`/`/LJ`/`/ML` for
  twenty-three sessions).
- **A cache is a claim that two things are the same, and the currency of the claim is the key.**
  The font cache said it in the weakest one available — a resource name, which §7.8.3 scopes to the
  dictionary that defines it — and handed a form `XObject`'s `/F1` the page's glyphs for
  thirty-one sessions. Every other cache keys on object identity. ADR 0115.
- **A display list holding the right commands can still draw nothing, and no report will say so.**
  A type 5 mesh was complete, correct and 180 points from where it belonged. Between "we could not
  build it" and "we drew it" there is a third state only the oracle catches.
- **A representation can forbid a correct answer.** No evenly spaced array of colours can express a
  discontinuity. Ask what a data structure *cannot say*.
- **A parser that recognises a delimiter without parsing it will be read as parsing it.**
- **An operator that is matched and ignored may still be a rule.** `BX`/`EX` sat with `MP`/`DP` for
  thirty-one sessions; §7.8.2 makes them the one place an unrecognised operator is not an error.
- **Where a clause states arithmetic exactly, two independent implementations are worth more than
  one shared one** — trap 2 sends a device *decision* to the shared crate; §11.3.5.3's formulas are
  the other kind. **Two rasterisers disagreeing is information; two agreeing is not proof.**
- **An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
  backend demultiplied Vello's output for fifteen sessions; every scene rendered onto an opaque
  background.
- **Ask which arm of your own enum no test has ever taken.** `Rendered` has a variant for a
  tier-1 host and one for a tier-2 host; twelve tests played tier 1 and the tier-2 path asked
  for the same frame for ever. The variant existed, the doc comment explained it, and nothing had
  ever sent it. ADR 0117.
- **A number computed to fit must be checked against the rounding of whatever consumes it.** A
  page fitted to a window by `viewport / extent` is one pixel too tall about half the time,
  because `TargetSpec::for_page` rounds a raster *up* to contain the page and the nearest `f32`
  to the exact ratio is above it as often as below — a fitted page with a scrollbar. The fix is
  not an epsilon: step to the next representable scale until the consumer's rounding lands. ADR
  0116.
- **Two copies of a constant is one defect waiting.**
- **A constant that is a property of the state must reach every paint, including the ones that
  replace the colour.** A shading replaces the current colour, and the line that returned it
  dropped `ca`.
- **A clamp is a decision.** `width.max(0.0)` reads as hygiene and was this program's whole answer
  to a value §8.4.3.2 forbids. Ask what a `max`, `clamp` or `unwrap_or` *decides*.
- **A fallback that fills the page is worse than one that leaves it blank.** "If nothing else
  matched, the code is the glyph index" drew `v 0' ' W` for `What's an interval?`. **What makes a
  fallback legitimate is where the answer comes from, and it is measurable**: §9.10.2's permission
  is taken by asking the *program* what it drew, and the readback rose 96.5% → 97.8% with **no
  document moving the other way**. A fallback that invents text lowers a score somewhere.
- **An optional entry must not erase what the clause states**, which is now four ADRs: a line
  ending (0106), an `/Encoding` name Table 112 does not permit (0111), a `/DA` font `/DR` lacks
  (0112), a missing `/BBox` or `/Rect` (0113, 0114). **And a stand-in may not fall short** — the
  first version of ADR 0112 drew six dots of Arabic punctuation on an otherwise empty page.
- **A shortcut right on the common case is worse than one wrong on all of them.** The Cal-space
  pass-through was nearly correct for `/Gamma 2.2` and badly wrong otherwise, and nothing
  distinguishes the two at runtime.
- **Silent caps are defects, not safety**, and **a bound written for the pathological case can
  refuse a reasonable one** — the bound belongs on the *growth*.
- **A panic in a dependency is a symptom, not a diagnosis**, especially where its arithmetic is
  modular. **Being right for the wrong reason is worse than being wrong.**
- **A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
  `DCTDecode`, `skrifa` font parsing, `flate2` Flate, `tiny-skia` rasterisation. ADR 0014. **A
  dependency can implement more of a specification than the clause cites** — `read_fonts::ps::agl`
  gives the Adobe Glyph List *and* its specification's algorithm. **Look in `read-fonts` before
  writing font-format code**: an earlier handover specified ~80 lines of CFF charset parsing that
  already existed. ADR 0006.
- **The interesting half of a "viewer feature" is usually a clause.** Of the click that follows a
  link, the mouse is four lines and the rest is Table 176's three conditions, §12.5.2's coordinate
  space and §7.7.3.3's rotation.

---

## Things worth knowing

- **The oracle's artefacts are the fastest diagnostic in the tree.** Every non-agreeing page leaves
  `<target>/tmp/oracle/<stem>/p<n>/` with our render, each reference's, a side-by-side strip and a
  heatmap per reference. **Open the side-by-side first**: one image, four panels, ours leftmost,
  and it has explained every page it was pointed at. Agreeing pages have theirs deleted, so what is
  on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text or vector
  tolerance from our own render's content, so a change that adds glyphs also loosens the bound.
  Since session 31 the question is `Interpretation::glyphs` — "did glyphs mark the page" — rather
  than "did we read text back", which had made a page of unnameable CJK a vector page and a page of
  invisible OCR text a text page, both backwards.
- **The sandbox is a flag and the default is the safe one.** `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety. There is deliberately no path that falls
  back to in-process decoding when the worker fails to start.
- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the only
  channel a font has, so a font that maps *some* of its document's codes draws those and says
  nothing about the rest. The general case needs a report where a glyph is *shown*, which needs
  `LoadedFont` to distinguish "this code has no glyph" from "this code's glyph is blank", which a
  space legitimately is. Not hard; not done; measure the volume first.
- **`doc/md/` is the specification in a form code can read** — markdown conversions of the 14
  PDFs, committed, so a test may depend on them without a skip path.
  `ISO_32000-2_sponsored_EC3.md` is 24 MB and its 860 `##` headings give a clause number, title and
  line range apiece, which is the whole basis of the citation checker and the ledger. **Three
  caveats, each met the hard way**: it is a *conversion*, so a quotation the checker cannot find
  may be an artefact — check `doc/`'s PDF before editing the comment; one heading number
  (`14.8.4.7.3`) occurs twice; and **the conversion drops content** — Table 164's `/Di` row ends
  "Default value: 0." in the PDF and the markdown has no such line, so a reading taken from
  `doc/md/` alone would record a silence the standard does not have. `pdftotext -layout` over the
  PDF is the check, and a **table** is where to expect it. When a gate accuses the standard of a
  gap, suspect the conversion first. Extract spec data from here rather than writing it from
  memory; `grep -v '^!\[Image\]'` first, the files carry base64 images inline.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's note on
  black point compensation by ISO 32000's own co-project-leader, and it settled a design question
  the base specification leaves to ISO 18619 — while sitting unread as the same question was being
  answered by looking at other renderers.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding` must be
  one of three names; it does not say what those encodings contain.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `render-cpu/src/lib.rs`, ADR 0010. The device transform handed to a command already carries the
  band's row offset, and the clip mask is band-tall and page-wide because `tiny-skia` needs it to
  share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello a layer stack;
  both translate. That neither library's model is native is the evidence the neutral form is right.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter. A test
  pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second metric.** The references disagree
  with each other at worst-tile 26–28 on text pages — hinting, not error — and no threshold fixes
  that. `raster_compare::Comparison::structural_similarity` asks whether the same shapes are in the
  same places; `Tolerance` bounds it at 0.99 for vector, 0.90 for text, measured over 153
  reference-against-reference pairs. The distribution is **continuous** — 0.8990, 0.8993, 0.8998,
  0.9009 all occur — so 0.90 is a choice about which population to exclude, not a discovered
  boundary.
- **Reference renderers are given 30 seconds and then killed.** `Command::output` waits forever;
  `Reference::render_within` polls and kills, and there is deliberately no unbounded variant.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That pairing let
  the harness work before a parser existed.
- **Debug builds are ~15× slower here**, and it changes what a test can assert: the corpus gate is
  2 s in release and minutes in debug. Run timing assertions in release and say so.

## Environment

Arch Linux. GPU: AMD Strix (RDNA 3.5), RADV, X11. The agent runs as user `AI` via `sudo -u AI`,
reaching `/home/cl/projects/pdf-viewer` through the `coders` group.

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`: `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie**, so anything needing *the user's* display fails at
  `XOpenDisplayFailed`. **The viewer can still be run, and this file said otherwise for dozens of
  sessions** (ADR 0126): `Xvfb` and `lavapipe` are installed, so the real window, the real event
  loop and the real vello surface all work, `xdotool` drives them and `xwd` photographs the
  result.

  ```sh
  Xvfb :77 -screen 0 900x1100x24 &
  DISPLAY=:77 pdf-viewer --trace doc/ISO_32000-2_sponsored_EC3.pdf &
  DISPLAY=:77 xdotool windowfocus --sync $(DISPLAY=:77 xdotool search --name ISO_32000 | tail -1)
  DISPLAY=:77 xdotool key --delay 300 Right Right Right
  DISPLAY=:77 xwd -root -silent | magick - screen.png
  ```

  **This is the only way to exercise the loop** — key press to command to request to frame to
  window — which is where every defect of sessions 140 to 142 lived and which no gate touches.
  Not a gate itself: `Xvfb` and `xdotool` are not build dependencies and a test that skipped
  silently would be worse than none.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via `~/.cargo/config.toml`,
  so the two users never fight over `target/`. Do not "fix" this. `pdfref` needs `--work-dir` for
  the same reason.
- **`cargo-fuzz` needs `+nightly`** explicitly; `rust-toolchain.toml` pins stable 1.97.1
  deliberately. `cargo-deny` is in the agent's `~/.cargo/bin`.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build without
  `git submodule update --init`.
- KDE Frameworks 6 packages on Arch have no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).

---

## How the project got here

One line per session; the argument is in the ADR, and every durable lesson is in Traps or Habits
above rather than here.

| Session | What landed | Where |
|---|---|---|
| 5 | The reference oracle, over every page of the corpus | ADR 0011 |
| 6 | `CalGray`/`CalRGB` through XYZ; annotation appearance streams | 0012, 0013 |
| 7 | JBIG2 and JPEG 2000 in a sandboxed worker; the first speed comparison | 0014 |
| 8 | §9.6.5.4, the `TrueType` code-to-glyph algorithm, in full | 0015 |
| 9 | The conformance ledger and citation checker; optional content | 0016, 0017 |
| 10 | Type 3 fonts; dashed lines, which had never been dashed | 0018 |
| 11 | Inline images; `/Interpolate`; `Indexed`, `Separation`, `DeviceN` images | 0019 |
| 12 | A cache for the oracle's reference renders; `CCITTFaxDecode`; `/Rotate` | 0020, 0021 |
| 13 | All eight text rendering modes; §9.3 and §9.4 reviewed | 0022 |
| 14 | `/Mask` in both forms; §11.6.4 reviewed | 0023 |
| 15 | Soft masks at any resolution and `/Matte`; a shading carries `ca` | 0024 |
| 16 | Area averaging for reduced images; §10.7 reviewed, and it forbids what was built | 0025 |
| 17 | Transparency groups; the page group is isolated | 0026 |
| 18 | Soft masks in an `/ExtGState`; overprinting is silent | 0027 |
| 19 | `/SA` and the device's thinnest line; overprinting is *not* a gap | 0028 |
| 20 | Embedded `CMap`s and `/CIDToGIDMap`; the whole of §9.7 reviewed | 0029 |
| 21 | Constructed annotation appearances; `/CA` belongs to the construction | 0030 |
| 22 | Encryption, every revision and method §7.6 states; a locked file is not unreadable | 0031 |
| 23 | §12.7.4.3's variable text; regenerating an appearance is a splice | 0032 |
| 24 | §8.5.3.2's degenerate strokes; an empty clipping path admits nothing | 0033 |
| 25 | §8.9.5.2's `/Decode` array in full; a fast path inherits no clauses | 0034 |
| 26 | An image's colour space is a fill's; an exact memo where a grid was obvious | 0035 |
| 27 | `LZWDecode`, the last standard filter; a corpus stating an invariant about itself | 0036 |
| 28 | A shading's `/BBox`; a contradicted page's diagnosis refuted by measuring it | 0037 |
| 29 | `/UserUnit`, and the geometry list emptied | 0038 |
| 30 | An embedded program's own encoding is the base; `/MissingWidth` is 0 | 0039 |
| 31 | Bare Type 1 fonts (`/FontFile`); the oracle's tolerance asks whether glyphs were drawn | 0040 |
| 32 | All five bit depths; an inline image's abbreviated keys win; `BX`/`EX` | 0041 |
| 33 | §10.4.2.5 exists; Table 57's `/Font`; the whole of clause 10 reviewed | 0042 |
| 34 | §12.5.6.10's text markup appearances; `REVIEW_OWED` emptied | 0043 |
| 35 | §8.11.4.4's usage application dictionaries — the last original `silent` row | 0044 |
| 36 | Vertical writing: §9.2.4's second set of metrics, §9.7.4.3's `/W2` and `/DW2` | 0045 |
| 37 | The blend-mode scene nobody had written; clause 11 completed as a review | 0046 |
| 38 | Clause 8 completed as a review — the graphics clause, 20 rows | 0046 |
| 39 | §11.3.5.3's four modes taken back from `tiny-skia` | 0047 |
| 40 | Four unexplained pages are one shared ICC profile; clause 7 complete | 0048 |
| 41 | A CID into a bare Type 1 program; clause 9 complete | 0049 |
| 42 | A suffixed glyph name is the program's, not the AGL's | 0050 |
| 43 | One mesh raster instead of 4096 flat triangles | 0051 |
| 44 | A font's own tables say which way round its offsets are | 0052 |
| 45 | A contradicted page's label measured and replaced; §12.6's actions reviewed | — |
| 46 | Clause 12 completed as a review; the median page profiled at last | — |
| 47 | A negative line width is a choice, written down; §14.7 reviewed | — |
| 48 | Name and number trees, and §12.4.2's page labels on top of them | 0053 |
| 49 | §12.3.2's destinations, all three spellings | 0054 |
| 50 | §12.3.3's outline, and a `/Count` the clause states as an algorithm | 0055 |
| 51 | A tiling pattern's cell clipped to its `/BBox`, per cell | 0056 |
| 52 | A pattern inside a form maps to the form's space | 0057 |
| 53 | A glyph filled with a tiling pattern is tiled | 0058 |
| 54 | A ramp that can hold a step | 0059 |
| 55 | §14.9.4's `/ActualText`, and the property list that was never a dictionary | 0060 |
| 56 | **The ledger reaches zero unreviewed rows** | 0061 |
| 57 | A click follows a link | 0062 |
| 58 | Everything re-measured, and one feature's cost attributed | — |
| 59 | The corpus's own bug trackers read; a written conclusion corrected | — |
| 60 | §14.9's four accessibility entries, in both places each may sit | 0063 |
| 61 | The page's top edge is raster row zero; 11 contradicted pages agree | 0064 |
| 62 | §12.6's actions, and the third input a viewer has | 0065 |
| 63 | A third gate: the text, over the whole corpus | 0066 |
| 64 | §9.10.2's closing sentence is a permission, and three documents took it | 0067 |
| 65 | A ramp is not a gradient: 144 G instructions become 54 G | 0068 |
| 66 | §14.9 completed, and the text gate's remaining list is all naming | — |
| 67 | Table 99's layer-panel half: `/Order`, `/ListMode`, `/Locked` | — |
| 68 | Two contradicted pages are §10.7.4's own departure, measured | — |
| 69 | §12.4.4.2's sub-page navigation, on the control two sessions built | — |
| 70 | §12.4.4.1's page transitions, read from Table 164 and played by nobody | — |
| 71 | §11.4.6's knockout groups, drawn where a shape is a coverage | — |
| 72 | §9.3.8's text knockout and §11.6.2's one object in parts, on it | — |
| 73 | One shading object, built once: a fifth of the corpus's worst page | 0069 |
| 74 | §10.7.3's `/SM`, the silence that was hiding inside a `partial` row | — |
| 75 | Eight "unexplained" contradicted pages are one population, measured | — |
| 76 | Table 170's rollover and down appearances, on the pointer | — |
| 77 | §12.6.3's trigger events, and the one precedence rule Table 197 states | — |
| 78 | §14.7.2's structure tree, read downwards at last | — |
| 79 | Everything re-verified, including the fuzzers and `cargo deny` | — |
| 80 | §12.6.4.8's URI by RFC 3986, and §12.6.4.12's four page commands | 0070 |
| 81 | §12.2's viewer preferences, and the two that decide which boundary is displayed | 0071 |
| 82 | §14.7.6's attributes and §14.7.4's namespaces | 0072 |
| 83 | §14.8.2's artifacts and reversed show strings; an inline array nobody parsed | 0073 |
| 84 | §7.12's extensions and §12.11's requirements | 0074 |
| 85 | §12.5.6.7's leader lines, and the first corpus ratchet in six sessions | 0075 |
| 86 | §7.11.4's embedded files, listed; everything re-measured | 0076 |
| 87 | §14.13's associated files: one array in seven places | 0077 |
| 88 | §14.8.4's forty-one standard structure types | 0078 |
| 89 | §14.8.5's owners and the five-step priority for an attribute's value | 0079 |
| 90 | §12.4.3's articles and §12.6.4.7's thread action | 0080 |
| 91 | §12.3.4's thumbnails, and the page as its own producer drew it | 0081 |
| 92 | §12.9's viewports and §12.10's geospatial dictionaries | 0082 |
| 93 | §12.3.5's collections, §12.3.6's navigators, clause 7's last two silences | 0083 |
| 94 | §14.8.2.5's logical content order | 0084 |
| 95 | §14.8.5.6's `PrintField`: clause 14 reaches zero silences | 0085 |
| 96 | §9.8.3's substitution hints, the ledger's oldest silences | 0086 |
| 97 | §12.7.6.3's reset-form action, and nine refusals the viewer had swallowed | 0087 |
| 98 | §12.8's signatures: what a renderer can say about one without a trust store | 0088 |
| 99 | §12.8.4's store, §12.8.7's attestations, and everything re-verified | 0089 |
| 100 | §12.7.8's forms data format read and §12.7.6.4's import performed | 0090 |
| 101 | §12.7.7's named pages, and **the ledger reaches zero silences** | 0091 |
| 102 | §7.9.4's dates audited over 1542 corpus strings; §8.9.5.4's alternate images | 0092 |
| 103 | §11.4.4's NOTE 5: the non-isolated group that need not be built | 0093 |
| 104 | §12.6.4.4's embedded go-to: the document inside the document | 0094 |
| 105 | Table 192's `/R` drawn; §12.5.6.23's overlay read as writer-side | 0095 |
| 106 | A fifth fuzz target; everything re-verified | 0096 |
| 107 | Two recovery rules from the file's own declarations: 11 pageless → 5 | 0097 |
| 108 | §7.10 audited out of the file-only evidence population, and two claims corrected | 0098 |
| 109 | §12.6.4.15's transition action, which is §12.4.4's table at a different moment | 0099 |
| 110 | §7.5's free entries: an update that deletes an object is no longer undone | 0100 |
| 111 | Clause 8's file-only rows audited; a retired claim found in four places | 0101 |
| 112 | The file-only evidence population reaches zero, over four audits | 0102 |
| 113 | A to-do item measured at a hundredth of its listed price, and removed | 0103 |
| 114 | §7.11's file specifications, read whole and opened never | 0104 |
| 115 | Five `reported` rows that understated what the tree already does | 0105 |
| 116 | §12.5.6.7's `/LE` and `/Cap` named beside the line rather than instead of it | 0106 |
| 117 | §11.6.7's implicit group: a pattern's alpha belongs to the pattern | 0107 |
| 118 | §12.6.4.5's `GoToDp`, and a sweep for reasons that had expired | 0108 |
| 119 | Everything re-verified: four gates, five fuzzers, both performance numbers | — |
| 120 | §12.5.6.4's seven icons: the *shall* hiding behind a silence about artwork | 0109 |
| 121 | §11.7.4.4's other half: a glyph filled and stroked is one object too | 0110 |
| 122 | A sweep for entries claimed unread; an `/Encoding` name that erased a font | 0111 |
| 123 | A `/DA` font `/DR` lacks is stood in for — and the stand-in must not fall short | 0112 |
| 124 | Everything re-verified; the interpretation drift floor measured as an A/B | — |
| 125 | An appearance with no `/BBox` gets §12.7.4.3's default box rather than a refusal | 0113 |
| 126 | And the same rule the other way: no `/Rect`, so the appearance's own box | 0114 |
| 127 | The font cache was keyed by a resource *name*; a form's `/F1` is not a page's | 0115 |
| 128 | Every other cache's key checked; the decompression item priced and found small | — |
| 129 | Everything re-verified after ten sessions of change | — |
| 130 | MIT; `CLAUDE.md`'s writer exclusion amended; §0's UI boundary specified | — |
| 131 | `viewer-core` is real: a vocabulary, a scheduler, and a consumer with no display | 0116 |
| 132 | The window becomes a consumer; a locked file is asked for its password at last | 0117 |
| 133 | The text layer, and the click that had been mapped to the wrong half of the page | 0118 |
| 134 | Text is selected, and the shapes cross as geometry for the host to draw | 0119 |
| 135 | An edit is a log beside the document; a replaced value was not being drawn | 0120 |
| 136 | §7.5.6's incremental update: the one kind of writing this project does | 0121 |
| 137 | The ledger re-read against six sessions of new capability; `/H` became reachable | 0122 |
| 138 | Table 192's `/H`: the clause about a moment that had never happened | 0123 |
| 139 | Everything re-verified after eight sessions of change | — |
| 140 | Search, and a contradicted page that measured our own grid-fitting for us | — |
| 141 | A page turn walked the page tree once per outline item: 380 ms → 9 ms | 0124 |
| 142 | A frame that failed was reported as one that was drawn; the CPU draws it now | 0125, 0126 |
| 143 | A page the device drew nothing of, and said nothing about; banded, and the backend question asked properly | 0127, 0128 |
| 144 | §7.6.2 on the way out: an encrypted document can be saved | 0129 |
| 145 | §12.7.4.3's appearance stream is written into the file, not owed to the reader | 0130 |
