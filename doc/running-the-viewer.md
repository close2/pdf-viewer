# Run it

Status: **standing** — what the program does when a person starts it.
Read by: whoever is running the viewer rather than the gates. `doc/todo/02-every-round.md` §5 is
what puts the binaries where a person can reach them; `doc/verify.md` is the instruments.

`doc/HANDOVER.md`'s "Run it" is the pointer to this file.

**One of the six binaries `doc/todo/02` §5 installs is not a viewer at all**: `target/pdf-retrieve`
answers a *program*'s questions about a document as JSON on stdout — a page, a section addressed by
its clause number, and the annotations over either — and nothing here applies to it. `doc/todo/36`
and ADR 0257 are its two files, and `pdf-retrieve` with no arguments prints what it takes.

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

`--page N` opens at a page, **and so does Annex O's fragment identifier** —
`pdf-viewer 'doc/ISO_32000-2_sponsored_EC3.pdf#page=100&zoom=150'` opens at page 100 of 1023 and
asks for an 893×1263 raster, which is 150% of a 595×842 page; `#nameddest=`, `#view=`, `#viewrect=`,
`#comment=`, `#structelem=`, `#search=` and `#ef=` are the others carried out, and the ones that are
not are printed by name — `tools/state.sh annex-o` says which, and it reads the program rather than
this sentence (ADRs 0209, 0250, 0310). **`#ef=` hands the embedded file it names to the host and the
host declines to write it**, because a URI is not a person asking: the note says so and names §O.2.1,
which is the annex's own "may choose to prompt the user or even prevent opening of the file". The argument is split at its first `#` only when the whole of it does not name
an existing file, so a document called `a#b.pdf` still opens.

**`--cpu` means *no graphics device*, since the three-hundred-and-eighty-fourth session** (ADR
0221): no `wgpu::Instance`, no adapter, no device, nothing that loads a driver — `strace` over a
`--cpu` run opens 17 shared objects where it opened 56 and three Vulkan libraries before. The page
is drawn by `render-cpu` and reaches the window through `softbuffer`, with the sidebar, the
selection and the modal card composited on the processor rather than handed to quorra as geometry
(`viewer_ui::software`). It is the flag to reach for when the graphics driver is what is broken,
and until that session it was the one flag that could not help: page one to first present is
**57 to 68 ms** where it was 128 to 135.

**`--backend vulkan|dx12|metal|gl`** names which driver stack talks to the GPU — not which GPU,
which is what `Options::adapter` selects and cannot express. **Refused rather than ignored** where
this machine has no adapter behind the name: the stage that failed, the adapters behind the
instance, the adapters on the machine by every route, and what to try, then exit 1. A word that
is not one of the four is refused at parse with the list. **On Windows this build asks for DX12
first** and falls back to every backend with a note where there is none, because with no
restriction the choice falls to wgpu's hub order — which puts Vulkan first, and is not a choice
this project had made. No machine here runs Windows: that default is argued in ADR 0221, not
measured.

**`--trace` prints every window event, command, event and frame with its duration** — the last
line printed is the step that did not finish, and, since the three-hundred-and-eighty-fourth
session, two lines about the backend: `backend asked for: dx12 (--backend)`, which is a fact about
the command line, and `rendering with llvmpipe (…) (Cpu, Vulkan)`, whose parenthesis is the
backend actually chosen. `--trace` also installs a receiver for what `wgpu`,
`vello` and `naga` say about themselves, at `PDFVIEWER_LOG`'s level (default `warn`): those three
write to the `log` facade and a facade with nothing behind it drops every record, which is how a
page that would not draw produced no output at all.

**Since the three-hundred-and-ninetieth session it says *what* a slow frame spent its time on, and
it is no longer all-or-nothing** (ADR 0227, from the project owner's Windows trace of a 30 MB
document that felt slow). Four things changed:

