# Run it

Status: **standing** — what the program does when a person starts it.
Read by: whoever is running the viewer rather than the gates. `tools/batch.sh install` is what
puts the binaries where a person can reach them, and `doc/todo/02-every-round.md` section 5 says why
each is there; `doc/verify.md` is the instruments.

**A person runs `target/quorra`**, in the main checkout, and the other files beside it are
what it and its siblings need — `tools/batch.sh`'s `install_binaries`, `install_libraries` and
`install_featured` are the list: every merge of a batch ends with `tools/batch.sh install`, which
builds them `--release` from the commit `main` names and writes `target/installed-from` — that
commit and each file's SHA-256 — beside them. `tools/state.sh binaries` says which commit that is
and how far `main` has moved since (ADR 1511).

`doc/HANDOVER.md`'s reading table is the pointer to this file.

**One of the programs `tools/batch.sh install` puts there is not a viewer at all**: `target/quorra-retrieve`
answers a *program*'s questions about a document as JSON on stdout — a page, a section addressed by
its clause number, and the annotations over either — and nothing here applies to it. `doc/todo/63`
and ADR 0257 are its two files, and `pdf-retrieve` with no arguments prints what it takes.

**`target/quorra-transform archive` reads one thing no install puts there: the CJK pack, a second
download** (`doc/questions/A569`, ADRs 2302, 2303). A composite font of an Adobe character
collection that the file did not embed is converted only from that collection's program in the
pack — a directory named `cjk-pack` unpacked beside the `quorra-transform` executable, or named with
`--cjk-pack <dir>` — and held to the digests the program carries; without it each such font is
refused with a sentence saying the pack is not installed and where it goes. Nothing fetches it and
no window reads it. `doc/cjk-pack.md` is its layout and how `tools/cjk-pack.py` builds it; how it is
published beside a release is `doc/questions/Q596`, open to the owner.

```sh
cargo run --release -p viewer-ui --bin quorra -- doc/PDF20_AN001-BPC.pdf
```

`--page N` opens at a page, **and so does Annex O's fragment identifier** —
`quorra 'doc/ISO_32000-2_sponsored_EC3.pdf#page=100&zoom=150'` opens at page 100 of 1023 and
asks for an 893×1263 raster, which is 150% of a 595×842 page; `#nameddest=`, `#view=`, `#viewrect=`,
`#comment=`, `#structelem=`, `#search=` and `#ef=` are the others carried out, and the ones that are
not are printed by name — `tools/state.sh annex-o` says which, and it reads the program rather than
this sentence (ADRs 0209, 0250, 0310). **`#ef=` opens the embedded file it names in a tab of its own**,
with the rest of the fragment applied to it, in all three windows — `quorra-gtk
'issue17056.pdf#ef=destination-doc.pdf&page=3'` shows *destination-doc.pdf — 3 — page 3 of 30*
beside the document that holds it (ADR 1316). An embedded file that is not a PDF is not written,
because a URI is not a person asking: the note says so and names §O.2.1, which is the annex's own
"may choose to prompt the user or even prevent opening of the file". The argument is split at its first `#` only when the whole of it does not name
an existing file, so a document called `a#b.pdf` still opens.

**`--cpu` means *no graphics device*** (ADR
0221): no `wgpu::Instance`, no adapter, no device, nothing that loads a driver — `strace` over a
`--cpu` run opens 17 shared objects where it opened 56 and three Vulkan libraries before. The page
is drawn by `render-cpu` and reaches the window through `softbuffer`, with the sidebar, the
selection and the modal card composited on the processor rather than handed to quorra as geometry
(`viewer_ui::software`). It is the flag to reach for when the graphics driver is what is broken,
and it helps there because it loads none: page one to first present is **57 to 68 ms**, against
the 128 to 135 ADR 0221 measured while it still brought a device up.

**The window shows its own background before it shows a page** (ADR 0462), and the reason is
a crash rather than an appearance. The surface is configured
by putting one opaque texel of the surround on it, at the one moment in the process where no render
thread exists to submit beside the configure — because a *first* configure that fails is answered by
wgpu with a panic rather than a status, which under `panic = "abort"` is the core dump the project
owner met on launch. `--trace=launch` names it: a `surface configured` milestone between `graphics
device` and `document joined`, and a line saying what putting the ground up cost. **A device that
cannot manage even that draws on the processor instead and says so**, which is the same answer
`--cpu` gives and the same one a device that will not come up at all has had since ADR 0221.

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
line printed is the step that did not finish, and two lines are about the backend (ADR 0221): `backend asked for: dx12 (--backend)`, which is a fact about
the command line, and `rendering with llvmpipe (…) (Cpu, Vulkan)`, whose parenthesis is the
backend actually chosen. `--trace` also installs a receiver for what `wgpu`,
`vello` and `naga` say about themselves, at `PDFVIEWER_LOG`'s level (default `warn`): those three
write to the `log` facade and a facade with nothing behind it drops every record, which is how a
page that would not draw produced no output at all.

