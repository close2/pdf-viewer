# Handover

Written 2026-07-26, rewritten and halved 2026-08-01 at the end of the **hundred-and-thirtieth**
session, halved again in the **three-hundred-and-ninety-fifth**, and kept current. Read `/CLAUDE.md` first — the five
principles, what *done* means, and the closed exclusion list. **Principle 5 is the one that changes how you work**: the specification is the
only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it
right, never the definition of right.

**`doc/todo/` holds one file per piece of owed work**, numbered so that `ls` sorts by priority;
its `README.md` is the index and `02-every-round.md` is what a round does. `doc/PLAN.md` holds the
phases and the ledger's design; `doc/adr/` holds every decision's argument;
`doc/conformance/ledger.toml` holds one row per subclause; **`doc/RENDER_LIBRARY.md` is what a
rendering library would have to be to fit this viewer** and `doc/QUORRA_FEEDBACK.md` is what came
back when one was built to it; `doc/JPEG2000_FEEDBACK.md` is the same shape one dependency over.

**This file is the state of play, the traps and the numbers** — where something is written
elsewhere, this is a pointer, and the pointer is the whole entry.

**A lesson lives here exactly once**: in a trap if it changes how you write code, in
`doc/habits.md` if it changes how you work, in the numbers if it is a fact about today. A session's
narrative belongs in its ADR and in `doc/history.md`, nowhere else. This file was halved in the
fifty-ninth session, again in the hundred-and-thirtieth and again in the
three-hundred-and-ninety-fifth; if you find yourself retelling a session here, you are undoing that.

**What was moved out in the three-hundred-and-ninety-fifth, and where it went** (ADR 0232). Each of
these is *all* of what it holds, not a précis: a round that needs the detail opens one file.

| file | holds | opened when |
|---|---|---|
| [`doc/history.md`](history.md) | "How the project got here" — one row per session, and the block summary above it | you are asking *when* something landed |
| [`doc/habits.md`](habits.md) | the six method sections: reading the specification, judging against other implementations, tests and gates, the ledger, measuring, code and dependencies | before a clause read, a comparison, a gate or a measurement |
| [`doc/ui-boundary.md`](ui-boundary.md) | §0 — `viewer-core`'s vocabulary, the five rules, the three pixel tiers, the text layer and the edit log | you are writing a host or adding a message |
| [`doc/performance.md`](performance.md) | §3b and §4 — the launch path, the two backends compared, what the window itself has been measured at, every instruction count and what each is of | before optimising, or before quoting a number |
| [`doc/oracle-and-corpus.md`](oracle-and-corpus.md) | §3 and §3a — the contradicted groups, the incomplete population, the ambiguous bucket and what it has produced | you are taking a page off a ranking |
| [`doc/ledger-and-claims.md`](ledger-and-claims.md) | the seven shapes a ledger row goes wrong in, and the four times ours has | before writing or believing a row |
| [`doc/third-party-data.md`](third-party-data.md) | §1 — what was vendored, under what terms, and every dependency decision including the one that came out *no* | before taking a dependency |
| [`doc/verify.md`](verify.md) | every instrument that is not the round's gate sequence: `deny`, the fourteen fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI | you have a reason to run one |
| [`doc/running-the-viewer.md`](running-the-viewer.md) | the flags, the keys, `--trace`, the confined examples, what the snapshot ships | you are running the program |
| [`doc/crate-map.md`](crate-map.md) | one row per crate and the decision that sits in it | you are looking for where something lives |

---

## Where we are

A PDF **viewer**, and until the hundred-and-thirty-first session that word would have been a
claim rather than a description.

It **draws** what a page says: geometry, colour, images, shadings, patterns, embedded text,
transparency groups, soft masks, and annotations both from stored appearance streams and
constructed where the standard states one — including §12.5.6.4's seven icons, whose artwork is
this processor's own because the clause requires one and draws none — and, since the
two-hundred-and-sixty-sixth session, §12.5.6.15's four and §12.5.6.16's two, whose clauses only
*recommend* one and whose names name objects. Two backends (CPU and GPU)
that agree to the channel — over `test-scenes`' fixtures **and, since the hundred-and-forty-third
session, over real pages at a real window's resolution**, which is where they did not (ADR 0127).
The GPU backend **bands a target the device cannot draw in one pass**, because Vello's working
buffers are fixed constants with no knob and a page of small text at a laptop's resolution can
exceed them.
JBIG2 and JPEG 2000 in a confined worker. Encryption at every revision
and method §7.6 states, and since the hundred-and-forty-fourth session in both directions. §12.3.2's destinations, §12.3.3's outline, §12.4.2's page labels,
§12.5.6.5's links performing **eleven of §12.6's actions**, §14.9's accessibility entries,
§12.4.4's whole presentation read **and, since the three-hundred-and-ninety-third, played** —
seven of Table 164's twelve transition styles drawn frame by frame and the other five reported by
name (ADR 0230) — and everything a document
says *about itself*: §14.7's logical structure, §14.8's tagged-PDF vocabulary, §7.11.4's embedded
files, §14.13's associated files, §12.2's viewer preferences, §12.11's requirements, §7.12's
extensions, and since the **two-hundred-and-ninety-fourth session §14.3.2's XMP** — the last
large population this tree read nothing of, 319 documents, closed by a dependency decision
nobody had been asked for (ADR 0186).

It is **used**, which is what the nine sessions from the hundred-and-thirty-first added. A
locked document asks for its password (§7.6.4.1, owed since the twenty-second session); the page
zooms and scrolls; the cursor knows what it is over and §12.5.5's appearances follow it, as does
§12.5.6.19's `/H`; a drag **selects text**, whose shapes cross to the host as geometry so that it
draws them in its own colour; **a person can search the whole document since the
four-hundred-and-fourteenth** — `/` opens a find bar in all three hosts, every occurrence on the
page is highlighted under the selection, and Enter walks to the next occurrence anywhere, one page
read per turn of the host's event loop because 1023 pages of ISO 32000-2 are 5.84 s of
interpretation and nothing here blocks for it (ADR 0250) — **and searching the same document twice
stopped costing twice since the four-hundred-and-twentieth**, which kept the readback under a 4 MiB
per-document bound with least-recently-used eviction and took a repeated sweep from 5.45 s to
**7.27 ms**, 0.021 s in the window against a cold 4.79 s (ADR 0256); a person can **fill in a form field** — in the window since the three-hundred-and-forty-ninth session, where the host keeps the *point* it clicked and never the text, so §12.7.5.3's truncation is read back rather than predicted (ADR 0201), **with a caret since the three-hundred-and-seventy-first** that says where the next character goes and moves with the arrow keys, so correcting the middle of a value is no longer deleting back to it (ADR 0211) — undo it and redo it; a click on a markup annotation **opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about activation and was owed to a capability this program had had for a hundred and eighty sessions (ADR 0191); a person can **add an annotation** — §12.5.6.10's four markups over what is selected since the three-hundred-and-twenty-first (ADR 0196), and since the four-hundred-and-first **§12.5.6.6's free text, drawn as a rectangle and typed into**, which is the one markup subtype whose text *is* the annotation and therefore the one whose geometry has to come from a drag rather than from a selection (ADR 0238); and the
result can be **saved** — the file it was opened from, unchanged, with §7.5.6's incremental update
appended, which is the one kind of writing `CLAUDE.md` permits.

**Page one goes to the graphics device**, decided by the project owner in the
two-hundred-and-seventy-third session and written into `CLAUDE.md`'s startup rules. GPU bring-up
is therefore *on* the critical path by choice, which makes what it costs a number to keep rather
than a cost to hide.

The rest of that timeline — what each step costs, what came off it and what is left — is
[`doc/performance.md`](performance.md)'s first section, and the open half is
[todo 42](todo/42-the-launch-path.md).

**And it has chrome, which is what the ten sessions from the hundred-and-sixty-sixth added and
what this project owed for thirty before that.** A sidebar of four tabs, drawn with `pdf-font`'s
compiled-in Helvetica and a `pdf-render` display list so that both backends draw it: §12.3.3's
outline, where a click **activates the item** and the document decides whether that is a jump or
a URI; §8.11.4.3's layers, where a switch turns one on unless Table 99's `/Locked` forbids it;
§7.11.4's embedded files, where a click writes the file beside the document — and where, since
the three-hundred-and-fifty-second session, a document that states §12.3.5's `/Collection` gets
its folder tree and the schema's columns instead of a flat list, because a collection is how a
document *arranges* its files rather than a new population of them (ADR 0202); and §14.3.3's
`/Info`. **A fifth tab arrived in the two-hundred-and-sixty-first**: §12.3.4's thumbnails, one row
per page with the miniature fitted above §12.4.2's label, where a click shows that page. **A sixth
in the three-hundred-and-forty-seventh**: §12.4.3's article threads, listed with their `/I` title
and followed on a click — to Table 163's `/R` rather than to the page the first bead sits on,
because activating one composes §12.6.4.7's own thread action rather than adding a second route
(ADR 0200). Not one corpus document states a thread, which is said out loud rather than around. `?` puts
`/NOTICE` over the page in Courier. **The document chooses what opens**: Table
29's `/PageMode` names a panel and four of its six values now name one that exists, and §12.2's
`/DisplayDocTitle` puts the document's own title in the title bar. §12.6.3's trigger events are
raised by the pointer at last. Four clauses closed on it — §12.3.3, §7.11.4, §7.11.4.1 and
§14.3.3 — and three of the four had a ledger row whose reason was "this program has no ___".

All of it sits behind **`viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API. Two consumers —
`viewer-ui`'s winit window and a headless test harness — and §0 is the whole story. ADRs 0116 to
0121.

**And since the three-hundred-and-eighty-first there is a third consumer, which is a process with
no filesystem.** `viewer-confined`'s `pdf-view-worker` holds a `Viewer` and `render-cpu` behind
seccomp-BPF, Landlock and a 4 GiB address-space ceiling, and `viewer_confined::Confined` speaks
`Command`/`Event` and `Query`/`Reply` to it over a pipe — with the **pixels** coming back, because
the confined process owns the rasteriser and the display list therefore never leaves. That is
principle 3's other half, owed since ADR 0014 confined the three image codecs and left the larger
surface — the document, the interpreter and the rasteriser — in process. **Nothing in `viewer-core`
had to change**: rules 2, 3 and 4 already forbid it a filesystem, a clock and threads it was not
handed, which is a description of a confined process. Verified by drawing: a page byte-identical to
the one this process draws, a page turn, a magnification, a JBIG2 document decoded *inside* the
confinement — and a confined process that cannot open a file, cannot open a socket and cannot start
a program, each asked of the kernel rather than of the source. It costs 1.1 ms to start and confine
a worker and about twice as long to put a page in a host's hands, most of that difference being
4.1 MB of pixels down a pipe. **All twenty-eight questions cross**: twenty-five since the three-hundred-and-eighty-sixth,
including the eleven a panel is made of — an outline, a layer order, an attachment list, a
collection, article threads, a thumbnail, §14.3.3's properties with §14.3.2's packet beside them,
Table 29's opening pair, Table 147, popups and §14.7's structure — so a host on this boundary has
the sidebar's data and `examples/confined_panels` prints it; and **§12.7's whole form since the
three-hundred-and-ninety-eighth**, which is a twelfth panel encoding and the thing that lets a
confined host build native controls rather than take a form as pixels (ADR 0235). The largest of them is ISO 32000-2's
outline at 88 KB, which is a fiftieth of that page (ADR 0223). **The window does not use it**,
deliberately: `viewer-ui` is a tier-2 host and this boundary is tier 1, so putting it there is a
change of tier and a decision with a number attached rather than a switch. ADR 0218, `doc/todo/34`.

**And since the four-hundred-and-eighth there is a fourth consumer, and since the
four-hundred-and-tenth a fifth — both of them somebody else's toolkit.** `crates/viewer-gtk`'s `pdf-viewer-gtk` is a real GTK4 application on the same boundary:
§12.3.3's outline, §8.11.4.3's layers and §7.11.4's files in a `GtkListView` over a
`GtkTreeListModel`, §12.7's fields as a `GtkEntry`, a `GtkPasswordEntry`, a `GtkCheckButton`, a
`GtkDropDown` and a `GtkListView` placed over the page, the selection and §12.5.1's focus ring drawn
in the theme's own colour, and the two decisions a host owns — §12.7.6.4's file and §7.6.4.1's
password. `doc/todo/30`'s order made GTK4 first because `gtk4-rs` is Rust-safe with no C++ bridge,
and the crate keeps `#![forbid(unsafe_code)]` to prove it; `gtk4` is named by that manifest and by no
other. **Tier 1, because GTK4 admits no other**: a widget has no native surface and GSK hands out no
device, so `Query::Frame`'s raster becomes a `gdk::MemoryTexture` with no conversion at all — 2.69 MB
in about 0.8 ms. **It needed no new message.** What it *did* produce is six things the boundary was
missing, the largest of them the page drawn without its widget appearances, now a photograph rather
than a prediction. ADR 0244, `doc/todo/30`. **That one was taken in the four-hundred-and-ninth**
(ADR 0245): `Command::Delegate` is §6.3.2.2's "unless otherwise instructed", `pdf-viewer-gtk` sends
it by default, and what the picture then exposed is the *scale* — 11 of 76 controls on
`160F-2019.pdf` are wider than the `/Rect` they cover and all 76 are taller.

