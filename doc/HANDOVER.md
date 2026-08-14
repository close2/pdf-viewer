# Handover

Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed exclusion list.
**Principle 5 is the one that changes how you work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right.

**This file is the state of play and the traps, and it carries no numbers.** `tools/state.sh`
prints those. That is deliberate rather than tidy: a round told to *measure* something must not be
able to read the answer in a document, because a table of gate figures is exactly what lets a
round write "unchanged" without running anything. ADR 0281.

**A lesson lives here exactly once**: in a trap if it changes how you write code, in
[`doc/habits.md`](habits.md) if it changes how you work. A session's narrative belongs in its ADR
and in [`doc/history/`](history/README.md), nowhere else. This file has been halved four times; if you
find yourself retelling a session here, you are undoing that (ADRs 0232, 0281).

**And the round's own record is one *new file*, never an edit to an existing one.** Write
`doc/history/<session>-<slug>.md` — a file that did not exist before, named so that `ls` sorts it
last — and write nothing about the round anywhere else. A round does **not** append to
`doc/history.md`, which holds sessions 5 to 445 and is closed; it does not extend a table here, in
[`doc/todo/README.md`](todo/README.md) or in [`doc/todo/02-every-round.md`](todo/02-every-round.md);
and it does not touch a neighbouring session's file. One round, one added file, and
[`doc/history/README.md`](history/README.md) says what goes in it.

## Which file this round needs

Each of these is *all* of what it holds, not a précis. Open the one your round is about — that is
what these files are split by.

**Every round, whatever it is about:**

| | |
|---|---|
| [`doc/todo/README.md`](todo/README.md) | the index of owed work, one file per item, `ls` sorting by priority |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) | what a round does around whatever it takes: the gates, the sweeps, the binaries, the commit |
| [`tools/state.sh`](../tools/state.sh) | what the numbers are today — `quick` in seconds, the whole thing in minutes |
| [`doc/environment.md`](environment.md) | the machine, the agent's account, the display, the build directory, the working agreements, and the one command a fresh clone needs |
| the traps below | each is a mistake somebody actually made |

**And then by what the round is:**

| a round that | opens |
|---|---|
| reads a clause, or writes a ledger row | [`doc/habits.md`](habits.md) *Reading the specification* and *The ledger*, [`doc/ledger-and-claims.md`](ledger-and-claims.md), [`doc/errata-read.md`](errata-read.md), [`doc/todo/01`](todo/01-ledger-partial-rows.md) |
| judges a page against other renderers | [`doc/habits.md`](habits.md) *Judging against other implementations*, [`doc/oracle-and-corpus.md`](oracle-and-corpus.md), [`doc/todo/00`](todo/00-ambiguous-bucket.md) |
| **measures** anything | [`doc/habits.md`](habits.md) *Measuring*, [`doc/performance.md`](performance.md), [`doc/verify.md`](verify.md) — and `tools/state.sh`, because the number has to be printed rather than quoted |
| writes a host, or adds a message | [`doc/ui-boundary.md`](ui-boundary.md), [`doc/todo/30`](todo/30-a-native-host.md)–[`33`](todo/33-annotation-editing.md) |
| adds or questions a dependency | [`doc/stack.md`](stack.md), [`doc/third-party-data.md`](third-party-data.md), [`doc/PLAN.md`](PLAN.md) §1 |
| runs the program | [`doc/running-the-viewer.md`](running-the-viewer.md), [`doc/environment.md`](environment.md) |
| runs an instrument that is not a §2 gate | [`doc/verify.md`](verify.md) — `deny`, the fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI |
| looks for where something lives | [`doc/crate-map.md`](crate-map.md), [`doc/PLAN.md`](PLAN.md) |
| asks *when* something landed | [`doc/history/`](history/README.md), one file per round from 446 on, and [`doc/history.md`](history.md) for the rows before it — that is the only place session bookkeeping goes |

---

## Where we are

A PDF **viewer**, and every sentence below is a capability rather than a plan.