**It says *what* a slow frame spent its time on, and it is not all-or-nothing** (ADR 0227, from
the project owner's Windows trace of a 30 MB document that felt slow), in four parts:

- **Every line carries a clock** — the seconds since `main`'s first instruction — so a gap in the
  log is legible as a gap and the interval a person waited can be read off two lines.
- **One line per frame, with the stages in it.** `frame p3 2822cmd presented 75.3 | host 0.0 scene
  2.3 device 73.0 settle 0.0 attend 2.9 | 793 up, 12 culled`: this host's own queries, the
  display-list-to-scene walk, `raster_gpu::Device::render`, the cache eviction, and — *outside* the
  frame's own number, which is not the frame's to carry (ADR 0227) — the accessibility
  publication. `fallback` and `attend` appear only when they are not zero, so an ordinary frame is
  two lines. A legend prints once. **None of the device's stages
  is a fabricated boundary**: quorra already measured `encode`, `upload` and `execute` and already
  blocked on the device before returning, and `render-raster` was discarding the whole `Frame`.
- **Percentiles at exit** — median, p90, max and sum for every stage, by nearest rank, plus the
  `elsewhere` inside `Device::render` that its three named phases do not cover. **That row is a
  bound rather than a duration and the summary says so**:
  where `execute` came from the adapter's timestamp queries, `elsewhere` subtracts a device clock
  from this host's wall clock, so it carries whatever the two disagree by along with the acquire,
  the present and the readback (ADR 0228 §4, `QUORRA_FEEDBACK.md` §13).
- **A `handover` row nested under `scene`** (ADR 0423):
  of the display-list-to-scene walk, the part spent inside quorra's own `upload_*` calls, which is
  the only part of it this host does not do itself. It is a *subdivision* rather than a stage beside
  the others, so adding it to the rows above it counts the same milliseconds twice. **And
  `--trace=launch`'s table ends with the first frame's own phases** — scene, handover, encode,
  transfer, execute, elsewhere and present — because `first scene built` and `first present` are one
  frame, and the percentiles above are medians over a run rather than an answer about the frame a
  person waited for.
- **`--trace=<topics>`**, comma-separated, of `launch, frames, events, window, pointer, access,
  selection` or `all`; a `-` prefix subtracts, and a list starting with one means "everything
  except". `--trace` alone is everything. `--trace=frames` is 64 lines where `--trace` is
  453 on the same session; `--trace=-pointer` is the answer to the 285 pointer-move lines that
  raised the item. It costs **0.30 µs a frame unconditionally** (seven clock reads and a copy) and
  **0.23 µs** for each line printed; over three identical scripted sessions the viewer's own CPU
  time was 5.51/4.87/4.89 s with no flag against 5.17/4.68/4.90 with `--trace`, which is inside
  the spread of either.

**`PDFVIEWER_ZOOM_PROBE=1` makes the two toolkit windows say what a zoom reached** (ADRs 2224,
2225): versioned `zoom-record` lines beside the trace's own — a wheel callback, its modifiers, the
branch taken and the host's outcome counted apart, and the content widget's geometry in each
coordinate space it is measured in — and `quorra-gtk` with `PDFVIEWER_ZOOM_CAPTURE_DIR=<dir>` also
writes each settled whole-page source joined to the generation that painted it (ADR 2240).
`tools/zoom-probe-drive.py --x11 --host gtk|qt --binary <program> --pdf <file> --output <dir>
--parser tools/zoom-probe-records.py` drives an owned window through them by XTest, so only an X11
window — on a Wayland session that is Xwayland's, which is why `--x11` is required to say so —
`--headless` on an Xvfb of its own; the records say what input and geometry reached and nothing
about latency or sharpness (ADRs 2228, 2229). **On Wayland the wheel is driven from inside the
program**, because no compositor admits synthetic pointer input: `quorra-qt` with
`PDFVIEWER_ZOOM_PROBE=1 PDFVIEWER_ZOOM_DRIVE=notches=8,every=133,after=3000` posts eight
Control+wheel notches 133 ms apart, three seconds after its first frame (`angle=15` sends each as a
high-resolution wheel's eighths, a negative `notches` zooms out, `x`, `y` place the pointer as
fractions of the page area), through the same handler a real wheel reaches (ADR 2266).
`tools/zoom-probe-wayland.py` compares the GTK window's settled page with a nested KWin's output at
each scale, and `--rescale 1.6:1.25` changes the scale under one open window to test the surface's
scale hook alone (ADRs 2250, 2251). On that nested KWin the probe sends input itself, through
KWin's own EIS interface: `--wheel` reads a window's count of a wheel off the output — zoom steps of
four Control notches, a notch's scroll, the eighths that moved the page — and `--drift` puts the
pointer on a marker, sends ten notches in and ten out, and splits the marker's offset into the core's
anchor and the window's placement (ADRs 2274, 2275), and `--touchpad` sends a touchpad's smooth
travel and reads how far the page moved (ADR 2291). On a KDE session `tools/zoom-probe-activate.py <pid> <file>`
activates one window and confirms it is the active one, so that a capture of the active window is
of that window (ADR 2266).

**The binaries a person runs are the merge's to install, and one command does it.** At a batch
boundary — after the fast-forward, before `close` — the orchestrator runs, from the main checkout:

```sh
tools/batch.sh install
```

It refuses a batch worktree holding uncommitted work or a branch `main` has not been fast-forwarded
to; otherwise it builds every program of `tools/batch.sh`'s `install_binaries` in one `--release`
invocation (each is a whole-graph fat link, and one invocation runs them beside each other — ADR
0222) and the two C libraries in a second, in the batch's own build directory, which it asks Cargo
for in the worktree — `/home/AI/cargo-target/pdf-viewer-batch`, named by the worktree's
`.cargo/config.toml` (ADR 1440). Then it installs each into the main checkout's `target/`, prints
each path with the commit it was built from, and names anything older there that it did not put
there. `doc/todo/02-every-round.md` section 5 says why each file is there; the short of it is that
every worker is looked for beside the running executable, and a program that cannot find its worker
refuses the work rather than falling back.

**To rebuild by hand with no batch open**, as the agent user in the main checkout, which builds
where `~/.cargo/config.toml`'s `target-dir` says (`/home/AI/cargo-target/pdf-viewer`):

```sh
names=$(sed -n 's/^install_binaries="\(.*\)"$/\1/p' tools/batch.sh)   # the one list, never a copy
built=$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)/release
cargo build --release $(printf -- '--bin %s ' $names)
for binary in $names; do install -Dm775 "$built/$binary" "target/$binary"; done
```

— and `target/installed-from` then names a commit those files are not of, which `tools/state.sh
binaries` reports as a changed file; that is the record telling the truth, and the next merge's
`install` makes the two agree again.

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

**And what a document costs the confinement in *address space*, which is a different question and
a different counter**:

```sh
cargo run --release -p viewer-confined --example confined_peak -- doc/PDF20_AN001-BPC.pdf
```

`VmPeak` off the live worker, against the ceiling `RLIMIT_AS` compares it to — the two are the same
counter, which is why this reads `VmPeak` and not `VmRSS`. It takes any number of documents in a
row on one worker, and it prints what the *host* was told for one it could not open, which is the
interesting case: a document past the worker's message budget comes back as a sentence and leaves
the worker running. ADR 0597.

**And a sidebar's worth of a document read out of the same confinement** (ADR 0223):

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
binaries, and a stale executable is a measurement of the past. `doc/todo/02` section 5 owns that rule and
the incident it is argued from; this was a second copy of both.

**`p` runs §12.4.4's presentation**: the window drives the clock, a page with a `/Dur` advances by
itself, and the page arrived at has its `/Trans` **drawn** — every one of Table 164's twelve
styles, `R` being the cut the table defines it as (ADRs 0230, 1299). Press it again to stop.

**And it is a mode rather than only a clock** (ADR 0316). `p` sends
`Command::Present(PresentationMode::On)`, which is §12.4.4.2's own condition — NOTE 3 respects a
page's navigation nodes "only when in presentation mode" — so while it is running the **arrow keys
walk a page's states before they turn the page**, a page turned by hand plays its `/Trans` as the
clock's advance does, and §8.11's groups are put back as they were when the
presentation stops.

**And it is a window** (ADR 0470): §12.4.4 states the timing and the transitions and nothing
about a window, and Table 29 states the window, so `p` takes the window full screen and hides the chrome Table 29's `FullScreen` names — "no menu bar, window controls, or any other
window visible" — with §12.2's `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` obeyed whether a
presentation is running or not. **Escape comes back**, which no clause states and which this program
chooses so that a file cannot keep a reader there; on the way out §12.2's `/NonFullScreenPageMode`
says which panel opens. **All three hosts do it, on the same letter**: `quorra`, `quorra-gtk`
and `quorra-qt`.

**And all three drive the clock** (ADR 0473): `/Dur` advances the page and Table 164's effects
animate in `quorra`, `quorra-gtk` and `quorra-qt` alike, on one shared decision — `viewer_host::Clock`, because how often to tick and when `/D` has elapsed are the
clause's questions and not a toolkit's. Nothing is armed when nothing is presenting, and a page
stating no `/Dur` costs a still window a wakeup and no repaint. **A presentation nobody can see is
parked rather than ended**: a hidden or minimised window, or another tab in front, counts none of a
page's `/Dur` and draws no transition frame — one pump every window's timer turns,
`viewer_host::presentation_pump`, and in `quorra` its own winit turn (ADRs 2300, 2317). In
`quorra-gtk`, `quorra-qt` and a C caller's window a question, a script's dialogue or a password
prompt the document raises waits until the window leaves full screen; `quorra`'s cards are drawn
inside the presentation and are not held (ADR 2309).

No corpus document states a `/Trans`, a `/Dur` or a `/PresSteps`, so the thing to open is a
fixture — whose fourth slide is the one with states:

```sh
cargo run --release -p pdf-model --example presentation_fixture -- /tmp/slides.pdf
cargo run --release -p viewer-ui --bin quorra -- /tmp/slides.pdf     # then press p

# and the window, which the file itself can ask for (Table 29's /PageMode /FullScreen):
cargo run --release -p pdf-model --example presentation_fixture -- /tmp/full.pdf --opens-full-screen
target/quorra /tmp/full.pdf        # opens presenting; Escape opens the Files panel, which is
target/quorra-gtk /tmp/full.pdf    # §12.2's /NonFullScreenPageMode /UseAttachments
target/quorra-qt /tmp/full.pdf
```

Under `Xvfb` with `lavapipe`, starting a transition costs **8.3 ms** (two 800×1000 page rasters,
drawn once) and a frame of one costs a median of **3.8 ms**; under `--cpu`, **11.1 ms** and
**16.0 ms**. `--trace=frames` prints the first as a `TRANSITION` line.

**More than one document is a strip of tabs, in all three windows** (ADRs 1264, 1275). Every path
after the first opens as a tab behind it once page one is on the screen, and **Ctrl + O** opens
another beside the one showing — a file dialogue in `quorra-gtk` and `quorra-qt`, and in `quorra` a
line over the page where the path is typed, Enter to open and Escape to leave it. Ctrl + Tab moves
between them and Ctrl + W closes one; a tab says the document's own §14.3.3 `/Title` where it
states one, and otherwise the file's name. A document opened behind obeys its Table 29 `/PageMode`
the first time it comes to the front, not while it is behind, and a presentation hides the strip
with the rest of the chrome (ADR 1303).

```sh
target/quorra a.pdf b.pdf c.pdf    # a.pdf in front, the other two as tabs behind it
```

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
field**, which ADR 0201 decided and ADR 0211 made
hold: the field is asked for the key before any of this host's own bindings. The title bar names how many things on
the page could not be drawn and the things themselves are printed. **A click inside a form field aims the keyboard at it**, and so does a Tab walk that lands on one:
characters, Space and Enter go in at the caret, the **left and right arrows, Home and End** move
it, **Backspace and Delete** take out the character on either side of it, and Escape gives the
keyboard back to the page (ADRs 0201 and 0211). The caret is a black line where the next character
will be drawn — the standard states no caret, so its colour and width are this host's, as the focus
ring's are, and a moving caret redraws the window without re-interpreting the page.
**And the caret goes where the click went** (ADR 0225):
`Query::Offset` is the inverse of `Query::Caret`, so a press names the byte it landed nearest rather
than the end of the value. **A drag inside a field selects part of it**, highlighted in the same blue
the page's own selection uses and sending no command at all; **shift with an arrow, Home or End
extends** that selection, and **Ctrl + C, X and V** copy, cut and paste it — into this program's own
clipboard, because the system's belongs to a platform and a native host owns that end. Those three
verbs needed no new message: the characters are a slice of the value the host has already read back,
and the edit is the `Edit::SetField` a keystroke already sends. **While a field has the keyboard it
takes every character key**, `o` and `?` included — both of them reach the field rather than the
sidebar and the About card, because the field is asked before any of this host's own bindings (ADR
0225). A click follows §12.5.6.5's
links and performs the eleven §12.6 actions this program can, and on a markup annotation it **opens the popup window §12.5.6.14 gives it** — a card over the page with Table 172's `/T` in its title bar, Table 166's `/C` behind that and `/Contents` under it, closed again by a second click (ADR 0191) — printing every refusal — including
§12.7.6.4's import, which reads an FDF file **beside the open document** and nowhere else. A
locked document is asked for its password at the terminal (§7.6.4.1), three times, with an empty
line to give up. **`h` marks up what is selected** with §12.5.6.10's highlight and **`k`** with its strikeout, and the mark is written into the file by `s` — §7.5.6's update, with the appearance stream this program draws beside it, so another reader shows the same marks (ADR 0196). **`f` arms the next drag to draw §12.5.6.6's free text annotation**, and what is drawn takes the keyboard at once: the caret is in the box, the arrow keys and Backspace work there exactly as they work in a field, Escape gives the keyboard back to the page, and a later click inside the box picks it up again. That subtype's text *is* the annotation, so an empty one draws nothing and only the caret says it is there (ADR 0238). **`/` opens the find bar**, and while it is open it takes every key: what is typed highlights every occurrence on the page being shown at once, **Enter** goes to the next occurrence *anywhere in the document* and **shift-Enter** to the previous, and **Escape** closes it. The bar says which page the occurrence is on, or counts down the pages still to read — a search reads one page per turn of the event loop, because interpreting all 1023 pages of ISO 32000-2 is 5.84 s and nothing in this program blocks for it (ADR 0250). Annex O's `#search=%22word%22` starts the same search as the document opens, which is what the standard's "selecting the first matching word in the document" asks for. **A signed document says whether its own bytes moved and whether its signature verifies**: for
every signature, who signed it, what its range covers, whether the bytes that range names still hash
to the digest the signature records (ADR 0215), and
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
A build with no bridge (macOS, Windows) says so in its first lines. **`quorra --licences`** prints `/NOTICE` and exits, which is what both
licences covering the compiled-in standard 14 fonts oblige a binary to carry. `--no-sandbox`
decodes JBIG2 and JPEG 2000 in-process — faster by a spawn and a pipe round trip, appropriate for trusted
documents, and it prints what it gave up.