**And the four-hundred-and-tenth built the second of the two, which is the one that costs a C++
bridge.** `crates/viewer-qt`'s `pdf-viewer-qt` is a real Qt 6 Widgets application on the same
boundary — the three panel answers in a `QTreeView` over a `QAbstractItemModel`, §12.7's fields as a
`QLineEdit`, a `QCheckBox`, a `QComboBox` and a `QListWidget`, the selection drawn in
`QPalette::Highlight` and both host decisions — and **it needed no new message either**. That was
`doc/todo/30`'s condition on the C ABI (*"do not freeze a C ABI until two Rust consumers have shaken
the API out"*), so **the ABI could be frozen**, with three amendments named first:
`pdf_render::RasterFormat` is `#[non_exhaustive]` and crosses the boundary, `Answer::Outline`
borrows where its two siblings are owned, and `Answer::Field` answers a password field with bullets
nothing says cannot be read back. A Rust host writes one line it should not have to; a C consumer
cannot fail to compile.

Three things came with it. **`crates/viewer-host`**, because the second host wanted four of
`viewer-gtk`'s eight modules unchanged — the panel rows, the control decision, §12.7.6.4's file
policy and the launch timeline named no GTK type, and `viewer-gtk`'s whole public interface is now
`Host` and `HostError`. **One hand-written `unsafe` token in the tree**, the `unsafe extern "C++"`
header `cxx` requires, under `#![deny(unsafe_code)]` with one exemption on `mod bridge` and
`tests/unsafe_position.rs` asserting its position and that no other crate lifts the denial. And
**the tier-1 copy measured on both toolkits, cold and warm**: 234 µs and 231 µs in the steady state
on 2.7 MB, ≈11.5 and ≈12.0 GB/s, which corrects ADR 0244's ≈3.2 GB/s as a first-frame number.
ADR 0246, `doc/todo/30`.

**And the four-hundred-and-eleventh built the third and last host, which is the one that cannot
fail to compile.** `crates/viewer-ffi` is a C ABI over the same vocabulary: 43 `extern "C"` entry
points since the four-hundred-and-fourteenth, a hand-written `include/pdf_viewer.h`, and `c/open_a_page.c`, which a test compiles with
`gcc -Wall -Wextra -Werror` and runs — it opens a document, prints every event, draws the first page
through the round trip, turns to the second, reads §12.3.3's outline and copies the pixels out.
**It needed no new message either**, three hosts running. Four shapes decide it, each because C
takes something away that Rust gave: **commands are functions**, because a union's size is part of
an ABI and a symbol is not, so a command added later costs a compiled caller nothing; **events and
answers arrive owned in a batch the caller frees**, so no borrow of the viewer crosses and
re-entrancy stops being a rule anybody keeps; **a render request is an opaque handle** the caller
may move to its own thread, because a display list is clauses 8 and 9 in a data structure and a
frame comes back by copy into the caller's own buffer; and **a variant added later is named,
described and counted** — every event answers a one-sentence description whatever its kind, and
`pdfv_abi_check` turns "fails to compile in every consumer" into "fails to start, once, naming the
number that moved", which is weaker and is the strongest thing C admits. Read off the run:
`PDF20_AN001-BPC.pdf` opens in 4.4 ms and its first page is drawn and handed back at 12.3 ms;
`ISO_32000-2_sponsored_EC3.pdf` at 63.1 and 76.3 ms with 1023 pages and a 988-row outline.

**The three amendments came first and one of them was a bug.** `Answer::Field`'s password value was
supposed to be "one sentence in a doc comment"; it is now `Option<pdf_model::view::ShownValue>` —
the characters beside Table 231 bit 14's `obscured` — because `viewer-ui` read a password field's
value back after every keystroke (ADR 0201) and sent the bullets as the next value. Reading the
clause for it found a second sentence nobody had read: **this program wrote a person's typed
password into the file it saved**, against that table's own NOTE, and no longer does — `save` writes
neither the value nor the appearance for such a field and reports each one it withheld. ADR 0247,
`doc/todo/30`.

**And the four-hundred-and-twelfth closed the one gap three hosts found, without adding a message.**
`doc/todo/30` had recorded §12.7.5.4's list box as *"the one place the boundary genuinely limits a
host"*: Table 233 bit 22 says "more than one of the field's option items may be selected
simultaneously", `Edit::SetField` carried one string, and GTK4, Qt and the headless harness had each
asked for *single* selection deliberately rather than send one of what a person chose. **The reading
half was already complete** — `ChoiceControl` has carried Table 234's `/Opt`, the selection as
indices and bit 22 itself since ADR 0235 — so the gap was one direction wide, and the fix is the
shape ADRs 0166, 0167 and 0247 established rather than a new channel: `Edit::SetField`'s value is
`pdf_model::view::Entered` now (`Cleared`, `Text`, `Chosen(Vec<usize>)`), and every consumer failed
to compile until it said what it does. **A selection is named by index rather than by label**,
because `/V` holds labels and two `/Opt` entries may carry the same one — which is exactly the
ambiguity Table 234's `/I` exists for — and the resolution to a label happens **once**, in
`ViewState::set_field`, so the appearance and the file cannot disagree. Both shapes of `/V` are
written (a string for one item, an array for several, removed for none) with `/I` ascending beside a
selection and **removed** where the new value is not one. Bit 22 is *obeyed*: a selection sent to a
single-select field is cut to its first index, ADR 0197's shape. **The C ABI did not move and that is
the finding** — `Command::Edit` is not among its 39 entry points, so `PDFV_EVENT_KIND_COUNT` is 15
before and after. Driven under `Xvfb` on `issue17492.pdf`, one of the corpus's **4** bit-22 widgets:
two rows selected at once in each toolkit's own highlight, and **both hosts wrote byte-identical
71 524-byte files** carrying `/V [ (Oracle) (DB2) ]` and `/I [ 0 2 ]` over the producer's 70 166
bytes intact. ADR 0248, `doc/todo/30`, `doc/todo/22`.

**And since the four-hundred-and-twenty-first a *program* can ask it questions.** `tools/pdf-retrieve`
is a JSON-on-stdout command line over the readers this tree already had, and what it adds is the
three joins between them `doc/todo/36` named: §12.3.3's outline turned into the range of pages a
section occupies, the text cut at that section's own two headings, and §12.5.6.10's `/QuadPoints`
deciding which *section* an annotation belongs to rather than which page. So `pdf-retrieve section
doc/ISO_32000-2_sponsored_EC3.pdf 12.5.2 --subtype StrikeOut,Caret` is the clause's text and the 23
errata over it in one call and 66 ms. **Its default answer is `Interpretation::text` byte for byte**,
which a test asserts: that is the string `tests/text_extraction.rs` measures against `pdftotext`, and
a tool that tidied it would put itself between a caller and the only independent measurement this
project has of its own extraction. ADR 0257, `doc/todo/36`.

**And since the three-hundred-and-seventy-seventh it can tell a person that a signed document
changed after it was signed, and since the three-hundred-and-ninety-second whether its signature
verifies.** §12.8.1 divides verifying a signature into three questions and only the third needs the
trust store the whole clause had been refused for. `Signature::integrity` recomputes the digest over
§12.8.1's `/ByteRange` — with the six algorithms Table 260 and Table 256 name — and compares it with
what `pdf_model::cms` reads out of §12.8.3.3's `SignedData`, over a bounded in-tree X.690 reader that
allocates nothing and is fuzzed at 1 000 000 runs (ADR 0215). `Signature::authenticity` then finds
the certificate the `SignerInfo` names among the ones the signature itself carries, reads its key
with `pdf_model::x509` and verifies with `pdf_model::pkcs1` — RFC 8017's RSASSA-PKCS1-v1_5, in tree,
no dependency taken and none needed (ADR 0229). **Four of the corpus's ten signature dictionaries no
longer hash to what they record and all ten verify**, which is a stronger statement than either half:
those four are real signatures whose documents were re-saved underneath them. The sentences the
program uses keep every asymmetry — a mismatch is decisive, a match is the absence of one kind of
evidence, and a certificate that arrived in the same file as the signature it verifies proves the two
are consistent with each other and nothing about who made either. Nothing here says a signature is
valid.

**And since the three-hundred-and-seventy-sixth session it speaks a page**, which is the sentence
this paragraph carried as missing for two hundred and twenty-seven: `viewer-accessibility` maps
§14.8.4's forty-one standard structure types onto `accesskit::Role`, and `accesskit_unix` puts the
result on AT-SPI — where a real client walks it off the bus, `Frame` → `DocumentFrame` → the page
named by §12.4.2's own label → §14.7's elements, with §14.9.3's `/Alt` where the document states
one and a `StatusBar` group carrying **what the page could not draw**, because the person who
cannot see the page is the one for whom a count in the title bar is no answer. An untagged page
says that it is one rather than being given an invented reading order. The one async runtime this
tree has is confined to that crate, it is Linux-only in its own manifest, and the adapter is
created **after** the first frame is presented — the launch timeline's steps are unmoved to a
millisecond (ADR 0214).

**And since the three-hundred-and-ninety-third it plays a slide show.** It has *advanced* one
since the hundred-and-fiftieth (`Command::Tick`, ADR 0135); now `p` drives the clock and
**seven of Table 164's twelve transition styles are drawn frame by frame** — `Wipe`, `Split`,
`Box`, `Cover`, `Uncover`, `Push` and `Fade`, which are the ones whose frame the table's own words
determine. `viewer_core::transition` shapes a frame at a *fraction* of the way through, because
rule 3 keeps the clock out of that crate, and the frame is a `pdf-render` display list of two page
rasters, so both backends draw it. The other five are **reported by name** rather than drawn as a
cut, each for a quantity the clause does not state (ADR 0230).
**What it still does not do**: those five styles, and §12.4.4.2's sub-page navigation, which is
read and walked by no control.

### The gates, today

**The four-hundred-and-thirtieth ran the whole sequence before and after and every line reproduced
except three counts** — tests 1562 → **1567**, citations 6349 → **6357**, quotations 592 → **595**.
It took four whole SafeDocs archives (4000 documents, 5.04 GiB) and fixed two defects out of them:
a three-component `DCTDecode` frame whose Adobe APP14 marker was read as its component count, which
lost 21 images over four documents, and a `/Contents` part the file states is empty, which was
reported as drawing the page had lost. **Neither moves the 974**, and that is the expected result
rather than a disappointment: no document of the pdf.js corpus carries either shape, so the
instrument that answers the round is the web sample and `doc/todo/02` §7's second habit applies in
reverse — a count that does *not* move is not evidence that nothing happened. The corpus's 974 with
68 incomplete, the oracle's 1794 at 1690/104 with 905/68/786 and the undiagnosed list empty,
quorra's 910/36/11/17, both text gates, the dates, the XMP and the JPEG 2000 lines are what this
table says. `doc/todo/00`'s step 7 is **not owed**, and that is an artefact rather than an argument: a scan
found **16** of the 974 carrying a three-component JPEG with an Adobe APP14 marker and **8** whose
stream dictionaries pair a `/Filter` with `/Length 0` — one is called
`multiple-filters-length-zero.pdf` — so `examples/display_list_digest` was run here and in a
detached worktree at `c1c9e62`, and the two files are **identical over 975 lines**. ADR 0266.

**Every number in this table was printed by the gate beside it in the three-hundred-and-ninety-eighth
session**, which ran the whole of `doc/todo/02-every-round.md` §2 and read each figure off the
output. Nothing in it is arithmetic performed here — that has been wrong twice — and nothing in it
is carried forward from a previous round.

**The three-hundred-and-ninety-ninth ran the whole sequence again and every line reproduced
character for character except the test count**, which its own seven tests moved. That is the
strongest thing it has to say about a change to the correctness oracle's rasteriser that halved the
corpus's worst page (ADR 0236).

**And so did the four-hundred-and-second**, which was a sweep round that also drew §12.5.6.19's
push-button icon: the test count moved by its six, the citation and quotation counts by what it
wrote, and every other figure in the table below reproduced exactly. That is the expected result and
it is worth stating — the entries it implemented cannot change a mark on any of the 974, counted
before the work by `examples/push_button_census` (ADR 0239), so the instrument that answers this
round is the ledger and not the corpus.

**And the four-hundred-and-fourth**, which added a cancel to the confined viewer and changed how
both ends of that pipe write a frame header (ADR 0241). The whole sequence ran and **every line
below reproduced except the test and citation counts** — its four tests and one citation. The
transport change is the one worth saying that about: the header is the same nine bytes in the same
order, written in a separate call rather than in front of a copy of the payload, so the wire format
did not move and no gate could see it. What *did* move is a measurement no gate takes — 4.1 MB of
raster crossing the pipe, 5.64 → 3.74 ms median over nine runs each way.

**And the four-hundred-and-third, which changed what a form field's baseline is read from and is
the strongest instance of this yet.** It ran the sequence twice, before and after, and diffed the
*output* rather than the summaries: the oracle's 1794 verdict lines are identical but for its two
timing lines, the corpus's 974 and quorra's 957 are identical when sorted, and the text gate's
output is identical outright. Only the test, citation and quotation counts moved. `doc/todo/00`'s
step 7 is therefore **not owed** — nothing was drawn differently — and that was
`examples/variable_text_census`'s prediction before the code was written rather than a hope after
it (ADR 0240).

**And the four-hundred-and-fifth, which is the first of these in a while to move a *verdict*.** It
worked the contradicted list rather than the ambiguous one and found a width defect behind a
hundred-and-eighty-session-old diagnosis (§9.6.2.1, `issue4304.pdf`): **one page of 1794 changed
verdict**, contradicted to agreeing, and the oracle's other 1793 lines, quorra's 957, the corpus's
974 and the text gate's output are unmoved. `doc/todo/00`'s step 7 was run **before and after**
anyway, because pixels did move, and **all 786 lines are byte-identical** — which is the sweep
saying what it can: its population is the ambiguous bucket, and a page moving between *contradicted*
and *agrees* is invisible to it, so the identity means no ambiguous page's ink changed. The test,
citation and quotation counts moved by the round's own two tests and its citations.