It **draws** what a page says: geometry, colour, images, shadings, patterns, embedded text,
transparency groups, soft masks, and annotations both from stored appearance streams and
constructed where the standard states one — including §12.5.6.4's seven icons, whose artwork is
this processor's own because the clause requires one and draws none, and §12.5.6.15's four and
§12.5.6.16's two, whose clauses only *recommend* one and whose names name objects — and a markup
annotation drawn from **the group it belongs to** rather than from itself, which is §12.5.6.2's
nine shared entries. **Three rasterisers behind one display list**: `render-cpu` is the correctness
oracle, `render-gpu` is Vello and the backend it is compared against — they agree to the channel
over `test-scenes`' fixtures **and over real pages at a real window's resolution**, which is where
they did not (ADR 0127) — and `render-quorra` is the third, over the document renderer this project
commissioned (`doc/RENDER_LIBRARY.md`), **what the window actually presents with**, held against the
processor's raster over the whole corpus at the page's own scale and at four times it. The Vello
backend **bands a target the device cannot draw in one pass**, because its working buffers are fixed
constants with no knob and a page of small text at a laptop's resolution can exceed them. JBIG2 and
JPEG 2000 in a confined worker. Encryption at every revision and method §7.6 states, in both
directions. §12.3.2's destinations, §12.3.3's outline, §12.4.2's page labels, §12.5.6.5's links
performing eleven of §12.6's actions, §14.9's accessibility entries, §12.4.4's whole presentation
read **and played** — the Table 164 transition styles whose frame the table's own words determine,
drawn frame by frame, and the rest reported by name for the quantity the clause does not state
(ADR 0230), with §12.4.4.2's states walked inside a page before an arrow key turns it, on the mode
a host states because that clause conditions its whole state machine on one (ADR 0316) — and
everything a document says *about itself*: §14.7's logical structure, §14.8's
tagged-PDF vocabulary, §7.11.4's embedded files, §14.13's associated files, §12.2's viewer
preferences, §12.11's requirements, §7.12's extensions and §14.3.2's XMP.

It is **used**, which is a separate claim from the one above and was owed for a long time. A
locked document asks for its password (§7.6.4.1); the page zooms and scrolls; the cursor knows
what it is over and §12.5.5's appearances follow it, as does §12.5.6.19's `/H`; a drag **selects
text**, whose shapes cross to the host as geometry so that it draws them in its own colour; `/`
**searches the whole document**, one page read per turn of the host's event loop because a
thousand pages of interpretation is not something the launch path may block for, with the readback
kept under a per-document bound so that searching the same document twice does not cost twice
(ADRs 0250, 0256); a person can **fill in a form field** — where the host keeps the *point* it
clicked and never the text, so §12.7.5.3's truncation is read back rather than predicted (ADR
0201), with a caret that says where the next character goes so that correcting the middle of a
value is not deleting back to it (ADR 0211) — undo it and redo it; a click on a markup annotation
**opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about
activation (ADR 0191); a person can **add an annotation** — §12.5.6.10's four markups over what is
selected (ADR 0196), and §12.5.6.6's free text drawn as a rectangle and typed into, which is the
one markup subtype whose text *is* the annotation and therefore the one whose geometry has to come
from a drag rather than from a selection (ADR 0238) — **and the producer's own free text annotation
can be retyped**, which is §7.5.6's second case rather than a second kind of writing, with Table
167's `LockedContents` asked as a policy and its `Locked` deliberately not, on the table's own
sentence (ADR 0304); and the result can be **saved** — the file it
was opened from, unchanged, with §7.5.6's incremental update appended, which is the one kind of
writing `CLAUDE.md` permits.

**Page one goes to the graphics device**, decided by the project owner and written into
`CLAUDE.md`'s startup rules. GPU bring-up is therefore *on* the critical path by choice, which
makes what it costs a number to keep rather than a cost to hide. What each step of that timeline
costs is [`doc/performance.md`](performance.md)'s first section, and the open half is
[todo 42](todo/42-the-launch-path.md).