**And there is a second program, with a second toolkit.**

```sh
cargo run --release -p viewer-gtk --bin quorra-gtk -- doc/PDF20_AN001-BPC.pdf
```

`quorra-gtk` is the GTK4 host (ADR 0244, `doc/todo/30`). It takes one document, Annex O's
`#fragment` after it, and `--trace[=launch,frames,events,panel]` in the same line format
`quorra` uses, so the two hosts' launch timelines can be read side by side — `document joined` is
where the first allocation takes in the document opened, and page one interpreted, on a thread
while GTK came up (ADR 1539). It is deliberately a
**separate binary** rather than a flag: the two differ in their toolkit and in nothing else, which
is the claim `viewer-core` exists to make and which one binary linking both would stop making.

What it binds: `Left`/`Right`/`Up`/`Down`/`Page_Up`/`Page_Down` to turn pages, `Home`/`End`,
`+`/`-`/`0` to zoom, `w` to magnify until §12.7's controls fit their rectangles, `a` to select all,
`Escape` to select nothing, `s` to save, `z`/`y` to undo and redo, a drag to select, and the sidebar's three tabs to §12.3.3's outline, §8.11.4.3's layers and
§7.11.4's files — each a real `GtkListView`, with a layer's switch a real `GtkCheckButton` and an
attachment's row writing the file beside the document. §12.7's fields are real controls placed over
the page, and the page underneath them is drawn **without** the
document's own pictures of those fields (ADR 0245) — §6.3.2.2's "unless otherwise instructed", which
this host instructs by default. `--draw-widget-appearances` puts them back, which is what the
standard asks of a processor nobody has instructed and what the two pictures in ADR 0245 were taken
with. **The one thing left to know** is the size: a GTK control has a theme's minimum and a widget's
`/Rect` can be smaller, so on `160F-2019.pdf` all 76 controls are taller than the rectangle they
cover. **`w` is what fixes it** (ADR 0346): `--trace=panel`
prints the magnification at which every control would fit, and the key sends it as
`Zoom::Scale` — no message the vocabulary did not already have. It is offered rather than applied,
because a viewer that magnified a page by itself because a form is on it would be answering a
question nobody asked.