- **Every line carries a clock** — the seconds since `main`'s first instruction — so a gap in the
  log is legible as a gap and the interval a person waited can be read off two lines.
- **One line per frame, with the stages in it.** `frame p3 2822cmd presented 75.3 | host 0.0 scene
  2.3 device 73.0 settle 0.0 attend 2.9 | 793 up, 12 culled`: this host's own queries, the
  display-list-to-scene walk, `quorra_gpu::Device::render`, the cache eviction, and — *outside* the
  frame's own number, which is the measurement defect that round fixed — the accessibility
  publication. `fallback` and `attend` appear only when they are not zero, so an ordinary frame is
  no longer than the two lines this replaced. A legend prints once. **None of the device's stages
  is a fabricated boundary**: quorra already measured `encode`, `upload` and `execute` and already
  blocked on the device before returning, and `render-quorra` was discarding the whole `Frame`.
- **Percentiles at exit** — median, p90, max and sum for every stage, by nearest rank, plus the
  `elsewhere` inside `Device::render` that its three named phases do not cover. **That row is a
  bound rather than a duration and the summary says so since the three-hundred-and-ninety-first**:
  where `execute` came from the adapter's timestamp queries, `elsewhere` subtracts a device clock
  from this host's wall clock, so it carries whatever the two disagree by along with the acquire,
  the present and the readback (ADR 0228 §4, `QUORRA_FEEDBACK.md` §13).
- **`--trace=<topics>`**, comma-separated, of `launch, frames, events, window, pointer, access,
  selection` or `all`; a `-` prefix subtracts, and a list starting with one means "everything
  except". `--trace` alone is still everything. `--trace=frames` is 64 lines where `--trace` is
  453 on the same session; `--trace=-pointer` is the answer to the 285 pointer-move lines that
  raised the item. It costs **0.30 µs a frame unconditionally** (seven clock reads and a copy) and
  **0.23 µs** for each line printed; over three identical scripted sessions the viewer's own CPU
  time was 5.51/4.87/4.89 s with no flag against 5.17/4.68/4.90 with `--trace`, which is inside
  the spread of either.

**Put the binaries where a person can run them, at the end of every round.** The agent builds
into `/home/AI/cargo-target/pdf-viewer/`, which the human's shell never looks at, so the last step
of a round copies what a person would run into the project's own `target/`:

```sh
cargo build --release --bin pdf-viewer --bin pdf-sandbox-worker --bin pdf-view-worker
  # one invocation, not three: each is a whole-graph fat link and Cargo runs three of them beside
  # each other where three commands run them one after another — 109.7 s to 79.3 s (ADR 0222)
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer         target/pdf-viewer
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-sandbox-worker target/pdf-sandbox-worker
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-view-worker    target/pdf-view-worker
```

All three, and all three beside each other: `pdf-sandbox-worker` is a separate executable the
viewer spawns for JBIG2 and JPEG 2000, and a viewer that cannot find it refuses those images rather
than falling back. **`pdf-view-worker` is the third and is new in the three-hundred-and-eighty-first**
— the whole viewer confined, which `pdf-viewer` does not yet spawn and which
`viewer_confined::Confined` and the example below do. `doc/todo/02-every-round.md` is the rest of
what a round does.

**A page drawn where it cannot be read from**, which is the confined path end to end and needs no
window:

```sh
cargo run --release -p viewer-confined --example confined_page -- doc/PDF20_AN001-BPC.pdf 1 out.png
```

It prints the confinement the worker reached — or `Confinement::shortfall`'s sentence where it
reached less — and then what each step cost: **1.09 to 1.14 ms** to start and confine a worker,
**6.7 to 8.7 ms** to open, interpret and draw page 1 at 849×1200 against **6.0 to 6.4** in this
process, and **3.4 to 4.8 ms** for the 4.1 MB of pixels to cross the pipe. On
`doc/ISO_32000-2_sponsored_EC3.pdf`, 19.2 MB, page one costs 66.9 and 82.6 ms, most of it the
document crossing once. ADR 0218.