**And it has chrome.** A sidebar of six tabs, drawn with `pdf-font`'s compiled-in Helvetica and a
`pdf-render` display list so that both backends draw it: §12.3.3's outline, where a click
**activates the item** and the document decides whether that is a jump or a URI; §8.11.4.3's
layers, where a switch turns one on unless Table 99's `/Locked` forbids it; §7.11.4's embedded
files, where a click writes the file beside the document — as does a click on §12.5.6.15's
paperclip, because §7.11.4.1 gives an embedded file two homes and a file hung on a *page's* own
annotation is in the one the name tree does not list — and where a document stating §12.3.5's
`/Collection` gets its folder tree and the schema's columns instead of a flat list, because a
collection is how a document *arranges* its files rather than a new population of them (ADR 0202);
§14.3.3's `/Info`; §12.3.4's thumbnails, one row per page with the miniature fitted above
§12.4.2's label; and §12.4.3's article threads, followed on a click to Table 163's `/R` rather
than to the page the first bead sits on, because activating one composes §12.6.4.7's own thread
action rather than adding a second route (ADR 0200). Not one corpus document states a thread,
which is said out loud rather than around. `?` puts `/NOTICE` over the page in Courier. **The
document chooses what opens**: Table 29's `/PageMode` names a panel and §12.2's
`/DisplayDocTitle` puts the document's own title in the title bar. §12.6.3's trigger events are
raised by the pointer. Four clauses closed on the sidebar without anybody picking them off a list,
and three of the four had a ledger row whose reason was "this program has no ___" — which is the
lesson in [`doc/habits.md`](habits.md)'s ledger section rather than a fact about the sidebar.

**All of it sits behind `viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API.
[`doc/ui-boundary.md`](ui-boundary.md) is the whole story; ADRs 0116 to 0121.

**Six consumers on that boundary, and not one of them has ever asked for a new message.** That is
the boundary's own evidence, and it is what let the C ABI be frozen — `doc/todo/30` made freezing
conditional on two Rust consumers shaking the API out first. **A *clause* has asked for one, which
is the other direction and is why the sentence is worded about consumers**: §12.4.4.2 conditions its
whole state machine on being in presentation mode, and full screen is chrome, so `Command::Present`
is a statement only a host can make (ADR 0316). `doc/ui-boundary.md` holds the test a message has to
pass.

- **`viewer-ui`'s winit window**, and a **headless test harness**.
- **`viewer-confined`'s `pdf-view-worker`** — a `Viewer` and `render-cpu` behind seccomp-BPF,
  Landlock and an address-space ceiling, with `viewer_confined::Confined` speaking
  `Command`/`Event` and `Query`/`Reply` to it over a pipe and the **pixels** coming back, because
  the confined process owns the rasteriser and the display list therefore never leaves. That is
  principle 3's other half, owed since ADR 0014 confined the three image codecs and left the
  document, the interpreter and the rasteriser in process. **Nothing in `viewer-core` had to
  change**: rules 2, 3 and 4 already forbid it a filesystem, a clock and threads it was not
  handed, which is a description of a confined process. Verified by drawing a page byte-identical
  to this process's, and by asking the *kernel* rather than the source whether the worker can open
  a file, open a socket or start a program. **Every `Query` crosses**, including the eleven a
  panel is made of and §12.7's whole form — which is the thing that lets a confined host build
  native controls rather than take a form as pixels — and a hostile document's draw is stoppable,
  because a cancel a hostile document can decline is not one. **The window does not use it**,
  deliberately: `viewer-ui` is a tier-2 host and this boundary is tier 1, so putting it there is a
  change of tier and a decision with a number attached rather than a switch. ADRs 0218, 0223,
  0235, 0241; `doc/todo/34`.
- **`viewer-gtk`'s `pdf-viewer-gtk`**, a real GTK4 application on the same boundary: the panels in
  a `GtkListView` over a `GtkTreeListModel`, §12.7's fields as native widgets placed over the
  page, the selection and §12.5.1's focus ring drawn in the theme's own colour, and the two
  decisions a host owns — §12.7.6.4's file and §7.6.4.1's password. `doc/todo/30`'s order made
  GTK4 first because `gtk4-rs` is Rust-safe with no C++ bridge, and the crate keeps
  `#![forbid(unsafe_code)]` to prove it. **Tier 1, because GTK4 admits no other**: a widget has no
  native surface and GSK hands out no device, so `Query::Frame`'s raster becomes a
  `gdk::MemoryTexture` with no conversion at all. What it produced is six things the boundary was
  missing, the largest of them the page drawn *without* its widget appearances — §6.3.2.2's
  "unless otherwise instructed" as `Command::Delegate` — which then exposed the *scale* a form
  host draws at. ADRs 0244, 0245.