**And there is a third program, with a third toolkit.**

```sh
cargo run --release -p viewer-qt --bin quorra-qt -- doc/PDF20_AN001-BPC.pdf
```

**`w` works here too** (ADR 0436): the `(asked, minimum)` pairs
cross the `cxx` bridge and `viewer_host::ControlFit` — the same arithmetic, the same trace line —
answers for both hosts. The magnifications differ (4.667 here, 3.278 in GTK on `160F-2019.pdf`)
because a control's minimum is its *style's*, which is the point of measuring rather than assuming.

`quorra-qt` is the Qt 6 Widgets host (ADR 0246, `doc/todo/30`). It takes the same arguments
`quorra-gtk` does — one document, Annex O's `#fragment` after it, `--trace[=topics]` in the same
line format, `--draw-widget-appearances` — and binds the same keys, so the two hosts can be run side
by side and differ only in their toolkit. **One flag is its own**: `--quit-after=<ms>` closes the
window by itself, because a window under `Xvfb` has nobody to close it and a test that killed the
process could not tell a clean exit from a crash.

**The reader's three policy words are every window's**, in the same spelling and read by the same
`viewer_host::ReaderWords` — `quorra`, `quorra-gtk`, `quorra-qt` and `quorra-confined` alike
(ADRs 1580, 1581):

