# RFC 0009 — What each exclusion protects, and what lifting it now would mean

Status: **proposed** — ready for the owner. Nothing in this document changes a ledger status, a
`Cargo.toml`, a line of `CLAUDE.md` or a line of code that ships; the round that wrote it added no
instrument and no dependency, and every count in it is a reading of a command named beside it.
Round: 1358 — commissioned by the owner on 2026-10-05: *"please use one of the next rounds for a
RFC for features we have excluded until now and what it would mean, to implement them now (I am
thinking about printing, multimedia,...)"*.
Builds on: RFC 0004 (print, `partly built`), RFC 0005 (text editing, `draft`), RFC 0008 (scripts,
`proposed`), the exclusion list in `CLAUDE.md` as amended four times, ADR 1548 (every
`out-of-scope` row names its exclusion) and ADR 1535 (every `inapplicable` row quotes its
condition).
Companions: RFC 0007 and ADRs 1155, 1291 and 1527 (the four levels over an act a document asks
of this machine — the shape every *hand it to another program* proposal below reuses); ADR 0014
and `pdf-sandbox` (what a confined worker is for); ADR 1014, ADR 1120 and ADR 1123 (the three
amendments that moved the authoring line, and the two-part test the last of them states).

`§N` in this document is a clause of ISO 32000-2 and nothing else; this document's own sections
are written `section N`, and another standard's clause is written with its document's name in front.
Quotation marks mean verbatim from `doc/md/ISO_32000-2_sponsored_EC3.md`; every other text —
ECMA-363, ISO 14739-1, Adobe's XFA specification, a crate's documentation — is cited and never
quoted.

---

## 0. What is excluded today, in one paragraph

`CLAUDE.md` closes its scope with four exclusions, each with a reason: clause 13 (*a media engine,
not a rendering question*), XFA (*a PDF processor may choose to not implement this feature*, §K.1's
own permission), JavaScript (*a separate project with its own security argument* — RFC 0008 has
since argued that sentence down to a placement question), and authoring content from nothing
(*we do not compose pages*, amended four times, each by argument). Beside those sit thirty-four
ledger rows the tree calls `inapplicable` — a claim about the standard that `CLAUDE.md` says
*decays* — and one capability the owner named first that is **not** excluded at all: printing,
which RFC 0004 proposed, which two of four windows do, and which that RFC's own status block says
is unfinished. This document takes each in turn and asks the same five things of it: what the
standard requires, what a reader gets, what the tree already does, what building it would mean
(architecture, the libraries judged by `doc/stack.md`'s rules and `deny.toml`, the sandbox
argument, the cost to time-to-first-page and in surface, the ledger rows that would move), and
what this round recommends. Section 8 ranks them; section 11 puts the decisions to the owner as
`doc/questions/Q254`.

## 1. The reframe — an exclusion protects something, or it predates something

Four exclusions and one list of inapplicables are not five instances of one thing. Read against
the tree as it stands on 2026-10-05, they sort into three kinds, and the recommendation for each
follows from its kind:

1. **An exclusion that protects a property the project cannot do without.** The authoring line
   protects the oracle: `interpret` is a pure function of the file, and a program that invents
   marks has nothing to compare its output to. That one stays wherever the argument puts it, and
   section 5 says where.
2. **An exclusion written when a capability did not exist, that the capability has since arrived
   for.** JavaScript's *separate project with its own security argument* was written before the
   tree had two confined workers; RFC 0008 is the consequence. Clause 13's *media engine* was
   written when the tree had no sandbox to put a decoder in and no level shape to put a hand-off
   under; both exist now, and section 3 asks how far they reach.
3. **An exclusion the standard itself grants.** XFA is excluded on a sentence the standard wrote
   for the purpose, and the only question is whether the reader is told (section 4).

And one thing on the list is none of these: **printing is in scope, demand-backed, partly built,
and owed** (section 2). The owner named it first and this document does too.

Three facts the rest depends on, each counted today:

- **The ledger, by `python3` over `doc/conformance/ledger.toml`** (never `tools/state.sh ledger`,
  which rewrites the file): 883 rows — `implemented` 699, `out-of-scope` 93, `departed` 39,
  `inapplicable` 34, `partial` 11, `reported` 5, `writer-side` 2. Of the 93: **88 rest on clause
  13** (80 inside the clause, and §12.5.6.17, §12.5.6.25, §12.6.4.9, §12.6.4.10, §12.6.4.14,
  §12.6.4.16, §12.6.4.18 and §14.9.2.4 outside it), **2 on the script exclusion** (§12.6.4.17,
  §12.11.5), **3 on XFA** (§K, §K.1, §K.2). Of the 34 `inapplicable`: §10.6.1–§10.6.4 (halftones),
  §14.2 (procedure sets), §14.8.3.2 and four §14.8.5.4 rows (layout attributes), eighteen rows of
  §14.10 (web capture), §14.11.2.2, §14.11.4, §14.11.6.1, §14.11.6.3 and §14.11.7 (prepress), and
  §14.12.4.2 (document part metadata). Section 7 reads each.
- **The corpora, by a byte search over the two local corpora** (`grep -alE` over
  `doc/pdf.js/test/pdfs/*.pdf`, 974 files, and `doc/corpora`, 503 files; a search of the bytes
  misses what sits inside an object stream, so each number is a floor): `/XFA` 2 and
  `/NeedsRendering true` 3 in pdf.js, 0 and 1 in `doc/corpora`; `/Subtype /RichMedia` 1 and 0;
  `/Subtype /Screen` 1 and 2; `/Subtype /Sound` 1 and 1; `/S /Rendition` 1 and 2; `/Subtype /3D`,
  `/Subtype /Movie`, `U3D`, `PRC` and `/AlternatePresentations` 0 and 0. pdf.js's own manifest
  names 57 `xfa_*` documents, 56 of them `.link` files this checkout does not hold.
- **The tree's refusals are already loud.** Every excluded action is refused by name in
  `crates/pdf-model/src/action.rs` and printed by the windows; every excluded annotation without
  an `/AP` is refused with its clause's own sentence in `appearance::construct`; `3D`, `RichMedia`,
  `Movie`, `Screen` and `Sound` are in `annotation.rs`'s `STANDARD_SUBTYPES`, so whatever
  appearance stream a producer supplied is drawn. What this document proposes is never the first
  loud refusal — it is what replaces one.

## 2. Printing — not excluded, named first, and unfinished

### 2.1 What the standard requires of a page going to paper