- **`viewer-qt`'s `pdf-viewer-qt`**, a real Qt 6 Widgets application, and the one that costs a C++
  bridge: **one hand-written `unsafe` token in the tree**, the `unsafe extern "C++"` header `cxx`
  requires, under `#![deny(unsafe_code)]` with one exemption on `mod bridge` and a test asserting
  its position and that no other crate lifts the denial. It brought **`crates/viewer-host`**,
  because the second host wanted four of `viewer-gtk`'s modules unchanged — the panel rows, the
  control decision, §12.7.6.4's file policy and the launch timeline named no GTK type. ADR 0246.
- **`viewer-ffi`**, a C ABI over the same vocabulary, with a hand-written `include/pdf_viewer.h`
  and a `c/open_a_page.c` that a test compiles with `-Wall -Wextra -Werror` and runs. Four shapes
  decide it, each because C takes something away that Rust gave: **commands are functions**,
  because a union's size is part of an ABI and a symbol is not, so a command added later costs a
  compiled caller nothing; **events and answers arrive owned in a batch the caller frees**, so no
  borrow of the viewer crosses and re-entrancy stops being a rule anybody keeps; **a render
  request is an opaque handle** the caller may move to its own thread, because a display list is
  clauses 8 and 9 in a data structure and a frame comes back by copy into the caller's own buffer;
  and **a variant added later is named, described and counted** — `pdfv_abi_check` turns "fails to
  compile in every consumer" into "fails to start, once, naming the number that moved", which is
  weaker and is the strongest thing C admits. ADR 0247.

**The freeze's three amendments came first and one of them was a bug.** `Answer::Field`'s password
value was supposed to be a sentence in a doc comment; reading Table 231 bit 14 for it found a
second sentence nobody had read, and **this program wrote a person's typed password into the file
it saved** — against that table's own NOTE — and no longer does: `save` writes neither the value
nor the appearance for such a field and reports each one it withheld. ADR 0247.

**And a *program* can ask it questions.** `tools/pdf-retrieve` is JSON on stdout over the readers
this tree already had, and what it adds is the three joins between them `doc/todo/36` named:
§12.3.3's outline turned into the range of pages a section occupies, the text cut at that section's
own two headings, and §12.5.6.10's `/QuadPoints` deciding which *section* an annotation belongs to
rather than which page. **Its default answer is `Interpretation::text` byte for byte**, which a
test asserts: a tool that tidied it would put itself between a caller and the only independent
measurement this project has of its own extraction. ADR 0257.

**It can tell a person that a signed document changed after it was signed, and whether its
signature verifies.** §12.8.1 divides verifying a signature into three questions and only the
third needs the trust store the whole clause had been refused for. `Signature::integrity`
recomputes the digest over §12.8.1's `/ByteRange` — with the algorithms Table 260 and Table 256
name — and compares it with what `pdf_model::cms` reads out of §12.8.3.3's `SignedData`, over a
bounded in-tree X.690 reader that allocates nothing (ADR 0215). `Signature::authenticity` then
finds the certificate the `SignerInfo` names among the ones the signature itself carries, reads its
key with `pdf_model::x509` and verifies with `pdf_model::pkcs1`, `pdf_model::pss` or
`pdf_model::dsa` — RFC 8017's RSASSA-PKCS1-v1_5 and RSASSA-PSS, the latter over the
`RSASSA-PSS-params` the signature's own algorithm identifier carries, and FIPS 186-4's DSA: two of
Table 260's three algorithm families, the RSA one under both of its paddings. The constructions,
budgets and refusal names are this tree's; the modular arithmetic under them is RustCrypto's
`crypto-bigint`, by owner decision (ADRs 0229, 0314, 0322, 0331). The third is elliptic-curve, and it is **refused with an
argument rather than half-written**: ISO/TS 32002 makes it eight curves across two group laws whose
domain parameters are in no document this tree holds, and every one of them is named at runtime by
the identifier the file states. The sentences the program uses keep every
asymmetry: a mismatch is decisive, a match is the absence of one kind of evidence, and a
certificate that arrived in the same file as the signature it verifies proves the two are
consistent with each other and nothing about who made either. **Nothing here says a signature is
valid.**