```sh
target/release/quorra-gtk --trust-anchors ~/anchors --accept-unknown-revocation signed.pdf
target/release/quorra-qt --reference-files ~/targets drawing.pdf
target/release/quorra --reader-name Ada --reader-organisation Acme --interface-language de layered.pdf
```

`--trust-anchors <dir>` names the certification authorities a signature's path may end at —
§12.8.1's third question, which is asked only once somebody names one (ADR 1039) — and
`--accept-unknown-revocation` acts on a verdict whose revocation the file's own §12.8.4 material
does not settle. `--reference-files <dir>` supplies the documents a §8.10.4 reference `XObject` may
import a page from, matched by §14.4's identifier and never by its path (ADR 1101).
`--reader-name`, `--reader-title`, `--reader-organisation` (each repeatable) and
`--interface-language <tag>` answer §8.11.4.4's `User` and `Language` categories (ADR 1106). Each
is sent before the document, a file in a named directory that does not take is said as a `note:`,
and a word with nothing after it, or a directory that is not one, stops the launch with the reason.

**A document's scripts run at the reader's level**, `--scripts off|ask|warn|on`, read by the same
`ReaderWords` and set again from the third group of every window's restrictions menu (ADR 1616):

```sh
cargo build --release -p pdf-script-worker --features engine --bin pdf-script-worker  # its own run
target/release/quorra-gtk --scripts ask form.pdf
```