**And a sidebar's worth of a document read out of the same confinement**, which is what the
three-hundred-and-eighty-sixth added:

```sh
cargo run --release -p viewer-confined --example confined_panels -- doc/PDF20_AN002-AF.pdf
```

All eleven of the panel-shaped questions, each timed: an outline printed as a tree (37 visible
items, **0.022 ms**), a layer list, an attachment list, a thumbnail's dimensions (**0.193 ms**),
§14.3.3's properties with the XMP packet's `dc:title` beside them (**0.049 ms**), and §14.7's
structure. Try `doc/PDF-Declarations.pdf` for two embedded files,
`doc/pdf.js/test/pdfs/issue15716.pdf` for §8.11.4.3's `/Order` and
`doc/ISO_32000-2_sponsored_EC3.pdf` for the largest answer in the tree — 988 outline items,
88 233 bytes. ADR 0223.

**And rebuild before saying anything about speed**: `cargo test` only ever builds the *debug*
binaries. The hundred-and-forty-second session was reported as "still lags" against a binary three
hours and six commits old, one of which was the 40× page-turn fix. A stale executable is a
measurement of the past.

**`p` runs §12.4.4's presentation**: the window drives the clock, a page with a `/Dur` advances by
itself, and the page arrived at has its `/Trans` **drawn** — seven of Table 164's twelve styles,
with the other five named in a note rather than cut in silence (ADR 0230). Press it again to stop.

**And it is a mode rather than only a clock, since ADR 0316.** `p` sends
`Command::Present(PresentationMode::On)`, which is §12.4.4.2's own condition — NOTE 3 respects a
page's navigation nodes "only when in presentation mode" — so while it is running the **arrow keys
walk a page's states before they turn the page**, a page turned by hand plays its `/Trans` where
only the clock's advance used to, and §8.11's groups are put back as they were when the
presentation stops. Full screen is still not part of it: what the clause states is the timing, the
transition and the states, and those are what this drives.

No corpus document states a `/Trans`, a `/Dur` or a `/PresSteps`, so the thing to open is a
fixture — whose fourth slide is the one with states:

```sh
cargo run --release -p pdf-model --example presentation_fixture -- /tmp/slides.pdf
cargo run --release -p viewer-ui --bin pdf-viewer -- /tmp/slides.pdf     # then press p
```

Under `Xvfb` with `lavapipe`, starting a transition costs **8.3 ms** (two 800×1000 page rasters,
drawn once) and a frame of one costs a median of **3.8 ms**; under `--cpu`, **11.1 ms** and
**16.0 ms**. `--trace=frames` prints the first as a `TRANSITION` line.

