# Where we are — what this program can already do

Status: **standing** — a capability list, not a plan. Every sentence below is something the
program does today.
Read by: a round asking whether the program already does the thing it is about to build, or which
clause a capability came from. **Not read to run a gate or to take a decision**, which is why
`doc/HANDOVER.md` points at it rather than holding it.

**It carries no counts.** `tools/state.sh` prints those; a population named here is named because
a *claim* rests on which population it is (ADR 0405), never as bookkeeping.

`doc/HANDOVER.md`'s reading table is the pointer to this file. What the program still owes is
[`doc/todo/README.md`](todo/README.md), one file per item.

## The state of play

A PDF **viewer**, and every sentence below is a capability rather than a plan.

It **draws** what a page says: geometry, colour, images, shadings — including §8.7.4.3 Table 77's
`/Background`, the wash a shading *pattern* asks for outside its own bounds, which §11.6.7 makes
one painting operation with the shading rather than two — patterns, embedded text,
transparency groups — composited in the blending colour space the page or an isolated group
states, four components as two rasters (ADR 0262), one component as one, `DeviceGray` on the
channel (ADR 0790) and `CalGray` or a one-component profile through its curve (ADR 0792), and
three CIE-based components — `CalRGB`, an RGB profile — as the space's own components through
curves and a grid (ADR 0797) — soft masks, whose `/Luminosity` groups in such a space take
§11.5.3's `Y` as three summed curves where the space decomposes, as a sampled grid where it
does not (ADR 0851), and — where the space has *four* components — off §11.4.7's pair of rasters
inside the mask, over a grid of the press's own four axes (ADR 0857), and annotations both from
stored appearance streams and
constructed where the standard states one — including §12.5.6.4's seven icons, whose artwork is
this processor's own because the clause requires one and draws none, and §12.5.6.15's four and
§12.5.6.16's two, whose clauses only *recommend* one and whose names name objects — and a markup
annotation drawn from **the group it belongs to** rather than from itself, which is §12.5.6.2's
nine shared entries. **Three rasterisers behind one display list**: `render-cpu` is the correctness
oracle — and it is one because it computes a path's coverage of a pixel as the **exact integral of
§8.5.3.3's winding number over §10.7.4's half-open pixel square** rather than sampling it on a
lattice, leaving the library beneath it the axis-aligned rectangle and the one case §11.6.2 forbids
compositing with itself (ADR 1082) — `render-cpu` is also where a **non-isolated group the file
composites under a mode other than Normal is drawn** instead of reported, by performing §11.4.4's
backdrop removal (ADR 1107), and where §10.7.4's substituted width for a mark too thin to measure
begins **one level of 255 below visible rather than one whole device pixel below it**, so between
those two widths every backend draws the shape the document states (ADR 1102); `render-gpu` is Vello and the backend it is compared against — they agree to the channel
over `test-scenes`' fixtures **and over real pages at a real window's resolution**, which is where
they did not (ADR 0127) — and `render-raster` is the third, over the document renderer this project
commissioned (`doc/RENDER_LIBRARY.md`), **what the window actually presents with**, held against the
processor's raster over the whole corpus at the page's own scale and at four times it. The Vello
backend **bands a target the device cannot draw in one pass**, because its working buffers are fixed
constants with no knob and a page of small text at a laptop's resolution can exceed them. JBIG2 and
JPEG 2000 in a confined worker. Encryption at every revision Table 21 lists and every method Table 25 names, in both
directions — including revision 5, whose algorithm is the Adobe extension the table points at
rather than a clause of the standard (ADR 0820). §12.3.2's destinations, §12.3.3's outline, §12.4.2's page labels, §12.5.6.5's links
performing eleven of §12.6's actions, §14.9's accessibility entries, §12.4.4's whole presentation
read **and played** — the Table 164 transition styles whose frame the table's own words determine,
drawn frame by frame, and the rest reported by name for the quantity the clause does not state
(ADR 0230), with §12.4.4.2's states walked inside a page before an arrow key turns it, on the mode
a host states because that clause conditions its whole state machine on one (ADR 0316) — and
everything a document says *about itself*: §14.7's logical structure, §14.8's
tagged-PDF vocabulary, §7.11.4's embedded files, §14.13's associated files in **both** of
§14.13.2's forms — the embedded one listed, and the one that lives outside the document named
out loud when it opens, since following it is refused and naming it never needed a filesystem
(ADR 0918), with §14.13.4's files of the page a reader is *on* listed beside the document's own,
which is where the clause puts them and as far as a panel may reach without walking the page tree
at launch (ADR 1186) — §12.2's viewer
preferences, §12.11's requirements, §7.12's extensions and §14.3.2's XMP.

It is **used**, which is a separate claim from the one above — and
the first sentence of it is **measured** rather than asserted: a gate drags across `pdftotext`'s own word boxes on every corpus document and asks
what came back, which is the first thing in this tree that clicks, and it found a press that set
no selection anchor at all (ADR 0421). A
locked document asks for its password (§7.6.4.1) — **in a window of the host's own in all three**,
where `viewer-ui` stopped reading `stdin` and
stopped leaving the process when there was no terminal (ADR 0545); the page zooms and scrolls; the cursor knows
what it is over and §12.5.5's appearances follow it, as does §12.5.6.19's `/H`; a drag **selects
text**, whose shapes cross to the host as geometry so that it draws them in its own colour, **and
that text can leave the program** — every one of the four consumers puts it on the platform's own
clipboard, in §14.8.2.5's logical content order where the document's structure tree reaches every
byte of the selection and in page content order otherwise, said out loud either way (ADR 0519) —
with **§14.8.2.2.2 taken at its word, that content the structure tree does not include is an
artifact even where nothing tagged it one**, so what a reader takes as text can have the page
furniture subtracted from it and the file's own declaration stays distinguishable from this
reader's inference (ADR 1100) — and §14.7.5.4's parent tree is read as the standard writes it,
its array an indirect object of its own where the producer made it one, and a widget the tree does
not key is still spoken by asking §14.7.5.3's `/OBJR` (ADR 1151);
`/`
**searches the whole document**, one page read per turn of the host's event loop because a
thousand pages of interpretation is not something the launch path may block for, with the readback
kept under a per-document bound so that searching the same document twice does not cost twice
(ADRs 0250, 0256); **a form's data can arrive and leave as a file**: §12.7.7's FDF and §12.7.8's XFDF both read into
the same fully qualified names a field carries, so an import means one thing whichever format came,
and an FDF carrying Table 246's `/EmbeddedFDFs` carries FDF *files*, each read as one and applied
in the array's order, with only the encrypted form refused because only that is deprecated
(ADR 1185) —
XFDF against ISO 19444-1, which ISO 32000-2 names and defines nowhere, with an `<annots>` element
**counted and said out loud rather than read**, because the grammar that would let it create an
annotation is in sections of that standard this tree does not hold (ADR 1108); a person can **fill
in a form field** — where the host keeps the *point* it
clicked and never the text, so §12.7.5.3's truncation is read back rather than predicted (ADR
0201), with a caret that says where the next character goes so that correcting the middle of a
value is not deleting back to it (ADR 0211) — undo it and redo it; a person can **choose an option
in §12.7.5.4's two controls in all three windows**, which is Table 233 bit 19 obeyed in both of the
directions it states rather than in the one that reads as a permission: the flag set is an editable
text box beside a drop-down list — composed in GTK4, which has no widget that is both — and the flag
clear is a drop-down and no way to type into it, which the host drawing its own chrome broke for the
whole of its life (ADR 0596); a click on a markup annotation
**opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about
activation (ADR 0191) — **in all three windows**, where two
of them drew nothing of it at all: the clause gives a popup "no appearance stream", so the window is
furniture rather than ink and each host places its own, over one reading of the two clauses that say
what goes in it (ADR 0613); a **cursor changes over §12.5.6.5's activation region** in all three,
which no clause states and which is therefore recorded as this program's convention; a person can **add an annotation** — §12.5.6.10's four markups over what is
selected (ADR 0196), and §12.5.6.6's free text drawn as a rectangle and typed into, which is the
one markup subtype whose text *is* the annotation and therefore the one whose geometry has to come
from a drag rather than from a selection (ADR 0238) — **and the producer's own free text annotation
can be retyped**, which is §7.5.6's second case rather than a second kind of writing, with Table
167's `LockedContents` asked as a policy and its `Locked` deliberately not, on the table's own
sentence (ADR 0304); a person can **put a file into the document and take one out** — §7.11.4's
embedded file, in either of §7.11.4.1's two homes: the name tree, or §12.5.6.15's annotation on
the page under the point it was dropped on, which draws its icon before anything is saved (ADR
0814) — with §7.9.6's one namespace over both homes, an undo that forgets it and a list that
answers the log rather than the file; and the result can be **saved** — the file it
was opened from, unchanged, with §7.5.6's incremental update appended, which is the one kind of
writing `CLAUDE.md` permits. Every annotation that update writes carries Table 166's `/M` where a
host has said what time it is, and none where none has: the renderer has no clock, so the instant
arrives as `Command::Clock` the way every other fact about the reader's machine does (ADR 1160).
**No window has a gesture for the attach yet**, by the owner's word
that the flows are being reviewed as mockups first; the C ABI has `quorra_attach` and `quorra_detach`,
because an ABI has no gestures.