`off` is the default (`doc/questions/A193`): nothing runs, no process starts, and every script that
is not one call of the `AF` form library is said as not run. `on` and `warn` run every script in
`pdf-script-worker`, a confined process of its own started at the document's first trigger, and
`warn` says of each run what it made of its field. `ask` puts one question per document at its
first script, with that script's first line; `Go ahead` runs what was withheld — the document's
open scripts and its calculations — and the rest as they come, `Do not` runs none until the
document closes. The worker is built in a cargo run of its own, so that no window links the engine,
and is found beside the window's executable or at `PDF_SCRIPT_WORKER`; without it the first
trigger says how to build it. `quorra-confined` is pinned to `off` and says so when asked for
another level.

**A script that asks — Adobe's `app.alert` and `app.response` — is answered in the window** (ADR
1628): `quorra` puts it on its question card, GTK and Qt on a dialogue of their own, each titled with
the document that asks and the script's own title beside it. An alert carries the buttons the script
asked for — OK; OK and Cancel; Yes and No; Yes, No and Cancel — and in `quorra` each is its first
letter, with Enter for the affirming one and Escape for the one a closed dialogue answers (Cancel,
else No, else OK); a response carries an entry holding the script's default, bullets for a password,
answered by Enter or by OK and cancelled by Escape. Each window prints the question as it puts it and
the answer as it is given, a password's text never. The script waits in its worker, never the
window, and a question left unanswered past the worker's wait — two minutes, or
`PDF_VIEWER_SCRIPT_ANSWER_WAIT_MS` — is answered as a closed dialogue: the card or dialogue comes
down by itself and the window says the question was withdrawn (ADR 1643); `quorra-confined`, pinned
to `off`, refuses one by name. A script's `this.pageNum = n` turns the page as a person's turn
would, `/C` and `/O` included.

**A script's timers fire and its sound is played or refused** (ADR 1702). `app.setInterval` and
`app.setTimeOut` run their expression on the window's own ticks — a `glib` one-shot, the Qt window's
clock timer, winit's `WaitUntil` — armed only while a document holds a timer, so a window whose
documents set none never wakes for one; while a presentation runs, its clock carries the time.
`app.beep` plays the system sound in `quorra-gtk` and `quorra-qt` and prints which of the
reference's five was asked for; `quorra`'s toolkit has no sound, and it says so.