**And the four-hundred-and-seventh, which moved no verdict at all and answered a question about a
number.** It asked whether the one bound 38 of the 68 contradicted pages fail — the differing
fraction — is catching something the averages hide or was never derived, and measured it the way
this project's structural floor of 0.90 was measured: over **9898 reference pairs**, each measure
taken over the pairs the other three bounds admit. On text pages `TEXT_HEAVY`'s four bounds reject
**0.0%, 1.2%, 0.5% and 29.4%** of the reference pairs that agree by everything else, so one of the
four sits below the spread of the implementations that set it — and the sentence claiming to derive
it names `mean_error`'s number. **It is left where it is**, because raising it to the derived 12.02%
takes the corpus from 905/68/786 to **1121/309/329**: the bound also decides whether two references
form a consensus, so 457 pages leave `ambiguous` and 278 arrive newly contradicted. **The whole diff
under `crates/` is `tests/oracle.rs` and `raster-compare`'s doc comments**, so no raster changed: the
oracle's 1794 verdict lines are identical to the round's own baseline but for two timing lines, and
every other gate reproduced. Step 7 **not owed**. ADR 0243, `doc/todo/12`.

**The four-hundred-and-ninth changed `interpret` and owed a demonstration that no existing caller
noticed.** ADR 0245 adds `Command::Delegate(WidgetAppearances)` — §6.3.2.2's "unless otherwise
instructed", reaching interpretation through `ViewState` where the magnification already sits — so a
native host can have the page without the pictures of the fields it draws itself. Summary numbers
are the wrong instrument for that claim, because two different display lists can rasterise to the
same verdict, so the **artefact** was compared: `pdf-model`'s `display_list_digest` example prints
page one's command count, `Debug` length and hash for every corpus document, and `89de636` in a
worktree against this tree gives an **empty diff over 975 lines**. Read that beside the fact that the
instrument caught a real difference first — 96 documents differed until trap 10's
`pdf-sandbox-worker` was built in *both* trees, `bug1815476.pdf` at 1490 against 1522 commands — which
is the only reason an empty diff means anything. Every gate below reproduced except the three counts
in the table (tests 1430 → **1435**, citations 5758 → **5812**, quotations 554 → **557**);
`doc/todo/00`'s step 7 is **not owed**, because no existing caller's display list moved. Run beyond
§2 and claimed: **the window under `Xvfb`**, twice over one binary, which is where the change is
visible at all.

**And the four-hundred-and-tenth ran it with two new crates in the workspace and every line below
reproduced except three counts.** It built `crates/viewer-qt` — the Qt 6 host, a C++ bridge and a
`build.rs` that runs `moc` — and `crates/viewer-host`, which is four modules *moved* out of
`viewer-gtk` rather than written; nothing that touches a page changed, so the test count moved by
its nineteen (1435 → **1454**) and the citations by 70 (5812 → **5882**), while the quotations stayed
at 557 and the corpus's 974 with 65 incomplete, the oracle's 1794 pages, quorra's, the text gate's
99.2% (24043/24243 words), the dates, the XMP and the JPEG 2000 lines are what the table says.
`doc/todo/00`'s step 7 is **not owed** and no raster can have changed: the whole diff under
`crates/` is one new directory, one moved directory and their two manifests. Four things beyond §2
were run and are claimed: **`cargo deny check`** — *advisories ok, bans ok, licenses ok, sources ok*
with `cxx`'s 23 packages in the graph and no new exception — **all three cross-target checks**,
which pass unmoved because they name their packages with `-p` and `viewer-qt` is excluded for the
same reason `viewer-gtk` is (asking for it says `failed to find tool "lib.exe"`), **the window under
`Xvfb`**, which is this round's whole point, and **both hosts' copies timed side by side**, which is
what corrected ADR 0244's ≈3.2 GB/s into a first-frame number. **Not run and therefore not
claimed**: the twelve fuzz targets, which have nothing to catch here — no parser, no decoder and no
document code changed.

**And the four-hundred-and-eighth ran the whole sequence with a new crate in the workspace, and
every line below reproduced except three counts.** It built `crates/viewer-gtk`, the GTK4 host — a
new crate, a new binary and a new *dependency*, and nothing that touches a page — so the test count
moved by its ten (1420 → **1430**), the citations by 98 (5660 → **5758**) and the quotations by one
(553 → **554**), while the corpus's 974, the oracle's 905/68/786, quorra's 912/36/9/17, the text
gate's 99.2%, the dates, the XMP and the JPEG 2000 lines are what the table says. `doc/todo/00`'s
step 7 is **not owed** and no raster can have changed: the whole diff under `crates/` is a new
directory nothing else depends on. Three things beyond §2 were run and are claimed: **`cargo deny
check`** — *advisories ok, bans ok, licenses ok, sources ok* with `gtk4` and its 41 packages in the
graph — **both cross-target checks**, which pass unmoved because they name their packages with `-p`
and `viewer-gtk` is deliberately excluded from them (ADR 0244), and **the window under `Xvfb`**,
which is this round's whole point: a page drawn at 96.5 ms median of five, two pages turned, a layer
switched from a native check button moving 6656 of 474 721 pixels, 76 controls placed over 67
fields, a field typed into, an attachment extracted and §7.6.4.1's password prompt opening an
encrypted document. **Not run and therefore not claimed**: the twelve fuzz targets, which have
nothing to catch here — no parser, no decoder and no document code changed.

**And the four-hundred-and-twelfth ran the whole sequence with one variant's shape changed across
six consumers, and every line below reproduced except three counts.** It changed
`viewer_core::Edit::SetField`'s value and touched `pdf-model`'s view state, the confined transport
and all three hosts — so the test count moved by its eight (1473 → **1481**), the citations by 52
(5913 → **5965**) and the quotations by four (561 → **565**), while the corpus's 974 with 65
incomplete, the oracle's 905/68/786 **line for line**, quorra's 912/36/9/17, the text gate's 99.2%
(24043/24243 words), the dates, the XMP and the JPEG 2000 lines are what the table says.
`doc/todo/00`'s step 7 is **not owed**: nothing in the interpreter or the rasteriser changed, and no
gate sets a field's value, so no gate can reach the code this round wrote. Two things beyond §2 were
run and are claimed: **the `confined_wire` fuzz target**, because the transport's decoder gained a
case — 13 632 129 executions in 181 s, no crash — and **both hosts under `Xvfb`**, which is where the
change is visible at all. **Not run and therefore not claimed**: the other eleven fuzz targets, which
have nothing to catch here, and `cargo deny`, because no dependency moved.

**And the four-hundred-and-thirteenth ran the whole sequence and every gate reproduced except the
three counts its own work moved** — tests 1481 → **1482**, citations 5965 → **5974**, quotations
565 → **567**. It was a sweep round: eleven sweeps over `ledger.toml` and over `crates/`, fourteen
ledger rows and eleven source comments corrected, and one behaviour added (`viewer-ui` copies the
page selection on `c`, asking §14.8.2.5's logical order first). The corpus's 974 with 65 incomplete,
the oracle's 905/68/786 **line for line** with the undiagnosed list empty, the text gate's 99.2%
(24043/24243), the dates, the XMP and the JPEG 2000 lines are what the table says. **Quorra was run
four times** rather than once, because a stale log from a previous session was momentarily read as a
new verdict — 912/36/9/17 every time, on `RADV STRIX1`. `doc/todo/00`'s step 7 is **not owed**:
nothing in the interpreter or the rasteriser changed and no gate presses a key. **Not run and
therefore not claimed**: the twelve fuzz targets, which have nothing to catch — no parser, no
decoder and no document code changed — `cargo deny`, because no dependency moved, and the window
under `Xvfb`, because the one thing to see there is a `println!`. ADR 0249.

**And the four-hundred-and-sixth, which moved no verdict and forty-three lines.** It worked the
contradicted list and found the defect in the *instrument*: a contradicted page's line was measured
against whichever reference had the largest tile — which need not be one the verdict rests on — and
printed **three** of `Tolerance::accepts`' **four** bounds. Thirty of the 68 lines therefore showed
every visible number inside the printed bound, and `smask_luminosity_oob_transfer.pdf` showed 27.02
against a bound of 1.11 where our distance from the pair that decides it is 1.25. **The whole diff
under `crates/` is `tests/oracle.rs`**, so no raster can have changed: the oracle's 1794 verdicts,
the corpus's 974, quorra's 957 and the text gate's output are identical, and `doc/todo/00`'s step 7
is **not owed**. It was run once anyway and holds for the ninth consecutive time, with the same five
names in the same order as the three-hundred-and-ninety-seventh's. ADR 0242.

**And the four-hundred-and-fifteenth ran the whole sequence and moved one number no round had
moved this way before: the corpus's incomplete count, in both directions at once.** It read the
blending colour space from where the standard puts it — §11.4.7's page group as the root and
§11.6.6's inheritance below it, a group's own `/CS` binding only where the group is isolated — so
`issue14200.pdf` lost a report that was never true and four documents whose *pages* composite in
`/DeviceCMYK` gained one they had always owed (65 → **68** incomplete; ADR 0251). **No raster
changed and that is checkable**: the whole diff under `crates/src` is report paths, and the oracle's
verdict counts are identical to the row below — 905 / 68 / 786 — with only the complete/incomplete
split moving, 1693/101 → **1690/104**. Quorra 912/36/9/17, the text gate's 99.2% (24043/24243), the
dates, the XMP and the JPEG 2000 lines are unmoved; tests 1491 → **1493**, citations 5980 → **6032**,
quotations 569 → **576**. `doc/todo/00`'s step 7 was run over all 786 ambiguous pages and holds for
the eleventh consecutive time — the same five names past −1 in the same order to the thousandth as
the three-hundred-and-ninety-seventh's run, with four labels newly `[incomplete]` and no number
moved. **Not run and therefore not claimed**: the twelve fuzz targets, which have nothing to catch —
no parser, no decoder and no document code changed — `cargo deny`, because no dependency moved, and
the window under `Xvfb`, because nothing a person presses is involved.

**And the four-hundred-and-sixteenth ran the whole sequence and moved nothing a raster is made
of.** It answered `doc/todo/48`'s census — what the fourteen specification PDFs' annotations are,
since the Markdown conversion the conformance gate reads dropped every one of them — and the answer
is that in three of the documents **the annotations are the errata**: 11 462 annotations in ISO
32000-2 (not the 882 the item recorded, which were `/Annots` *arrays*), **434 strikeouts over 4038
words across 252 sections**, each with a `Caret` carrying the replacement and a `/Subj` naming its
issue. `doc/md/` therefore presents **79 struck passages as the standard's current text**, and
three of this tree's own rustdoc quotations were quoting one — §7.9.4, §7.5.5 and §14.7.6.1, all
annotated in place here with their ledger rows. `tools/spec-errata` is the sidecar that reads them
back and **is not a gate and not a test**, because the checker must keep comparing quotations
against a conversion this project did not make. Every gate reproduced but the three counts this
round moved: tests 1493 → **1495**, citations 6032 → **6051**, quotations 576 → **577**. **Not run
and therefore not claimed**: the twelve fuzz targets, since no parser, decoder or document code
changed; `cargo deny`, since no dependency moved; the window under `Xvfb`, since nothing a person
presses is involved. ADR 0252.

**And the four-hundred-and-seventeenth read all seventy-nine of them, and one *is* about a raster.**
`doc/errata-read.md` carries a verdict apiece. Two clauses were implemented from retired text and a
third still is. **§12.5.2's `/BM` was being ignored on every stored appearance stream**: the closing
sentence that listed `BM` among the entries a reader ignores is struck — the list keeps `CA` and
`ca`, loses `BM` and gains `MK` — which leaves §12.5.5 and Table 166's own unconditional `/BM` row
with nothing against them, so `annotation::blend_mode` now reads it on both paths and the ledger's
recorded §12.5.2-against-§12.5.5 contradiction is half settled by an erratum this project could not
see. **§14.13.5's marked-content property list is keyed `/MCAF`**, not the `/AF` this tree inferred
from the tag, so a conforming PDF 2.0 file stated associated files and got an empty list with no
report. **§8.9.5.4's alternate-image algorithm is rewritten and is not corrected here**, because the
amended step a) reads as terminal and would leave the amended d) unreachable — recorded with the
carets' own words rather than guessed at.

**The instrument that named the seventy-nine was blind to 72 more.** Its comparison kept whitespace,
and both sides of it are extractions of the same glyphs by different programs — PDF positions glyphs
and not words, so one writes `inthe` where the other writes `in the`. Comparing with the spaces
taken out takes the count from **79 to 151**, and one of the 72 is the §12.5.2 sentence above. The
`/State` question is settled too: **no note in any of the fourteen documents is `Rejected`,
`Cancelled` or `None`** — 827 `Completed` and 265 `Accepted` — so session 416's filter was narrower
than the evidence rather than wrong, and the mirror-image mistake cannot be made from these files.
Every gate reproduced but the three counts this round moved: tests 1495 → **1497**, citations
6051 → **6070**, quotations 577 → **575** (three blockquotes of retired text replaced by prose, one
added). **Not run and therefore not claimed**: the twelve fuzz targets, since no parser, decoder or
document code changed; `cargo deny`, since no dependency moved; the window under `Xvfb`, since
nothing a person presses is involved. `doc/todo/00`'s step 7 was **not** re-run: `/BM` on a stored
appearance is a change to what gets drawn, and **the corpus does not exercise it** — the oracle's
1794 verdict lines and quorra's `912 agree, 36 differ, 9 refused, 17 not comparable` are identical
to the previous round's, which is the measurement rather than an expectation. ADR 0253.