**And it speaks a page.** `viewer-accessibility` maps §14.8.4's standard structure types onto
`accesskit::Role`, and `accesskit_unix` puts the result on AT-SPI — where a real client walks it
off the bus, `Frame` → `DocumentFrame` → the page named by §12.4.2's own label → §14.7's elements,
with §14.9.3's `/Alt` where the document states one, a table cell announced with **the headers that
describe it** — Table 384's `/Headers` where a producer wrote one and §14.8.4.8.3's own search where
none did — a `TH` carrying the axis §14.8.5.7 gives it rather than a guess, an element placed by
Table 379's `/BBox` where its content marked no text, and a `StatusBar` group carrying **what the
page could not draw**, because the person who cannot see the page is the one for whom a count in
the title bar is no answer. An untagged page says that it is one rather than being given an
invented reading order. The one async runtime this tree has is confined to that crate, it is
Linux-only in its own manifest, and the adapter is created **after** the first frame is presented.
ADR 0214.

---

## The numbers, today

**They are not in this file.** `tools/state.sh` prints them; this section is only how to read what
it prints.

```sh
tools/state.sh quick      # ledger, conformance, Annex O, populations, binaries, disk — seconds
tools/state.sh            # all of that, plus every gate doc/todo/02 §2 runs — minutes
tools/state.sh oracle     # one section; `--list` names them
```

- **Counts are ratcheted**: they may only improve, except where a rise is a *new report* and is
  written down as one (trap 5).
- **A gate's own number, never arithmetic beside it.** This project has twice carried a sum that
  was stale while the gate figure two lines above it was current, which is why `state.sh` filters
  a gate's output and adds nothing up. If you need a total, print it.
- **`silent` is the ledger status worth hunting**: it means a requirement this program fails
  without saying so. Every other status names what it owes, and all eight are defined at the top
  of `doc/conformance/ledger.toml`. How a row goes wrong is
  [`doc/ledger-and-claims.md`](ledger-and-claims.md); the sweeps that catch it are
  [todo 01](todo/01-ledger-partial-rows.md) and running them is
  [todo 02](todo/02-every-round.md) §4.
- **What is not implemented has a file.** Every one of them is *reported* at runtime rather than
  silently skipped, and [`doc/todo/README.md`](todo/README.md) is the index — the corpus
  witnesses, the clause and what it would cost live with the item.
- **Closed by decision rather than by work**, recorded in the ledger and owed to nobody:
  `/ColorTransform` (Table 13, whose one corpus witness contradicts the clause), a stream whose
  data is in an external file (§7.3.8.1 — the renderer has no filesystem, principle 3),
  §12.7.6.2's submit and §12.6.4's remote, launch, sound and movie actions (a network, a second
  file, a media engine), a filled degenerate subpath's device pixel (§8.5.3.3.1, which the clause
  itself calls "device-dependent and not generally useful"), grid-fitting a stroke's coordinates
  under `/SA` (see `doc/todo/_scan-conversion.md`), rendering intents beyond
  `AbsoluteColorimetric`, and **a glyph a document's own embedded subset does not contain**, which
  was traced to the end of every route the standard states: §9.7.4.2's and §9.6.5.4's rows carry
  the evidence, and `poppler` draws such glyphs from a face this machine has, which is a fallback
  rather than a reading. **The two halves of that last one are two different refusals** (ADR
  0270): a `loca` entry that is empty *is* the program's statement that the code makes no mark, and
  `Interpretation::codes_reaching_a_blank_glyph` counts it apart from a code that reaches nothing
  at all.