**What a document asserts over its reader is the reader's to set, in a menu every window has.**
`CLAUDE.md`'s four levels — off, on, ask, warn — stand one per operation and in two scopes: the
window's, which every document it opens inherits, and one document's departure from them, which
ends when that document closes. One of those operations is not a verb a person presses: §12.11.6's
requirements processing, where a document whose unmet requirements pass §12.11.3's penalty
threshold is refused, asked about or warned of *before it is opened at all* — `on` raising no
`Event::Opened`, which is what the clause's "the processing of the document shall not continue"
is, and `off` being the default (ADR 1167). The *ask* level puts its question on a modal window in
all three,
worded once in `viewer_host::restriction`; §12.2's `/HideMenubar` is read, answered in words and
deliberately not obeyed over that menu, because a file that could hide the reader's levels would be
taking away the control over itself (ADR 1145).

**A page goes to paper, and what a printed page shows is not what a screen shows.** §7.6.4.2's
Table 22 bit 3 is asked as an operation at those same four levels, and the grant puts the document
into print intent: §12.5.3's Table 167 bit 3 decides which annotations are drawn — "[i]f clear,
never print the annotation, regardless of whether it is rendered on the screen", with `NoView` not
consulted at all because its own row says the annotation may be printed anyway — §8.11.4.5's
`Print` usage applications run over the reader's own layer switches for the duration and revert
after, and §12.5.6.22's fixed print watermarks are placed against the sheet rather than against the
media box. The window shows the same thing while the operation stands, so what a reader sees is
what would print. `quorra-gtk` spools it through `GtkPrintOperation`, painting the page the
processor backend drew — the same backend the oracle certifies — into the print context; the other
two windows show it and say they have no printer, because `cxx-qt-lib` binds no `QtPrintSupport`
type and the winit host has no toolkit dialogue at all (ADRs 1179, 1180). The resolution is the
printer's, clamped to 150–600 dots per inch with 300 where none is reported, and §12.2's half of
Table 147 — scaling, duplex, tray, page range, copies — is what the dialogue opens on.

**And what a document may ask this machine to do is the reader's to set too, at the same four
levels running the other way.** §12.6.4.8's link is opened by the host and by nothing inside the
confinement, under `--links=refuse|ask|warn|open` in all three windows — `ask` by default, so
nothing reaches another program without a keypress, and only `http`, `https` and `mailto` are
handed over at all, whatever the level, because a document free to name a scheme would be choosing
which of this machine's handlers runs. The words are not the restriction menu's four: there *off*
is the permissive end and here it would be `open`, and one vocabulary for two directions is a trap
with a tick beside it (ADR 1155).

**§10.5's transfer function reaches the screen, and it is applied where §11.7.5.2 says.** The
clause chooses the function at a pixel by the topmost object whose shape there is nonzero, so the
mapping is applied once after compositing rather than per mark: the interpreter records every
elementary mark's shape and function as runs on the display list, in painting order, and both
backends resolve the runs top down over their own read-back, so the rule is stated once and the two
agree by construction; a page stating no function records nothing. A shading's colours and a
tiling's cells still take the function where they are made, reported by name (ADR 1125; ADRs 0479,
0430 for the two). **Colour is its own crate**: `pdf-colour` holds the colour spaces, the ICC
reader, the functions, shadings, meshes and transfer, below the interpreter with no cycle and
re-exported by `pdf-model` under the paths its callers knew (ADR 1131).

**A document is opened on disk and read where its own offsets point** — `startxref` from the
last two kilobytes, each cross-reference section from where the chain names it, each object from
its entry, through a window the parser grows until nothing at its end was examined — so what a
file costs to open is its trailer, its table and page one's objects rather than its length, and a
six-gigabyte document opens in the time a small one does; a damaged file, which a scan reads whole,
costs on disk what it cost in memory (ADR 0809) — and a scan the process cannot hold the file for
is refused by name and said once on the document's report. **The confined viewer opens the same
way**: the file crosses to its worker as an open descriptor beside `Command::Open`, read behind the
filter through `pread64` and nothing else, so the host holds no byte of it and the six-gigabyte
document opens through the confinement too (ADR 0812). A signature's `/ByteRange` is digested
through 64 KiB windows of the file rather than held whole. **Page one goes to the graphics device**, decided
by the project owner and written into `CLAUDE.md`'s startup rules. GPU bring-up is therefore *on* the critical path by choice, which
makes what it costs a number to keep rather than a cost to hide. What each step of that timeline
costs is [`doc/performance.md`](performance.md)'s first section, and the open half is
[todo 42](todo/42-the-launch-path.md).