**A widget's and a page's `/AA` scripts run at their events** (ADRs 1750, 1752): a cursor entering
and leaving a widget, a press and a release in it, its taking and losing the focus, and the page
shown changing each hand Table 197's or Table 198's script to the runner, in all three windows — a
press on a toolkit's own entry or list included, which the window reads before the control and
passes on. A script whose edits change nothing a page draws — a `/E` that only logs — leaves the
page as it was interpreted; a script reads the window's Shift and Control as `event.shift` and
`event.modifier`, and its `console.show`, `hide` and `clear` are said by name, every window's console
being its log (ADR 1771). Step 68, 69 and 70 of `tools/drive-windows.sh` drive the three
classes, step 68 with `quorra`'s count of renders a logging `/E` asked for; `--step
script_triggers` runs them alone; step 71 (`--step script_keys`) drives the keys and the console.

**A note's popup window shows its rich text formatted** (ADR 1642): where Table 172's `/RC` states
the same characters as `/Contents`, each run is drawn in its face, weight, posture, size, colour,
underlines, line through, rise and letter spacing, each paragraph in its alignment and list indent
and a right-to-left one in UAX #9's order across its runs, a list item's tag at its start edge, a tab
at the paragraph's stops — Pango spans and a tab array in `quorra-gtk`, a `QTextDocument` built run by
run in `quorra-qt`, `quorra`'s own chrome, which sets a run in the one of §9.6.2.2's families its
`font-family` reaches (ADRs 1654, 1666); chapter 27's font scales are drawn in `quorra` and
`quorra-qt`, a tab's leader in all three (ADRs 1679, 1722), and a right-to-left paragraph's leftward tabs in
all three, the two toolkits handed each stop as its distance from the line's right edge at every
width (ADR 1690) — and whatever the window did not draw of it (in the two toolkit windows a decimal
tab in a right-to-left paragraph; in `quorra-gtk` a font scale) is said under
the note. A text note that states no popup opens a window of its own beside its icon, three inches
by two, where the file's `/Open`, a click or a script's `popupOpen` opens it (ADR 1723).

**A text note is retyped in its window** (ADR 1726): a press on an open window whose text is a
§12.5.6.4 note gives it the keyboard — a text view over the window in `quorra-gtk` and `quorra-qt`,
the window's focus ring and a caret in `quorra`, which a press places in the window's text and the
four arrows, Home and End move, a window drawn from `/RC` as much as a plain one (ADRs 1739, 1770);
the toolkits' text view starts with its caret at the character the press was on and draws a note
from `/RC` in its runs' faces while you type, until a character you type makes the window plain,
saying under the text what of its paragraphs it does not draw (ADR 1782) —
every change is the note's
`/Contents`, an undo takes it back, a save writes it with the producer's icon kept, and Escape gives
the keyboard back to the page. Another markup annotation's window takes no press. A reply that states
no popup of its own is a comment in the window of the note it answers (ADR 1727), in all three, and
`tools/drive-windows.sh` steps 64, 65 and 67 drive them; `quorra-confined` draws no popup window.

**A signature's published policy is fetched at the submissions level** (ADR 1738): where a
§12.8.3.4.4 signature names a URL a copy of its policy document is published at, every window asks
the level `--submissions=` and the restrictions menu set — `send` and `warn` fetch it, `ask` puts the
URL to you first, `refuse` says it was not fetched — and a copy that hashes to the digest the signer
signed is opened beside the document; any other answer is a sentence, and every one says the
policy's constraints were not enforced. `quorra-confined` has no network and fetches nothing. Step
66 of `tools/drive-windows.sh` serves one from the drive's loopback server. A C caller fetches the
copy itself and hands it to `quorra_event_policy_bind`, which says the same sentence (ADR 1753).

**A measured point on a geospatial map is written in the file's display system** (Table 269's
`/DCS`, ADR 1678): beside its latitude and longitude, in degrees where `/DCS` is geographic and as an
easting and a northing in the system's own unit, to two places, where it is projected — in all four
windows, `m` and a press, as `tools/drive-windows.sh` step 58 drives it.

An editable combo box's text — Table 233 bit 19 — is typed into and committed like a single-line
field's in all four windows: Tab out of it or Enter in it runs its `/K` commit form and `/V`.

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
`QPlainTextEdit`, `QCheckBox`, `QRadioButton`, `QComboBox` and `QListWidget` placed over the page. A
push-button is not one of them: the page draws its own appearance, a click on it is the pointer's, and
Tab on the page walks §12.5.1's annotations in the document's order, so Space or Enter presses the one
the walk is on (ADR 1357).

**In both toolkit windows a text field's control shows what the field displays** — `$12.50` where
the field's `AFNumber_Format` formats `12.5` — whenever it does not have the keyboard, and the field's
own characters while it does, so typing starts from the value the format was made of rather than from
characters its keystroke script would refuse. Enter in a single-line field commits it and gives the
keyboard back to the page, which is when the control turns to the displayed string (ADR 1604).

**It needs Qt 6's development files to build** — `qmake6`, `moc` and a C++ compiler — which is what a
native host binding a platform means, and is why it is in none of the three cross-target checks.

**And there is a window whose viewer is not in the window's process at all.**

```sh
cargo build --release -p viewer-confined --bins    # the worker, found beside the executable
cargo run  --release -p viewer-ui --bin quorra-confined -- doc/PDF20_AN001-BPC.pdf
```

`quorra-confined` is the first host on `viewer-confined`'s boundary (ADR 0713): the document,
the interpreter and the rasteriser run in `pdf-view-worker` under seccomp-BPF, Landlock and the
address-space ceiling, and the window never parses a byte of the file. It takes one document and
`--trace[=topics]` in the shared line format, binds arrows, Page Up and Down, Space, Home, End,
`+`/`-`, the wheel and `q` — and **Escape is the abort**: it ends the worker with a kill the
document cannot decline and takes back the window's own drawing thread, which is the marks arm's
draw (ADRs 0241, 0650). It is deliberately the smallest complete host — no panels, no forms, no
selection, no password prompt, each refused by name — and it is *not* one of `doc/todo/30`'s three
level hosts; its scope and the argument are ADR 0713's. Its worker rasterises a page that crosses as pixels on as
many threads as the machine has, the pool built inside the confinement (ADR 1554), and reads the
outline on a thread of its own after an open, so the caption names page one's section once the
next answer arrives (ADR 1553).

**And there is a fourth window, on Windows alone.** `hosts/windows/Quorra.App/` is the WinUI 3
window over the C ABI (RFC 0010, `doc/questions/A501`): CI's `window` job builds it on
`windows-latest` with `viewer_ffi.dll` beside it, and it is not in the snapshot until RFC 0010's
step N4 packages it. On this machine `tools/dotnet-check.sh` compiles its C# against the real SDK
references, with the XAML-generated parts stubbed (ADR 2184).

**CI is the gate on what this machine cannot see** (ADR 0189), because every other gate here runs
on one machine with one libm and one driver. `render-gpu`'s bounded wait takes
`wgpu::PollError::Timeout` from the *slice* as "still working" and loops for the sixty seconds its
constant states, which a CI runner's `lavapipe` needs; and `pdf-render` decides **whether an image
is magnified, how far its grid reduces, whether a miter passed §8.4.3.5's limit and whether a dash
has no length** with `geom::length`, `(dx * dx + dy * dy).sqrt()` — IEEE operations only — rather
than `f32::hypot`, which libm does not promise to round correctly and Miri's deliberate float
non-determinism varies, so those four decisions are the same on every platform, as the two-backend
comparison assumes. Before a push, `tools/platform-check.sh x86_64-pc-windows-msvc` and
`tools/platform-check.sh aarch64-apple-darwin` are the cross-target compile CI runs, over the same
packages (ADR 2204), and `tools/dotnet-check.sh` compiles the WinUI window's C# where a .NET SDK is
present (ADR 2184); `doc/verify.md` has both.

**And a person can get it without a toolchain.** Every
push to `main` whose CI run completes with every job green retags a rolling `snapshot` pre-release
(`snapshot.yml`, ADR 2374), carrying
**every program `tools/batch.sh install` puts in `target/` that the target can build**, with `LICENSE` and `NOTICE` beside
them because both vendored-font licences oblige a *binary* distribution to carry their notices (ADR
0188), and a generated `MANIFEST` saying what each file is. **The archive is flat and stays that
way**: each worker is looked for beside the running binary, and a program that cannot find its
worker refuses the work — `quorra` refuses JBIG2 and JPEG 2000 rather than decoding them in
process. What a given target omits, and why, is ADR 1152's table; the short of it is that
`quorrafs` is Linux's, `pdf-vfs-worker` is absent on Windows because it takes a descriptor over a
Unix socket, and the two native hosts are built where their toolkits are. **The WinUI window is
the fourth archive**, `quorra-winui-x64-<commit>.zip`: a self-contained folder, .NET and the Windows
App SDK inside it, whose `Quorra.App.exe` runs on a double click (ADR 2374).
**Three platforms** — x86_64 Linux, aarch64 macOS,
x86_64 Windows — **and the confinement is Linux's alone.** `pdf-sandbox` compiles where
seccomp-BPF and Landlock do not exist because the project owner asked for the other two
executables, and the argument against it — "a sandbox that silently does nothing on another
platform is worse than no sandbox" — is answered by nothing being silent (ADR 0194).
The **worker process** is there on all three, so a decoder panic still costs one image rather than
the viewer, and so is the request deadline — rebuilt on Windows as a reader thread, because `poll`
is POSIX and on a platform with no address-space ceiling that deadline is the only bound left on a
hostile file's decode. What is missing is named by `Confinement::shortfall`, carried in the worker's
handshake, and printed by `quorra` in its first line. `doc/todo/35` is what a real confinement
for each would take.