---

## What to do next

**The work itself lives in [`doc/todo/`](todo/README.md)**, one file per item, numbered so that
`ls` sorts by priority — `00`–`09` standing, `10`–`19` defects, `20`–`29` owed features with
corpus demand, `30`–`39` capability, `40`–`49` measured performance, `50`–`59` blocked.
`doc/todo/02-every-round.md` is what a round does around whatever it takes.

What stays here is the *shape* of choosing, which is the part that has been wrong before.

**Two tracks, and take from both in every round.** *Demand-driven* is what the corpus and the
oracle name; *spec-driven* is the ledger's `reported` rows and the notes on its `partial` ones. A
project running only the first finishes when the corpus goes quiet, which can happen with much of
the standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises. This is a principle-5 rule, not a suggestion.

**But the map is not the territory.** Four of the six findings in the ten sessions from the
hundred-and-twentieth were on no list at all: a `shall` hiding behind a silence about artwork (ADR
0109), a clause with two populations where the row named one (0110), a malformed optional entry
that erased a font (0111), and a font cache keyed by a name that drew wrong glyphs in silence for
thirty-one sessions (0115). None was `silent`, none was `reported`, and no gate could see the
last. Three were found by reading the clause beside the code; the fourth by measuring something
else.

**The six shapes a refusal takes when it has outlived its reason** — a reason that names a
vocabulary, a reason that names an architecture, a capability that arrived and announced nothing,
a capability that reached the crate and never reached the program, a row that would have
survived the capability arriving, and a row *corrected* by naming the capability that arrived while
the entry it turns on stayed unread — are in [`doc/habits.md`](habits.md)'s ledger section, beside
the sweeps that find each. They are the highest-yield reading this project has.

---

## Run it

```sh
cargo run --release -p viewer-ui  --bin pdf-viewer     -- doc/PDF20_AN001-BPC.pdf
cargo run --release -p viewer-gtk --bin pdf-viewer-gtk -- doc/PDF20_AN001-BPC.pdf   # the GTK4 host
cargo run --release -p viewer-qt  --bin pdf-viewer-qt  -- doc/PDF20_AN001-BPC.pdf   # the Qt 6 host
```

**[`doc/running-the-viewer.md`](running-the-viewer.md)** is the rest and is all of it: every flag
(`--page`, Annex O's `#fragment`, `--cpu`, `--backend`, `--trace=<topics>`, `--no-sandbox`,
`--licences`, `--ignore-restrictions`), every key the window binds, the confined examples, the
presentation fixture, and what the snapshot release ships on which platform.

**And rebuild before saying anything about speed**: `cargo test` only ever builds the *debug*
binaries, and [todo 02](todo/02-every-round.md) §5 is what puts the release binaries and the one
library where a person can run or link them. `tools/state.sh binaries` says what is there now.

## Verify it

**The round's gate sequence is [todo 02](todo/02-every-round.md) §2, which owns those commands.**
This file used to state them a second time and they drifted, which is why it no longer does.

**[`doc/verify.md`](verify.md)** is everything else, and a round runs the ones its change can
reach: `cargo deny`, the cross-target checks under `-D warnings`, the fuzz targets and which need
a seeded corpus, the callgrind counters, the census and ladder examples, and the AT-SPI recipe.

## Crate map

**[`doc/crate-map.md`](crate-map.md)** — one row per crate, its one responsibility, and the
decision that lives in it.

---

## Traps — read these before writing code

### 1. The metrics lie. Look at the page.

**The most important thing in this file.** `Interpretation::is_complete()` says what the
interpreter *knows* it skipped. It cannot say a font loaded and produced garbage, a page is upside
down, or a gradient came out opaque. The archetype: wiring bare-CFF support in made every affected
document report `unsupported: []` and render **almost no text**.