**And it has chrome, and all three windows have the same six panels**, because the *list* of them is one value every host matches exhaustively on
(`viewer_host::Tab`, ADR 0564), which is what `viewer_host::keys` is for a key press. A sidebar of
six tabs, drawn in `viewer-ui` with `pdf-font`'s compiled-in Helvetica and a `pdf-render` display
list so that every rasteriser draws it, and in the two native hosts with a `GtkNotebook` of
`GtkListView`s and a `QTabWidget` of `QTreeView`s: §12.3.3's outline, where a click
**activates the item** and the document decides whether that is a jump or a URI; §8.11.4.3's
layers, where a switch turns one on unless Table 99's `/Locked` forbids it — and **§8.11.4.4's
`User` and `Language` categories are answered from an audience a *host* supplies and never read off
this machine**, a category with nobody named staying unanswerable rather than falling to the
clause's default (ADR 1106); §7.11.4's embedded
files, where a click writes the file beside the document — as does a click on §12.5.6.15's
paperclip, because §7.11.4.1 gives an embedded file two homes and a file hung on a *page's* own
annotation is in the one the name tree does not list — and where a document stating §12.3.5's
`/Collection` gets its folder tree and the schema's columns instead of a flat list — in Table 153's
`/Sort` order, which is a `shall` about the rows and is resolved once below the three windows
because the values it orders by are §7.11.6's collection items no host holds (ADR 1168) — because a
collection is how a document *arranges* its files rather than a new population of them (ADR 0202);
§14.3.3's `/Info` with §14.3.2's XMP under it; §12.3.4's thumbnails, one row per page with the
miniature fitted above §12.4.2's label and **fetched only for the rows about to be drawn**, which is
`CLAUDE.md` section 2 reaching a panel rather than a preference — Table 29's `/PageMode /UseThumbs`
opens that tab as a document opens, so the whole list was on the launch path until ADR 0564; and §12.4.3's article threads, followed on a click to Table 163's `/R` rather
than to the page the first bead sits on, because activating one composes §12.6.4.7's own thread
action rather than adding a second route (ADR 0200). **Not one *pdf.js* document states a thread**, while four
documents under `doc/corpora/` state one with 115 beads between them, two of them named for the
fact. Which population a claim is about is part of the claim; ADR 0405. `?` puts `/NOTICE` over the page in Courier, **and it does so in all three windows** — the two native hosts ship the same compiled-in standard 14
font programs and had no surface for their licences at all (ADR 0526). **What a key means is one
value, `viewer_host::keys`, that all three hosts translate their toolkit's key into**: three tables
that disagreed about the arrow keys, about `f` and about Escape are one, and each host has a test
that fails when it stops translating the whole of it. **A draw a person is waiting for says so and
offers a key**: a page — or, on the window that draws whole frames, a view — still being drawn
after `viewer_host::drawing::WARN` puts a sentence in the status bar naming Escape, and Escape then
takes the drawing thread back without reporting anything to the core, so the page keeps its picture
and is drawn again when the view moves (ADR 0729). It is the owner's "warn the user and allow the
user to abort, however don't block", and it is a *warning* rather than the deadline ADR 0657
measured and refused. **The
document chooses what opens**: all six of Table 29's `/PageMode` values reach a window in all three
hosts — four name a panel, `UseNone` names none and `FullScreen` is §12.4.4's presentation — and
§12.2's `/DisplayDocTitle` puts the document's own title in the title bar. **A document this program
cannot open, and one whose page tree has no leaves, are two sentences rather than an exit**, in all
three windows (ADR 0564). §12.6.3's trigger events are
raised by the pointer. Four clauses closed on the sidebar without anybody picking them off a list,
and three of the four had a ledger row whose reason was "this program has no ___" — which is the
lesson in [`doc/habits.md`](habits.md)'s ledger section rather than a fact about the sidebar.

**All of it sits behind `viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API.
[`doc/ui-boundary.md`](ui-boundary.md) is the whole story; ADRs 0116 to 0121.

**Seven consumers on that boundary, and not one of them has ever asked for a new message.**
[`doc/ui-boundary.md`](ui-boundary.md) names all seven; the list below is grouped by crate, so the
seventh — `viewer-ui`'s second window, `quorra-confined` — sits inside another bullet rather
than having one. **Three host *crates*, four *windows***, which is what a sentence below counting
three means: `tools/state.sh windows` excludes the fourth for the same reason, saying so in its own
output. That is
the boundary's own evidence, and it is what let the C ABI be frozen — `doc/todo/30` made freezing
conditional on two Rust consumers shaking the API out first. **A *clause* has asked for one, which
is the other direction and is why the sentence is worded about consumers**: §12.4.4.2 conditions its
whole state machine on being in presentation mode, and full screen is chrome, so `Command::Present`
is a statement only a host can make (ADR 0316). `doc/ui-boundary.md` holds the test a message has to
pass.

- **`viewer-ui`'s winit window**, and a **headless test harness**.
- **`viewer-confined`'s `pdf-view-worker`** — a `Viewer` and `render-cpu` behind seccomp-BPF,
  Landlock and an address-space ceiling, with `viewer_confined::Confined` speaking
  `Command`/`Event` and `Query`/`Reply` to it over a pipe and **either the pixels or the marks**
  coming back — chosen per page, in the confined process, by comparing two byte counts. That is
  principle 3's other half, owed since ADR 0014 confined the three image codecs and left the
  document, the interpreter and the rasteriser in process. **One outcome in `viewer-core` had to
  change and nothing else did** — `Rendered::Listed`, ADR 0640, below: rules 2, 3 and 4 already
  forbid that crate a filesystem, a clock and threads it was not
  handed, which is a description of a confined process. Verified by drawing a page byte-identical
  to this process's, and by asking the *kernel* rather than the source whether the worker can open
  a file, open a socket or start a program. **Every `Query` crosses**, including the eleven a
  panel is made of and §12.7's whole form — which is the thing that lets a confined host build
  native controls rather than take a form as pixels — and a hostile document's draw is stoppable,
  because a cancel a hostile document can decline is not one. **A window uses it** — `quorra-confined`, the one host on that
  boundary and not a fourth toolkit: it holds no `Viewer`, no document bytes and no parser, which is
  why `doc/todo/30`'s level-hosts rule does not reach it and why what it owes is decided by the
  operation it can perform rather than by its size (ADR 1190). Both payload arms are on its screen,
  Escape ends the worker and the
  in-flight draw together (ADR 0713), §7.6.4.1's prompt puts the password across into the
  confinement as `Command::Open`'s `Secret` (ADR 0718), `Query::View` and `Command::View` restore
  exactly the view the worker produced when one dies — two messages no established window uses
  (ADR 0737) — **and the graphics device draws its
  pages** (ADR 0725) — the marks as they crossed and the worker's rasters
  wrapped as one-image lists, on a render thread of the window's own, with the interruptible CPU
  thread kept for the frames the device refuses and `--cpu` the window with no device; the same
  round made an unchanged page's `Arc` identity survive the pipe, which every host-side cache was
  silently missing without — while the three established windows stay in process: for
  `viewer-ui` putting the flagship there is a
  change of tier and a decision with a number attached rather than a switch — **and the number
  exists now** (ADR 0597), so what is left is the move rather than the measurement. **The
  tier question is answered too** (ADR 0607): a window on this boundary receives *display lists*
  and not pixels, because a process holding a graphics device cannot be confined at all, and the
  raster payload stays as a per-page fall-back chosen by size. **The codec for that payload
  exists** (ADR 0626) — both sides, `Arc` identity preserved, the two deferred producers refused
  by name into the raster arm, and a fuzz target — **and it is what a frame carries** (ADR 0633):
  `Framed::payload` is the marks or the pixels, the target crosses beside the marks so no host
  rebuilds one, and the decoder refuses a target past what a render request is held to — the one
  length on this boundary with no bytes behind it. **And a page it ships as marks is not drawn at
  all** (ADR 0640): `viewer_core::Rendered::Listed` says *the host took this request's own list*
  about one page rather than about the viewer, so the raster budget stays on inside the
  confinement and `Query::Frame` goes on answering for the pages that must cross as pixels. What
  the cancel then covers on that arm is the interpretation, the drawing having been the host's all
  along — **and that drawing is stoppable too, by a different mechanism with a different name**
  (ADR 0650): `pdf_render::Interrupt` is *raised* and honoured between commands, where a
  `Canceller` *ends a process*, and it works there because on the host's side the loop is this
  tree's own rather than the document's. **And it is raised by a policy** (ADR 0657), which turned
  out to belong to a host that already exists rather than to this boundary: a draw is abandoned
  exactly where finishing it would produce a picture the program has already decided it will never
  show, which is `doc/todo/37`'s own stand-in question asked of a frame not yet drawn. It reads no
  clock, because a deadline separates nothing — a document picks its own cost, and the corpus and
  the amplification fixture are two orders of magnitude apart with legitimate pages on both sides
  of anything between them. **All three windows raise them**
  (ADR 0668): `viewer_host::drawing` gave the two native ones the thread they were missing, and on a
  tier-1 host the same policy has a *provable* form — `viewer_core` drops a `RenderReady` whose token
  is not the one outstanding, so a draw the viewer has stopped holding a token for cannot change a
  pixel however long it runs. The C ABI is the one host still without a way to raise a flag, which is
  an entry point rather than an arrangement (`doc/todo/30`). **A
  document too large for the ceiling is refused by name instead of killing the worker**, on a budget
  the worker derives from the ceiling it was given, and a worker that is killed anyway carries its
  own last line to the host rather than a bare signal number. **And it can be *given* a face** (ADR 0880): a worker that cannot walk
  `/usr/share/fonts` — and is killed rather than told no for trying (ADR 0870) — sends a
  *description* instead, a family and a weight and the characters a script needs, and its broker
  matches, reads and answers. **The allow-list did not move for it and no host can move it**; what
  a host can do is decline, which is the default everywhere. Over `doc/pdf.js` it is 40 pages that
  differed from what this machine draws unconfined and are now byte-identical to it, twelve of them
  blank before. **The allow-list did move one round later, and by one *command* rather than one
  call** (ADR 0888): a worker handed a descriptor has to give it back, and `OwnedFd::drop` asks
  `fcntl(fd, F_GETFD)` first — so that command is permitted on the interpreter profile and every
  other command of the same call still kills, which three probes and a decoder's fourth say rather
  than claim. ADRs 0218, 0223,
  0235, 0241, 0597, 0607, 0626, 0633, 0640, 0650, 0657, 0870, 0880, 0888, 0889; `doc/todo/34`,
  `doc/todo/15`, `doc/todo/59`, `doc/todo/61`.
- **`viewer-gtk`'s `quorra-gtk`**, a real GTK4 application on the same boundary: the panels in
  a `GtkListView` over a `GtkTreeListModel`, §12.7's fields as native widgets placed over the
  page, the selection and §12.5.1's focus ring drawn in the theme's own colour, and the three
  decisions a host owns — §12.7.6.4's file, §7.6.4.1's password, and how much of what the document asserts over its reader this
  window obeys (ADR 0604). `doc/todo/30`'s order made
  GTK4 first because `gtk4-rs` is Rust-safe with no C++ bridge, and the crate keeps
  `#![forbid(unsafe_code)]` to prove it. **Tier 1, because GTK4 admits no other**: a widget has no
  native surface and GSK hands out no device, so `Query::Frame`'s raster becomes a
  `gdk::MemoryTexture` with no conversion at all. What it produced is six things the boundary was
  missing, the largest of them the page drawn *without* its widget appearances — §6.3.2.2's
  "unless otherwise instructed" as `Command::Delegate` — which then exposed the *scale* a form
  host draws at. ADRs 0244, 0245.