Arrows / Page Up / Down / Space turn pages, Home and End jump, `+`/`-`/`0` zoom, **Tab and shift-Tab walk the page's annotations in the order Table 31's `/Tabs` states** (§12.5.1, all five values), **with a ring drawn round whichever one holds the focus** — the clause says nothing about showing one, so the ring is this host's colour and a native host would use its platform's, the up and down
arrows scroll a page larger than the window, the wheel scrolls whatever is under it and
**Ctrl + the wheel magnifies the page about the pointer** (ADR 0166), **`o` shows
the sidebar** — three tabs: §12.3.3's outline, where a click on a title goes there and a click on
the triangle opens a subtree; §8.11.4.3's layers, where a click on a switch turns a layer on or
off unless Table 99's `/Locked` forbids it; and §7.11.4's embedded files, where a click writes the
file beside the document. The sidebar's fourth tab is §14.3.3's `/Info`, with §14.3.2's XMP
listed under it where the document carries a packet — both, rather than one, because §14.3.4
leaves a disagreement between them "at the discretion of the PDF processor". **`?` puts `/NOTICE` over the page**,
scrollable, which is the About panel and the visible half of what both vendored-font licences
oblige a binary to carry. Escape quits — **unless a field has the keyboard, where it leaves the
field**, which ADR 0201 decided and which was dead code until the three-hundred-and-seventy-first
session found the event handler answering the key three branches earlier. The title bar names how many things on
the page could not be drawn and the things themselves are printed. **A click inside a form field aims the keyboard at it**, and so does a Tab walk that lands on one:
characters, Space and Enter go in at the caret, the **left and right arrows, Home and End** move
it, **Backspace and Delete** take out the character on either side of it, and Escape gives the
keyboard back to the page (ADRs 0201 and 0211). The caret is a black line where the next character
will be drawn — the standard states no caret, so its colour and width are this host's, as the focus
ring's are, and a moving caret redraws the window without re-interpreting the page.
**And the caret goes where the click went, since the three-hundred-and-eighty-eighth** (ADR 0225):
`Query::Offset` is the inverse of `Query::Caret`, so a press names the byte it landed nearest rather
than the end of the value. **A drag inside a field selects part of it**, highlighted in the same blue
the page's own selection uses and sending no command at all; **shift with an arrow, Home or End
extends** that selection, and **Ctrl + C, X and V** copy, cut and paste it — into this program's own
clipboard, because the system's belongs to a platform and a native host owns that end. Those three
verbs needed no new message: the characters are a slice of the value the host has already read back,
and the edit is the `Edit::SetField` a keystroke already sends. **While a field has the keyboard it
takes every character key**, `o` and `?` included — both of them toggled the sidebar and the About
card from inside a field until that session, which is the third instance of the ordering defect ADR
0211 found in Escape. A click follows §12.5.6.5's
links and performs the eleven §12.6 actions this program can, and on a markup annotation it **opens the popup window §12.5.6.14 gives it** — a card over the page with Table 172's `/T` in its title bar, Table 166's `/C` behind that and `/Contents` under it, closed again by a second click (ADR 0191) — printing every refusal — including
§12.7.6.4's import, which reads an FDF file **beside the open document** and nowhere else. A
locked document is asked for its password at the terminal (§7.6.4.1), three times, with an empty
line to give up. **`h` marks up what is selected** with §12.5.6.10's highlight and **`k`** with its strikeout, and the mark is written into the file by `s` — §7.5.6's update, with the appearance stream this program draws beside it, so another reader shows the same marks (ADR 0196). **`f` arms the next drag to draw §12.5.6.6's free text annotation**, and what is drawn takes the keyboard at once: the caret is in the box, the arrow keys and Backspace work there exactly as they work in a field, Escape gives the keyboard back to the page, and a later click inside the box picks it up again. That subtype's text *is* the annotation, so an empty one draws nothing and only the caret says it is there (ADR 0238). **`/` opens the find bar**, and while it is open it takes every key: what is typed highlights every occurrence on the page being shown at once, **Enter** goes to the next occurrence *anywhere in the document* and **shift-Enter** to the previous, and **Escape** closes it. The bar says which page the occurrence is on, or counts down the pages still to read — a search reads one page per turn of the event loop, because interpreting all 1023 pages of ISO 32000-2 is 5.84 s and nothing in this program blocks for it (ADR 0250). Annex O's `#search=%22word%22` starts the same search as the document opens, which is what the standard's "selecting the first matching word in the document" asks for. **A signed document says whether its own bytes moved and whether its signature verifies**: for
every signature, who signed it, what its range covers, whether the bytes that range names still hash
to the digest the signature records (ADR 0215), and — since the three-hundred-and-ninety-second —
whether the signature verifies under the RSA key in a certificate **the file itself carries** (ADR
0229); and then, once, that this program answers two of a signature's three questions and not the
third, because it has no certificate store and makes no network request, so "a signature that
verifies here was made by whoever holds the key in a certificate that arrived with the document,
which is not the same as a valid signature. Nothing here says valid".
`doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf` is the loud one, and it is louder than it was: both
its signatures **verify**, over attributes recording a digest its bytes no longer produce — a real
signature whose document was re-saved underneath it.
**A screen reader gets the page**: on Linux the window puts §14.7's structure on AT-SPI as soon as
one attaches — nothing is created until the first frame is on the screen, and nothing is published
while `org.a11y.Status.IsEnabled` is false — with §12.4.2's page label naming the page, §14.9.3's
`/Alt` where the document states one, and what the page could not draw in a status group beside it.
A build with no bridge (macOS, Windows) says so in its first lines. **`pdf-viewer --licences`** prints `/NOTICE` and exits, which is what both
licences covering the compiled-in standard 14 fonts oblige a binary to carry. `--no-sandbox`
decodes JBIG2 and JPEG 2000 in-process — faster by a spawn and a pipe round trip, appropriate for trusted
documents, and it prints what it gave up.