**And the four-hundred-and-eighteenth read the other fifty-four and swept the population nothing
checked.** All **120** distinct passages `spec-errata check` names now carry a verdict
(`doc/errata-read.md`). **A third clause was implemented from retired text**: Errata Collection 3
replaces §9.6.4's two-place rule for a Type 3 glyph description's resources with a pointer to
§7.8.3, whose amended four-step search starts at "the stream dictionary of that glyph description
content stream" — a step `Type3Font::resources` did not have, so a glyph stream stating its own
`/Resources` was read against the font's or the page's, and a name that resolves to nothing draws
nothing and says nothing. **ADR 0249's 977 unchecked ledger spans are swept**, with none of the
syntax that ADR priced: a span matching a sentence an erratum struck out is the standard's by
construction, whatever else the ledger quotes. The sweep found a **third** population on the way —
quotation marks inside ordinary rustdoc *prose*, which `CLAUDE.md` binds exactly as hard as a
blockquote and which the gate's `> ` scanner walks straight past — and prose is the worst of the
three: **eleven stale quotations, 1 blockquote, 6 prose, 4 in ledger notes**, over 18, 39 and 21
landings respectively. Two of the six are the standard-14 `shall`s the round before corrected in
three files and missed in two others, and **four of the eleven were in the bucket `check` prints as
"a repeated phrase rather than a finding"** — a clause heading straddling a page break puts a real
landing there, which is the third round running that `Landing::in_clause` has proved to be a sort
order rather than a verdict. **Two
facts about the instrument**, both worth carrying: a struck passage `doc/md/` still shows is not
always a retired one — §7.5.4's Issue #113 deletes one of *two identical printed copies* of the
same sentence, and nothing in an annotation says which — and `check`'s two questions have different
populations, since three of the errata acted on here are not among the 151 at all. Every gate
reproduced but the two counts this round moved: tests 1497 → **1498**, citations 6070 → **6087**;
quotations stay at **575**. **Not run and therefore not claimed**: the twelve fuzz targets, since no
parser or decoder changed; `cargo deny`, since no dependency moved; the window under `Xvfb`, since
nothing a person presses is involved. `doc/todo/00`'s step 7 was **not** re-run: no corpus document
gives a Type 3 glyph description its own `/Resources`, and the oracle's 1794 verdict lines and
quorra's counts are identical to the previous round's. ADR 0254.

**And the four-hundred-and-nineteenth ran the whole sequence and moved the two counts a new report
is supposed to move, and nothing else.** It made a `Do`, a `gs` or an `scn` naming a resource
§7.8.3's current resource dictionary does not define say so — Table 86's two `shall`s on the
operand, Table 56's and §8.7.3.2's — which had been the one silence left among the six categories
`Interpreter::resource` and `resource_entry` look names up in. Corpus 68 → **70** incomplete, both
of them new reports on files whose producer asked for a mark the file cannot carry
(`issue6541.pdf`'s pattern names an `/XObject` only the page defines and the object is an empty
stream; `issue8702.pdf`'s form XObject is written inside an object stream, which §7.5.7 forbids from
holding one, so `poppler` refuses it too). Oracle complete 1690 → **1688** and incomplete 104 →
**106**, with **every verdict count identical** — 905/68/786 — because both pages *agree* and simply
left the complete column. Quorra 912/36/9/17, the dates, the XMP and the JPEG 2000 lines are
unmoved; the text gate's numerator and denominator each fell by the same 56 words, so 99.2% holds;
tests 1498 → **1506**, citations 6087 → **6115**, quotations 575 → **579**. `doc/todo/00`'s step 7
is **not owed** and it is provable rather than argued: the whole diff under `crates/*/src` is
`self.note(…)` in front of a `return` that already existed. **The report found a defect in this
tree on its first run** — `soft_masks.rs`'s alpha-mask fixture named two `/ExtGState`s its group's
`/Resources` did not define, so §11.5.2's test asserted one number twice about a difference the file
did not contain. **Not run and therefore not claimed**: the twelve fuzz targets, since no parser or
decoder changed; `cargo deny`, since no dependency moved; the window under `Xvfb`, since nothing a
person presses is involved. ADR 0255.

**And the four-hundred-and-twenty-first ran the whole sequence with a new tool in the workspace and
every line below reproduced except three counts.** It built `tools/pdf-retrieve` — `doc/todo/36`'s
three missing joins, a JSON-on-stdout CLI over readers that already existed — and changed one thing
that touches a page's *reading* rather than its drawing: `pdf_model::structure::Tree::walk` had a
bound of 65 536 items over the whole tree, so §14.8.2.5's logical order on ISO 32000-2 was a prefix
of the tree with nothing said, and its visited set was a `Vec<Dictionary>` searched linearly, so the
walk was **16.8 s**. It is 129 389 items in **151 ms** now, bounded at 2²⁰ and *reported*. So the
test count moved by its ten (1515 → **1525**), the citations by 63 (6127 → **6190**) and the
quotations by two (580 → **582**), while the corpus's 974 with 70 incomplete, the oracle's
905/68/786 **line for line** with 1688/106 and the undiagnosed list empty, quorra's 912/36/9/17, the
text gate's 99.2% (23987/24187 words), the dates, the XMP and the JPEG 2000 lines are what the table
says. `doc/todo/00`'s step 7 is **not owed** and it is provable rather than argued: nothing in the
interpreter or the rasteriser changed — the whole diff under `crates/*/src` is a new `retrieval`
module nothing on a drawing path calls, and `structure.rs`, which no display list reads. **Not run
and therefore not claimed**: the twelve fuzz targets, since no parser or decoder changed; `cargo
deny`, since no dependency moved; the window under `Xvfb`, since nothing a person presses is
involved. Two things beyond §2 *were* run and are claimed: **`spec-errata check`**, which names 151
struck passages and 28 quotations landing in the clause they cite — **none of them in this round's
files** — and the substitution-cost measurement `doc/todo/48` was missing. ADR 0257.

**And the four-hundred-and-twenty-second ran the whole sequence with three new *corpora* in the
tree and every corpus-scale line below reproduced exactly.** It answered `doc/todo/03`: three
shallow submodules under `doc/corpora/` — `pdf20examples` (CC BY-SA 4.0), `pdf-differences`
(Apache-2.0) and a **partial, sparse** `pdfbox` (Apache-2.0, 12 MB of a 118 MB repository) — and
`tools/safedocs`, a fetcher for the one corpus that cannot be a submodule: SafeDocs'
`CC-MAIN-2021-31` is 7 933 ZIP archives of a gigabyte each and close to 8 TB uncompressed. The
owner's constraint — clarified mid-session as **"impossible by accident", not "impossible"**,
because the mobile connection is the exception and unmetered fibre is the rule — is met
structurally: the archive is addressed a member at a time and never as an object, a plan is
resolved through its own central directory (a `HEAD` and 182 KiB), a fetch is *one contiguous
byte range*, nothing moves without `--download`, and a plan over 32 MiB is refused **in bytes and
in the `--budget-mb` that would admit it** — the budget has no ceiling and `--all` takes every
member, so `--all --budget-mb 1610 --download` is the whole 1.6 GiB archive and is a sentence
somebody typed. Every member is checked against the CRC-32 its own archive records. **No package was added to `Cargo.lock`** — the transport is `curl` as a
subprocess and the ZIP reading is 200 lines in tree (ADR 0258, `doc/third-party-data.md`).

**The short test was run and it found a silence on its first pass.** 24 documents fetched from
archive `0000` (19.9 MiB in 13 s) and 108 more from the submodules, surveyed with
`safedocs survey`: 7/7, 30/37, 63/64 and 22/24 complete, with every report but one belonging to a
population `doc/todo/21` or `23` already names. The one that did not was
`UnknownFilter-PageContentStream.pdf`, which came back **complete with zero commands** — its
content stream object's dictionary ends with one `>` where §7.3.7 requires two, so §7.3.10 makes
the reference null and §7.3.9 makes null an absent entry, and Table 31 makes an absent `/Contents`
a blank page. Conforming, and silent, which is a different thing: `ContentIssue::Unreachable` now
says it, and **the pdf.js corpus's count did not move by one document**, which is the sharpest
statement available of why a second corpus was worth a session. Tests 1525 → **1539**, citations
6190 → **6203**, quotations 582 → **583**; the corpus's 974 with **70** incomplete, the oracle's
905/68/786 with 1688/106 and the undiagnosed list empty, quorra's 912/36/9/17, the text gate's
99.2% (23987/24187), the dates, the XMP and the JPEG 2000 lines are what the table says.
`doc/todo/00`'s step 7 is **not owed** and it is provable rather than argued: the whole diff under
`crates/*/src` is `Page::content_with_report` keeping each `/Contents` entry as written beside the
resolved one and pushing a report where a reference lands on null — the bytes it returns are
byte-for-byte what they were, and the oracle's 1794 verdict lines are identical. **Not run and
therefore not claimed**: the twelve fuzz targets, since no parser or decoder changed; `cargo
deny`, since no dependency moved; the window under `Xvfb`, since nothing a person presses is
involved. ADR 0258.