- **`viewer-qt`'s `quorra-qt`**, a real Qt 6 Widgets application, and the one that costs a C++
  bridge: **one hand-written `unsafe` token in this crate**, the `unsafe extern "C++"` header `cxx`
  requires, under `#![deny(unsafe_code)]` with one exemption on `mod bridge` and a test asserting
  its position — and asserting that the crates lifting the denial are exactly the ones the
  workspace names, every other crate still *forbidding* it. **The claim is about this crate rather
  than about the tree**: a C ABI writes its entry points as `pub unsafe extern "C" fn` by the
  hundred, which is what a C ABI is, and the crates that lift the denial are read off the test
  rather than counted here — `only_the_three_named_crates_in_the_tree_lift_the_denial`.
  It brought **`crates/viewer-host`**,
  because the second host wanted four of `viewer-gtk`'s modules unchanged — the panel rows, the
  control decision, §12.7.6.4's file policy and the launch timeline named no GTK type. ADR 0246.
  **That crate's newest module is the one that decided the *shape* of a thread for both hosts**:
  Rust never calls a Qt object here, so a finished page cannot be pushed into `QApplication::exec`
  and is *pulled* instead, on a timer whose interval `viewer-host` decides and each toolkit arms —
  which is what `Clock` and the accessibility drain already do, and is why `viewer-gtk` does not use
  the file descriptor GTK would have given it (ADR 0668). **A pull has one moment it cannot be made
  at**, found by the quiet-machine launch A/B: a poll asks the
  toolkit's loop for a turn, and at launch that loop is inside its own first frame — so GTK's page one
  drew in 3.3 ms and waited 61.5 for the timer, and the launch cost 53 ms against 9.5. A host with
  nothing on the screen yet therefore *waits* for page one, out of a one-refresh budget spent once
  over the whole launch, and polls for everything after it (ADR 0678, trap 21).
- **`viewer-ffi`**, a C ABI over the same vocabulary, with a hand-written `include/quorra.h`
  and a `c/open_a_page.c` that a test compiles with `-Wall -Wextra -Werror` and runs. Four shapes
  decide it, each because C takes something away that Rust gave: **commands are functions**,
  because a union's size is part of an ABI and a symbol is not, so a command added later costs a
  compiled caller nothing; **events and answers arrive owned in a batch the caller frees**, so no
  borrow of the viewer crosses and re-entrancy stops being a rule anybody keeps; **a render
  request is an opaque handle** the caller may move to its own thread, because a display list is
  clauses 8 and 9 in a data structure and a frame comes back by copy into the caller's own buffer;
  and **a variant added later is named, described and counted** — `quorra_abi_check` turns "fails to
  compile in every consumer" into "fails to start, once, naming the number that moved", which is
  weaker and is the strongest thing C admits. **How ADR 0346 landed is the shape's own evidence**:
  two thirds of the ABI arrived in one round and `QUORRA_EVENT_KIND_COUNT` did not move, because a
  `Command` is a symbol and only an `Event` is a number. **How much of `Command` and `Query` a C
  caller reaches is counted rather than claimed**: `tools/state.sh hosts` says, and names what it
  does not (ADR 0509). **Every `Query`
  reaches a symbol again**, and the sentence is now held up by a
  test rather than by a round's care: `every_query_reaches_the_abi.rs` matches exhaustively over the
  enum, so a question added to the boundary fails to compile in this crate (ADR 0576). **And
  `tools/state.sh windows` asks the same question of each window** — the instrument for "all three
  hosts stay level" (ADR 0577). **It prints the *reading* beside the count**, one line per unreached variant saying whether it is a
  debt and why, checked in both directions — because a count of what a window does not reach is not
  a list of debts, and two rounds read "eleven queries" off it and walked past a window that could
  not turn a document's restrictions off (ADRs 0603, 0604). ADRs 0247, 0509.