The standard describes marks, not devices, and it says so twice over for the print case. §10.1:
"[f]or the purpose of clause 10, it is irrelevant whether a raster output device physically exists
and is actually used for rendering, or is just assumed". And the one place it names the medium's
properties, it names where they come from: "[a]n interactive dialogue conducted when the user
requests viewing or printing". What a print *does* differently is stated in four places and the
tree reads all four: Table 167's bit 3 (print, not view), §8.11.4.5's `Print` usage event over
optional content, §12.5.6.22's fixed print watermark against the sheet, and Table 147's eight print
entries — five of which are, in their own words, what a dialogue opens with ("[t]he page scaling
option that shall be selected when a print dialogue is displayed for this document"), and two of
which decide pixels: `/PrintArea`, "the area of a page that shall be rendered when printing the
document", and `/PrintClip`, "[t]he name of the page boundary to which the contents of a page shall
be clipped when printing the document". Table 22's bits 3 and 12 are the document's two assertions
over a print.

### 2.2 What is built, read off the tree on 2026-10-05

RFC 0004's status block, `doc/state-of-play.md`'s *A page goes to paper* paragraph, ADRs 1179,
1180, 1203, 1204 and 1227, `crates/viewer-host/src/printing.rs` (706 lines) and
`grep -rln Print crates/viewer-*/src crates/pdf-model/src` (43 files) agree:

- **Print intent exists and is the same interpretation under a flag**: Table 167's bit 3 decides
  the annotations, §8.11.4.5's `Print` applications run over the reader's own layer switches and
  revert, §12.5.6.22's watermark is placed against the sheet, `Page::print_box` and
  `Page::print_clip_box` stand beside the screen's pair and `Page::render_for_printing` selects
  (ADR 1227). The window shows the same thing while the operation stands, so preview *is* the
  print at screen resolution.
- **Table 22's two bits are two operations at the four levels**: `Operation::Print` and
  `Operation::PrintFaithfully` in `pdf-model`'s `restriction.rs`, carried over the confined wire
  as their own codes (ADR 1203).
- **Table 147's print half is read whole**: `viewer_host::printing::Defaults` is the five
  dialogue entries, `/PrintScaling` is answered by `Scaling`, `/PrintArea` and `/PrintClip` by the
  second pair of boxes, Table 148's `/Enforce` is reported and not obeyed because a document
  telling a reader what they may not change is a restriction and `CLAUDE.md` makes every one of
  those the reader's (ADR 1180).
- **Two windows print.** `quorra-gtk` through `GtkPrintOperation`, `quorra-qt` through a
  hand-written `QPrintDialog`/`QPrinter` bridge with the scale mode and pages-per-sheet on a tab
  of its own (ADR 1204); both paint the processor backend's raster — the oracle's — at the
  printer's resolution clamped to 150–600 dpi with 300 where none is reported.
- **Two windows show.** `quorra` (winit) has no toolkit and prints by showing: its `printing`
  flag enters print intent and the same key leaves it, with a doc comment saying what it would
  take not to. `quorra-confined` answers the grant and sends no `Command::Print` (ADR 1180).
- **The C ABI prints without a dialogue**: `quorra_print`, `quorra_print_page`,
  `quorra_print_placed` — a caller states the sheet and the resolution, and Table 147's second
  sentence for `/PrintScaling` ("If the print dialogue is suppressed and its parameters are
  provided from some other source, this entry nevertheless shall be honoured") has a path.

### 2.3 What RFC 0004 left open, and what has changed since

RFC 0004's own status names the residue: the docked print panel with live paper composition; the
winit panel and IPP submission; the spool container for the confined window and with it section 9's
scope sentence (its open question 5); page tiling, which §12.5.6.22's first post-EXAMPLE bullet is
about and which the model expresses and no host composes; and its questions 2 (the DPI clamp) and 6
(IPP-direct or the portal). Two things have changed underneath that list:

- **RFC 0002's serializer exists.** RFC 0004 deferred Route A — handing the PDF itself to the
  print system — *once RFC 0002 exists to build it on*, because a passthrough that does not bake
  the view state (typed values, layer switches, constructed appearances) prints a different
  document from the one on the screen. `quorra-transform` now has `update`, `pages`, `render` and
  an `executor`; baking is a derivative the suite can produce. Route A is no longer blocked, and
  section 2.9 re-prices it.
- **The `ipp` crate has a blocking client.** RFC 0004 named `ipp` as pure Rust and `CLAUDE.md`
  bars an async runtime. On 2026-10-05 `ipp` 7.0.0 (`MIT/Apache-2.0`) has a `client` feature
  over `ureq` — the HTTP client already in `viewer-host` for §12.7.6.2 (ADR 1291) — and a
  `client-rustls` feature over the same `rustls-native-certs` the tree carries. Its default
  feature is the async `reqwest` client and would be turned off. So the winit submission path is
  one dependency on a stack the lock already holds, with no runtime.

### 2.4 §10.6 — does the condition flip when this program prints?

The row is `inapplicable` on §10.6.1's sentence, and `CLAUDE.md` says such a claim decays, so it is
re-read here with the device changed. The condition: "Some output devices can reproduce
continuous-tone colours directly. Halftoning is not required for such devices; after gamma
correction by the transfer functions, the colour components shall be transmitted directly to the
device." §10.1 makes the step conditional in the same words — "[i]f the raster output device
supports PDF-defined halftoning, apply halftoning according to 10.6" — against an unconditional
transfer-function step one line above it. And §10.6.2 names the devices the clause describes: "The
foregoing description also applies to colour output devices whose pixels consist of primary colours
that are either completely on or completely off. Most colour printers, but not colour displays,
work this way."

So the honest question is **which device this program transmits to**, and under every route that
exists or is proposed here the answer is a continuous-tone one: Route B hands eight-bit RGB raster
to a toolkit context or a spool file, and the print system's own driver screens it downstream;
Route A hands the PDF itself over, and the halftoning is the RIP's. **The condition holds, the row
stays `inapplicable`, and this is the first time the reading is written against a print path rather
than a screen.** It flips on exactly one route, which nobody proposes and which the row should name
so that the decay is visible: this program *itself* producing the device's bilevel or
low-bit-depth pixels — a one-bit spool for a driver that takes one — would be the device "that
do[es] require halftoning", and §10.6.3's spot functions and §10.6.4's threshold arrays would then
be requirements. Priced for the record: a threshold-array screen is a few hundred lines over the
PDF function evaluator the tree has; Type 5's per-colourant screens are a dictionary walk
`ext_gstate.rs` already does for the transfer-function entry. It is not expensive; it is not owed.

### 2.5 §10.5 on the print path

Implemented and route-independent: `content::Transfer` is applied where a colour becomes the value
a device receives, before the raster is handed anywhere, and the ledger's §10.6.1 row says so for
both (*the screen's and a print's*). Nothing is owed here except the sentence in the §10.5 row that
RFC 0004 section 9 asked for, which the §10.6.1 row now carries instead. The one sentence of §10.5
that is inapplicable — the gray-to-CMYK rule, conditional on a CMYK device — would become
*applicable* under a CMYK spool (section 2.6), which is a second reason to price that spool
honestly.

### 2.6 §14.11.5's output intents on the way to paper

The tree reads output intents and uses the profile's "to CIE" direction, because Table 401 says
the "from CIE" direction is for the output transformation and the other "may optionally be used to
remap source colour values to some other destination colour space, such as for screen preview or
hardcopy proofing", and a screen is the optional case (ledger §14.11.5, ADR 0272). **On paper the
same code runs and produces the same RGB**, and that is the limit to state plainly: what prints is
the screen's colour laid on paper by the print system's own management, not a proof through the
document's press profile. Doing the latter means a CMYK spool through the intent's BToA tables —
an evaluator `pdf-colour` does not have (its LUT evaluator is the AToB direction; BToA is the same
table shape read the other way, so it is work rather than research) — and a spool format that
carries CMYK, which a toolkit's RGB context does not. **Not recommended until a spool path exists
at all**; what is recommended is the report the §14.11.5 row already names as owed in a neighbouring
case: a print of a document that states a `/DestOutputProfile` says, once, that the intent was used
for meaning and not for output.

### 2.7 The two windows that do not print

- **`quorra` (winit, the GPU window and the one page one is drawn in).** RFC 0004 section 5's
  recommendation stands and its dependency has lost its objection (section 2.3): a print panel in
  `viewer-ui`'s own chrome, and a job to CUPS over IPP from `viewer-host` using `ipp`'s blocking
  client on `ureq`. Printer discovery is the CUPS extension operation the crate exposes; media,
  copies, duplex and tray become IPP attributes; the job body is the spool container. On a machine
  with no CUPS the panel says so by name, which is what the window says today without the panel.
  The sandbox argument is RFC 0004's: the host spools, nothing confined touches a socket.
- **`quorra-confined`.** The worker already renders for paper (print intent and `Purpose::Print`
  cross the wire as a field on `Command::Print`); what is missing is the host-side spool writer and
  a submission. RFC 0004's shape holds and the portal's shape is the same: the worker ships raster,
  the window process writes the spool into its own runtime directory and hands a descriptor to
  `org.freedesktop.portal.Print` or submits over IPP. The seccomp and Landlock profile of the
  worker is untouched.

### 2.8 The spool container, and RFC 0004's scope sentence

A spool is a PDF whose every page is one image XObject of this program's raster. RFC 0004 asked the
owner whether writing one crosses the authoring line (its question 5). Read against the line as it
stands after ADR 1120: the test is *provenance of the marks*, and the marks are the producer's page
as this program drew it — a photograph of the page, no mark invented, placed at a position the
print operation (a person) stated. The serializer emits structure and the content is one `Do` per
page. This document's reading is that it is on the near side, as a transport envelope this program
alone consumes; it is still the owner's sentence (section 11, question 1).

### 2.9 Route A, re-priced: hand the file to CUPS

CUPS takes PDF natively and `pdftopdf` does ranges and n-up; the cost RFC 0004 named was that the
printed page becomes Ghostscript's reading of the file and that nothing of the view state travels.
With the serializer, the view state *can* travel: an incremental update (§7.5.6, the one form of
writing this tree already does for a person's edits) carrying the typed values, the layer states
as a new `/D` configuration, and the constructed appearances, written to a temporary derivative and
handed over. What does **not** travel, and cannot: this program's own interpretation. Route A
prints the other RIP's rendering of a file this program prepared, and principle 5 says agreement
with it is evidence and disagreement a question — but a *printed page* is the one output a user
cannot take back to the clause. **Recommendation: Route A as a named option, never the default**,
labelled in the panel as *send the file to the printer (its own renderer draws it)*, built after
the IPP path because it rides the same submission and adds only the derivative.

### 2.10 Preview, tiling, and the rest

- **The docked panel with live paper composition** is chrome in each window's idiom; the
  composition (`Sheet`, `Scaling`, `PagesPerSheet`, `placed`, `cell`) exists in `viewer-host`.
  Three windows, three panels, one model.
- **Tiling** — §12.5.6.22's first post-EXAMPLE bullet — is the remaining placement the model
  expresses and no host composes; it is the `PagesPerSheet` arithmetic inverted and is small.
- **§14.11.2.2's guidelines** ("[i]nteractive PDF processors **may** offer the ability to display
  guidelines") are a permission the tree declines on its own row, and the row says where they
  belong: *the day a viewer grows a prepress view*. A print panel that shows the sheet is that
  view, and Table 397's dash and colour are things the tree already draws.
- **RFC 0008's `/WP` and `/DP`** become sites once printing is an operation every window performs;
  that is RFC 0008's to build, under its own answer.

### 2.11 The ledger, and the cost

Rows: §10.6.1 unchanged, with the route that flips it named; §10.5 unchanged; §12.2 stays
`departed` for `/HideMenubar` and `/CenterWindow`, and its print half gains the two windows;
§12.5.6.22's tiling sentence closes; §14.11.2.2 leaves `inapplicable` for `implemented` when a panel
shows the sheet; §14.11.5 gains the report. **Time-to-first-page: zero** — printing is an operation
after open, and `ipp` is constructed on the first job. **Surface**: one crate (`ipp`, without its
default features) on a stack already in the lock, a spool writer in `viewer-host`, three panels.
**No rule is amended**; question 1 asks the owner to close RFC 0004's open sentence.

### 2.12 Recommendation

Finish RFC 0004 in this order: the winit panel and IPP submission (the window page one is drawn in
is the one that cannot print); the confined window's spool writer over the same submission; the
docked preview panel in all three; tiling; Route A as a named option; the output-intent report.
§10.6 stays inapplicable and the row names its one flipping route.

## 3. Clause 13 — multimedia and 3D

### 3.1 What the standard requires

The clause's own summary, §13.1: "This clause describes those features of PDF that support
embedding and playing multimedia content". §13.2.1's second capability is the one that decides the
architecture: "Embedded media, as well as referenced media outside a PDF file, may be played with a
variety of player software. (In some situations, the player software may be the interactive PDF
processor itself.)" — so the standard itself puts the player *outside* the processor as the
ordinary case, and §13.2.4.2 says how the processor finds one: "the media clip object shall provide
enough information to allow an interactive PDF processor to locate an appropriate player", by a
`/CT` content type ("[i]f this entry is present, any player that is selected shall support this
content type") or a `/PL` players dictionary. §13.2.2's viability algorithm — `MH` ("must honour")
and `BE` ("best effort") — is addressed to that selection: "If an object is considered non-viable,
the media should not be played."

The `shall`s on a processor that *does* play: Table 307's controls for a movie ("A flag specifying
whether to display a movie controller bar while playing the movie"), its `Synchronous` ("the movie
player shall retain control until the movie is completed or dismissed by the user"), §13.3's
formats ("interactive PDF processors should support at least the following formats" — raw, signed
and µ-law PCM at three sample rates, one or two channels, 8 or 16 bits; "Sound players shall convert
between formats, downsample rates, and combine channels as necessary"), §13.6.2's two states for 3D
("Inactive (the default initial state): the annotation displays the annotation's normal appearance"
and "Active: the annotation displays a rendering of the 3D artwork"), Table 310's activation
(`PO` "as soon as the page containing the annotation is opened", `PV` "as soon as any part of the
page containing the annotation becomes visible", `XA` "until explicitly activated"), and §13.7.2.1's
same two states for rich media.

Three sentences decide what a *still* page owes, and none of them needs a player:

- Table 306's `/Poster`: "If this value is a stream, it shall contain an image XObject (see 8.9,
  "Images") to be displayed as the poster. If it is the boolean value true , the poster image shall
  be retrieved from the movie file; if it is false , no poster shall be displayed."
- §13.6.2 and §13.7.2.1, in the same words: the annotation "shall provide an appearance stream in
  its AP entry" with a normal appearance, which "may be used by applications that do not support
  3D annotations and by all applications for the initial display of the annotation" — and NOTE 3's
  reading of what that stream usually is: "a pre-rendered bitmap of the default view of the 3D
  artwork".
- Table 310's `/A` for a processor that does not interact: "In non-interactive applications, such
  as printing systems or aggregating interactive PDF processors, PO and PV indicate that the
  annotation shall be activated when the page is printed or placed; XA indicates that the
  annotation shall never be activated and the normal appearance shall be used."

And what the clause itself retired: §13.3, §13.4, §13.5 and §12.5.6.17 each open with a deprecation
sentence and hand over to §13.2 or §13.7; §13.1 says "13.3, "Sounds" and 13.4, "Movies" describe
deprecated features superseded by 13.7". Rich media's configurations, Table 342: "Valid values are
3D , Sound , and Video ." — **there is no Flash subtype in ISO 32000-2**; the Flash-era rich media of
Adobe's extensions is not in the standard this project reads, and the word does not occur in
clause 13 (`grep -ci flash` over the clause: 0).

### 3.2 What a reader gets, by tier

| tier | a reader gets | what it needs |
|---|---|---|
| **A — the still page** | the movie's poster image; every producer-supplied appearance (drawn already); a controller bar's *appearance* where `ShowControls` asks for one; a report per media annotation saying what it is, what format (`/CT`) and whether this program could find a player; **the media file saved or handed to this machine's player under a level** | no decoder, no engine, one level |
| **B — sound this program plays** | §13.3's own PCM and µ-law sound objects played through the speakers; a sound annotation's click doing what the clause says | a decoder in the confined worker (trivial for §13.3's formats), PCM to the host, an audio output library in the host |
| **C — audio and video in a region of the page** | §13.2's renditions played in the screen annotation's rectangle or a floating window, with volume, duration, repeat; compressed audio (MP3, AAC, Vorbis) and video (H.264, AV1) | container demuxers and codec decoders in a confined worker; frames and samples across the wire; a presentation surface in each window |
| **D — 3D artwork, interactive** | the model rendered, panned, zoomed and rotated in the annotation rectangle; the views of `VA`; render modes, lighting, cross sections | a U3D and a PRC parser; a mesh renderer; the view and camera model of §13.6.4 |

### 3.3 What the tree already does

Read in `crates/pdf-model/src/appearance.rs`, `annotation.rs`, `action.rs` and the ledger on
2026-10-05:

- Every one of `3D`, `RichMedia`, `Movie`, `Screen` and `Sound` is a standard subtype whose
  `/AP` is placed under §12.5.5 like any other; `Sound`'s `Speaker` and `Mic` icons are drawn as
  objects the clause names (§12.5.6.16, `implemented`); `Screen` without an `/AP` draws nothing and
  says so with the clause's sentence (§12.5.6.18, `implemented`); `3D` and `RichMedia` without one
  are refused with §13.6.2's and §13.7.2.1's requirement; `Movie` without one is refused naming the
  poster (ADR 1548).
- Every clause-13 *action* — `Sound`, `Movie`, `Rendition`, `GoTo3DView`, `RichMediaExecute` — is
  refused by name in `action.rs` and printed by the windows, tested end to end (ADR 1367).
- **§13.4 is `reported`** since round 1356: the owner approved `doc/questions/Q33`'s
  recommendation (`A33`, 2026-09-06, *Recommendation approved*) that the stream form of `/Poster`
  comes off the exclusion, the boolean form staying with the player. The build is one arm reading
  `/Movie` → `/Poster` and drawing the stream as an image in `/Rect` — the construction
  §12.5.6.19's `/MK /I` icon already uses. It is in `doc/todo/65`'s bucket 6 and is owed under
  the current rules, whatever this RFC decides.
- The media *data* is reachable through machinery that exists: Table 285's `/D` is a file
  specification, usually §7.11.4's embedded file stream, and this tree extracts, saves and — for a
  PDF — opens embedded files under the `EmbeddedDocuments` level (`viewer_host::policy`).
- Multi-language text arrays (§14.9.2.4) are read nowhere because every entry that holds one is a
  clause-13 object; Tier A's report is the first consumer, and the row would move with it.

### 3.4 The libraries, judged by `doc/stack.md` and `deny.toml`

Read on crates.io on 2026-10-05 (`curl` against the registry API; versions are the newest stable):

| crate | version | licence against `deny.toml` | what it is | verdict |
|---|---|---|---|---|
| `symphonia` | 0.6.1 | **MPL-2.0 — fails** (the file says why MPL is not on the list) | pure-Rust demuxers and audio decoders (MP3, AAC, FLAC, Vorbis, WAV) | out on licence before anything else; the one pure-Rust audio stack, which is the cost of that line |
| `hound` | 3.5.1 | Apache-2.0 | WAV reader, no dependencies | admissible; §13.3's RIFF external files |
| `lewton` | 0.10.2 | MIT OR Apache-2.0 | pure-Rust Vorbis | admissible, last release 2021 |
| `claxon` | 0.4.3 | Apache-2.0 | pure-Rust FLAC | admissible, last release 2020 |
| `cpal` | 0.18.2 | Apache-2.0 | audio *output*; on Linux through `alsa` → `alsa-sys` (MIT), **C linkage to `libasound`** | the host's, like GTK and Qt; receives PCM this program decoded, parses nothing untrusted |
| `rodio` / `kira` | 0.22.2 / 0.12.5 | MIT OR Apache-2.0 | playback layers over `cpal`; `rodio` decodes through `symphonia` optionally | output only, decoders off |
| `gstreamer` (`gstreamer-rs`) | 0.25.4 | bindings MIT OR Apache-2.0; **GStreamer itself LGPL C**, loaded with its plugins from the filesystem | the system's media framework | a C decoder stack in the unconfined host, fed untrusted bytes — the thing principle 3 forbids outside the sandbox; and a Landlocked worker with no filesystem cannot load its plugins |
| `ffmpeg-next` | 9.0.0 | WTFPL (the binding); FFmpeg LGPL or GPL C | the other system framework | same argument, larger surface |
| `rav1d` | 1.1.0 | BSD-2-Clause | Rust port of `dav1d`, AV1 only, `unsafe` throughout | admissible *in the worker* under ADR 0014's precedent; AV1 is not what embedded PDF video is |
| `openh264` | 0.9.8 | BSD-2-Clause (binding); Cisco's C library downloaded at build | H.264 | C in the worker, justified in writing as principle 3 asks — and the one codec embedded video actually uses |
| `mp4`, `matroska` | 0.14.0, 0.30.1 | MIT; MIT/Apache-2.0 | pure-Rust demuxers | admissible; a demuxer is a parser of untrusted input and goes in the worker |
| `u3d`, `prc`, `xfa` | — | — | **no crate of these names exists** on crates.io; `prc-rs` is a game's parameter format | a U3D or PRC reader is written in-tree or not at all |

The pattern is the one RFC 0008 found for engines: the pure-Rust stack that exists fails the
licence list (`symphonia`, like Boa's rivals `nova_vm` and `mozjs`), and the admissible C goes in
the confined worker under ADR 0014's sentence. ECMA-363 (U3D) is freely published (its page answers
200 today); ISO 14739-1 (PRC) is a paid standard and nothing of it is in `doc/md/`.

### 3.5 The architecture, by tier

**Tier A is a reading, a report, one drawing and one level.**

- `pdf_model::media` reads §13.2's rendition tree, §13.4's movie dictionary and §13.3's sound
  object as data — `/CT`, `/D`, `/PL`, `MH`/`BE`, `/Poster`, `ShowControls`, `/R`/`/C`/`/B`/`/E` —
  and runs §13.2.2's viability algorithm as the clause writes it, with the players list this
  program knows (none) as the input. The outcome is a sentence per annotation: *a video, `video/mp4`,
  12 MB embedded; no player here*. That is §13.2.2 honoured in the one way a processor that plays
  nothing can honour it, and it is trap 5's loudness with the clause's own words.
- The poster, as ADR 1548 describes it; and a controller bar's *appearance* is **not** drawn — the
  clause says "while playing the movie", and nothing plays. Named so that nobody draws it.
- **The hand-off.** A media clip's `/D` is an embedded file or a URL. Saving the file is
  attachment extraction, which exists. *Opening* it — `xdg-open` through the portal's `OpenURI`,
  or a `/CT`-matched player — is a program this machine starts on a document's word, which is
  exactly ADR 1155's sentence for a link ("a link is a program this machine starts, so the reader
  says whether it may"). The level is the `Links` shape, `refuse|ask|warn|open`, `ask` by default;
  whether it *is* `Links` or a `Media` level of its own is question 3. A URL `/D` goes through
  `Links` as a URI, and only `http`, `https` and `mailto` ever leave, as today.
- **Nothing on the launch path.** A still page's media is read when its annotation is drawn, and
  the report is composed then.

**Tier B is one decoder in the worker and one output library in the host.** §13.3's own formats
need no library: raw, signed and µ-law samples at `/R` samples per second are a loop. A sound object
with an `/F` external file is one of the self-describing formats §13.3's NOTE lists (AIFF, RIFF,
snd), each a header and PCM, each `hound`-sized. The decode runs in `pdf-view-worker` (the samples
are small and bounded by `RLIMIT_AS` like an image's) and PCM crosses the wire as one more arm; the
host plays it through `cpal`, which parses nothing. The sandbox argument is ADR 0014's exactly,
and the one new thing in the host is `libasound` linked, which the GTK and Qt windows already
carry through their toolkits. **Cost**: `cpal`'s 41 normal dependencies, most of them other
platforms' and compiled out; a wire arm; a play/stop control in each window's chrome. **Demand**:
one sound annotation in each local corpus, both with an `/AP`; no `/S /Sound` action anywhere the
census has looked (§12.6.4.9's row).

**Tier C is a second confined worker and a presentation surface.** A `pdf-media-worker` on
`confined-transport`'s wire (built to be shared — RFC 0008 section 6.2 proposes the same for a
script), holding a demuxer and the decoders, its `RLIMIT_AS` sized for a frame buffer, spawned on
the first `Rendition` action the level lets run, never at open; frames cross as raster into a
region each window composites over the page the way the GPU window already composites a panel.
What it contains is the whole problem: embedded PDF video is overwhelmingly H.264 in MP4 with AAC,
and no pure-Rust decoder of either exists that passes `deny.toml`. So Tier C is **C in the
worker** (`openh264`, and an AAC decoder with the same shape), justified in writing under ADR 0014,
with the protocol fuzzed from the first commit. It is a media player's worth of work — synchronisation,
a clock, seeking, Table 307's floating window, §13.2.6's screen parameters — and the standard's own
first position is that the player is somebody else's software.

**Tier D is a parser nobody has written in Rust and a renderer this tree has.** The renderer is the
cheap half: a triangle mesh with normals and a camera under `wgpu` is what `render-raster` already
runs under, and §13.6.4's projection, background, render-mode and lighting dictionaries map onto a
few hundred lines of shader. The parser is the expensive half: U3D (ECMA-363) is a bit-stream with
its own context-adaptive compression, continuous-level-of-detail meshes and a 200-page
specification; PRC (ISO 14739-1) is a CAD exchange format whose specification this project does not
hold. Neither pdf.js nor poppler renders either; Acrobat is the only reader that does. `OnInstantiate`
and `3DI false` tie 3D to RFC 0008's engine, and Table 310's `PO` and `PV` put instantiation on the
*page-open* path, which RFC 0008 section 6.6's ordering — after the first present, under a budget —
would have to govern here too.

### 3.6 The sandbox argument, stated once

A decoder for untrusted media is clause 13's attack surface and goes where the JBIG2 and JPEG 2000
decoders went: a worker with no filesystem, no network and an address-space limit, receiving one
object's bytes and returning samples or pixels. The host receives only what it can present —
PCM, raster — and the one thing that may not happen is a system media framework in the host
taking the document's bytes, which rules out GStreamer and FFmpeg *as decoders* (section 3.4) and
leaves them one role: the player this machine's user already has, started under a level with the
file handed over, which is what §13.2.1's "variety of player software" describes.

### 3.7 The ledger, honestly

88 rows rest on this exclusion, and **anyone arguing this on coverage is overselling it**: most of
clause 13's rows describe the engine's behaviour — play parameters, screen parameters, player
selection, 3D views — and would stay excluded under Tier A and move to `partial` at best under
Tier C. What Tier A moves: §13.4 leaves `reported` with the poster drawn; §13.2.2 and §13.2.4.2
become `partial` (viability computed and reported, nothing played); §14.9.2.4 moves (the array is
read for the report); §12.5.6.17 gains the poster in its drawn half. Tier B moves §13.3. Tier C and
D move the rest of §13.2, §13.6 and §13.7 to `partial`, and some of §13.6's view dictionaries
further.

### 3.8 Recommendation

**Tier A, now, and amend the exclusion's sentence so that it says what it excludes** — a *player*,
not a page: question 2 has the words. Tier B when a document asks for it (none does today) — it is
small, admissible and deferred on demand rather than on cost. **Tier C and D stay excluded**, each
with its trigger named in the exclusion so that it can be re-argued when the trigger fires: for C,
an H.264 and AAC decoder admissible under `deny.toml` or the owner's written acceptance of C in the
media worker under ADR 0014's precedent; for D, a U3D reader in Rust, or a decision to write one.

## 4. XFA — Annex K

### 4.1 What the standard requires, and what it permits

§K.1 opens "This annex is deprecated in PDF 2.0" and grants the exclusion in a sentence written for
it: "The implementation of such a schema driven page generation involves considerable effort beyond
that for a simple PDF viewer and therefore a PDF processor may choose to not implement this
feature." It states one conditional obligation — "If an interactive processor supports XFA forms,
that processor shall clearly indicate to the user when they are interacting with an XFA form" — and
one sentence about what a page is: "Whether or not the pages generated by the processing of an XFA
schema are materialized as PDF pages is implementation dependent." §K.2 puts the writer under the
rules that make declining safe: "The other entries in the interactive form dictionary shall be
consistent with the information in the XFA resource", "PDF interactive form field objects shall be
present for each field specified in the XFA resource", and "[t]he XFA field values shall be
consistent with the corresponding V entries of the PDF field objects". Table 29's `/NeedsRendering`
is "[a] flag used to expedite the display of PDF documents containing XFA forms. It specifies
whether the document shall be regenerated when the document is first opened."

### 4.2 What a reader gets today, measured

- **A hybrid (static) XFA file** is a form: §K.2's consistency rules mean the AcroForm this tree
  reads *is* the form, and every field draws, fills and saves as any AcroForm does. What is lost is
  what the template packet states and the AcroForm does not: XFA's own calculations, validations
  and formatting (in FormCalc or JavaScript — RFC 0008's exclusion and a second language's), and
  any layout the template would regenerate on data that changes the number of fields.
- **A dynamic XFA file** shows the page the producer wrote for processors that decline, which is
  a notice. Measured today with `pdftotext -l 1` over the three pdf.js documents whose bytes state
  `/NeedsRendering true`: `issue14130.pdf` and `xfa_issue14315.pdf` say the document requires
  Adobe Reader 8 or higher; `xfa_filled_imm1344e.pdf` says *Please wait…* and that if the message
  is not replaced the viewer may not be able to display this type of document. Drawing that page
  is drawing what the file says, as the §K.1 row notes; **what the tree does not do is say why**.
  Nothing under `crates/pdf-model/src` reads `/XFA` (`pdf-archive` reads it to withdraw it for
  PDF/A, and `pdf-transform` to carry it), and no window tells the reader that the notice is a
  notice.
- **Population**: 2 of 974 pdf.js files state `/XFA` in their uncompressed bytes and 3 state
  `/NeedsRendering true`; pdf.js's manifest names 57 `xfa_*` documents, 56 of them links this
  checkout does not hold; `doc/corpora` has 1 `/NeedsRendering true` and no `/XFA` in the bytes.
  The byte search is a floor, and government forms (the IMM series among the pdf.js names) are the
  population that would feel it.

### 4.3 What implementing it would mean

The XFA 3.3 specification is Adobe's, hosted by the PDF Association; its download answered 403 to
this round today at `pdfa.org/norm-refs/XFA-3_3.pdf` and through the Internet Archive, so its size
is not re-measured here and is not stated. What is measured is pdf.js's implementation of it,
counted in this checkout on 2026-10-05: **16 379 lines in 27 files under `src/core/xfa/`**, plus
415 in the display layer — against 6 287 for its whole scripting API (RFC 0008 section 12) — and
the shape of those files is the point: a template parser, a data binder, a **layout engine**
(`layout.js`, flowing subforms across pages), an HTML emitter, and a FormCalc parser and a script
host. XFA is *schema-driven page generation*: given a template and data, decide how many pages
there are and where every field goes. **That is the fourth exclusion's own subject.** An XFA
processor is a layout engine with a form vocabulary, and building one puts this program on the far
side of the authoring line not at its edge but in its middle, with its own typesetting (XFA text
is laid out from font metrics, not from positioned glyphs), its own scripting (FormCalc has no
other home), and its own event model that RFC 0008's design would have to be extended to serve.
Then the deprecation: PDF 2.0 retired the annex, and every producer that still writes dynamic XFA
writes it for Adobe Reader by name.

Costs, if it were built: the launch path — `/NeedsRendering` is "regenerated when the document is
first opened", the one feature in this document that puts a layout engine between open and the
first frame; the oracle — a generated page has no producer's bytes to compare against; the surface
— an XML stack (the tree has one for XMP and XFDF) plus everything above.

### 4.4 Recommendation

**Keep the exclusion, on the two grounds it now has** — §K.1's permission and the authoring
exclusion — and **build the sentence**: read `/XFA` and `/NeedsRendering` in `pdf-model`, and
have every window say, once per document, *this is an XFA form; this program shows the pages the
producer wrote for processors that do not implement XFA, and the form it fills is the AcroForm*.
The row stays `out-of-scope` and its note names the report; no amendment is needed. A hybrid file's
lost half is RFC 0008's to serve where its scripts are JavaScript, and nobody's where they are
FormCalc.

## 5. Authoring content from nothing

### 5.1 The line, as it stands after four amendments

`CLAUDE.md`'s fourth exclusion has one test and one example: *does the operation invent marks?*,
and the watermark stamp *is the first feature on the far side of the redrawn line*. ADR 1120
sharpened the test to **provenance** — the sentence that draws the line is about where the marks
came from, not about who writes — and ADR 1123 built the one relocation a clause requires and
positions (§12.5.5's matrix for an appearance a target will not admit). ADR 1014 admitted a page
composed solely of the document's own content. So the admissible set today is: structure (RFC
0002's serializer), relocation under a clause that fixes the position, pages that carry the
document's own content, and §12.7.4.3's variable-text appearances, which a clause requires.

### 5.2 What is excluded, step by step, and what each step would cost

| step | what it is | whose marks | what fixes the position | what crossing it costs |
|---|---|---|---|---|
| **1. Overlay / underlay from a PDF** (qpdf `--overlay`, pdftk `stamp`/`background`) | a page from another file drawn over or under each page: `q cm /Fx Do Q` and a form XObject | a producer's — the other file's page | **a person**, not a clause | the oracle sees a composed page with no producer reference; the exclusion's two-part test fails on its second part, so an amendment is needed |
| **2. Text watermark, header, footer, page number, Bates stamp** (Stirling, pdftk) | glyphs this program sets from a string and a font | **this program's** | a person | the first genuinely invented mark; it needs typesetting of this program's own, which `pdf_font::shaping` has only for text a clause requires; the oracle cannot compare it to anything |
| **3. Layout engine, HTML to PDF, form creation** | pages composed from a source that is not a PDF | this program's, wholesale | a layout algorithm | a different product with a different correctness claim; §14.10's web capture is literally this and is `inapplicable` for the reason |

Two things sit beside the line and are not re-proposed here: **RFC 0005**'s text editing without
reflow, which changes which glyphs a producer's run shows and moves nothing else — it is an edit
to the producer's marks, not an invention, and has its own draft; and **form flattening**, which
the fourth amendment names as a later site of the same shape as ADR 1123 (a producer's appearance
relocated under §12.5.5's matrix into the page's content, a clause fixing the position) and which
needs no amendment, only a build.

### 5.3 What the line protects

- **The oracle's comparison.** Every page this program draws today is a page some producer
  specified, and `raster_golden` and the cross-renderer oracle rest on that: there is a reference.
  A composed page has no reference, and its correctness becomes "what this program drew last
  time". RFC 0008 accepted that cost for scripts (its section 8, item 7) because the standard
  requires scripts to run; nothing requires a watermark.
- **The scope of the correctness claim.** *Every PDF renders as its producer specified* is a
  claim about reading. The moment this program composes, it owes generator obligations for what it
  composed — the §7.5 structures the serializer already emits, and then §9's font embedding, §14.7
  and §14.8 structure for what it typeset — and `CLAUDE.md` says generator obligations come into
  scope only where the serializer emits the construct. Step 2 brings fonts and tagging with it.
- **The project's shape.** The exclusion was written *to stop this program becoming a layout
  engine* (ADR 1014's argument). Step 3 is that engine.

### 5.4 Recommendation

**Step 1 is the only one worth the owner's sentence, and only if qpdf parity matters to the
owner.** Its amendment would read, in the shape of the fourth: *placing a producer's page over or
under the pages of another document, at a position a person states, is on the near side; every
mark is a producer's and what this program composes is the placement.* Its cost is the oracle's,
and its mitigation is the one ADR 1014 used — the report says a page was composed, and the
composed output is held by name against this program's own earlier output. **Steps 2 and 3 stay
excluded**, and the owner is asked to say so once more so that the sentence is dated (question 7).

## 6. JavaScript — one pointer

RFC 0008 is the proposal and `doc/questions/Q193` its eleven questions; nothing is re-argued here.
What this document adds is where scripts touch the other exclusions, so that no later round reads
one without the other:

- A **rendition action** (Table 218) carries a `/JS` beside its `/OP`, and the standard's own
  fallback sentence is the one RFC 0008 quoted: `/OP` "is considered a fallback that shall be
  executed if the interactive PDF processor is unable to execute ECMAScripts" — so Tier A's media
  report reads `/OP`, never `/JS`.
- **Rich media**'s `/C` command (Table 223) and **3D**'s `OnInstantiate` and `3DI false` are
  scripts addressed to a player; RFC 0008 section 4.3 excludes the 3D and rich-media host objects
  outright, so lifting clause 13 to Tier C or D later would reopen one row of that table and not
  the engine question.
- **XFA**'s calculations are FormCalc or JavaScript against an XFA object model that is neither
  Adobe's Acrobat API nor ISO 21757-1's; nothing in RFC 0008 serves it, and section 4 above does
  not ask it to.
- **Printing**'s `/WP` and `/DP` (Table 200) are sites RFC 0008 names as waiting on RFC 0004;
  section 2 finishing RFC 0004 is what gives them a site.

## 7. Everything else the ledger excludes, read row by row

The 34 `inapplicable` rows and the two `writer-side` rows, grouped by what would move them. Each
quotation below is the one the row carries and `conformance::ledger::grounding` verifies against
`doc/md/` (ADR 1535).

| rows | why inapplicable | what would flip it | this RFC's reading |
|---|---|---|---|
| §10.6.1–§10.6.4 (4) | "Halftoning is not required for such devices" | this program producing a bilevel device's pixels itself | section 2.4: stays, with the route named |
| §14.2 (1) | procedure sets are for "a PostScript language compatible output device" | a PostScript spool — Route A hands a PDF, Route B a raster; neither is one | stays |
| §14.8.3.2, §14.8.5.4.1, §14.8.5.4.4, §14.8.5.4.6, §14.8.5.4.7 (5) | "Layout attributes specify parameters of the layout process used to produce the appearance described by a document's PDF content" — this program performs no such process | **the fourth exclusion's step 3, or XFA**: these rows are the authoring exclusion's in the ledger's vocabulary, and the owner should know the two are one decision | stays while section 5's line stays |
| §14.10 family (18) | "The features described in this clause are deprecated with PDF 2.0", and every operation is a capturing application's — "Retrieve additional material from the Web and add it to an existing PDF file" | *HTML to PDF* — the fourth exclusion's step 3 by its older name | stays, for the same reason |
| §14.11.2.2 (1) | a permission declined — "[i]nteractive PDF processors **may** offer the ability to display guidelines" | a view that shows the page boxes: the print panel of section 2.10 | **moves with printing** |
| §14.11.4, §14.11.6.1, §14.11.6.3, §14.11.7 (4) | preseparated files, trapping and OPI are a press's, each deprecated | a prepress product; RFC 0004 section 8 named prepress *out* deliberately | stays |
| §14.12.4.2 (1) | "application-specific information communicated between the creator of PDF data and a receiving system" | being that receiving system | stays |
| §7.6.7, §E.2 (`writer-side`, 2) | an unencrypted wrapper's and a second-class name's rules bind a producer | the serializer emitting either construct | the serializer's rows, not an exclusion's |

Two `reported` families are not exclusions and are named so that nobody mistakes them for one:
§7.6.5's public-key security handler is held by `doc/questions/A66` and `A168` on a trigger (a
document encrypted to a key a real reader holds), and §13.4 is section 3's. **Nothing in this table
is a fifth exclusion hiding under another word** — which is the question `CLAUDE.md`'s decay rule
asks, and the answer this round found.

## 8. The ranking

By reader value against cost and risk, with the standard's own hints weighed:

1. **Printing's remainder.** Not an exclusion; demand-backed in RFC 0001 (its ninth-section heading finding: *print is demand-backed,
   not just owner interest*); the model, the intent, the levels and two windows exist; the one
   dependency it needed has lost its objection; nothing in `CLAUDE.md` moves. The window page one
   is drawn in is the window that cannot print, which is the wrong window to leave for last.
2. **Clause 13's still page (Tier A).** One drawing is already owed under the current rules
   (§13.4's poster, `A33`); the report is §13.2.2 honoured in the one way a non-player can honour
   it; the hand-off reuses a level shape built three times over (links, remote documents,
   embedded documents, submissions) and the standard's own first position is that the player is
   other software. Cost: a module, a level, no dependency. Risk: none to the sandbox.
3. **XFA's sentence.** One round: read two keys, say one sentence per document. The exclusion
   stays on two grounds.
4. **§14.11.2.2's guidelines**, as part of 1.
5. **Clause 13 Tier B** (PCM sound through the worker and `cpal`): admissible, small, and
   deferred on demand — no corpus document asks.
6. **Authoring step 1** (overlay from a PDF): only on the owner's word that parity with qpdf's
   `--overlay` is wanted; the amendment is drafted in section 5.4.
7. **Not now, each with its trigger named**: clause 13 Tier C (an admissible H.264 and AAC decoder,
   or C in a media worker accepted in writing), Tier D (a U3D reader in Rust), XFA proper (a
   layout engine the project has decided not to be), authoring steps 2 and 3.

## 9. The current restrictions, each with its rationale, and what the unconstrained design is

`doc/rfc/README.md`: an RFC names a standing rule as a *current restriction with its original
rationale* and proposes the unconstrained design. Eight bear on this one.

| restriction | its rationale | the unconstrained design | what this RFC proposes |
|---|---|---|---|
| **Clause 13 excluded**: *a media engine, not a rendering question* | a player is a product of its own, and a decoder for untrusted media is attack surface the tree had no place for when the sentence was written | a `pdf-media-worker` decoding in confinement, frames composited over the page, 3D rendered under `wgpu` | **amend the sentence to exclude the player and admit the page** (question 2): posters, appearances, the viability report, the file handed over under a level; the engine excluded with its two triggers named |
| **XFA excluded** on §K.1's permission | the standard grants it; the effort is "considerable" by the standard's own word | an XFA template engine with FormCalc | **keep**, and add the second ground (authoring); build the sentence to the reader |
| **JavaScript excluded** | RFC 0008's section 9 | RFC 0008's design | **nothing here**; section 6 |
| **Authoring from nothing excluded**, four amendments | the oracle's purity; the project's shape | overlay, stamp, headers, a layout engine | **keep steps 2 and 3**; step 1 only by the owner's word, with its sentence drafted |
| **No C dependency outside the sandbox** | untrusted input never reaches unsafe code | GStreamer or FFmpeg in the host as the player | **kept as written**: a system framework is the *user's player*, started under a level with the file; decoders this program runs are in the worker, C admissible there under ADR 0014 |
| **No async runtime** | a thread pool is not a reason for one | `ipp`'s default async client; `tokio` | **kept**: `ipp` 7's blocking `client` feature over the `ureq` already in the lock |
| **Nothing eager on the launch path** | time-to-first-page is the number a user judges by | Table 310's `PO` activating 3D at page open; `/NeedsRendering` regenerating at open | **kept**: every tier here is constructed on first use; 3D's `PO`/`PV`, if ever built, after the first present under a budget, as RFC 0008 section 6.6 orders scripts; XFA's open-time regeneration is one reason section 4 declines it |
| **`pdf_syntax::Document` is immutable** | `interpret` is a pure function of the bytes | a watermark written into the document | **kept**; every write proposed here is a derivative (a spool, a baked copy for Route A) or an incremental update, never a mutation |

## 10. The costs, each written down

1. **Printing**: one crate (`ipp`, default features off) on an existing stack; a spool writer that
   is this tree's second file producer after the serializer and deserves the same fuzzing; three
   panels in three idioms; Route A as a named option prints another renderer's reading and the
   label must say so.
2. **Clause 13 Tier A**: a `pdf_model::media` module reading six dictionaries for a report; a
   level; a hand-off that starts a program on this machine under `ask`. No decoder. The 88 rows
   barely move (section 3.7), and that is to be said plainly.
3. **Tier B**: `libasound` linked into the winit host (the toolkit hosts already have it); a wire
   arm; `cpal`'s dependency count. Deferred on demand.
4. **Tier C and D**: a third worker, a media player's engineering, C in the worker justified in
   writing or a parser written from a 200-page specification; neither recommended, both priced.
5. **XFA's sentence**: two keys read, one sentence; nothing else. XFA proper: a layout engine on
   the launch path and a second scripting language — declined.
6. **Authoring step 1**: an oracle that holds a composed page against itself; a generator
   obligation (§7.5's structures) the serializer already meets; an amendment's sentence.
7. **Every amendment is an edit to `CLAUDE.md`'s closed list**, which the file says is revisited
   *by argument, never by attrition*; this document is the argument, and sections 2–5 are where
   the owner can see what each step buys before buying it.

## 11. The questions for the owner, numbered, each with a recommendation

The convention (`doc/questions/README.md`) puts every open question in a `Q` file; this round's
number is `Q254`, which points here and asks the owner to answer these by number in one `A254`,
as `A54`–`A60` answered RFC 0007's.

1. **Is RFC 0004 finished as section 2.12 orders it, and are its three open questions closed as
   follows?** The DPI clamp stays at 150–600 with 300 default (its question 2); the spool
   container is a transport envelope on the near side of the authoring line (its question 5,
   section 2.8's reading); the winit host submits over IPP first and the portal second (its
   question 6). *Recommendation*: **yes to all three**, in that order of build.
2. **Is clause 13's exclusion amended so that it excludes the player and admits the page?**
   *Recommendation*: **yes**, replacing the entry with:

   > **Clause 13, the playing of multimedia and the rendering of 3D artwork** — a media engine,
   > not a rendering question. **In scope**: what a still page shows of a media object — a
   > movie's stream-form poster, every producer-supplied appearance, §13.2.2's viability computed
   > and reported in the clause's own terms — and the media file saved or handed to this machine's
   > own player under *A document's restrictions are the reader's to set*, `ask` by default.
   > **Excluded still**, each with the trigger that reopens it: decoding or presenting audio and
   > video (an H.264 and AAC decoder admissible under `deny.toml`, or C in a confined media worker
   > accepted in writing under ADR 0014), and rendering U3D or PRC artwork (a U3D reader in Rust).
3. **Is the hand-off's level `Links` or a `Media` level of its own?** *Recommendation*: **its
   own**, `--media=refuse|ask|warn|open`, `ask` by default, because the act differs from a link in
   one way a reader may care about — the file is the document's own bytes, not a URL — and because
   `Links` is pinned to three schemes for a reason that does not apply to a file.
4. **Is Tier B (this program playing §13.3's PCM and µ-law sound objects) built now, or on
   demand?** *Recommendation*: **on demand** — the first document in a corpus or a report whose
   sound annotation has no `/AP` to show or whose `/S /Sound` action a person clicks; it is small
   and admissible, and nothing asks.
5. **Does XFA stay excluded, now on two grounds (§K.1's permission and the authoring exclusion),
   with the per-document sentence built?** *Recommendation*: **yes**; the row stays
   `out-of-scope` and its note names the report.
6. **Does Route A — the file handed to CUPS as a baked derivative — become a named print option
   after the IPP path exists?** *Recommendation*: **yes, named, never default**, labelled as the
   print system's own renderer drawing it.
7. **Does the authoring exclusion stay as written for steps 2 and 3 (text stamps, headers,
   footers, page numbers; a layout engine; HTML to PDF), with the owner's sentence dated?**
   *Recommendation*: **yes**. And separately: **is step 1 (a producer's page overlaid or underlaid
   at a person's position) wanted?** *Recommendation*: **not unless qpdf parity is a goal the
   owner states**; section 5.4 has the sentence if it is.
8. **Does §10.6 stay `inapplicable` on a print path, with its one flipping route named in the
   row?** *Recommendation*: **yes** (section 2.4).
9. **Is a CMYK spool through the output intent's BToA tables wanted at all?** *Recommendation*:
   **no**, until a spool path exists and a document on it asks; the report of section 2.6 instead.

## 12. What is built when

1. **Now, under the current rules, whatever the owner decides**: §13.4's poster (owed since
   `A33`); the XFA sentence to the reader (a report, no exclusion touched); §14.11.5's print-time
   report.
2. **On `A254` question 1**: the winit print panel and `ipp` submission; the confined spool
   writer over the same submission; the docked preview in all three windows; tiling;
   §14.11.2.2's guidelines in the panel.
3. **On question 2 and 3**: `pdf_model::media`, the viability report, the `Media` level in
   `viewer_host::policy` and the three windows' menus, the hand-off through the portal.
4. **On question 6**: Route A through the transform suite, as a named option.
5. **On question 4's demand, if it comes**: the PCM decoder in `pdf-view-worker`, the wire arm,
   `cpal` in the hosts.
6. **Never, under this RFC**: Tier C, Tier D, XFA proper, authoring steps 2 and 3 — each until its
   named trigger is re-argued.

## 13. Sources, all read on 2026-10-05

- ISO 32000-2, from `doc/md/ISO_32000-2_sponsored_EC3.md`: §10.1, §10.5, §10.6.1, §10.6.2, §12.2
  (Table 147, Table 148), §12.5.6.16, §12.5.6.17 (Table 189), §12.5.6.18, §12.6.4.14 (Table 218),
  §13.1, §13.2.1, §13.2.2, §13.2.4.2 (Tables 285–287), §13.3 (Table 305), §13.4 (Tables 306, 307),
  §13.5, §13.6.1, §13.6.2 (Tables 309, 310), §13.6.3.1 (Table 311), §13.7.1, §13.7.2.1 (Table 333),
  §13.7.2.3.3 (Table 342), §14.9.2.4, §14.11.5, Table 29's `/NeedsRendering`, Annex K (§K.1, §K.2).
- The tree: `CLAUDE.md`'s exclusion list; `doc/rfc/0004`, `0005`, `0008` and `README.md`;
  `doc/state-of-play.md`; `doc/conformance/ledger.toml` by regular expression (the counts in
  section 1); ADRs 0014, 1014, 1120, 1123, 1155, 1179, 1180, 1203, 1204, 1227, 1291, 1367, 1527,
  1535, 1548; `doc/questions/A33`, `Q193`; `crates/viewer-host/src/printing.rs`,
  `crates/viewer-host/src/policy.rs`, `crates/pdf-model/src/appearance.rs`, `annotation.rs`,
  `action.rs`, `restriction.rs`, `crates/viewer-ui/src/bin/quorra/app.rs`,
  `crates/viewer-confined/src/protocol.rs`, `crates/pdf-sandbox/src/lib.rs`, `deny.toml`.
- The corpora: `grep -alE` over `doc/pdf.js/test/pdfs/*.pdf` and `doc/corpora`;
  `doc/pdf.js/test/test_manifest.json`; `pdftotext -l 1` over the three `/NeedsRendering true`
  documents; `wc -l doc/pdf.js/src/core/xfa/*.js` — evidence about the size of the work, never a
  target.
- crates.io on 2026-10-05, through its API: `symphonia` 0.6.1, `hound` 3.5.1, `lewton` 0.10.2,
  `claxon` 0.4.3, `cpal` 0.18.2, `rodio` 0.22.2, `kira` 0.12.5, `gstreamer` 0.25.4,
  `gstreamer-sys` 0.25.4, `alsa-sys` 0.6.1, `ffmpeg-next` 9.0.0, `rav1d` 1.1.0, `openh264`
  0.9.8, `mp4` 0.14.0, `matroska` 0.30.1, `ipp` 7.0.0 (its features and dependencies), `ashpd`
  0.13.13, `web-audio-api` 1.7.0; `u3d`, `prc` and `xfa` answering *not found*.
- `gstreamer.freedesktop.org`'s licensing page (LGPL); `ecma-international.org`'s ECMA-363 page
  (answers 200); `pdfa.org/norm-refs/XFA-3_3.pdf` and its Internet Archive copy (both answer 403
  to this round, so the specification's size is not stated).