**And since the four-hundred-and-eighth there is a second program, with a second toolkit.**

```sh
cargo run --release -p viewer-gtk --bin pdf-viewer-gtk -- doc/PDF20_AN001-BPC.pdf
```

`pdf-viewer-gtk` is the GTK4 host (ADR 0244, `doc/todo/30`). It takes one document, Annex O's
`#fragment` after it, and `--trace[=launch,frames,events,panel]` in the same line format
`pdf-viewer` uses, so the two hosts' launch timelines can be read side by side. It is deliberately a
**separate binary** rather than a flag: the two differ in their toolkit and in nothing else, which
is the claim `viewer-core` exists to make and which one binary linking both would stop making.

What it binds: `Left`/`Right`/`Up`/`Down`/`Page_Up`/`Page_Down` to turn pages, `Home`/`End`,
`+`/`-`/`0` to zoom, `w` to magnify until §12.7's controls fit their rectangles, `a` to select all,
`Escape` to select nothing, `s` to save, `z`/`y` to undo and redo, a drag to select, and the sidebar's three tabs to §12.3.3's outline, §8.11.4.3's layers and
§7.11.4's files — each a real `GtkListView`, with a layer's switch a real `GtkCheckButton` and an
attachment's row writing the file beside the document. §12.7's fields are real controls placed over
the page, and since the four-hundred-and-ninth the page underneath them is drawn **without** the
document's own pictures of those fields (ADR 0245) — §6.3.2.2's "unless otherwise instructed", which
this host instructs by default. `--draw-widget-appearances` puts them back, which is what the
standard asks of a processor nobody has instructed and what the two pictures in ADR 0245 were taken
with. **The one thing left to know** is the size: a GTK control has a theme's minimum and a widget's
`/Rect` can be smaller, so on `160F-2019.pdf` all 76 controls are taller than the rectangle they
cover. **`w` is what fixes it, since the five-hundred-and-eleventh** (ADR 0346): `--trace=panel`
prints the magnification at which every control would fit, and the key sends it as
`Zoom::Scale` — no message the vocabulary did not already have. It is offered rather than applied,
because a viewer that magnified a page by itself because a form is on it would be answering a
question nobody asked.

**And since the four-hundred-and-tenth there is a third program, with a third toolkit.**

```sh
cargo run --release -p viewer-qt --bin pdf-viewer-qt -- doc/PDF20_AN001-BPC.pdf
```

`pdf-viewer-qt` is the Qt 6 Widgets host (ADR 0246, `doc/todo/30`). It takes the same arguments
`pdf-viewer-gtk` does — one document, Annex O's `#fragment` after it, `--trace[=topics]` in the same
line format, `--draw-widget-appearances` — and binds the same keys, so the two hosts can be run side
by side and differ only in their toolkit. **One flag is its own**: `--quit-after=<ms>` closes the
window by itself, because a window under `Xvfb` has nobody to close it and a test that killed the
process could not tell a clean exit from a crash.