**A password field's value does not reach the file it is saved into.** Table 231 bit 14's own
NOTE is the reason: `save` writes neither the value nor the appearance for such a field, and
reports each one it withheld. ADR 0247.

**And a *program* can ask it questions.** `tools/pdf-retrieve` is JSON on stdout over the readers
this tree already had, and what it adds is the three joins between them `doc/todo/63` named:
§12.3.3's outline turned into the range of pages a section occupies, the text cut at that section's
own two headings, and §12.5.6.10's `/QuadPoints` deciding which *section* an annotation belongs to
rather than which page. **Its default answer is `Interpretation::text` byte for byte**, which a
test asserts: a tool that tidied it would put itself between a caller and the only independent
measurement this project has of its own extraction. ADR 0257.

**And a program can ask it whether a document is an archival one.** `crates/pdf-archive` holds a
document to ISO 19005-2 or ISO 19005-4 — six targets, the levels and flavours being an
applicability column over one requirement table rather than six implementations — and
`quorra-retrieve archive-check` is the front door, JSON on stdout. **A verdict says what it did
not check**, by name, per requirement, per target, with the reason each one is unchecked: a reader
this tree lacks, a standard the project does not hold, or a clause that genuinely decides nothing.
That is `A20`'s requirement and `doc/adr/0923`'s shape, and it is why a coverage line here excludes
the obligations ISO 19005 places on a *processor* rather than on a file — a validator cannot pass
or fail a document for those. Non-conformance is an answer rather than an error, so the exit status
stays zero and the JSON carries the verdict.

Its own reading is compared against the veraPDF corpus clause by clause, and **the comparison is
adjudicated rather than tolerated**: where the corpus and the clause disagree, the clause is read
first and the ruling recorded with its reasoning, because a disagreement is not evidence of
ambiguity by itself. The approved PDF Association errata are an input beside the standard, one of
which *withdraws* a rule from part 4. `tools/state.sh archive` prints where the comparison
stands, and `doc/todo/02` §2 runs it every whole-sequence round (ADR 1015).

**And a separate ledger says how much of the *standard* the viewer implements**, clause by clause, one row per subclause in `doc/conformance/ledger.toml`. Its statuses gained a word in answer to `doc/questions/Q63`: `departed`, for a clause every requirement of which is executed except one sentence decided against with its cost recorded, so a deliberate departure stops wearing `partial`'s word for unfinished work and `tools/state.sh` counts it as its own figure (ADR 1119).

**And it can *make* one.** `pdf-transform`'s `archive` verb brings a document to a stated target in
three stages — validate with `pdf-archive`, decide each failed requirement as a refusal, an
authorised loss or a default, then rewrite through the same structure-preserving serializer `split`
and `merge` use — and **the output is validated again before a byte is written**, so what the report
claims about it is a measurement rather than a promise (ADR 0947). A source that already conforms is
copied rather than rewritten (ADR 1006), each loss the run is willing to take is asked for by name
on that run, and an annotation that cannot stay does not take its marks with it (ADRs 1099, 1105):
**where §12.5.5 fixes where its appearance goes, the marks go back onto the producer's own page**
under that clause's matrix — a `q` prepended as its own stream and §8.4.2's balance closed after
the producer's operators, with the appended page kept for the two cases it refuses, a content
stream that pops further than it pushes and a remaining, unhidden annotation whose `/Rect` the
moved appearance would be drawn over, since `/Annots` order is not a painting order (ADR 1123).
**And content a target will not hold where it was is kept on a page the conversion composes** — the
XMP packet the producer wrote, set verbatim in a face the document itself carries (ADRs 1014, 1025)
— with that page described in the document's own structure tree where the document describes its
content: §14.7.5.2's marked-content sequence per line, §14.7.2's `Part` holding a `P` per line, and
§14.7.5.4's `/StructParents`, parent-tree entry and `/ParentTreeNextKey`, so the page is real
content rather than §14.8.2.2.1's artifact by absence (ADR 1163).
**Behaviour a target forbids is taken out and the behaviour behind it is not.** ISO 19005 forbids
whole action types, an `/AA` on four kinds of holder, and a widget's or field's `/A`; the converter
removes each where a configuration answers the site, names every action that went in the report, and
promotes a removed action's §12.6.2 `/Next` subtree into its place, so the permitted actions behind a
forbidden one still run in the order NOTE 1 states (ADR 1175). **And one syntax repair reaches inside
a content stream**: §7.3.4.3 states the value of a hexadecimal string with an odd digit count, so the
converter writes the final digit the clause already assumed and the string reads the same — the one
repair whose result the standard itself states, with the sibling rule about a byte that is not a
digit still refused because the clause gives it no value to transcribe (ADR 1176).
**An encryption does not cross into an archive, and what it claimed does.** Both parts forbid the
trailer's `/Encrypt` outright, and §7.6.2 makes encryption a property of the file, so the objects the
conversion carries are already decrypted and the output needs no key — an *Ask*, never a default,
because §7.6.4.2's Table 22 flags stop being asserted with it. Every flag the source withheld is
named in the report and written into the output's own `xmpMM:History`: the enforcement cannot
survive and the statement can (ADR 1187). **And one ink defined twice is decided by whoever owns the
archive.** ISO 19005 requires every `Separation` array naming one colourant to state the same
alternate space and tint transform, and §8.6.6.4 makes those what an additive device paints the tint
through — so the operator names which of the file's *own* definitions wins, by object order or by how
many arrays state it, and every byte the rewrite writes is the producer's (ADR 1188).
A refusal is a question answered in advance: a configuration names each refusal site and its
remedy, `--remedy-sites` prints every site a target binds with what this version carries out and its
own total of what is not built yet — and, given a profile, the answers that profile gives which this
version does not carry out and a total of those — and
six shipped profiles under `doc/profiles/` answer them for a purpose each (RFC 0007, ADR 1012).
**It is `quorra-transform archive` and a library verb; no window reaches it.**