**The four-hundred-and-twenty-third took `doc/todo/03`'s item 4 and got a defect back.**
`doc/corpora/pdfbox` carries `*.pdf.txt` and `*.pdf-sorted.txt` beside **40** of its PDFs —
Apache PDFBox's own `PDFTextStripper` output, checked in — so `text_extraction.rs` now has a
*frozen* second reference beside the `pdftotext` it runs live, in the same file and by the same
rule. **It costs 0.4 s and no new line in `doc/todo/02` §2**, which already runs every ignored
test in that binary; its reference is a file rather than a process, where the pdf.js gate spends
30 s of its 31 waiting for `pdftotext`. The first run was 99.8% (14254/14281) with **5 below the
0.90 floor**, and reading all five before ratcheting is what the round was for: **three were one
defect.** §9.10.2's third method excludes an `Identity-H` font over `Adobe-Identity` **by name**
— "(except Identity -H and Identity -V )" — so its `/ToUnicode` is its only method, and where
that answers for some codes or none, every method has failed and the clause's own next sentence
is in force. `LoadedFont::text_from_program` refused a composite font outright and so declined a
permission whose precondition the file had met. The route is the same data one step longer: the
`CMap` gives a CID, §9.7.4.2's `/CIDToGIDMap` gives the glyph, the program names it. **The
pdf.js gate went 23987 → 24003 of `pdftotext`'s 24187 words and its named list 25 → 23**, with
no document moving the other way, and one of the two that left — `issue16553.pdf` — had been
recorded there as undiagnosed for **357 sessions**. Of the four that remain against PDFBox, two
are right-to-left text in painting order and in presentation forms (§14.8.2.5.1, and no document
in the 108 new ones writes §14.8.2.5.3's `/ReversedChars`, measured), and **two are a choice**:
their `/ToUnicode` is an Identity CID `CMap` rather than the `bfchar`/`bfrange` mapping §9.10.3
requires, their embedded programs carry neither a `cmap` nor a `post` table, and PDFBox reads the
two-byte code itself as a Unicode value where this tree declines — `text_from_the_code`'s
argument is that §9.6.5's encodings are one byte per code, and a guard added this session says
so. A SafeDocs chunk was taken from archive `3500` (16.8 MiB, 24 documents, every CRC-32
matched): 22 complete, and **both reports belong to populations already named**, so nothing was
promoted and the budget's running total stays at 0 MB. Tests 1539 → **1542**, citations
6203 → **6223**, quotations 583 → **584**; every corpus-scale count identical — the corpus's 974
with **70** incomplete, the oracle's 905/68/786 with 1688/106 and the undiagnosed list empty,
quorra's 912/36/9/17, the dates, the XMP and the JPEG 2000 lines. `doc/todo/00`'s step 7 is
**not owed**, and it is provable rather than argued: the whole diff under `crates/*/src` is
`LoadedFont::text_from_program`, whose two other production callers — `substitutes_notdef` and
`codes_by_character` — both return early for anything but a simple font, and whose simple-font
branch reads the same two sources in the same order it did before. Drawing cannot see it, and
the run agrees: corpus 70 incomplete, oracle 905/68/786 over 1688/106, quorra 912/36/9/17, every
one of them the number the round before printed. **Not run and
therefore not claimed**: the twelve fuzz targets, since no parser changed; `cargo deny`, since no
dependency moved; the window under `Xvfb`. ADR 0259.

**The four-hundred-and-twenty-fourth measured the owner's own question and found the answer had a
false premise.** `doc/todo/49` item 1 said `pdf_syntax::Document` being `!Sync` behind `RefCell`
caches was "the actual blocker" for a parallel search. **It was not**, and the measurement says so
plainly: **N documents in N threads needs nothing from `pdf-syntax`** and has been available the
whole time — 1023 pages of ISO 32000-2 in **1.18 s against 6.11** on 24 threads. What `!Sync`
blocked was the *cheaper in memory* arrangement, not the faster one. Before touching anything, a
**temporary counter build** asked what the five caches are for: `Document::get` is called **829
times a page**, answers **92.7%** of a cold sweep from the cache and 100% of a repeat, and **a fully
warm object cache is worth 5.5% of the wall clock** — 6.15 s against 5.81 — so a sweep's seconds are
not in the cache at all. It also found a module comment that was false: `decoded_stream_data` runs
12 717 times over one sweep and **11 975 times over the second sweep of the same document**, which
is a filter chain re-run and not the cache the comment claimed. **The `RwLock` swap ships** at
**+0.021%** of `callgrind_interpret`'s instructions (2 208 807 721 → 2 209 269 060), **−0.14%** of
`callgrind_open`'s, a cold sweep inside its own spread over seven interleaved samples apiece, and a
launch whose `document joined` did not move — the whole-launch figure is deliberately not quoted
against itself, because `EventLoop::new` ran 28 to 55 ms across twenty-four launches and which end a
run landed on depended on whether the X server had just gone idle, which reversing the order
reversed. The one part of the swap that was not mechanical is the `loading` set, now **per thread**:
shared, it would answer §7.3.10's null to the second of two threads that wanted one object at one
moment, which is a wrong answer produced by timing. **The parallel search itself is declined**, on
memory rather than on taste: **625 MB shared or 966 MB per-thread of peak resident against 225**,
where the owner's own bar is that 1 GB is definitely too much — and `doc/todo/49` item 3, the API
that hands a pool in, is now a memory argument rather than a lock. Spec track: §7.5.7's ledger row
gained the two sentences reading it again found — a compressed object that is solely a reference,
tolerated and bounded rather than enforced, and an object stream inside an object stream, refused
outright — and `spec-errata` says no quotation in §7.5.7's or §7.5.8's rows quotes struck text.
Tests 1542 → **1544**; ledger 875 rows at 401/251 unmoved, citations and quotations unmoved. Every
corpus-scale count identical: corpus 974 with **70** incomplete, oracle 905/68/786 over 1688/106
with the undiagnosed list empty, quorra 912/36/9/17, text 99.2% (24003/24187) and PDFBox 99.8%
(14257/14281), dates, XMP and JPEG 2000 line for line. `doc/todo/00`'s step 7 is **not owed**: the
whole diff under `crates/*/src` is `document.rs`'s five field types and two helpers, which change no
answer — the oracle's 1794 verdicts and quorra's 957 say so. **Not run and therefore not claimed**:
the twelve fuzz targets, since no parser changed; `cargo deny`, since no dependency moved; a key
press under `Xvfb`, since nothing a person presses is involved — the window was launched
twenty-four times for the timeline and nothing else. ADR 0260.

**The four-hundred-and-twenty-fifth spent 5.4% of the owner's 50 GB and got a crasher back.**
`doc/todo/03` asked for a **stratified** sample on the premise that "[f]iles inside one archive
are correlated — they come from one neighbourhood of one crawl". The sampling rule was
**archive `50 + 100k` for k = 0 … 78**, the first 24 members of each: **2731.0 MiB of member byte
ranges and 14.1 MiB of central directories, 2.68 GiB, 1896 documents, 79 fetches, 0 failures,
every CRC-32 matched**. **The premise is false and the sample is what disproved it**: the corpus
is the whole crawl sorted by **SHA-256** and cut into 7933 equal pieces — over 1944 members a
file's own number and its digest as a fraction of 2²⁵⁶ agree to **2.6 × 10⁻⁴**, which is the
fluctuation 7 932 878 uniform order statistics have by construction — so an archive is a hash
bucket, any window anywhere is an unbiased sample of all of it, and a round may go deep without
paying 182 KiB of directory per archive for spread it does not need.

**The survey: 1896 documents in 42.1 s: 4 unopenable, 1 locked, 0 encrypted beyond us, 3
pageless, 86 incomplete, 0 slow**, 862 codes reaching no glyph in silence over 12 documents — a
baseline for that chunk and never a ratchet. **Nothing failed to open for a reason that is this
tree's**: all seven unusable documents were opened by hand and are crawl artefacts, four HTML
pages saved under a `.pdf` name and three PDFs the origin server truncated at about a kilobyte.
**And the failure modes are overwhelmingly already-named ones, which is the round's number rather
than its disappointment: 67 of the 86 are §11.4.7's page-group blending space** — 3.5% of the web
against 0.7% of the 974 — which makes `doc/todo/23`'s standing item the largest correctness gap
this tree has against real files by a factor of six over everything else together. 7 more are
`doc/todo/21` §3 and 4 are §11.4.4; the remaining 8 are singletons.

**One of the eight was one clause with two defects.** §7.10.4's Table 41 bounds k **nowhere** —
`/Functions` is "( Required ) An array of k , 1-input functions that shall make up the stitching
function" and the only value singled out is "The value of k may be 1" — while the same subclause
fixes the other quantity outright, "Domain shall be of size 2 (that is, 𝑚  =  1 )". One constant
was serving both, and it was documented as bounding a component count: a 256-stop gradient is
k = 255, and `2750009.pdf` lost **four shadings** to it, with a present 510-number `/Encode`
silently replaced by the identity behind that. **And a `/Functions` array naming its own object
was a stack overflow** — nothing in the standard forbids one — which killed
`target/pdf-retrieve`, a *shipped* binary, on a 720-byte file. That is this project's **first
crasher** — **696 bytes rather than 720**, measured off the fixture's own construction in the four-hundred-and-twenty-eighth — `crates/pdf-model/tests/hostile_functions.rs` is the six-test regression `CLAUDE.md`
requires, and its fixtures are **generated**, so the promotion budget is still at **0 MB**.
**Thirteen fuzz targets and not one could have found it**: only `confined_wire` and
`variable_text` reach `pdf_model::interpret`, and neither can build a shading. **The conclusion
held and both of its reasons were wrong** — the four-hundred-and-twenty-eighth asked `nm` and
found `pdf_model::interpret` in *one* of the thirteen binaries rather than two (`confined_wire`
fuzzes the wire decoders, and the worker that interprets is on the other side of a pipe), and
`cargo-fuzz` was installed here throughout. ADR 0264 and the `page` target. Tests 1544 →
**1550**, citations 6223 → **6240**, distinct tables cited 217 → **218**; ledger 875 rows at
401/251 unmoved and §7.10.4's row keeps `implemented` with the reading added. Every corpus-scale
count identical: corpus 974 with **70** incomplete, oracle 905/68/786 over 1688/106 with the
undiagnosed list empty, quorra 912/36/9/17, text 99.2% (24003/24187) and PDFBox 99.8%
(14257/14281), dates, XMP and JPEG 2000 line for line. `doc/todo/00`'s step 7 is **not owed** and
it is provable: the whole diff under `crates/*/src` is `function.rs`'s bounds, and the only
document in any gated population whose drawing could change would have to state a stitching
function of more than 64 subfunctions — the oracle's 1794 verdicts and quorra's 957 pages are
identical, which says none does. **Not run and therefore not claimed**: the thirteen fuzz
targets, since `cargo-fuzz` is not installed here and none of them reaches this code anyway
(**the first half of that clause is false** — see the correction above);
`cargo deny`, since no dependency moved; the window under `Xvfb`. ADR 0261.

**And the four-hundred-and-twenty-seventh ran the whole sequence twice, before and after, and
changed one line of 1794 and one of 786.** It gave §11.7.2 the conversion *into* §11.4.7's blending
space that ADR 0262 named as the blocker on 61 of 69 web witnesses, and the clause that decided it is
the paragraph *above* the black-generation bullets ADR 0262 read: §11.7.5.3 names the conversion's
**target** — "in the transparent model it may instead be the group colour space of a transparency
group into which an object is being painted" — where the opaque model's is "the native colour space
of the output device". Same conversion, different target, so it sits on §10.3's branch by §10.4.2.1's
ranking and is a **right inverse of the ink cube**: one colour model per page and no boundary between
two, which is the defect ADR 0262 photographed. The cost is a *gamut* rather than a colour — recorded
as a choice, because §11.7.5.3 says an intent maps "taking into account the target space's colour
gamut" and ISO 15076-1 says how. **Web sample 82 incomplete with 62 naming the space → 34 with 13**,
56 of 69 drawn in ink rather than 7; corpus 69 → **68**; oracle 1689 → **1690 complete** with every
verdict count identical; quorra 911/36/10/17 → **910/36/11/17**; text, PDFBox, dates, XMP and
JPEG 2000 line for line. **The instrument was the pictures**: 61 witnesses rendered before and after,
12 byte-identical, and `0950007.pdf`'s green panel — the one ADR 0262 refused to ship grey — is
`#007A61` on both sides to the byte. Against `poppler` 46 of 61 move *away* and 3 toward, mean
3650.8 → 3711.3 of 65535, which is recorded rather than acted on: `poppler` composites these pages on
the device's three components, so a page group's blending space cannot move it. Step 7 over all 786
ambiguous pages, before and after: one line differs and the alarm holds for the thirteenth
consecutive time. **Not run and therefore not claimed**: the thirteen fuzz targets, since no parser
or decoder changed; `cargo deny`, since no dependency moved; the window under `Xvfb`, since nothing
a person presses is involved. ADR 0263.

**And the four-hundred-and-twenty-eighth asked why thirteen fuzz targets missed a 696-byte
crasher, and the linker answered.** `CLAUDE.md` puts fuzzing among the non-negotiables and
session 425's first crasher came from a *download*, so this round audited the instrument instead
of adding to it. **`nm` finds `pdf_model::interpret` in one of the thirteen binaries** — twelve do
not contain the interpreter at all, and the thirteenth, `variable_text`, calls it on a page with
no `/Resources`, so no fuzzer here had ever built a shading, a pattern, a form XObject or a
function of any type. Three of the thirteen *could* have written the crasher's bytes
(`document`, `forms_data` and `crypt` hand a whole file to `Document::open`) and none could
execute them. libFuzzer's own numbers say the same thing twice over: `variable_text` and the new
target link the same 238 744 / 237 171 instrumented edges, and their corpora cover **6483 against
28 535** — the difference is the *input shape* and nothing else, because 5813 evolved seeds inside
a fixed page reach 2.7% of what they link. **Both of the reasons the gap had been left open were
false.** `cargo-fuzz` has been in `~/.cargo/bin` since 26 July; it is not on `PATH`, so `which`
reports a false negative, and two rounds wrote "not installed here" from one. And ADR 0261's
"`confined_wire` reaches `interpret`" is wrong — that target fuzzes the wire decoders and the
worker is on the other side of a pipe. **`fuzz/fuzz_targets/page.rs` is the fourteenth target**: a
whole document, page one, through `interpret`, with a purity check (the same bytes must give the
same geometry digest, text and glyph count) that nothing in the tree had ever run on an input
nobody wrote on purpose. **It was verified against the crasher historically** — a worktree at
`5fbf72a`, one input, no mutation, 2044 frames of `parse_stitching` and an ASan stack overflow;
the same bytes take 4 ms today. `fuzz/seed_page.py` seeds it from every document on disk (1944 +
108 + 974 → **1882** under the target's 256 KiB ceiling, `cmin` to 1535) and prints what they
state, because a corpus stating no shading seeds nothing about §8.7.4.5: 100 `/Function`, 62
`/Pattern`, 58 `/Shading`, 332 form XObjects, 234 `/SMask`. **50 373 runs over six forks in 3411 s:
0 crashes, 0 OOMs, 1 timeout — and the timeout is the sanitiser's**, 0.70 s in `target/pdf-retrieve`
against 19.43 s in the fuzz binary on an idle machine. **No budget was touched.** The same seeding
was measured on an existing target for a number the tree has never had: `document`, 7730 evolved
corpus files at **3010** edges, goes to **4351 (+44.6%)** on adding the same 1884 real documents,
then to **4568** after its own 50 000 runs in 515 s. Two things nothing was looking at were fixed
because the fuzz crate is its own workspace and neither §2 gate reaches it: three targets were not
rustfmt-clean and `x509.rs` had a clippy warning; `doc/verify.md` now carries the two commands.
**Citations 6345 → 6346** — `page.rs` cites §7.3.10 and `tools/conformance`'s `SOURCE_ROOTS`
includes `fuzz` — and **every other gate is line for line**: tests 1561/11 skipped, corpus 974 with
**68** incomplete, oracle 1794 pages at 1690/104 with 905/68/786 and the undiagnosed list empty,
quorra 910/36/11/17, text 99.2% (24003/24187) with 23 below the floor, dates 1514/1545, XMP
318/319, JPEG 2000 14 byte-identical, ledger 875 rows at 401/252/18/83/8/113 and quotations 592.
`doc/todo/00`'s step 7 is **not owed**: the whole diff under `crates/` is empty. **Not run and
therefore not claimed**: the other twelve fuzz targets, since nothing they cover changed; `cargo
deny`, since no dependency moved; the window under `Xvfb`. ADR 0264.

**The four-hundred-and-thirty-first ran §2 whole and every line is unchanged** — tests 1568/11
skipped, corpus 974 with 68 incomplete, oracle 1794 at 1690/104 with 905/68/786 and the undiagnosed
list empty, quorra 910/36/11/17, text 99.2% (24003/24187) with 23 below the floor, PDFBox 99.8%
(14257/14281), dates 1514/1545, XMP 318/1, JPEG 2000 14/13/3, ledger 875 rows at 403/250/18/83/8/113
with 595 quotations and 217 tables — because **no pixel moved**: its whole diff under `crates/` is
comments plus one string comparison in a tool. `doc/todo/00`'s step 7 over the ambiguous bucket is
therefore not owed either, on the four-hundred-and-sixth's argument that a before/after pair would
compare a file with itself; the same loop was run over the **contradicted** list instead, for the
first time, and found nothing unexplained on it (ADR 0267, `doc/todo/00` §7). **Not run and
therefore not claimed**: the fourteen fuzz targets, since no parser changed; `cargo deny`; the
window under `Xvfb`.

| gate | what it printed | where |
|---|---|---|
| tests | `1561 tests run: 1561 passed, 11 skipped` — the tenth skip is the four-hundred-and-seventh session's `the_fixed_bounds_against_the_references_own_spread`, which derives the oracle's own bounds and is run explicitly — and `cargo test --workspace --doc` **1 passed** beside it, so `cargo test --workspace` reports **1551**. The four-hundred-and-twelfth added **eight**: five in `pdf-model` for §12.7.5.4's two shapes of `/V` and Table 234's `/I`, and one apiece in `viewer-core`, `viewer-confined` and `viewer-qt` for the same clause read from a host. The four-hundred-and-thirteenth added **one**, in `viewer-ui`, for which of §14.8.2.5's two orders a copy takes. The four-hundred-and-fourteenth added **nine** for a document-wide search: three in `viewer-core`'s new `search` module, three in its headless harness, and one apiece in `select`, `viewer-ui`'s panel test and the fragment tests. The four-hundred-and-fifteenth added **two**, both in `pdf-model`'s `transparency_groups`: which space §11.6.6's inheritance puts in force, and what compositing in `/DeviceCMYK` costs against compositing on the device. The four-hundred-and-sixteenth added **two**, both in the new `spec-errata`: which of Table 172's `/RT` values makes an annotation a reply rather than a group member, and that a strikeout covering no glyph retires nothing. The four-hundred-and-seventeenth added **two**: that a marked-content section's associated files are named by §14.13.5's `/MCAF` since Errata Collection 3, and — in `spec-errata` — that the two extractions of one sentence disagree about where its spaces are, which is what had hidden 72 struck passages. The four-hundred-and-eighteenth added **one** — that a Type 3 glyph description finds the resources its own stream names, which is §7.8.3's first step since Errata Collection 3 — and the gate printed **1498**, two above what this line had been carrying. The four-hundred-and-nineteenth added **eight**, all in a new `pdf-model/tests/missing_resources.rs`: the two requirements Table 86 puts on `Do`'s operand, a form that states no `/Resources` against one that states an incomplete one, the same `Do` inside a hidden optional-content section and outside it, and `gs` and `scn` naming nothing — and the gate printed **1506**, so this line was not behind. The four-hundred-and-twentieth added **nine**: five in `viewer-core`'s new `readback` module for the bound and its eviction rule, two in its headless harness — that a second search answers what the first did without interpreting a page again, and that an edit forgets every page's readback — one in `pdf-model`'s `composite_fonts` for §9.4.4's vertical displacement, and one in `viewer-ui` asserting that `--trace`'s "every topic" mask is the topic list's own arithmetic — **and the gate printed 1515**, so this line was not behind for the second round running. The four-hundred-and-twenty-first added **ten**: four in the new `tools/pdf-retrieve` for a section's edges, its annotations and the readback it must not tidy, one in its `json` writer for the control characters a PDF's text contains, two in `pdf-model`'s new `retrieval` module for what a clause number is and how an address is matched, one in `pdf-model/tests/structure.rs` for the walk that used to see half of ISO 32000-2's tree, and two more inside the tool's own tests — **and the gate printed 1525**, so this line was not behind for the third. The four-hundred-and-twenty-second added **fourteen**: six in a new `pdf-model/tests/contents_entry.rs` for the four ways Table 31's `/Contents` reaches a blank page and which two of them are worth a word, and eight in the new `tools/safedocs` — an archive name becoming the URL its README specifies, one outside the set refused before the network, a ZIP member found through the tail and verified on the way out, a CRC that does not match, a body that is not a ZIP at all, a member name that is a path, a plan over the budget and the budget's own arithmetic — **and the gate printed 1539**, so this line was not behind for the fourth. The four-hundred-and-twenty-third added **three**, all in `pdf-model/tests/composite_fonts.rs` for §9.10.2's permission reaching an `Identity-H` font: a partial `/ToUnicode` completed by the program, an Okular signature appearance that reads back its signer's name, and a two-byte code that is *not* read as a character when nothing names it — **and the gate printed 1542**, so this line was not behind for the fifth. The four-hundred-and-twenty-fourth added **two**, both in `pdf-syntax`'s `document`: that eight threads reading one document all get the object — with a compile-time assertion that `Document: Send + Sync` beside it — and that the recursion guard is per thread rather than per document, which is the one hazard `RefCell` → `RwLock` could have introduced silently — **and the gate printed 1544**, so this line was not behind for the sixth. The four-hundred-and-twenty-fifth added **six**, all in a new `pdf-model/tests/hostile_functions.rs` for §7.10.4: a stitching function naming its own object, two of them naming each other, a chain of eight the bound admits, a k of 255 whose `/Encode` is read rather than defaulted, a k above the budget refused by count, and the size of `Function` the budget is priced against — **and the gate printed 1550**, so this line was not behind for the seventh. The four-hundred-and-twenty-sixth added **eight** for §11.4.7's page group drawn in the space it states: three in `pdf-render`'s new `blending` module for the conversion out of the ink cube, two in `pdf-model`'s `colour` — that the table a backend is handed is this crate's own conversion over 2401 points of the cube, and that §10.4.2.4 round-trips through §10.4.2.5 and not through the cube — and three in `transparency_groups` for a page composited in ink, a colour from outside its space, and §11.3.5.3's rule for the black component — **and the gate printed 1558**, so this line was not behind for the eighth. The four-hundred-and-twenty-seventh added **three** for §11.7.2's conversion *into* that space: one in `pdf-model`'s `colour` — that `rgb_to_ink` is a right inverse of the ink cube over 1296 colours of its own image, with §10.4.2.4's route put back beside it — and two in `transparency_groups`, for a `DeviceRGB` colour converted into ink and coming back, and for a document that names the press its `DeviceCMYK` is — **and the gate printed 1561**, so this line was not behind for the ninth. The four-hundred-and-twenty-ninth added **one**, in `pdf-model`'s `saving`, for the undo `doc/todo/01`'s fifth sweep found reachable from nothing at all — a field forgotten leaves the producer's own `/V` and logs no edit — **and the gate printed 1562**, so this line was not behind for the tenth. The four-hundred-and-thirtieth added **five** — three in a new `pdf-model/tests/dct_components.rs` for a three-component `DCTDecode` frame whose Adobe APP14 marker says transform 0, one marked transform 2 and the control with no marker at all, and two in `contents_entry.rs` for a `/Contents` part the file states is empty against one truncated to nothing — **and the gate printed 1567**, so this line was not behind for the eleventh round running. The four-hundred-and-thirty-first added **one**, in `tools/spec-errata`, for a quotation that lowers the sentence's first letter because it starts mid-sentence — nine words identical and one letter apart from §9.6.2.2's struck sentence, which the comparison could not see while it kept case — **and the gate printed 1568**, five above what this line had been carrying. Where the five came from was not chased; the precondition is the one the four-hundred-and-eighteenth wrote down, that this line is current only for a round that ran the gate and copied its number. The **11** skipped is the four-hundred-and-twenty-third's own new `#[ignore]`d gate against `doc/corpora/pdfbox`. `clippy --workspace --all-targets` silent under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`; `fmt --all --check` clean | `cargo nextest run --workspace`, **31.0 s** |
| corpus (974 pdf.js documents, page one) | `974 documents in 3.1s: 0 unopenable, 8 locked, 2 encrypted beyond us, 5 pageless, **68 incomplete**, 0 slow` — 65 until the four-hundred-and-fifteenth, which read §11.4.7's page-group `/CS`: one false report left and four silent departures arrived (ADR 0251, trap 5's kind of rise); **68 until the four-hundred-and-nineteenth**, which made a `Do`, a `gs` or an `scn` naming a resource the file never defines say so — `issue6541.pdf` and `issue8702.pdf`, both of them files whose producer asked for a mark the file cannot carry, and neither of them a page whose ink moves (ADR 0255). **Unmoved by the four-hundred-and-twenty-second**, which added a report for a `/Contents` naming an object the file does not define: no document in the 974 does that, and the witness came from `doc/corpora/pdf-differences` instead (ADR 0258). **69 since the four-hundred-and-twenty-sixth**, which draws §11.4.7's page group in the `DeviceCMYK` space it states — `personwithdog.pdf` left, and the other five keep the report with the reason it now names (ADR 0262). **68 since the four-hundred-and-twenty-seventh**, which gave §11.7.2 its conversion into that space: `issue12798_page1_reduced.pdf` left, three of the seven such documents are drawn in ink rather than one, and the four that remain report a different clause each (ADR 0263) | `tests/corpus.rs`, **4.5 s** |
| oracle (1794 pages vs poppler, mupdf, ghostscript) | `1794 pages (1690 we call complete, 104 incomplete)` — 1693/101 until the four-hundred-and-fifteenth, whose four new reports moved only that split, and 1690/104 until the four-hundred-and-nineteenth, whose two did the same; **905 agree / 861 of them complete**, **68 contradicted / 66 complete**, **786 ambiguous / 752 complete**, our geometry 1/0, reference geometry 2/2, not comparable 14/9, no render 18/0 — and **the undiagnosed ambiguous list printed empty**, which is the ratchet holding. Nothing moved in the four-hundred-and-sixth — it changed only what the gate *prints* about a contradicted page (ADR 0242) — nor in the four-hundred-and-seventh, which added a second `#[ignore]`d test to the same file that re-derives the bounds this gate judges by (ADR 0243) — and the run now ends with a second ranking, `contradicted, and furthest from the nearest reference`, headed by `bitmap-symbol-context-reuse.pdf` at 28.91 nearest. **Every number here is unmoved by the four-hundred-and-thirty-first**, which re-derived five pages of `CONTRADICTED_SUBSTITUTED_FONT` that had been admitted together and never opened and moved them to `CONTRADICTED_GLYPH_EDGES` — 17 → **12** and 21 → **26** — a group being a hypothesis this gate holds as one ratchet, deliberately, so that a hypothesis turning out wrong does not fail the build (ADR 0267) | `tests/oracle.rs`, **41.1 s** |
| text (vs `pdftotext`, same 974) | `overall 99.2% (24003/24187 words), 23 below 90%`, with 24 skipped and **65** incomplete and not gated — 62 until the four-hundred-and-fifteenth moved the corpus's own count and this line was not read, 65 until the four-hundred-and-nineteenth moved it again. Both the numerator and the denominator fell by the same 56 words, which is the two new documents leaving the gated set and is what that denominator is for. **23987 → 24003 and 25 → 23 in the four-hundred-and-twenty-third**, on §9.10.2's permission reaching an `Identity-H` composite font; `issue16553.pdf` and `javauninstall-7r.pdf` left the named list and nothing joined it | `tests/text_extraction.rs`, **31.3 s** |
| **text (vs PDFBox's frozen extraction, 40 documents)** | `40 documents in 0.4s: 0 skipped, 0 incomplete and not gated; overall **99.8% (14257/14281 words)** against PDFBox's stream order and 99.8% against its position-sorted output, **4 below 90%**` — new in the four-hundred-and-twenty-third, and the first run was 14254/14281 with 5 below, one of which was the defect above. The four are named in `PDFBOX_BELOW_FLOOR` with the reading beside each. **No new line in `doc/todo/02` §2**: it runs inside the `--ignored` invocation the row above already makes | `tests/text_extraction.rs`, **0.4 s** |
| **quorra vs the CPU oracle** (974 documents, page one, same display list) | `957 pages compared in 27.0s: **910 agree, 36 differ, 11 refused, 17 not comparable**` — one refusal is `bug1721218_reduced.pdf`, whose coverage outgrows a 16384×16384 scratch image; **four are §11.4.6's stated shape**, which needs two Porter-Duff operators quorra's `Compose` does not have (ADR 0234, `QUORRA_FEEDBACK.md` section 14); **four more are §11.4.4's non-isolated group**, whose buffer has to start as a copy of the page where `GroupSpec` opens one on transparency (ADR 0237, section 16). All eight used to *agree* about a picture both backends drew wrongly. **A tenth in the four-hundred-and-twenty-sixth**: `personwithdog.pdf`, whose page composites in `DeviceCMYK` and is therefore two rasters where a `Scene` renders one (ADR 0262, section 17) — and that one used to be *reported* rather than agreed. **An eleventh in the four-hundred-and-twenty-seventh**, `bug1365930.pdf`, which is the sharpest case section 17 has: it reports nothing and never did, because nothing on it composites, so the refusal replaces an agreement about a *correct* picture (ADR 0263) | `render-quorra/tests/corpus.rs`, **33.5 s** |
| dates | `1545 date strings in 974 documents: **1514 conform** to §7.9.4 (97.99%), 31 do not, over 22 distinct strings` | `tests/dates.rs`, **0.9 s** |
| **§14.3.2's XMP** (same 974) | `319 documents carry §14.3.2's stream: **318 read, 1 refused**, 3191 properties between them, 106 state dc:title` — the refusal is a fuzzed file whose stream does not decode at all | `tests/xmp.rs`, **0.4 s** |
| **JPEG 2000 vs ISO/IEC 15444-5's reference software** | 30 corpus codestreams: **14 byte-identical, 13 differing, 3 not comparable**, and no remaining difference exceeds one level. `doc/JPEG2000_FEEDBACK.md` §§7–8 has the two defects behind that | `tests/jpeg2000.rs`, **13.8 s** |
| conformance | **6364 citations** (6357 until the four-hundred-and-thirty-first read §9.5, §9.8 and §9.8.1 against the code and put the cap-height measurement in the last two, 6349 until the four-hundred-and-thirtieth's two fixes, 6317 until the four-hundred-and-twenty-seventh gave §11.7.2 its conversion, 6345 until the four-hundred-and-twenty-eighth's `page` fuzz target cited §7.3.10 — `SOURCE_ROOTS` is `crates`, `tools` **and `fuzz`** — and 6346 until the four-hundred-and-twenty-ninth's sweeps), all naming clauses the standard has; **595 quotations**, all verbatim — 592 until the four-hundred-and-thirtieth quoted §7.4.8 and §7.3.8.1/§7.3.8.2 for two fixes; **217** distinct tables cited by this tree and **251** named in the ledger's notes — **this figure said 218 from the four-hundred-and-twenty-fifth until the four-hundred-and-thirtieth ran the gate twice, before and after its own work, and read 217 both times**; where the difference went was not chased, and the rule it restates is that this table carries the gate's own number rather than the last round's arithmetic; **875 ledger rows** (**403** implemented, **250** partial, 18 reported, 83 inapplicable, 8 writer-side, 113 out-of-scope) — 401/252 until the four-hundred-and-twenty-ninth's sweep round moved §9.4 and §9.7.5.1, both `partial` on reasons their own children's rows had retired, 401/251/19 until the four-hundred-and-twenty-seventh moved §11.7.2 off `reported`, and 400/252 until the four-hundred-and-twentieth, which read §9.4.4's row against the code and found it `partial` on one sentence that had been false since the thirty-sixth session: "[t]he vertical branch is not: ty is always 0, because nothing here reads a vertical writing mode", against §9.2.4's own row two clauses above saying "[b]oth writing modes, from the thirty-sixth session" (ADR 0256). **What that gate does not cover is now measured, and the four-hundred-and-sixteenth found a second thing it cannot see**: the 577 are every rustdoc blockquote in `crates/`, checked against a conversion that dropped every annotation in all fourteen documents — so **151 passages Errata Collection 3 struck out verify as the standard's current words** (ADR 0252, `tools/spec-errata`). That number was 79 until the four-hundred-and-seventeenth session read all 79 and found the checker itself blind to a space — both sides are extractions of the same glyphs by different programs, so one writes "inthe" where the other writes "in the" — and the 72 it had been missing included the sentence `crate::appearance` and `ledger.toml` were both quoting as §12.5.2's live rule (ADR 0253, `doc/errata-read.md`). And the ledger's own notes hold **977** quoted spans that nothing has ever checked, 417 of which occur in no document under `doc/md/`. ADR 0249 decides that as a sweep rather than a gate and says what a gate would cost; **the four-hundred-and-eighteenth ran that sweep against the errata** — which needs none of ADR 0249's syntax, because an erratum supplies the other side of the comparison — and found a *third* unchecked population beside it, quotation marks inside ordinary rustdoc prose, where five of this round's nine stale quotations were (ADR 0254). **The four-hundred-and-nineteenth found a fourth and a fifth**, both by walking into one of them while reading §7.8.3 for something else: a quotation inside an ordinary `//` comment, which the prose scanner skipped on a stated reason that `CLAUDE.md` contradicts, and a quotation with an ellipsis in it, which `overlaps` compared whole and so could never match a struck passage longer than itself. Twenty-one further landings, **six of them stale quotations in six files** — two of which are the two the round before recorded itself as having missed (ADR 0255) | `cargo test -p conformance`, **2.2 s** |
| **the round itself** | **not measured as one span this round**, and the honest number is what the gates themselves printed: **154 s** of test execution summed from the ten lines above (25.7 + 4.2 + 46.9 + 30.1 + 34.1 + 0.6 + 0.3 + 9.9 + 2.0), with each gate's incremental build on top and each run separately rather than back to back. `doc/todo/02` records **268 s** for §2 *and* §5's binaries together, from 608 s until the three-hundred-and-eighty-fifth measured every step (ADR 0222); the three-hundred-and-ninety-seventh read 287 s off file timestamps for §2 alone | ADR 0222, `doc/todo/43` |

**Two things beyond §2 were run in the three-hundred-and-ninety-eighth and are claimed**: the
`confined_wire` fuzz target, because the round added a decoder to the confined transport —
**13 942 159 runs in 181 s, clean** — and the **window under `Xvfb`**, because the round's whole
point is something a host does with a pointer. A click at the check box's own centre printed
`note: setting the field typeScript to Yes` and `note: this document has unsaved changes`, and a
second click printed `note: setting the field typeScript to Off`; ADR 0126's recipe, with the window
name taken from §12.2's `/DisplayDocTitle` (`PDF Form Example — page 1 of 1`) rather than from the
file.

**The four-hundred-and-fourth ran two of the same instruments**, because it changed how both ends of
the confined transport write a frame header: `confined_wire`, **13 175 908 runs in 241 s, clean**, and
`cargo deny` — *advisories ok, bans ok, licenses ok, sources ok*. **Not the two cross-target
checks**, and the reason is that nothing moved for them to see: no manifest changed and no
dependency was added, which is the round's own finding about item 5 rather than an oversight.

**Not re-run in either round and therefore not claimed**: the other eleven fuzz targets, and (in
the three-hundred-and-ninety-eighth) `cargo deny` and the two cross-target checks. That round added a
*description* of §12.7's fields and one appearance-state selection — no document parser, no decoder,
no dependency, and nothing on any gate's path, since no gate sets a field's value — so those
have nothing to catch that the workspace
suite and the three corpus-scale gates do not; that is a reason not to run them, not evidence that
they pass. `doc/verify.md` is what each of them is and when a round owes it.

Counts are **ratcheted**: they may only improve, except where a rise is a new report and is
written down as one (trap 5). The 14 specification PDFs in `doc/` — including ISO 32000-2 itself,
1023 pages, 101 318 objects — all parse, draw page one with nothing reported, and extract 100% of
`pdftotext`'s words.

### The ledger

All **823** subclauses of the eight technical clauses have been read against this code, since the
fifty-sixth session — **and, since the three-hundred-and-sixtieth, the 52 numbers of the standard's
eight normative annexes** (ADR 0206). Counts come from `cargo run -p conformance --bin ledger`,
which prints them — **not** from arithmetic in this file, which has been wrong about them twice.
Read off that command in the four-hundred-and-twenty-ninth:

| status | rows | |
|---|---|---|
| `implemented` | 403 | every normative requirement in the clause is executed |
| `partial` | 250 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so — **Annex O's five were the last, and they were built in the three-hundred-and-sixty-ninth** |
| `inapplicable` | 83 | a press, a layout engine, a production workflow — **and read at last** (ADR 0205) |
| `out-of-scope` | 113 | principle 5's closed exclusions, which the row names |
| `reported` | 18 | not implemented, detected and named at runtime |
| `writer-side` | 8 | addresses a PDF *generator* |

**`silent` is zero.** There is no requirement in the standard — the eight technical clauses or the
eight normative annexes — that this program fails without saying so. That is a narrow claim:
`partial` and `reported` are 268 rows between them and each names what it owes.

**How a row goes wrong is [`doc/ledger-and-claims.md`](ledger-and-claims.md)** — the seven shapes
found so far, the four times this project's ledger has been wrong, and what `REVIEW_OWED` and
`FILE_ONLY_EVIDENCE_CEILING` are for. The sweeps that catch them are
[todo 01](todo/01-ledger-partial-rows.md), and running them is
[todo 02](todo/02-every-round.md) §4.

### What is not implemented

Every one is *reported* at runtime rather than silently skipped, and **each has a file in
`doc/todo/`** carrying the evidence, the clause and what it would cost. The count is how many of
the 974 documents' first pages it affects.

| Missing | Corpus | |
|---|---|---|
| A fill under an eighth of a device pixel; a tiling cell's two halves; a hairline at the raster's edge — **two of the three are `render-cpu`'s alone since the three-hundred-and-forty-fourth session**, and the graphics device draws every shape they lose to within 2% of its area | 4 | [todo 11](todo/11-shapes-that-still-disappear.md) |
| A substitute that cannot be addressed; **24 codes over 8 documents that reach no glyph in silence**; a per-character fallback, owed with no witness | 40 | [todo 21](todo/21-font-substitution.md) |
| A `/DA` font `/DR` does not define **and cannot be spelled** (Arabic, one document); a composite `/DA` font, with no witness in 974 files. **§12.7.5.4's list box is measured and left refused** — 10 widgets over 8 documents, every one with its own `/AP`, so the refusal takes no mark off any page (ADR 0240) | 1 | [todo 22](todo/22-variable-text-edges.md) |
| Transparency departures (§11.4.4, §11.4.7, §11.6.6, §11.7.2) — **§11.5.3's population closed in the three-hundred-and-eighty-third** (ADRs 0217, 0220), **§11.4.6's shape in the three-hundred-and-ninety-seventh** (ADR 0234), **§11.4.4's non-isolated group in the four-hundredth** (ADR 0237) and **§11.4.7's four components in the four-hundred-and-twenty-sixth** — which needed no raster format at all, because §11.3.4 applies the compositing formula per component and a page is therefore drawn twice (ADR 0262). **and §11.7.2's conversion *into* the space in the four-hundred-and-twenty-seventh** — §11.7.5.3 names that conversion's *target* rather than its algorithm, so it belongs on §10.3's branch and is a right inverse of the ink cube, which leaves one colour model on a page and no boundary between two (ADR 0263). What stands needs a *second colour space* rather than a second direction, and **the four-hundred-and-thirtieth's 4000-document sample reordered the five**: a **four-component `ICCBased`** blending space is **14 of 4000**, where this row had it at one witness, and it is now the largest of them; a document that says what its own `DeviceCMYK` is (§8.6.5.6, §14.11.5) is 8; a group inside the page with its own space is 4; §11.3.5.3's rule for the black component is 2; and Table 57's black generation is 0 in that sample (ADR 0266). Plus §11.4.6's knockout where the elements blend. Reports with no corpus member sit inside the closed ones, `/AIS` among them | 6 | [todo 23](todo/23-transparency-departures.md) |
| JPEG 2000 at a reduced resolution level — **written, measured and waiting on one push**: the decoder's API was never missing (it has had `target_resolution` since December 2025) and the real cost was an allocation sized from the full-resolution image, fixed on `close2/hayro`'s `feat/reduced-resolution-allocates-less` (`1dc833f7`, ADR 0233). Nothing on this path is committed here, because against the pinned revision the same code trades a refusal for a dead worker; a sampled shading on `render-gpu` alone. **The mask at a grid the bound refuses is closed** (ADR 0210) | 1 | [todo 24](todo/24-image-sampling-intent.md) |
| `/FixedPrint`, which waits on a printing path | 15 | [todo 25](todo/25-view-dependent-annotations.md) |
| An icon for `Stamp`, whose standard names are legends rather than symbols | 1 | [todo 26](todo/26-icons-a-clause-only-recommends.md) |
| A character of the *document's own text in this host's chrome* — an outline title, a layer name, an `/Info` value — that §9.6.2.2's fourteen have no code for. Drawn as a box since the three-hundred-and-sixteenth session, and what a box cannot say is which character | **74** documents, 9 strings that used to draw as nothing | [todo 27](todo/27-the-interfaces-own-font.md) |
| Signature *trust* — a certificate store, a certification path, revocation — plus DSA and ECDSA signatures, public-key handlers (§7.6.5), `/R` 5. **The digest and the RSA verification are done** (ADR 0215, ADR 0229) | 1 | [todo 51](todo/51-signatures-and-public-keys.md) |
| **Not** sandboxing the interpreter and rasteriser — that is built and draws real pages (ADRs 0218, 0223). What is owed is that `viewer-ui` does not use it, a hostile document has no cancel, and §12.3.5.1's `/D` is decided by nobody. **This row said "Sandboxing the interpreter and rasteriser" flatly for fourteen rounds after the three-hundred-and-eighty-first built it**, while this file's own "Where we are" described it | — | [todo 34](todo/34-sandbox-the-interpreter.md) |

**Closed by decision rather than by work**, recorded in the ledger and not owed to anybody:
`/ColorTransform` (Table 13, whose one corpus witness contradicts the clause), a stream whose data
is in an external file (§7.3.8.1 — the renderer has no filesystem, principle 3), §12.7.6.2's
submit and §12.6.4's remote, launch, sound and movie actions, a filled degenerate subpath's device
pixel (§8.5.3.3.1, which the clause itself calls "device-dependent and not generally useful"),
grid-fitting a stroke's coordinates under `/SA`, rendering intents beyond
`AbsoluteColorimetric` (read and recorded; `A2B0` not yet selected for `Perceptual`), and
**a glyph a document's own embedded subset does not contain**, which the
two-hundred-and-forty-eighth session traced to the end of every route the standard states:
`issue14821.pdf`'s eight are four `Identity-H` CIDs whose `loca` entries are empty by the table's
own statement and whose `/W` does not list them, plus three ASCII codes in a subset whose `(3,1)`
`cmap` maps them to glyph 0 and whose `post` is version 3.0 with no names at all. §9.7.4.2's and
§9.6.5.4's rows carry the evidence; `poppler` draws them from a face this machine has, which is a
fallback rather than a reading.

Two that *were* here and are closed by work: optional content's interactive half (session 167)
and a page whose scene overflows Vello's buffers (banded in 143, and the page that motivated it
stopped overflowing in 147 — ADRs 0127, 0132).

## What to do next

**The work itself lives in `doc/todo/`**, one file per item, numbered so that `ls` sorts by
priority — `00`–`09` standing, `10`–`19` defects, `20`–`29` owed features with corpus demand,
`30`–`39` capability, `40`–`49` measured performance, `50`–`59` blocked. `doc/todo/README.md` is
the index, and `doc/todo/02-every-round.md` is what a round does around whatever it takes.

**The one item that outranked the choosing is done, and it was not engineering.** The fourteen ISO
and PDF Association documents in `doc/` and their Markdown conversions under `doc/md/` were
**tracked in the clear, and the project owner is not licensed to redistribute them** — free to
obtain is not the same permission, and a repository carrying them passes them on to everyone who
clones it. In the three-hundred-and-eleventh session they left the tree, the index and **all 436
commits of the history**, and came back **encrypted** (ADR 0187): `doc/specifications.zip`, 37 MB,
ZipCrypto, all twenty-eight files, with `.gitignore` covering what `unzip` puts back. `git log
--all --name-only` finds no path under `doc/md/` and no `doc/*.pdf` in any commit, which is the
only check worth trusting on this. **This tree may be published**; nothing else here had to be
true first.

**Run this once in a fresh clone, and everything below works:**

```sh
unzip -P <password> doc/specifications.zip    # from the workspace root; ask the owner
```

**Every reference to the documents stays as it was**, decided by the owner in that session: four
tests and eleven measurement examples open `doc/ISO_32000-2_sponsored_EC3.pdf` or
`doc/PDF20_AN001-BPC.pdf` and fail loudly until you have, and `cargo test -p conformance` checks
no citation without `doc/md/ISO_32000-2_sponsored_EC3.md`. **CI is a developer like any other
here** and unpacks the archive from the `SPEC_ZIP_PASSWORD` repository secret before its tests;
a pull request from a fork gets no secret, and the step says so rather than failing obscurely.


What stays here is the *shape* of choosing, which is the part that has been wrong before.

**Two tracks, and take from both in every round.** *Demand-driven* is what the corpus and the
oracle name; *spec-driven* is the ledger's `reported` rows and the notes on its `partial` ones. A
project running only the first finishes when the corpus goes quiet, which can happen with much of
the standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises. This is a principle-5 rule, not a suggestion.

**A reason that names a vocabulary is the fourth of these shapes**, found in the
two-hundred-and-fifty-seventh: §12.6.3's `/Fo` and `/Bl` were owed "keyboard focus, which
`viewer-core` does not have — there is no focus model in `Command` at all, and adding one is a
vocabulary change rather than a clause". No message was needed. The clause says what happens when
an annotation receives the input focus and nothing about how it comes to, so a press inside a
widget's active area gives it — a choice, and the one every pointing interface makes. All ten of
Table 197's events are raised now. **Ask what the program already receives before adding a way to
receive it.**

**A reason that names an architecture is two reasons wearing one coat**, which the
two-hundred-and-seventeenth session found: §12.5.3's `NoZoom` and `NoRotate` were both refused
because they "make an appearance's placement depend on the view, which a resolution-independent
display list cannot express". `NoRotate` depends on §7.7.3.3's `/Rotate`, which is in the *file* —
it was never a view-dependence at all — and `NoZoom`'s real cost is one flag on the interpretation
and a re-read of 51 documents out of 974 (ADR 0168). **Split a refusal into one claim per entry
before believing it.**

**A capability makes clauses reachable, and nothing announces it.** The ten sessions from the
hundred-and-sixty-sixth closed four clauses without anybody picking them off a list: §12.3.3
because a panel existed to display an outline in, §14.3.3 because a panel existed to display
`/Info` in, §7.7.2's `/PageMode` and §12.6.3's trigger events because a sidebar and a pointer had
arrived. Each of those rows said some version of *this program has no ___*, and each stayed true
for between seven and forty-one sessions after it stopped being true. The three sweeps that catch
it are in `doc/todo/01-ledger-partial-rows.md`, the hundred-and-ninety-first session found a
`shall` that had been binding for fifty-six, the two-hundred-and-first found the longest one
yet — §12.3.2.1's magnification and window position, owed since the **hundred-and-thirty-second**
session put scrolling and zoom in the vocabulary, still explained by "a window with scrolling and
zoom, which this program does not have" sixty-nine sessions later (ADR 0162) — and the
two-hundred-and-fourth found the same row family's other half, §12.6.3's four page-scoped trigger
events blocked on "a page-visibility model a one-page-at-a-time window does not have", which is
what a window that turns pages is (ADR 0164).

**And the two-hundred-and-fifty-third and -fourth found the inverse, which no sweep was asking
for: a capability that reached the crate implementing the clause and never reached the program.**
§12.5.6.19's `/H` was `implemented`, argued in ADR 0123, tested with pixels — and `viewer-core`
took the annotation under the pointer from `link_at`, which returns a `/Subtype /Link` and nothing
else, so no host could press a widget for a hundred and fifteen sessions. **The question the
sweeps do not ask is "the model implements this — who calls it?"** Widening the region then turned
a latent default into a wrong pixel in the same sitting: §12.5.6.19's `/H` defaults to `I`, two
tables define the entry and no others do, and a `Square` had been one caller away from inverting
under the cursor. ADR 0177. **The sweep that asks it is `doc/todo/01`'s fifth** — every `pub fn`
in `pdf-model`, grepped against the two host-side crates — and it found §8.11.4.3's `/ListMode`
on its first run, read into `OptionalContent::list_mode` and asked by nothing with a layer panel
on the screen (ADR 0178).

**And the two-hundred-and-fourteenth found a row that would have survived the capability arriving.**
§14.9.3 said `/TU` "names a field in a user interface this program does not have" — false since
the hundred-and-thirty-second — but the window was never the blocker: `Query::FieldAt` answered
with one string, and §14.9.3's `shall` needs two, because the name that *addresses* a field is not
the name a person is shown. **Ask what the program would have to say to obey the clause, not only
what it would have to have** (ADR 0167).

**But the map is not the territory.** Four of the six findings in the ten sessions from the
hundred-and-twentieth were on no list at all: a `shall` hiding behind a silence about artwork (ADR
0109), a clause with two populations where the row named one (0110), a malformed optional entry
that erased a font (0111), and a font cache keyed by a name that drew wrong glyphs in silence for
thirty-one sessions (0115). None was `silent`, none was `reported`, and no gate could see the
last. Three were found by reading the clause beside the code; the fourth by measuring something
else.

### 0. The UI boundary — built, with six consumers on it and three native hosts among them

**[`doc/ui-boundary.md`](ui-boundary.md)** — the vocabulary (`Command`, `Event`, `Query` →
`Answer`), why each message exists, the five rules, the three pixel tiers, the text layer, the edit
log, and what is still owed. ADRs 0116 to 0121, and ADR 0244 for what the first *native* host found
in it, and ADR 0247 for what the *third* found. What is left of it is *hosts*:
[30](todo/30-a-native-host.md), whose three are all built and whose remainder is surface — the C
ABI's 43 entry points are not the whole vocabulary, and every missing one is a symbol,
[31](todo/31-accessibility-host.md) the four edges the AccessKit bridge does not cover,
[32](todo/32-presentation-player.md) the presentation player's remaining five styles, and
[33](todo/33-annotation-editing.md) editing a free text annotation the *file* states.

### 1. Third-party data: shipped, and the record of what was read

**[`doc/third-party-data.md`](third-party-data.md)** — the four data sets, the source examined for
each, the terms, what `/NOTICE` owes and what the GPL trap in `poppler-data`'s second half is; and
every dependency decision since, including the one in the three-hundred-and-ninety-second that came
out **no** (ADR 0229) and the three *corpora* the four-hundred-and-twenty-second added, where the
licence question is about what may be **promoted out of** a submodule rather than about what ships
(ADR 0258).

### 2. The ledger, and where a false claim can still hide

**[`doc/ledger-and-claims.md`](ledger-and-claims.md)**, and the reading task itself is
[todo 01](todo/01-ledger-partial-rows.md).

### 3. What the corpus still names

**[`doc/oracle-and-corpus.md`](oracle-and-corpus.md)** — **the four populations** since the
four-hundred-and-twenty-second (its §2: the 974, three submodules under `doc/corpora/` and whatever
`tools/safedocs` has fetched — **5944 documents over 85 archives since the four-hundred-and-thirtieth** — with each sample's own baseline and the licence of each), the 66
contradicted pages on complete documents grouped with their diagnoses, the 70 incomplete documents
split by report kind, and the two cautions the contradicted list earned. Its **§3c** is the four-hundred-and-seventh session's
answer to why 38 of the 68 fail one bound and nothing else: that bound rejects **29.4%** of the
reference pairs that agree by every other measure, where its three siblings reject 0.0%, 1.2% and
0.5%, and it is left where it is for reasons ADR 0243 measured rather than asserted.

### 3a. The ambiguous bucket — watched since the hundred-and-seventy-sixth, and emptied in the three-hundred-and-seventy-ninth

**[`doc/oracle-and-corpus.md`](oracle-and-corpus.md)**, second half — 0 undiagnosed from 754, what
forty-five sessions of it produced (fourteen defects, thirteen fixed), and the instruments that came
out of it. The task and the method are [todo 00](todo/00-ambiguous-bucket.md), and its step 7 is
what a round that changes pixels re-runs.

### 3b. The quorra backend, and what a corpus-scale comparison found in it

**[`doc/performance.md`](performance.md)** — the gate, the three findings the first run produced,
where it stands and what the two backends cost each other.

### 4. Performance

**[`doc/performance.md`](performance.md)** — the launch path, the `hayro` comparison, the parallel
rasteriser and why its strips are chosen by cost, the interpretation and rasterisation counters,
where a page turn goes, and the four items priced and refused. Still open, each with a file:
[todo 24](todo/24-image-sampling-intent.md), [todo 40](todo/40-mask-chain-crop.md),
[todo 41](todo/41-decoded-stream-cache.md), [todo 42](todo/42-the-launch-path.md),
[todo 45](todo/45-where-a-frame-goes.md).

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
binaries, and `doc/todo/02-every-round.md` §5 is what puts the three release binaries where a person
can run them.

## Verify it

**The round's gate sequence is `doc/todo/02-every-round.md` §2, which owns those commands.** This
file used to state them a second time and they drifted — it said 1369 tests where the gate printed
1371, and never listed `render-quorra`'s corpus gate at all.

**[`doc/verify.md`](verify.md)** is everything else, and a round runs the ones its change can reach:
`cargo deny`, the two cross-target checks under `-D warnings`, the **fourteen** fuzz targets and
which need a seeded corpus — five of them do — the callgrind counters, the census and ladder
examples, and the AT-SPI recipe.

## Crate map

**[`doc/crate-map.md`](crate-map.md)** — one row per crate, its one responsibility, and the decision
that lives in it.

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
a `no render` count is a to-do list of pages nobody has looked at, and it is 18 — one left it in
the hundred-and-seventy-seventh session when a page the file's own cross-reference table had
hidden started rendering (ADR 0148).

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect. `issue6621.pdf` and `issue7901.pdf`
were both code that was right about the clause it cited.

**A contradicted page's group names a hypothesis, not a diagnosis — eight for eight on being
wrong**, the newest being `issue4304.pdf` in the four-hundred-and-fifth session, which spent a
hundred and eighty sessions inside `CONTRADICTED_SUBSTITUTED_FONT` while the difference was six
spaces of zero width and the side-by-side said so in one look. Open the artefact before believing the label — **and measure it, because a label this
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
  nothing about the rest. The general case needs a report where a glyph is *shown*, which needs
  `LoadedFont` to distinguish "this code has no glyph" from "this code's glyph is blank", which a
  space legitimately is. Not hard; not done; measure the volume first.
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
  DISPLAY=:77 target/pdf-viewer --trace doc/ISO_32000-2_sponsored_EC3.pdf &
  sleep 20   # 1023 pages: the window is up long before this, but the title is not
  DISPLAY=:77 xdotool windowfocus --sync $(DISPLAY=:77 xdotool search --name "ISO 32000" | tail -1)
  DISPLAY=:77 xdotool key --delay 400 Right Right Right Right Right
  DISPLAY=:77 xwd -root -silent -out screen.xwd && magick xwd:screen.xwd screen.png
  ```

  **Two corrections from the two-hundred-and-thirteenth session's run, both of which cost time.**
  `xdotool search --name ISO_32000` finds nothing: that document sets `/DisplayDocTitle`, so its
  title bar reads *ISO 32000-2:2020 (PDF 2.0)…* with a space, which is the feature working. And
  `xwd … | magick - screen.png` fails with *no decode delegate*, because this machine's
  ImageMagick no longer sniffs xwd from a pipe; `-out` plus `magick xwd:<file>` does.

  **A wheel notch is two events here.** `xdotool click 4` is a button press *and* a release, and
  winit's X11 backend turns both into `MouseWheel`, so one `click` is two `Command::Scroll`s or
  two zoom steps. It has always been so — the sidebar's scrolling has doubled the same way since
  it landed — and it is a fact about this instrument rather than about the code: divide before
  believing a step count measured this way. Found in the two-hundred-and-fourteenth session,
  checking Ctrl + wheel in the window.

  **And the pointer has to be inside the window**, which is 800×1000 on a 900×1100 screen: a
  `mousemove` to 850 produces no wheel event at all and looks exactly like a binding that does
  not work. `xdotool getwindowgeometry` first.

  **This is the only way to exercise the loop** — key press to command to request to frame to
  window — which is where every defect of sessions 140 to 142 lived and which no gate touches.
  Not a gate itself: `Xvfb` and `xdotool` are not build dependencies and a test that skipped
  silently would be worse than none.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via `~/.cargo/config.toml`,
  so the two users never fight over `target/`. Do not "fix" this. `pdfref` needs `--work-dir` for
  the same reason.
- **`cargo-fuzz` needs `+nightly`** explicitly; `rust-toolchain.toml` pins stable 1.97.1
  deliberately. `cargo-deny` is in the agent's `~/.cargo/bin` — **and so is `cargo-fuzz`, which is
  not on `PATH`**, so `which cargo-fuzz` answers nothing and `cargo fuzz` fails with "no such
  subcommand". Sessions 425 and 426 read that as "cargo-fuzz is not installed here" and left a
  target unwritten on the strength of it; it has been there since 26 July. Prefix the run:
  `PATH=$HOME/.cargo/bin:$PATH cargo +nightly fuzz …`. **`which` answers a question about `PATH`,
  not a question about the disk** (ADR 0264).
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build without
  `git submodule update --init`.
- KDE Frameworks 6 packages on Arch have no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).
- **`tmp/hayro` is a checkout of the whole hayro workspace**, with the project owner's fork as
  `origin` and the maintainer's as `upstream`. **The owner's standing offer is that a fix goes on a
  branch there, they push it and open the pull request, and this tree depends on the fork
  meanwhile** — so a defect in `hayro-jpeg2000` or any other member is a branch to write rather than
  a dependency to wait on. `doc/JPEG2000_FEEDBACK.md` §9 has the detail and the precedent. **This
  changes what a todo file may call blocked**: "waits on the decoder's API" is a statement about
  effort, not about access.

---

## How the project got here

**[`doc/history.md`](history.md)** — one line per session, with the ADR that argues each, and one
summary of what a block of twenty rounds had in common. A round appends one row to it and nothing
else.