Both native hosts bind `/` and Ctrl+F to their toolkit's own find bar — a `GtkSearchBar` with a
`GtkSearchEntry` and a `QToolBar` with a `QLineEdit` and Previous/Next actions — and draw every
occurrence on the page under the selection, in the platform's colour at a lower alpha. Nothing about
either bar is drawn by this project, which is `doc/ui-boundary.md`'s rule applied to a find bar: the
geometry of the matches crosses and the furniture is the desktop's (ADR 0250).

What it shows that the GTK host cannot: the selection and §12.5.1's focus ring in the *desktop's*
colours — `QPalette::Highlight` and `QPalette::Accent`, which GTK 4.22 exposes no equivalent of at
all — and Table 233 bit 19's editable combo box, which `QComboBox` supports and `GtkDropDown` does
not. The sidebar's three tabs are `QTreeView`s over one `QAbstractItemModel` with two columns, a
layer's switch is `Qt::CheckStateRole` rather than a widget, and §12.7's fields are `QLineEdit`,
`QPlainTextEdit`, `QCheckBox`, `QRadioButton`, `QPushButton`, `QComboBox` and `QListWidget` placed
over the page.

**It needs Qt 6's development files to build** — `qmake6`, `moc` and a C++ compiler — which is what a
native host binding a platform means, and is why it is in none of the three cross-target checks.

**The pipeline is a gate this project had stopped reading, and both its failures were real** (ADR
0189). It had been red since 2026-08-02. `render-gpu`'s bounded wait was one second rather than the
sixty its constant and comment claimed, because `wgpu::PollError::Timeout` from the *slice* was
returned as a failure instead of looping — invisible on this machine and fatal on a CI runner's
`lavapipe`. And Miri's deliberate float non-determinism failed four `pdf-render` tests on the
runner and a different three here, all of them tracing to `f32::hypot`, which libm does not
promise to round correctly and which this crate used to decide **whether an image is magnified,
how far its grid reduces, whether a miter passed §8.4.3.5's limit and whether a dash has no
length**. `geom::length` is `(dx * dx + dy * dy).sqrt()` — IEEE operations only — so those four
decisions are now the same on every platform, which the two-backend comparison had assumed and
which was not true. Nothing moved: 856 agree, 68 contradicted, 749 ambiguous, to the number.
**No gate in this tree could have found it**, because all of them run on one machine with one libm.

**And since the three-hundred-and-eleventh session a person can get it without a toolchain.** Every
push to `main` that passes `check` and `test` retags a rolling `snapshot` pre-release carrying
`pdf-viewer` and `pdf-sandbox-worker` with `LICENSE` and `NOTICE` beside them
because both vendored-font licences oblige a *binary* distribution to carry their notices (ADR
0188). **Both executables, because one of them alone is a quietly reduced program**: a viewer that
cannot find the worker beside it refuses JBIG2 and JPEG 2000 rather than decoding them in process.
**Three platforms since the three-hundred-and-fifteenth session** — x86_64 Linux, aarch64 macOS,
x86_64 Windows — **and the confinement is Linux's alone.** `pdf-sandbox` used to refuse to compile
where seccomp-BPF and Landlock do not exist, on the argument that "a sandbox that silently does
nothing on another platform is worse than no sandbox". The project owner asked for the other two
executables and accepted that; what makes it not that failure is that nothing is silent (ADR 0194).
The **worker process** is there on all three, so a decoder panic still costs one image rather than
the viewer, and so is the request deadline — rebuilt on Windows as a reader thread, because `poll`
is POSIX and on a platform with no address-space ceiling that deadline is the only bound left on a
hostile file's decode. What is missing is named by `Confinement::shortfall`, carried in the worker's
handshake, and printed by `pdf-viewer` in its first line. `doc/todo/35` is what a real confinement
for each would take.