`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable` writes PNGs;
the oracle's artefacts are better. Two automated checks catch a wrong mapping, both in
`pdf-font/src/loading.rs`: `the_pdf_widths_agree_with_the_font_programs_own_advances` — the `/Widths`
and the charstring's own advance are independent statements of one fact — and
`an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`. Neither replaces looking.

**Every page a new feature makes drawable is a page nobody has ever looked at**, and the habit has
paid every session since the tenth: dashed squares that should not have been solid; a fax page
**upside down** because `/Rotate` 90 and 270 had been exchanged since the first page tree; a
gradient painted opaque because one `return` dropped §11.6.4.4's alpha; a `0 w` line invisible on
the GPU; `issue7901.pdf` drawing `üãÍ†Ë` because Table 115's presence condition was read as a
condition on meaning. **A page a feature makes drawable can be one that never rendered at all** —
the oracle's `no render` count is a to-do list of pages nobody has looked at, and `tools/state.sh
oracle` prints it — one left it in the hundred-and-seventy-seventh session when a page the file's
own cross-reference table had hidden started rendering (ADR 0148).

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect. `issue6621.pdf` and `issue7901.pdf`
were both code that was right about the clause it cited.

**A contradicted page's group names a hypothesis, not a diagnosis — nine for nine on being
wrong**, the newest being `calrgb.pdf`'s four pages, which spent four hundred and fifty-five
sessions inside `CONTRADICTED_SUBSTITUTED_FONT` differing from each other in one entry no voting
renderer reads. The one before it, `issue4304.pdf` in the four-hundred-and-fifth session, spent a
hundred and eighty sessions in the same group while the difference was six
spaces of zero width and the side-by-side said so in one look. Open the artefact before believing the label — **and measure it, because a label this
project wrote is still a label**. **A group's note that names another group's mechanism is the
cheapest tell there is**: `calrgb.pdf`'s said "a residue of colour management rather than of fonts"
and stayed under the *font* group's name for four hundred and fifty-five sessions after the session
that wrote it. Twice the instrument that settled one was the font's own `cmap`,
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

**And every resource lookup that can cost a mark says so, which is a statement rather than a list.**
Since the four-hundred-and-nineteenth session all six of Table 34's categories the interpreter looks
a name up in report a name the current resource dictionary does not define: `Font` and `Shading`
always did, `ColorSpace` reports through `ColourSpace::parse`, and `XObject`, `ExtGState` and
`Pattern` were silent until then (ADR 0255). `Properties` is deliberately outside it and the reason
is trap 11's — a missing property list costs no mark, so a report there would take a page off the
oracle's judged set for nothing. **The condition on all three is `is_hidden()`**: §8.11.3.1 skips
hidden content "as if there were no `Do` operator to invoke it", and a `Do` that was never invoked
cannot have failed.

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

**Two rules have been measured to be unreachable by all 974 documents, and the method is worth
as much as the finding.** §9.7.6.2's per-byte codespace test and §12.5.2's rule that a stored
appearance ignores `/CA` were each measured by breaking the rule and running both gates: all 1794
verdicts identical. **That turns "the corpus does not cover this" from a suspicion into a fact.**

**A third stood here for a hundred and eighty sessions and was wrong, and the way it was wrong is
the lesson.** It said §7.6.2's signature exception was unreachable because "eight documents carry a
signature dictionary, twenty-six an `/Encrypt`, and the two sets are disjoint, which is one `grep`".
The grep was right about the two sets it counted and **the sets were not what it thought**:
`issue17069.pdf` is in both, and the reason it was not counted is that the code being justified
could not see it — its signature dictionary states no `/Type`, which Table 255 permits, so
`is_signature_dictionary` said no and the 33 680-byte signature value went through AES and came back
empty (ADR 0215). **A measurement taken with the instrument under test is not independent of it.**
The two rules above were measured by *breaking* the rule and watching a gate move, which is the
method that does not have this failure; a census whose predicate is the thing being checked does.

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
- **And a fifth, found in the hundred-and-seventy-sixth: two references sharing a decoder can
  *disagree*, and that is worse than agreeing wrongly.** On nineteen JBIG2 refinement pages
  `jbig2dec` fails in both of them, `mupdf` renders black and `ghostscript` renders white — so
  instead of contradicting us they produce no consensus at all and the page becomes `ambiguous`,
  which nothing was watching. Shared code does not only manufacture agreement; it can also
  manufacture the *absence* of one, and the second is invisible where the first is at least
  listed. `AMBIGUOUS_SHARED_JBIG2_DECODER`.

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

**And there are now two of these, in a profile of their own.** Since the three-hundred-and-eighty-fifth
the corpus gates run under `--profile gates`, so the worker they spawn is `target/gates/pdf-sandbox-worker`
and not the release one; and `pdfref-hayro`, which the oracle spawns for a fourth reading, is a second
program under the same rule. That one is worse than trap 10's original shape rather than better: it
**fails silently**. `Reference::Hayro` votes on nothing, so its absence leaves every verdict intact and
only removes a picture — which is how it went unbuilt by `doc/todo/02` §2 for its whole life and was
noticed by a reference-render count falling 861 with nothing else moving (ADR 0222). Both are lines in
§2 now, and the tell is the same one trap 10a names: the hit rate.

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

**[`doc/habits.md`](habits.md)** holds all of them, in six sections. Open the one the round is
about; a habit is worth reading when you are about to do the thing it is about, which is why they
are no longer here.

| section | about |
|---|---|
| Reading the specification | what a modal verb means, what a silence is and is not, when a claim about the standard decays |
| Judging against other implementations | what an agreement is evidence of, what a reference is being asked, when a measurement is of the instrument |
| Tests, gates and reports | what discriminates, what a ratchet's direction means, what a suite of small scenes proves |
| The ledger, and claims about this tree | how a row, a comment or a todo file goes stale, and which greps find it |
| Measuring | A/B in one sitting, attribute by removing the suspect, and which number to quote for which change |
| Code, bounds and dependencies | what a cache's key claims, what a clamp decides, what a dependency is in a position to break |

**Three of them bind every round rather than a particular kind of work**, and
`doc/todo/02-every-round.md` §7 is where those live, beside the round they bind.
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
  nothing about the rest. `LoadedFont` distinguishes "this code has no glyph" from "this code's
  glyph is blank", which a space legitimately is, and the corpus gate prints both counts — **and a
  third, which is the one nobody had**: a code no method of §9.10.2 could name, which is the
  *reading* band rather than the drawing one and is two orders of magnitude wider (ADR 0311).
  `examples/unnamed_code_census` splits it by which method the font could have answered with (ADR
  0318). **None of the three is a report**, deliberately, on ADR 0152's arithmetic: a report takes
  a page off the oracle's judged set, and these are shortfalls in the readback of pages that mostly
  draw perfectly. The volume is measured; what to do with it is not settled.
- **`doc/md/` is the specification in a form code can read** — markdown conversions of the 14
  PDFs. **This entry said "committed" and had been false since the three-hundred-and-eleventh
  session**: `.gitignore` covers `/doc/md/` and `/doc/*.pdf`, and what is tracked is the encrypted
  `doc/specifications.zip` (ADR 0187). A test may still depend on them without a skip path, but for
  the other reason — a test that cannot find them **fails loudly** rather than skipping, which is
  what the owner decided in that session.
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
---

## How the project got here

**[`doc/history.md`](history.md)** — one line per session, with the ADR that argues each, and the
**block summaries**: what the twenty rounds from the three-hundred-and-fifteenth had in common, what
the thirty from the four-hundred-and-sixteenth did, and what the thirty from the
four-hundred-and-fifty-fifth did. **Its table ends at the four-hundred-and-forty-fifth and an
ordinary round appends nothing to it** — this sentence said "a round appends one row to it" for ten
rounds after ADR 0281 moved the record to [`doc/history/`](history/README.md), one file per round.
**A *closing* round is the one exception and it appends its block summary here**, which that file's
own preamble states: a summary is about a run of rounds and belongs beside the others.