**And it applies a redaction.** §12.5.6.23's `/Redact` annotation names a region — `/QuadPoints`
else `/Rect`, and *within* is bounding-box intersection, a documented choice — and `redact`
writes a **new** file, never §7.5.6's update, in which the bytes are gone: a text-showing operator
cut by the placed quad's own advance with no font metrics, held to the interpreter's code count;
an image XObject's samples zeroed in the region, an inline image spliced in the content stream, a
DCT or CCITT image decoded in the confined codec, cleared and re-encoded Flate, a JBIG2 image at
its own one bit. It refuses rather than cuts wrong — a Type 3 font, a composite font not
`Identity-H`, `sh`, a soft mask over the region, a shared image, a painted path or form, a JPX
image — each with its sentence, and the overlay text and fill it does not compose (A65's fence),
said as a departure in the report (ADRs 1124, 1126, 1132, 1133, 1143).

**And a program can ask it for a *file* derived from a document.** `pdf-transform` renders pages
to PNG, PPM or PGM — the last §10.4.2.2's grey of the RGB, through the one place this tree
states the NTSC weights — at a dpi (§8.3.2.3's 72 units to the inch, the oracle backend's own raster byte for
byte, in parallel across pages over one shared font cache) — any of Table 31's five boxes as the
raster's extent and clip, with the table's chained defaults and §14.11.2.1's intersection, and
with or without §12.5.3's annotation pass, **each page's report stating the sub-pixel strip of
raster the page does not reach** so that a caller comparing two rasters of differently shaped pages
can undo the placement instead of searching for it (ADR 0873) — extracts the image `XObject`s a
page reaches and
§8.9.7's inline images at every placement — decoded through the same path the viewer draws them
by, so the three confined codecs stay confined, or under `--native` as the JPEG or JPX file the
stream already is, with a mask that is an image (§8.9.6.3, §11.6.5.2) composited into the PNG's
alpha or written beside the image on its own grid — and lists or saves §7.11.4's embedded files
from all three of their homes, the name tree, the catalog's `/AF` and §12.5.6.15's annotations,
over one plan-and-sinks seam a KIO worker, a FUSE filesystem or a menu item can call the same way
(RFC 0002 §5). **And it writes three things, all by §7.5.6's incremental update** — the
source's bytes intact under it, the same plan the same bytes, through the writer the viewer's own
saves use: `attachments --attach` files a new embedded file in §7.7.4's tree, with Table 45's size
and checksum from the bytes and no date unless one is given; `--attach --to-page N` files it by a
§12.5.6.15 annotation on the page instead, drawn with this tree's own icon; and `--remove NAME`
takes an entry out of the tree and marks the objects it alone reached free, by the one of
§7.5.4's two mechanisms an update can use. **And it writes whole files as well**,
on `pdf_syntax::serialize` — RFC 0002 §10's structure-preserving serializer, admitted by
`CLAUDE.md`'s redrawn authoring exclusion, which the owner ratified on 2026-09-03. It emits
structure and never content: §7.5.2's header, a body of indirect objects, §7.5.4's table or
§7.5.8's stream in the form the sources themselves use, §7.5.5's trailer and §14.4's two
identifiers, with every stream's bytes crossing encoded and untouched and only its `/Length`
re-derived from what was written; a reference to an object the output does not hold becomes
§7.3.10's null and is counted. **And it emits §7.6 where a caller asks for it**: the standard
security handler at `/V` 5 and `/R` 6 with `AESV3`, which is the one configuration Table 20 and
§7.6.4.1 leave undeprecated, with §7.6.4.4.7's, §7.6.4.4.8's and §7.6.4.4.9's algorithms
computing `/U`, `/UE`, `/O`, `/OE` and `/Perms`, §7.6.2's exceptions applied where each object's
identity is known, and every unpredictable byte — the file encryption key, the four salts, the
filler, and §7.6.3.3's initialisation vector per string and stream — supplied from outside, so a
plaintext write stays byte-deterministic and an encrypted one is only as unrepeatable as the
clause requires. It is **asked for and never inherited**, because revision 6 stores each password
as a one-way hash and an opened document yields none to carry: `split`, `merge`, `pages` and
`optimize` warn and write in the clear where nobody supplied passwords, `redact` refuses, and
`quorra-transform` takes both from descriptors rather than from argv (ADRs 1161, 1162).
`split` is the first verb on it — one file per page, per group of
*n*, per comma-separated group of the selection, or **at §12.3.3's outline**, where a piece begins
on every page an item at the stated depth resolves to and the front matter ahead of the first is a
piece of its own. Each piece is the source's own page objects under a new one-level page tree,
§7.7.3.4's four inheritable attributes flattened onto each page because the ancestors that carried
them are not coming along, and the whole object closure carried with them. **What a piece carries
beside its pages is three clauses with three different answers**: §12.3.3's outline is permitted,
so the subset that reaches the piece's pages is written with Table 150's and Table 151's every
conditional entry rebuilt over it; §12.4.2's labels are permitted and the source's *tree* is
forbidden, because "[t]he tree shall include a value for page index 0" and a piece's indices are
its own, so the labels are recomputed one entry per page; and §12.3.2.4's named destinations are
subsetted to those that resolve inside the piece, because a name is not an indirect reference and
§7.3.10's null cannot stand in for one. `/Metadata` and the seven document-level constructs beside
it are still left behind, each named in the report rather than dropped in silence. **`merge` is
the second verb on the serializer, and its substance is the document-level reconciliations rather
than the machinery**: §8.11's optional content groups concatenated with their initial states
rewritten as one default configuration, §7.9.6's name trees merged with a colliding key renamed
and every `/Dest` and `/GoTo` that named it rewritten to match, §12.3.3's outlines spliced into one
chain, §12.4.2's labels written one entry per page so each keeps the label it had, §14.11.5's
output intent pushed onto each source's own pages where the sources disagree — which is the second
home the clause gives it, and which this tree's colour path now reads — and §12.7's interactive
form reconciled entry by entry, with §12.7.4.2's fully qualified field name **refused by name**
where two sources claim it with a different `/FT`, `/V` or `/DV`. A signature crosses without its
`/V`, because §12.8.1's digest was computed over bytes the merged file is not. **`pages` is the
third, and it is the same engine given one document's own page list to edit**: §7.7.3.3's
`/Rotate` written as an integer — absolute where the angle is unsigned, and where it is signed
composed with the value §7.7.3.4 gives the page rather than with what the page states — a
deletion whose every dangling destination becomes §7.3.10's null and whose labels follow their
pages, a reorder that needs no rewriting at all because §12.3.2.2 makes a destination a reference
to a page object, and an insertion that makes a page appear twice as **two page objects** with its
own annotations, since Table 31 gives a page one `/Parent` and Table 172 gives an annotation one
`/P` — refused by name where the page carries a §12.7 widget, whose fully qualified field name is
the field's identity. §12.5.3 was read for it and decides nothing about the file: `/NoRotate` is
the viewer's business, so no annotation's `/Rect` is touched. **All three carry §14.7's logical
structure**: the elements whose content is on a carried page together with the ancestors that hold
them, the content items that name a page the output does not hold pruned and counted, and
§14.7.5.4's parent tree rebuilt with the output's **own** keys — a page's `/StructParents` and an
annotation's or an XObject's `/StructParent` restated to match, `/ParentTreeNextKey` greater than
any of them. The marked-content identifiers inside the carried streams are untouched, because
§14.7.5.2 scopes one to its own content stream and §14.7.5.4 makes it an index into the array its
key names, so carrying the array at its own length moves both ends together. Table 354's three
colliding namespaces get three answers from three clauses: §14.7.3's role map is an *approximate
analogy*, so the first source's wins with a warning; §14.7.6.2 closes the set of things that name a
class, so a collision is renamed and every `/C` follows; Table 355 makes an `/ID` unique and the set
of things that name one is open, so a cross-source collision is **refused by name** (ADRs 0834,
0835). One
page-range grammar for every verb, with §12.4.2's
labels addressable as `@iv`. 

**And a document is a directory.** `pdf-vfs` is RFC 0003's shared core: one declarative table
that says what every path in a document-as-a-folder is, what generates it, and what writing to it
and deleting it would each mean — `pages/0007.pdf` a complete single-page PDF so that `cp` *is*
page extraction, `renders/150dpi/` and `renders/300dpi/` the same pages drawn, `images/0035/` a
page's pictures under the names the extraction itself gave them, `text/0007.txt` and
`text/document.txt` the extraction identity byte for byte, `attachments/` §7.11.4's embedded files
under the names the document files them by, and `meta/` §14.3.3's information dictionary,
§14.3.2's metadata stream and §12.3.3's outline. Six of the eight generators are a
`pdf_transform::Plan` and nothing else, and a test holds a page out of the tree byte for byte
against `pdf-transform`'s own piece, so the core cannot become a second implementation of
anything. The document is allowed to change underneath it: a generation key of (mtime, size,
§7.5.5's last `startxref` offset) is asked before every answer and a change of it throws away the
worker, the inventories and every cached output, while a file already open keeps the generation it
was opened under — so no reader is ever handed a splice of two documents. A `stat` **generates**,
because a kernel clamps reads at the size a `stat` reported and an estimate would truncate the
file. **And it is written to**: all five of RFC 0003 §5.2's verbs work — a PDF copied into
`pages/` inserts its pages at the position the name states and everything after moves down, `rm`
takes a page out, a file copied into `attachments/` is embedded and deleting one removes it, and
`meta/info.json` is overwritable, which is that document's fourth open question answered. §5.3's
four refusals stay refused by design, each with the sentence saying why and each with its own
`errno`. What made the write side a round rather than plumbing is that `pages` and `merge` write a
*new* file while `CLAUDE.md` permits only an append to a document somebody has open, so
`pdf-transform` grew a fourth writer: §7.7.3.2's page tree and §14.3.3's entries edited **in
place** by §7.5.6's update (ADR 0854). A POSIX write is `create`, several `write`s, `flush` and
`close`, so a staged write is visible in the tree and absent from the document until the flush;
abandoning one leaves the file byte for byte as it was; the commit is a temporary file synced and
`rename(2)`d over the original, with the broker checking §7.5.6's own prefix property against the
disk before it writes a byte; a write staged against a generation somebody else has replaced is
`ESTALE` rather than a clobber; and the generation our own commit produces says it is *ours*
rather than looking like somebody editing the file underneath the mount (ADR 0855). **All five of
those verbs are walked over the corpus** (ADR 0860): every
document the core opens is edited five ways, each on its own backing, and each commit is held to
§7.5.6's prefix property read off the file, to the document re-opening at the page count the edit
stated, to the renumbered listing, and to *every surviving page drawing bit-identically to the
page it was* — which, because an insertion moves every ordinal down and a deletion moves every
ordinal up, is a check of "an ordinal is a position" as well as of the writer. It found three
things nothing else could: a page-tree node with no `/Count` counted as zero, so an insertion left
a two-page document reading as one; two documents whose catalog does not reach the tree the edit
was splicing into, where an insertion "before page 1" came back after it; and §7.5.6's own "a
deletion does not destroy bytes" said where a page is deleted and not where an embedded file is.
**And there is a face**: `pdffs <file.pdf> <mountpoint>` mounts a document as a directory on
`fuser`'s pure-Rust path, with no C linkage, no layout knowledge of its own, an inode per *name*
because an ordinal is a position, every refusal logged as a sentence as well as returned as a
number, and RFC 0003 section 5.4's invalidation on a thread of its own (ADR 0861). **And there is
a second face**: `kio/` is a C++ `MODULE` plugin subclassing `KIO::WorkerBase` that serves `pdf:/`,
so Dolphin and every KIO-aware program browse into a document the way they browse into a tar — and
what it forwards over is `pdf-vfs-ffi`, a C ABI of thirty-five functions with the same self-checks
`viewer-ffi` has, because RFC 0003 section 7 records that KF6 admits no Rust worker at all (ADRs
0868, 0869). It is outside the cargo workspace, so a machine with no KDE builds and tests
everything unchanged. What it can do that a mount cannot is show a person **why**: RFC section
5.3's refusals reach a KIO job as the core's own sentence rather than as a category, and a
deletion's §7.5.6 consequence — the bytes stay in the file — arrives as a non-modal warning
instead of a log line nobody reads. **And it can put a *question***: the restriction decision is taken inside the confined generator, which has no channel to
a person by construction, so the question crosses instead — `quorra_vfs_consult` says whether the verb
would be restricted and hands back the sentence, `KIO::WorkerBase::messageBox` puts it, `quorra_vfs_answer`
carries the answer back, and the verb then runs unchanged, once, at the level a yes *is* (ADRs 0874,
0875). Driven by a KIO client on a machine with no session; never yet by Dolphin, and the dialogue
itself therefore by nothing. And **there is a confined worker** under all of it (ADRs 0840, 0841,
0846, 0847): `pdf-vfs-worker` confines itself before it reads a byte, takes the document as the
descriptor a broker sends it, and answers the same questions with the same answers as the
in-process one — question by question, both ways, asserted. It needs no system call the viewer's
worker does not, measured under `strace` rather than assumed, and six probes say it is *killed* for
asking the filesystem anything. The wire under the two *confined* workers — `pdf-view-worker` and `pdf-vfs-worker`, which is not
all three of this tree's workers — is one crate, `confined-transport`. **What a document asserts over its reader is read once, in
`pdf_model::restriction`, for every operation this tree performs**: every Table 22 bit is named
(one as consumed by nothing, saying why), the six operations — a field filled, an
annotation added, a page printed or rendered, a file extracted, a file written in, a document
assembled out of another's pages — each read their bit at
the document's revision and §12.8.2.2's certification besides — and §12.7.5.5's Table 236 `/P`,
the permission a signed signature field's lock states over the document, whose several instances
compose as the minimum its own words make them (ADR 1156) — and the four levels are one type
whose verdict a caller matches exhaustively. `pdf-transform` honours all four (`--restrictions`
takes `off`, the default, `on`, `warn` and `ask`,
which puts the question on the terminal where there is one and is a refusal saying nobody could
answer where there is not), **RFC 0003's mount honours all four for the same reason a pipe does** —
a file system has no dialogue, so its *ask* is a refusal with its own sentence and never a silent
proceed, and both it and *on* leave as `EACCES` — and **the viewer supplies all four**:
*refuse* is `Event::Refused`, *warn* is the edit done and `Event::Warned` after the `Dirty` it
caused, and *ask* is `Event::Asking` with the edit held until `Command::Answer` settles it — the
`Event::PasswordRequired` shape, and the condition `doc/todo/38` set for shipping a level at all.
All three windows put the question on a modal window (ADR 1145), a C host of `viewer-ffi` through
`QUORRA_EVENT_KIND_ASKING` and `quorra_answer`, and `quorra-confined`, which performs no restricted
operation, says out loud that nobody could answer rather than letting *ask* behave like *on*
(ADR 0814). The suite has its own gate, with
RFC 0002 §12's perf floor and its inventories held to the document's own structure, and three
corpus walks beside it: the writer's, `split`'s — every corpus document's first page taken
out, re-read, and drawn against the source page bit for bit — `merge`'s, which puts every
corpus document's first page beside a fixed second document and checks each reconciliation against
what the source stated as well as the two rasters, and `pages`'s, which rotates and deletes within
one document and holds every surviving page to its source page under the rotation stated. ADRs 0800,
0801, 0802, 0803, 0804, 0816, 0817, 0818, 0821, 0830.

**It can draw a page one document imports from another, where the person running it names the file
it comes from.** §8.10.4's reference `XObject` is a form carrying Table 95's `/Ref`, and §8.10.4.1
writes a `shall` for a processor that draws the referenced page and a `shall` for one that draws the
proxy the producer put there instead; which of the two this is depends on whether the target file is
in front of it, and with no filesystem it never is unless somebody puts it there. `--reference-files
<dir>` names a directory, `viewer_core::Command::References` carries the bytes in, and **which file
a reference names is decided by §14.4's identifier and never by the path Table 95 states** — a path
a document writes would let the file choose what this machine opens. A supplied file whose
identifier does not match is refused by name; one whose *changing* identifier moved is drawn with
the clause's own warning that it is "a different version of the correct PDF file"; and a reader who
names no directory gets every proxy drawn and is told nothing, which is what the clause asks of a
reader with no target file. The imported page is placed by the proxy's `/Matrix`, clipped to its
`/BBox`, and rendered with §8.10.4.3's annotation appearances inside that same box. ADR 1101.

**It can tell a person that a signed document changed after it was signed, whether its signature
verifies, and — where the person running it names a certification authority — whether the signature
is valid.** §12.8.1 divides verifying a signature into three questions, and the third is a host's
input rather than a list this program ships: `--trust-anchors <dir>` reads PEM or DER,
`viewer_core::Command::Trust` carries it in, and `pdf_signature::verdict::Verdict` is the only place
the word *valid* is reachable — its `Valid` has no public constructor and the only one that makes it
takes a proof no caller can build without a path that reached a supplied anchor. **A reader who
names nobody is told that nobody was named**, which is a statement about this program rather than
about any document (ADRs 1039, 1076). `Signature::integrity`
recomputes the digest over §12.8.1's `/ByteRange` — with the algorithms Table 260 and Table 256
name — and compares it with what `pdf_signature::cms` reads out of §12.8.3.3's `SignedData`, over a
bounded in-tree X.690 reader that allocates nothing (ADR 0215). `Signature::authenticity` then
finds the certificate the `SignerInfo` names among the ones the signature itself carries, reads its
key with `pdf_signature::x509` and verifies with `pdf_signature::pkcs1`, `pdf_signature::pss`,
`pdf_signature::dsa`, `pdf_signature::ecdsa` or `pdf_signature::eddsa` — RFC 8017's RSASSA-PKCS1-v1_5 and
RSASSA-PSS, the latter over the `RSASSA-PSS-params` the signature's own algorithm identifier
carries; FIPS 186-4's DSA; ANSI X9.62's ECDSA over RFC 5753's `ECDSA-Sig-Value`; and RFC 8032's
Ed25519, which signs the message rather than a digest of it. **That is all three of Table 260's
algorithm families and the fourth row ISO/TS 32002 section 5.1.2 adds beside them** (ADRs 0229,
0314, 0322, 0532). The constructions, budgets, encodings and refusal names are this tree's; the
modular arithmetic and the group law under them are RustCrypto's `crypto-bigint` and curve
packages, by owner decision (ADR 0331). **What is still refused is a *curve* rather than a family,
and each is named at runtime by the identifier the certificate states**: of ISO/TS 32002 Table 3's
six, brainpoolP512r1 alone, which has no package of reviewed arithmetic at all (ADR 1063); and of
its Table 4's two, Ed448, whose stable package carries the field arithmetic without the signature
scheme. The sentences the program uses keep every
asymmetry: a mismatch is decisive, a match is the absence of one kind of evidence, and a
certificate that arrived in the same file as the signature it verifies proves the two are
consistent with each other and nothing about who made either. **Without an anchor nothing here says
a signature is valid**, and with one the sentence that does says whose anchors made it sayable. **And §12.8.2.2's second question — what changed after the signature — is answered without
mutating anything**, because §7.5.6 makes a signed revision a byte prefix of the file: the prefix
is opened as a second immutable `Document` and the two are diffed object by object, each change
ranked by which *entries* the update wrote rather than by what the object is (ADRs 1043, 1049).
**All three of §12.8.2's transform methods are read, scoped by their own parameters, and ranked on
that comparison** — `/DocMDP` against Table 257's `/P`, `FieldMDP` against Table 259's `/Action` and
`/Fields`, and `UR` against Table 258's rights, which are a *grant* rather than a restriction, so a
save that outgrows what the signature granted withdraws the `/UR3` entry rather than leaving a grant
the new bytes do not support — and all of it reaches a reader rather than only the crate (ADRs 0159,
1096, 1104). **Where the file marks a part this program
still does not do**, it says that too: Table 255's `/V 1` states that "the Reference dictionary
shall be considered critical to the validation of the signature", so a `/Reference` naming any
*other* transform method is named in the note beside the questions that were answered (ADR 0637).

**Two more of §12.8's questions are answered from the file rather than from a network.**
`pdf_signature::revocation` reads §12.8.4's document security store and §12.8.3.3.2's archival
attribute, applies them to every certificate on the path, and **absence of evidence is *unknown*
and never *good*** — a host willing to accept an unknown status says so in the open rather than
having it assumed (ADR 1067). §12.8.5's document timestamps are *established* rather than read:
the token has to parse, be covered by the signer's own digest attribute, verify, and reach a
supplied anchor, with each token's path validated at the next one's stated time, and what a token
merely claims is reported as a claim (ADR 1071).

**And it speaks a page.** `viewer-accessibility` maps §14.8.4's standard structure types onto
`accesskit::Role`, and `accesskit_unix` puts the result on AT-SPI — where a real client walks it
off the bus, `Frame` → `DocumentFrame` → the page named by §12.4.2's own label → §14.7's elements,
with §14.9.3's `/Alt` where the document states one, a table cell announced with **the headers that
describe it** — Table 384's `/Headers` where a producer wrote one and §14.8.4.8.3's own search where
none did, each header said in the author's own short form where it states Table 384's `/Short` — a
table described by its stated `/Summary` (ADR 0715), a `TH` carrying the axis §14.8.5.7 gives it
rather than a guess, a list that §14.8.5.5 says **carries an earlier one on** saying so and
pointing at it with AT-SPI's `FlowTo` where the earlier one is on the same page (ADR 0748), an
element placed by
§14.8.3.3's content rectangle — everything its own content drew, then what the document says
about it, then what it encloses, which is what places a table cell whose only content is a widget
(ADR 0768) — and a `StatusBar` group carrying **what the
page could not draw**, because the person who cannot see the page is the one for whom a count in
the title bar is no answer. An untagged page says that it is one rather than being given an
invented reading order. **And a client may now *act* rather than only listen**: a check box says a
click may be asked of it and a person using a screen reader alone can tick one, an element says it
may be scrolled to, and the page says a caret may be put in it — each carried out as a place, in the
device pixels a pointer already works in, so the boundary gained no message and one definition of a
click serves the mouse and the bus alike (ADR 0425) — **in all three of this project's windows, and
one definition for the three of them** (ADRs 0623, 0630): a click on §12.7.5.2's check box or radio
button is decided once, by `viewer_host::form::Clicked`, so a person using a screen reader ticks the
same boxes and is refused the same read-only ones whichever window they opened the file in. The one
async runtime this tree has is confined
to that crate, it is Linux-only in its own manifest, and the adapter is created **after** the first
frame is presented. ADR 0214.
