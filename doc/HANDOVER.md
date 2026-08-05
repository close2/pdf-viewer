# Handover

Written 2026-07-26, rewritten and halved 2026-08-01 at the end of the **hundred-and-thirtieth**
session, and kept current since; the **three-hundred-and-eleventh** is the last one in it. Read `/CLAUDE.md` first — the five
principles, what *done* means, and the closed exclusion list. **Principle 5 is the one that changes how you work**: the specification is the
only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it
right, never the definition of right.

**`doc/todo/` holds one file per piece of owed work**, numbered so that `ls` sorts by priority;
its `README.md` is the index and `02-every-round.md` is what a round does. `doc/PLAN.md` holds the
phases and the ledger's design; `doc/adr/` holds every decision's argument;
`doc/conformance/ledger.toml` holds one row per subclause; **`doc/RENDER_LIBRARY.md` is what a
rendering library would have to be to fit this viewer** and `doc/QUORRA_FEEDBACK.md` is what came
back when one was built to it; `doc/JPEG2000_FEEDBACK.md` is the same shape one dependency over.

**This file is the state of play, the traps and the habits** — where something is written
elsewhere, this is a pointer, and the pointer is the whole entry.

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
§12.4.4's whole presentation read for a caller that has one to play, and everything a document
says *about itself*: §14.7's logical structure, §14.8's tagged-PDF vocabulary, §7.11.4's embedded
files, §14.13's associated files, §12.2's viewer preferences, §12.11's requirements, §7.12's
extensions, and since the **two-hundred-and-ninety-fourth session §14.3.2's XMP** — the last
large population this tree read nothing of, 319 documents, closed by a dependency decision
nobody had been asked for (ADR 0186).

It is **used**, which is what the nine sessions from the hundred-and-thirty-first added. A
locked document asks for its password (§7.6.4.1, owed since the twenty-second session); the page
zooms and scrolls; the cursor knows what it is over and §12.5.5's appearances follow it, as does
§12.5.6.19's `/H`; a drag **selects text**, whose shapes cross to the host as geometry so that it
draws them in its own colour; a person can **fill in a form field**, undo it and redo it; a click on a markup annotation **opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about activation and was owed to a capability this program had had for a hundred and eighty sessions (ADR 0191); and the
result can be **saved** — the file it was opened from, unchanged, with §7.5.6's incremental update
appended, which is the one kind of writing `CLAUDE.md` permits.

**Page one goes to the graphics device**, decided by the project owner in the
two-hundred-and-seventy-third session and written into `CLAUDE.md`'s startup rules. GPU bring-up
is therefore *on* the critical path by choice, which makes what it costs a number to keep rather
than a cost to hide. **Since the two-hundred-and-seventy-fourth session `--trace` prints the whole
launch as a timeline** — one `Instant` taken at `main`'s first statement, one mark per milestone,
printed when the first frame lands. It was **145 ms from process start to the first frame** on this
machine's software adapter under `Xvfb`, for a 5-page document *and* for ISO 32000-2's 1023 pages,
and is **98.8 to 119** after two rounds of taking things off it. Nothing the window needs has ever
looked at a PDF, so the document opens on a thread of its own (ADR 0182); and a `wgpu::Instance`
needs no window either, so **it is made on a second thread** and handed to quorra, which added the
entry point for it after `doc/QUORRA_FEEDBACK.md` §8 asked with a measurement (ADR 0185). Of what
is left: `EventLoop::new` 20 to 45 ms, the first present 48 to 53, the device **13 to 19**, the
instance 0.006 to 2.6 and the document's join 5.
**Two things on it broke a rule `CLAUDE.md` states and both are closed**, in
[todo 42](todo/42-the-launch-path.md): `Document::open` cost 12 to 22 ms on 101 318 objects
against 0.20 on a small file, where the rule is "a 500-page document must open no slower than a
5-page one", and `Outline::read` 3 to 7 ms for a panel nobody had opened. **41% of the first went
in the two-hundred-and-seventy-sixth** — 40% of it was §7.5.6's "most recent copy" rule being
re-decided once per cross-reference entry instead of once per file (ADR 0180, 130.7 M instructions
to 76.6 M) — and the rest of both went *beside* the window rather than in front of it. Measured in
the two-hundred-and-eighty-ninth: **1023 pages and 5 pages now cost the launch the same**, 5 ms of
join either way. The rule read as a statement about `Document::open` itself is still 10 to 13 ms
against 0.2, and todo 42 keeps that question open as a question about the function. **A third cost on
that path turned out to be a clause nobody had read**: §12.8's signature walk spent 1.681 ms
finding nothing on a document whose form could have said so in one integer, and §12.7.3's Table
225 exists precisely so that a processor need not scan — 0.017 ms now, with the ledger row that
called the entry "signature behaviour" corrected (ADR 0181). **Adapter selection is the largest part of bring-up and the backend set is not the
lever**: `examples/bring_up` shows Vulkan-only moving the cost out of instance creation and into
`request_adapter` with the total unchanged (ADR 0179). **On the machine's real adapter, headless,
the first frame costs 18.2 ms and the tenth 4.1** — and a one-second sleep before it changes
nothing, so the ~12 ms difference is first-use allocation rather than warmth. `CLAUDE.md`'s ban on
waiting for warmth therefore costs nothing measurable here; `examples/first_frame` is the
instrument and `doc/QUORRA_FEEDBACK.md` §9 is the ask. The same arithmetic puts a launch on the
real GPU at **75 to 90 ms** against `lavapipe`'s 145, and nobody has run that — the window half of
it is the user's to measure. The CPU backend keeps its other two jobs:
the correctness oracle, and the frame the device refuses.

**And it has chrome, which is what the ten sessions from the hundred-and-sixty-sixth added and
what this project owed for thirty before that.** A sidebar of four tabs, drawn with `pdf-font`'s
compiled-in Helvetica and a `pdf-render` display list so that both backends draw it: §12.3.3's
outline, where a click **activates the item** and the document decides whether that is a jump or
a URI; §8.11.4.3's layers, where a switch turns one on unless Table 99's `/Locked` forbids it;
§7.11.4's embedded files, where a click writes the file beside the document; and §14.3.3's
`/Info`. **A fifth tab arrived in the two-hundred-and-sixty-first**: §12.3.4's thumbnails, one row
per page with the miniature fitted above §12.4.2's label, where a click shows that page. `?` puts
`/NOTICE` over the page in Courier. **The document chooses what opens**: Table
29's `/PageMode` names a panel and four of its six values now name one that exists, and §12.2's
`/DisplayDocTitle` puts the document's own title in the title bar. §12.6.3's trigger events are
raised by the pointer at last. Four clauses closed on it — §12.3.3, §7.11.4, §7.11.4.1 and
§14.3.3 — and three of the four had a ledger row whose reason was "this program has no ___".

All of it sits behind **`viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API. Two consumers —
`viewer-ui`'s winit window and a headless test harness — and §0 is the whole story. ADRs 0116 to
0121.

**What it still does not do**: speak a page — though the `Query` is no longer what is missing.
`Query::AccessibilityTree` answers with §14.7's elements, §14.9's spoken form of each and the
quadrilaterals they cover (ADR 0134); what is left is a host that hands them to AccessKit. It
*advances* a slide show since the hundred-and-fiftieth (`Command::Tick`, ADR 0135) and does not
*animate* one: the transition is named and a host with a clock draws its frames. **It draws a
sidebar since the hundred-and-sixty-sixth and -seventh** — §12.3.3's outline, §12.3.4's
thumbnails, §8.11.4.3's layers
with their switches, and §7.11.4's embedded files, in `viewer-ui`'s own chrome, with `pdf-font`'s
compiled-in Helvetica and a `pdf-render` display list, so both backends draw it. A click on an
outline title sends `Command::Activate` and the *document* decides what that means — a jump, a
URI, a layer, whatever §12.6.2's chain says; a click on a layer's switch sends
`Command::SetGroup` and the page redraws, which is §8.11's interactive half working for the first
time; and a click on an embedded file sends `Command::Extract`, so the bytes come out decoded
with Table 45's checksum checked against them (§7.11.4, ADR 0145). **The document chooses which
tab is open**: Table 29's `/PageMode` names a panel, and four of its six values now name one
that exists (ADR 0146, and §12.3.4's `UseThumbs` in the two-hundred-and-sixty-first).

### The gates, today

| gate | number | where |
|---|---|---|
| tests | **1036** over the ten crates that touch PDF bytes, `clippy` silent under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`, `fmt` clean, `cargo deny` clean on all four, **nine fuzz targets clean at 50 000 runs**, the newest (`xmp`) also at 1 000 000 | re-run whole in the **three-hundred-and-fourth** and again at the end of the twenty: `deny`, `fmt`, `clippy`, all **nine** fuzzers at 50 000 with no artifact left behind, and all **eight** corpus gates — corpus, oracle, text, dates, XMP, JPEG 2000, quorra and the window |
| — | **1036 was 1033 was 1027 was 1015, and this row said 1011: the round that added twelve counted them, and four before them had never been counted.** The twelve are §12.5.6.14's popup window — seven in `pdf-model`, three in `viewer-core`'s headless host and two in `viewer-ui`'s panel test, which is the only gate that can see chrome at all; the four are somebody's and the arithmetic is how it was found, which is this row's whole purpose. The six after them are Table 179's line endings and the box a construction turned out not to have; the three after those are what a build without a confinement says about itself. Before that: 1007 was 1004 was 996. The three before them are §14.8.2.5's range map and its two callers, and the eight before those were one round's: §14.3.2's reader, its corpus gate and the two consumers it broke. Before that, 996 was 993, and the eleven rounds from the two-hundred-and-seventy-fourth added three tests and no gate. Counted with the quoted command in the two-hundred-and-eighty-fifth. **this row said 866 for at least one session and nobody had run it.** Counted as `cargo test -p pdf-spec -p pdf-syntax -p pdf-model -p pdf-font -p pdf-render -p render-cpu -p render-gpu -p pdf-sandbox -p viewer-core -p viewer-ui --no-fail-fast`, summing the `test result: ok. N` lines, it was **931** before the hundred-and-eighty-sixth session's fourteen, and the forty-four sessions from the hundred-and-eighty-sixth added forty-one. Quote the command with the number | — |
| corpus (974 pdf.js documents, page one) | 964 open, 959 reach page one, **886 draw with nothing reported**, **73 report something** — five of them net new over the hundred-and-eighty-first to -third: a stencil painted with a *tiling* pattern stopped being drawn in a colour nothing had set (2, ADR 0151), a substituted font that draws **none** of its characters stopped being silent (10, ADR 0152), and eight of those ten then drew, because a substitute is now chosen by coverage (ADR 0153) — 0 slower than 30 s | `tests/corpus.rs`, ~3 s |
| oracle (1794 pages vs poppler, mupdf, ghostscript) | of **1683** we call complete: **856 agree**, **68 contradicted**, 750 ambiguous — **678 of them diagnosed and 72 held by name since the hundred-and-seventy-sixth** (§3a) — 9 not comparable, 2 a reference's geometry | `tests/oracle.rs`, **36 s** |
| text (vs `pdftotext`, same 974) | **98.2%** of the reference's words (22 931 of 23 349), **36** named below the 0.90 floor — and the two figures above them were 22 970 of 23 390 for at least two sessions, which is a denominator nothing in this tree now produces | `tests/text_extraction.rs`, ~30 s |
| — | **and it had been failing for ten sessions**: session 156 lifted six documents to 100% and left them in `TEXT_BELOW_FLOOR`, so the ratchet fired *on the improvement*. Pruned in the hundred-and-sixty-sixth; the percentages never moved | see that constant's own comment |
| **quorra vs the CPU oracle** (974 documents, page one, same display list) | **913 agree, 43 differ, 1 refused**, 17 not comparable — 28 of the 43 are the two rasterisers' glyph antialiasing and not a defect list (ADR 0156). It was **failing from session 310 to 311** with six refused: `encode.rs` charged the GPU coverage lane's `rgba16float` winding texture unconditionally while `device.rs` allocated it only `if !winding.is_empty()`, so the **default CPU lane** paid eight bytes per texel of its whole scratch sheet for a texture nothing created. Diagnosed, reproduced with a patched local quorra, reported as `doc/QUORRA_FEEDBACK.md` §10 and **answered upstream at `0a1ffb13` the same day** — `device_bytes` now answers zero for an empty sheet, which says the condition once instead of twice | `render-quorra/tests/corpus.rs`, **24 s** |
| dates | 1545 date strings, 1514 conforming (97.99%) | `tests/dates.rs` |
| **§14.3.2's XMP** (same 974) | 319 documents carry a metadata stream: **318 read, 1 refused**, 3191 properties between them, 106 stating `dc:title` — the refusal is a fuzzed file whose stream does not decode at all. New in the two-hundred-and-ninety-fourth (ADR 0186) | `tests/xmp.rs`, ~0.3 s |
| **JPEG 2000 vs ISO/IEC 15444-5's reference software** | 30 corpus codestreams: **14 byte-identical, 13 differing, 3 not comparable**, and **no remaining difference exceeds one level** — down from 87. Two defects, one clause, and each hid the other: ISO/IEC 15444-1's E-6 reconstructs a nonzero coefficient at `r · 2^(Mb − Nb)`, and `hayro-jpeg2000` 0.4.0 applied *none* of that term while upstream `9cce046b` applies it except where `Mb − Nb` is zero — a **fully decoded** coefficient, where `2^0 = 1` makes the term `r` itself. The first was this project's hypothesis in the two-hundredth session, confirmed in the three-hundred-and-eleventh; the second was found in the three-hundred-and-eleventh by bisecting on resolution and fixed in `close2/hayro` `2a1abd14`, which this tree pins (ADR 0190). **3.4 M differing samples became 5 900**, the population never moved, and the residual — one level on 0.02% to 0.1% of a plate — is an open question with the rounding mode and FMA already ruled out. `doc/JPEG2000_FEEDBACK.md` §§7–8 | `tests/jpeg2000.rs`, ~8 s |
| the window | **Tab draws a focus ring** — `160F-2019.pdf` under `Xvfb`, two presses, the ring round its first widget, captured with `xwd` (ADR 0126's recipe, and `xdotool windowfocus` before `key --window` is what makes the press arrive). page one of ISO 32000-2 drawn in a real window on `Xvfb`, presented in **22.4 ms**, and five arrow keys turn to page 6 presenting in **9.5 to 15.9 ms** with nothing refused — re-run in the three-hundred-and-ninth at 9.6 to 22.7 ms over the same five keys, with the whole launch **125.4 ms** on a machine running the corpus gates beside it (`arguments` 0.016, `chrome fonts` 3.4, `event loop` 28.9, `window` 29.2, `graphics instance` 33.2, `graphics device` 55.0, `document joined` 58.9, first present 125.4). Measured undisturbed in the two-hundred-and-ninety-second, where the whole launch is **98.8 ms** (ADR 0179's timeline, ADR 0185's threads), on `lavapipe` through quorra and so not comparable with a real adapter, where `examples/first_frame` puts the first frame at 18.2 ms and the tenth at 4.1; the shape is what the row is for. **The sidebar opens by itself** — that document states `/PageMode /UseOutlines` — and its title bar reads *ISO 32000-2:2020 (PDF 2.0) including Errata Collection 3* rather than the file name, because it also sets `/DisplayDocTitle` — and since the two-hundred-and-ninety-fourth that string comes from the `dc:title` Table 147 actually names rather than from `/Info /Title`, which on this document happens to say the same thing | ADR 0126's recipe, session 175 |
| conformance | **416 quotations**, all verbatim, 4033 citations, 193 distinct tables cited by the tree, **823 ledger rows** | `-p conformance` |

Counts are **ratcheted**: they may only improve, except where a rise is a new report and is
written down as one (trap 5). The 14 specification PDFs in `doc/` — including ISO 32000-2 itself,
1023 pages, 101 318 objects — all parse, draw page one with nothing reported, and extract 100% of
`pdftotext`'s words.

**Read the oracle's 45% ambiguous with care.** 370 of those pages are two long books of dense
text at book size, where `Interpretation::glyphs` earns the page the *text* tolerance — 0.90
structural similarity, measured over 153 reference-against-reference pairs because the references
disagree with each other at worst-tile 26 to 28 on text. **This file said for many sessions that
those books are "set in fonts nobody embedded, so each renderer substitutes differently", and
`pdffonts` says otherwise**: `freeculture.pdf`'s four fonts are all embedded and nothing
substitutes on any of its pages (the two-hundred-and-twenty-ninth session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`).
That row means "reported nothing", not "drew it right" — and since the hundred-and-seventy-fifth session **emptying it is a task rather
than a caveat**: §3a.

**Both moving numbers move in both directions on purpose.** Contradicted pages: 174 → 65 over
sessions 6 to 61, steady at 65 until the hundred-and-forty-eighth took it to 70, the
hundred-and-fifty-sixth to **72** and the two-hundred-and-fifth back to **68** — the last two were `noembed-eucjp.pdf` and `noembed-sjis.pdf`,
recorded as drawing あいうえお "in a face the references do not have" — **and they were drawing
nothing at all**, which the hundred-and-eighty-second session found by making the silence loud
(ADR 0152). Both report now, and two widget-border pages left in the two-hundred-and-fifth session (ADR 0165), so the count is **68**. Five of the earlier ones were net,
argued and written down in `CONTRADICTED_SUBSTITUTED_FONT`: the standard 14 are compiled in now,
so we stopped reading the same URW faces off this machine's disk that the three C references
read, and the oracle noticed within one run (ADR 0133). Corpus documents drawing incompletely: 291 → 89 over
sessions 6 to 122, then 91 in the hundred-and-twenty-seventh, where two documents that had been
drawing the wrong font in silence started saying so, and **76** in the hundred-and-fifty-sixth.

### The ledger

All **823** subclauses of the eight technical clauses have been read against this code, since the
fifty-sixth session. Counts come from `cargo run -p conformance --bin ledger`, which prints them
— **not** from arithmetic in this file, which has been wrong about them twice.

| status | rows | |
|---|---|---|
| `implemented` | 384 | every normative requirement in the clause is executed |
| `partial` | 231 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so |
| `inapplicable` | 85 | a marking device, a layout engine, a production workflow |
| `out-of-scope` | 87 | principle 5's closed exclusions, which the row names |
| `reported` | 30 | not implemented, detected and named at runtime |
| `writer-side` | 6 | addresses a PDF *generator* |

**`silent` is zero** — there is no requirement in the eight technical clauses that this program
fails without saying so. That is a narrow claim: `partial` and `reported` are 261 rows between
them and each names what it owes.

**And a sixth was found in the two-hundred-and-sixteenth, by a sweep that is one `grep`:** a
sentence a session *retired* in one row, still standing in the other row that describes the same
mechanism. §11.6.4.3 recorded in the two-hundred-and-first that the graphics state's soft mask is
applied and had been since the eighteenth session; §8.9.6.1 went on saying it was "reported rather
than applied on 28 corpus documents" for fourteen more. **A correction is a string, and the string
is greppable** — `doc/todo/01`'s fourth sweep.

**A sixth way was found in the two-hundred-and-ninety-eighth, and it is the inverse of the first:
the note was corrected and the *status* was not.** §7.6.3 was `partial` above a note opening "both
algorithms are implemented in both directions"; §9.10 was `partial` above one saying all three of
§9.10.2's methods had been implemented a hundred and forty-two sessions earlier; §14.3 was
`partial` four rounds after the last of its four children closed. No grep in `doc/todo/01` can see
this, because the half a sweep reads is the half that is right. **The instrument is arithmetic**:
print every row that owes more than all of its own children, which is twenty lines over
`ledger.toml` and nothing else, and is `doc/todo/01`'s sixth sweep.

**And a fifth way for a row to be wrong was found in the hundred-and-seventieth to
-seventy-fourth**: not overstating, not understating, but *stale about its neighbour*. §7.7.2
listed eighteen catalog entries as unread that were read, most of them by the session that built
their clause; §12.6.3 said "this crate has no events" for forty-one sessions after
`Command::Pointer` landed; §14.3.3 was `inapplicable` because "this one has no panel" for seven
after one was drawn. **A family's parent row is not maintained by the sessions that implement its
members, because the clauses do not cite each other**, and neither is a row whose blocker was a
capability rather than a clause.

**This file's arithmetic was wrong about the oracle too, and session 154 corrected it by reading
the output.** The row above said "11 not comparable", which is 1665 minus the other three buckets;
the gate prints **seven** buckets and the missing two are `our geometry` (0 complete) and
`reference geometry` (2). Nothing had changed — the number had been derived rather than read, which
is the thing the paragraph below says not to do.

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

Every one is *reported* at runtime rather than silently skipped, and **each has a file in
`doc/todo/`** carrying the evidence, the clause and what it would cost. The count is how many of
the 974 documents' first pages it affects.

| Missing | Corpus | |
|---|---|---|
| A §10.7.4 mark that moves with sub-pixel placement | 1 | [todo 10](todo/10-hairline-mark-snapping.md) |
| A fill under an eighth of a device pixel; a tiling cell's two halves; a hairline at the raster's top edge | 4 | [todo 11](todo/11-shapes-that-still-disappear.md) |
| A substitute that cannot be addressed; **24 codes over 8 documents that reach no glyph in silence**; a per-character fallback, owed with no witness | 40 | [todo 21](todo/21-font-substitution.md) |
| A `/DA` font `/DR` does not define **and cannot be spelled** (Arabic, one document); a composite `/DA`, a list box, `/DS`, `/RV`; **Table 231's `DoNotScroll`, a `shall` binding since session 135** | 3 | [todo 22](todo/22-variable-text-edges.md) |
| Transparency departures (§11.4, §11.5.3, §11.6.6) | 19 | [todo 23](todo/23-transparency-departures.md) |
| A mask at a grid the bound refuses; JPEG 2000 at reduced resolution; sampled shadings on the GPU | 3 | [todo 24](todo/24-image-sampling-intent.md) |
| `/FixedPrint`, which waits on a printing path | 15 | [todo 25](todo/25-view-dependent-annotations.md) |
| An icon for `Stamp`, whose standard names are legends rather than symbols | 1 | [todo 26](todo/26-icons-a-clause-only-recommends.md) |
| Signature *validation*, public-key handlers (§7.6.5), `/R` 5 | 1 | [todo 51](todo/51-signatures-and-public-keys.md) |
| Sandboxing the interpreter and rasteriser | — | [todo 34](todo/34-sandbox-the-interpreter.md) |

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

**And a third, closed on the other side of a boundary.** A frame the graphics device refused left
the window blocking a second a present, for ever, and the mechanism was in the rendering library:
the swapchain texture was acquired *before* the frame budget was checked, and `Timeout` was the one
swapchain state that did not ask for a reconfigure. Written up in `doc/QUORRA_FEEDBACK.md` §7 and
**answered at `4aab7e2` in all three shapes it asked for**, including a `Device::invalidate_surface`
for a host that needs to say so itself. Re-measured by restoring this tree's own defect locally and
running the original report's recipe: a refused present costs **6 ms instead of 1.008 s**, nothing
reports `Timeout`, and the drag keeps updating. A page the device refuses for the other reason —
`bug1721218_reduced.pdf`, whose coverage outgrows a 16384 × 16384 scratch image — comes back on the
processor in 1.68 s and the window zooms, scrolls and opens its sidebar afterwards.

### The twenty rounds from the two-hundred-and-ninety-fourth

**A hundred-session refusal that turned out to be a question nobody had asked, two clauses whose
callers nobody maintained, and a sweep that is arithmetic.** What the twenty have in common is one
sentence:

> **A reason that names a decision expires the moment somebody takes it, and nothing announces
> that either** — which is the capability shape one level up, with the project itself in the
> place the capability used to be.

`doc/todo/50` had said for a hundred sessions that §14.3.2's XMP was blocked on "a dependency
decision this tree has not taken", with **319 corpus witnesses** behind it and, in the same file,
the four questions that would settle it: which crate, what its attack surface is, what it does on
malformed input, what the fuzz target looks like. Answering them took one round (ADR 0186).

- **§14.3.2 is read** — `xmlparser`, 2148 lines, no dependencies, `#![forbid(unsafe_code)]`, a pull
  tokenizer that resolves no entity and opens no file, so the billion-laughs expansion and the
  external entity are structurally absent rather than defended against. 318 of 319 packets read,
  3191 properties, a ninth fuzz target seeded with all of them and clean at 1 000 000 runs. §12.2's
  `/DisplayDocTitle` is a *reading* now rather than a documented departure, and the census answered
  the question the departure had been carrying: 93 documents state a title in both places and one
  disagrees.
- **Two clauses had a reader and no caller**, which is `doc/todo/01`'s fifth sweep from both ends.
  §14.8.2.5's logical order had been implemented since session 156 and a selection was still taken
  in content order; `Tree::logical_range` is the map between the two orders' *offsets* and it
  refuses rather than shortens, because a copy that silently dropped what the structure tree missed
  would be worse than one in the wrong order. And §12.5.1's `/Tabs` was owed to "a keyboard this
  program has no key for" — all five of Table 31's orders now, with `Command::Focused`, the key the
  clause itself names, and `Query::Focus` so the ring can be drawn round what it lands on.
- **A sixth sweep, and it is arithmetic rather than a grep**: print every ledger row that owes more
  than all of its own children. Five hits, four wrong, three of them in a shape no grep can see —
  **the note corrected and the status left behind**. §7.6.3 was `partial` above a note opening
  "both algorithms are implemented in both directions".
- **And the ambiguous bucket went 94 → 76**, on eleven pages and four documents, with two results
  about the *instrument* rather than about any page: step 6's assumption fails in the direction
  that is a finding — a difference identical at 8× and 16× is neither scan conversion nor the
  reduction, which is what found `AMBIGUOUS_ICC_MATRIX_PROFILE` — and a gap between two *clusters*
  is still a question for the ladder, on `prefilled_f1040.pdf` where the clusters were 2.4 of 255
  apart and neither was a defect.

**Three failures of this project's own process, all in the same place.** A correction recorded in
`doc/todo/01` and never made to the row it named (§7.9, seventeen rounds later). A count written by
a script whose `assert` sat before its write, so two rounds' commit messages reported numbers the
handover never received. And a habit written into `02-every-round.md` one round after the first
occurrence and one round before the second. **A habit recorded and not performed is the same
failure as a correction recorded in a todo file** — grep the number back out of the file.

**And one refusal taken and written down as a refusal.** Table 231's `DoNotScroll` is a `shall`
that has bound this program since the hundred-and-thirty-fifth session, found in §12.7.5.3's own
"Not read:" list behind the reason *"constrains typing"*: **a flag that constrains typing binds a
program that types.** It is owed rather than half-done, because what it costs is a measurement —
`variable_text::lay_out` reporting where it stopped — and because truncation without a caret is
half the behaviour the clause describes.

---

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

### 0. The UI boundary — built, with two consumers on it and one panel on top

**Everything a viewer still owes was blocked on one missing interface.** Since the
hundred-and-thirty-first session that interface is code — `crates/viewer-core`, ADRs 0116 to
0121 — with `viewer-ui` on it as a tier-2 host and `tests/headless.rs` driving it with no display
at all. This section is now half description and half instruction: read the first half to know
what is there, the second to know what is next.

#### Why it was the headline

Five owed items were the same item, and **four of them are done**: a password prompt, which this
file called "the missing piece, not the clause" for twenty sessions and which session 132 landed
in eleven lines of host code; an editable field (sessions 135 and 136); the layer panel, whose
data `Query::Layers` has answered with since 131 and which session 167 drew; and presentation
mode's clock (`Command::Tick`, 150). **The fifth is AccessKit**, and it is a host and not a
vocabulary: `Query::AccessibilityTree` has answered since 149 and nothing asks.

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
  `GoTo(PageTarget)`, `Zoom`, `Scroll`, `SetGroup`, **`Activate(ObjectId)`**, `Pointer { at, action }`,
  `Select`, **`Focused(FocusMove)`** (§12.5.1's tab key), `Edit(Edit)`, `Undo`, `Redo`, `Save`, **`Extract { name }`**,
  `Supply { purpose, bytes }`, `Tick { millis }`, `RenderReady { token, rendered }`. **`Activate` is what a panel row sends** — the object, not a
  payload, so the *document* decides what activating it means (ADR 0144).
- `Event`: `Opened`, `OpenFailed`, **`PasswordRequired`**, `Closed`, `PageChanged`,
  `NeedsRender(RenderRequest)`, `Damage(Rect)`, `OpenUri`, `NeedsFile`, `Transition`, `Dirty`,
  `Saved { bytes }`, **`Extracted { name, bytes }`**,
  `Reported { document, page: Option<usize>, notes }` — the `None` page is what the *document*
  says about itself (§12.11, §12.8, §7.11.4), said before any page is drawn.
- `Query` → `Answer`: `PageCount`, `CurrentPage`, `PageGeometry`, `LinkAt`, `FieldAt`,
  `Selection`, **`LogicalSelection`** (§14.8.2.5), **`Focus`** (§12.5.1's ring), `Find`, `Dirty`, `Outline`, `Layers`, `Attachments`, `AccessibilityTree`,
  **`Opening`** (Table 29's `/PageMode` and `/PageLayout`), **`Properties`** (§14.3.3's Table 349),
  `Preferences`, `Frame`, `Reports`. **`Selection` answers in device pixels
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

#### What is still owed

**The writer's two recorded costs are closed** — encryption on the way out (ADR 0129) and
§12.7.4.3's appearance stream written into the file rather than owed to the next reader (ADR
0130). What it still owes is an `Edit` variant carrying a *new object* rather than a field's
value: [todo 33](todo/33-annotation-editing.md), where `Update` in `view.rs` already allocates the
object numbers.

**The vocabulary is complete**, and ten sessions of building on it added five messages rather than
changing any — `Command::Activate`, `Command::Extract`, `Event::Extracted`, `Query::Opening`,
`Query::Properties`, each because a *clause* needed a channel — with one variant **removed** the
session after it was added, because the fuller reading of §12.3.3 made it a path nobody takes
(ADR 0144). **Two variants changed shape in the two-hundred-and-fourteenth**, both for the same
reason and neither adding a message: `Command::Zoom` gained the viewport point to hold still
(ADR 0166) and `Answer::Field` gained §14.9.3's second name (ADR 0167) — where a host needs two
things a variant carried one of, the variant changes and every consumer fails to compile, which is
what nothing being `#[non_exhaustive]` is *for*.

So what is left of §0 is **hosts**, and each has a file: [30](todo/30-a-native-host.md) a native
host and then `viewer-ffi`, [31](todo/31-accessibility-host.md) AccessKit, and
[32](todo/32-presentation-player.md) a presentation player. **Ctrl + wheel zooming landed in the
two-hundred-and-fourteenth session**, and the interesting half was in the core rather than in the
host: a zoom anchored at the pointer has to hold a page point that `Open::origin` knows about and
the scroll does not, because a page smaller than the viewport is *centred* (ADR 0166).

**The panels this file called "the largest single thing this project owes" for thirty sessions
are drawn.** `viewer_ui::chrome` is a sidebar of four tabs and a modal card, in a `pdf-render`
display list at an identity transform so that both backends draw it (ADRs 0142 to 0145). None of
the gates can see any of it, which is why `viewer-ui/tests/panel.rs` rasterises the panel's own
display list with `render-cpu` and **counts ink** rather than asserting a command count — checked
by deleting the glyph fill, which fails four of its eight cases.

**The one thing it needed that did not exist is worth naming, because it is not a UI problem.**
Text this program generates for *itself* had no font: every route into `pdf-font` takes a
`&Document` beside a `&Dictionary`, and an interface has neither. `LoadedFont::standard` loads one
of §9.6.2.2's fourteen through the ordinary `LoadedFont::load` against a new `Document::empty`, so
the encoding is §9.6.5.2's and the widths are the clause's own — and an interface set in Helvetica
is set in the same Helvetica on a machine with no fonts installed.

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
construction rather than by discipline. **This paragraph used to add "and it is not a compromise
here, because `CLAUDE.md` makes the CPU backend the startup path", and that reason is gone**: the
project owner decided in the two-hundred-and-seventy-third session that page one goes to the
graphics device, so tier 1 is a portability choice and not a startup one. Cost, with a number: 1920×1080 RGBA is 8.3 MB,
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
because the geometry was already there. **§14.8.2.5's *logical* order is the layer's fourth consumer** since the
two-hundred-and-ninety-sixth session: a selection is taken in content order — which is what its
shapes are in — so `Tree::logical_range` maps a *range* of the readback through the structure
tree's order and `Query::LogicalSelection` is what a host asks when a person presses copy. It
answers nothing where the tree does not reach every byte of the range, because a copy that
silently dropped what the tree missed would be worse than one handing back content order.
**What is still not built on it**: a caret, and word and paragraph selection.

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

Form-field editing landed in the hundred-and-thirty-fifth session and saving in the
hundred-and-thirty-sixth. What is left of *using* a document — markup and free-text annotations and a
caret, §14.8.2.5's logical order having gone in the two-hundred-and-ninety-sixth — is one file:
[todo 33](todo/33-annotation-editing.md). Editing the page's own text is far and deliberately out
of scope until those exist.

### 1. Third-party data: shipped, and the record of what was read

**This project is MIT** as of the hundred-and-thirtieth session (relicensed from MPL-2.0; one
author in the whole history, so nobody else's consent was needed). `deny.toml`'s allow-list
dropped MPL with it.

**All three items shipped**, in the order this section used to prescribe, and the table stays
because it is the record of what was read — off copies on this machine, not recalled:

| data | source examined | terms |
|---|---|---|
| Adobe predefined `CMap`s | `poppler-data`'s `COPYING.adobe` (1990–2019), `doc/pdf.js/external/bcmaps/LICENSE` (2009), `hayro-cmap`'s `assets/LICENSE.txt` (2023) | **BSD-3-Clause** |
| Foxit standard-14 programs | `doc/pdf.js/external/standard_fonts/LICENSE_FOXIT`, from PDFium | **BSD-3-Clause** |
| Liberation Sans | `LICENSE_LIBERATION` | **SIL OFL 1.1** (reserved font name: ship and use freely, do not modify and keep the name) |
| poppler's `cidToUnicode`, `nameToUnicode`, `unicodeMap` | `poppler-data`'s `COPYING` | **GPL-2 or GPL-3** — Glyph & Cog's, *not* Adobe's |

`BSD-3-Clause` costs three obligations: reproduce the notice and disclaimer "in the documentation
and/or other materials provided with the distribution", keep them in source, never use Adobe's or
Google's name to endorse this. **The surface for that is `/NOTICE`** — at the repository root,
`include_str!`d by `pdf-viewer --licences`, put over the page by `?` as the About panel, set in
§9.6.2.2's own Courier and deliberately *not* re-wrapped, because re-flowing text a licence
obliges this program to reproduce would be editing it. `viewer-ui/tests/notices.rs` checks that
every `.pfb` and `.ttf` under `data/` is named **by file name**, that the required sentences are
verbatim, and that the bytes still hash to `SHA256SUMS` — that test exists because `cargo deny`
reads Cargo metadata and **cannot see vendored data**.

**The trap is the last row.** `poppler-data` is two data sets under two licences and says so. A
`CMap` gets code → CID; getting a CID to a glyph in a **non-embedded** CJK font needs CID →
Unicode, which is the GPL half. The permissive equivalent is Adobe's own `Adobe-Japan1-UCS2`,
`Adobe-GB1-UCS2`, `Adobe-CNS1-UCS2`, `Adobe-KR-UCS2` — BSD files inside the `cMap` directory. For
an *embedded* CIDFont none of it is needed: the font's own charset or `/CIDToGIDMap` answers.

**What shipped**: `data/standard-fonts/` holds §9.6.2.2's fourteen font programs, 804 KB, so
those pages reproduce on any machine and `substitute.rs` is no longer the only machine-dependent
code in the tree (ADR 0133); `data/cmaps/` holds all 239 `CMap`s Adobe publishes, deflated one at
a time by `build.rs` into a 3.9 MB blob and inflated only when a document names one, so nothing is
decompressed at startup (ADR 0140). §9.10.2's third method came with the second, and it is where
the gates moved: 15 documents left the incomplete list, 9 more oracle pages agree, the readback
went 97.9% → 98.2%.

**Three things worth keeping out of the first**: PDFium's `.pfb` files are bare CFF programs
(`01 00 04 02`), a name-keyed substitute is addressed by glyph *name* and so needs no Adobe Glyph
List step — which is why `Symbol` and `ZapfDingbats` work — and a *composite* font cannot use them
at all, because §9.7.4.2 leaves it reachable only through `/ToUnicode`, which addresses by
character.

**What none of it fixed** is the 40 fonts naming an `Identity` ordering, where the codes index a
font nobody supplied: [todo 21](todo/21-font-substitution.md).

**And a todo file's claim decays exactly as a ledger row's does, with no sweep watching it.**
That file named two documents whose "characters no single face on this machine has" and the
two-hundred-and-fifty-sixth session opened the pictures: both draw every character, and had since
ADR 0153's coverage rule landed seventy-three sessions earlier. The claim was a *prediction* about
that rule which nobody re-checked after it shipped. `doc/todo/01`'s five sweeps read `ledger.toml`
and `crates/`; **`doc/todo/` is a third population and is watched by nothing**.

**Provenance is a principle-4 question**, and the tree has one precedent — `pdf-spec`'s Arlington
tables, built by `build.rs` from a pinned submodule. Vendored data arrives the same way: a
checked-in tool, a pinned upstream revision recorded beside the bytes, the licence file verbatim
next to what it covers.

### 2. The ledger, and where a false claim can still hide

**The reading task itself is [todo 01](todo/01-ledger-partial-rows.md)** — the three sweeps, the
five shapes a stale note takes, and what the last run found. What belongs here is the part that
is about *this file's* claims rather than about the ledger's.

- **Keep `REVIEW_OWED` empty.** A clause the code cites and nobody has read is the cheapest debt
  this project can accrue, and the list fails the build the moment one appears.
- **`FILE_ONLY_EVIDENCE_CEILING` is zero, asserted with `==`.** 58 → 0 over four sessions of
  auditing (ADRs 0098, 0100, 0101, 0102), **every one of which found a false or unheld claim**.
  It does *not* say the right test was named: three of the four false claims it hid were caught
  by the oracle rather than by a row.
- **A gate cannot see a cache.** ADR 0115's defect drew wrong glyphs on two documents in silence
  for thirty-one sessions: no report, no contradicted page, and one of them sat on the text gate's
  "undiagnosed" list at 83%. **Where a lookup is memoised, ask what the key claims.** Every cache
  in the tree keys on object identity, checked one by one in session 128.
- **A silence is not a gap**, and the first move on one is neither a report nor a feature: work
  out what the clause asks *of this device*. §10.7.5's `/SA` was implemented in the half a display
  can state and recorded as a departure in the half it cannot; §11.7.4's overprinting was six rows
  a reading of Table 146 removed altogether.

### 3. What the corpus still names

**The oracle's 68 contradicted pages**, grouped and ratcheted in both directions in `oracle.rs`,
where each group carries its own diagnosis and its measurement: 4 page rounding, 2 our own
anti-aliasing at a shape's edge, **21 glyph edges** whose ink matches the consensus to a fraction
of a level, 7 a shared JBIG2 decoder, 1 a shared *gap*, 3 a link border, 1 a sub-pixel image, 1 a
`CalRGB` alternate, 1 an eight-bit mask value, **5 a `DeviceCMYK` conversion**, 2 a reference that
drew nothing, 1 a CID width, 1 a negative line width, **21 substituted fonts**, **1 a tight consensus**, **0 unexplained**.

**The unexplained list is empty**, from 14 four sessions ago and from 42 at the start, and no
session that emptied it opened a debugger — the method is in
[todo 00](todo/00-ambiguous-bucket.md), which is the same method the ambiguous work uses. The
last two went to the two-hundred-and-forty-second and -third, both on the two-ladder closed form:

- `freeculture.pdf` page 313 → `CONTRADICTED_GLYPH_EDGES`. Ours at 8× is **6.0729** against a
  limit of 6.0658 and 6.0819, so the marks are right and the difference is 0.16 of 255 of glyph
  coverage at the page's own scale.
- `issue7891_bc1.pdf` → `CONTRADICTED_TIGHT_CONSENSUS`, the new name for what trap 12 describes.
  The two ladders agree to **0.0014 of 255** — the tightest limit in this file — and **ours at
  the page's own scale is 0.004 from it, the nearest of all five**, while `poppler` and `mupdf`
  are both 0.09 under. They vote because the bound is twice *their* spread, and they agree to
  0.009.

**Every printed metric on both pages is inside the class bound.** A verdict of contradicted can
be a statement about the consensus pair rather than about the page, and both of the last two were
that — which is the argument for the closed form: it is the one number derived from no reference
at all.

**Two cautions the contradicted list earned.** A page may be contradicted for a reason other than
the one its group names — seven for seven, so far, on the group being wrong — and "make it match
mupdf" is the failure principle 5 forbids. And a page can be contradicted by a departure this
project decided on purpose: `colors.pdf` pages 1 and 2 left the unexplained list in session 68 and
are *not* fixed, because §10.7.4 asks for the hard edge and this tree anti-aliases
(`CONTRADICTED_ANTIALIASED_EDGES`, and `doc/todo/_scan-conversion.md`).

**The 73 incomplete documents** — 74 until the three-hundred-and-fourteenth and 76 until the two-hundred-and-eighty-second, where a `Tf` naming
`/Helvetica` with an empty resource dictionary stopped meaning nothing, because §9.6.2.2 says those
fourteen names name something every processor has (ADR 0183): fonts (fewest since session 6 —
session 156's `CMap`s took 15 off this list), 19 transparency, **8 annotations over 8 documents** — Table 179's line endings took the ninth in the three-hundred-and-fourteenth (ADR 0192) — 8 malformed images, 8 operator soup
(`BT` without `ET`, `BDC` without `EMC`, fuzzed streams), 1 content, and since the
hundred-and-ninety-third 4 whose font program draws nothing (ADR 0157). Session 59's reading of
the corpus's own issue trackers says most of the font half is glyph rasterisation on files chosen
for having hard fonts, which session 68 then measured on one.

### 3a. The ambiguous bucket — watched since the hundred-and-seventy-sixth, and being emptied

**750 of the pages the oracle judges on documents we call complete come back `ambiguous` (786 of
all 1794), and until the hundred-and-seventy-sixth session no gate watched one of them.** 72 are
still undiagnosed, from 754: the twenty rounds from the two-hundred-and-fifty-first took three
populations at once and then worked the tail a page at a time. The verdict means "nobody's difference is large
enough to call anybody wrong", which is the right thing for the *ratchet* to do and is not the
same as "right". `issue7406.pdf` drew a JPEG cyan-on-black inside an `ambiguous` verdict for as
long as anybody looked, and it is correct now, and **nothing announced either event**.

The project owner's judgement, in the hundred-and-seventy-fifth session, is that the tree is far
enough along for this to be the work rather than a caveat. It is the last large population where
a defect can live without a name, and **the task, the instrument, the method and the next names
are [todo 00](todo/00-ambiguous-bucket.md)**.

**What it has produced, because that is the argument for keeping at it.** Forty-five sessions,
**fourteen defects found and thirteen of them fixed** — the newest being a page this tree drew
*nothing* on, which the ranking rated 0.73 and the step-7 sweep found at −1.783 (§12.5.6.4's text
annotation attached to a point) — — a page one that was page two (ADR 0148), a
photograph rendered black (0149), a shading painted as a square (0150), a stencil that drew
nothing (0151), a whole grid that disappeared (0154), a sentence drawn as one Greek letter
because the font's name ends in the word "Symbol" (0158), a stamp's gradient painted flat
(0160), a widget's border losing a fifth of its ink to a clip on its own edge and a comb field's
separators losing theirs to a miter bound (0165), a `loca` whose offsets descend so that 36 of
one font's 71 glyphs were refused in silence (0170), **§8.7.4.5.4's greatest *admissible*
root** — found in the two-hundred-and-sixth session, fixed in the two-hundred-and-thirty-second
on all three backends at once (0171), and the longest-standing of them because every gradient
library gets it wrong the same way — **a blurred word nobody drew** (0173): §8.6.8's
uncoloured restriction was still in force inside a soft mask's own group, so a `d1` glyph
procedure that set a `/Luminosity` mask had its mask evaluated to zero and painted nothing, with
every command present and nothing reported — and **a space that was a bar** (0174), where the
`loca` repair of sixteen sessions earlier read a glyph's length from its own bytes even where
the table said, in the standard's own spelling, that the glyph was empty.

Beside them: a pattern cell's clip worth 15% of a page's ink (0155), ten documents whose
substituted font drew none of its characters in silence (0152), the coverage rule that made
eight of them draw (0153), and a font program that draws nothing now saying so (0157).

**The eleventh is found and not fixed**, from the two-hundred-and-fifteenth: a stroke under a
pixel wide loses the half of `tiny-skia`'s hairline smear that falls outside the raster's top
edge, so `vertical.pdf`'s two hairlines carry 55% of their area at the page's top and 98%
everywhere else ([todo 11](todo/11-shapes-that-still-disappear.md) item 3). The bucket itself
went 754 → 492 undiagnosed and 257 pages carry a diagnosis; *eleven defects nobody could see* is
the number to watch.

**Step 6's own assumption failed for the first time in the two-hundred-and-sixteenth**, on
`issue2177.pdf`: the closed form takes a reference to eight times the resolution because a
renderer's departure from the geometry shrinks with the pixels, and `poppler` on a §8.7.3 tiling
pattern goes the other way — 34.15 → 18.03 → 16.32 from 72 to 2304 dpi, its strokes thinning
rather than its edges sharpening. Ours is flat across four scales and `mupdf` at 8× agrees with
us to four significant figures. **A limit is only a limit if the thing taking it is converging,
and one ladder cannot tell convergence from drift** — take two.

**And the two-hundred-and-fifteenth session cleared the whole ranking above 1.6 in one sitting —
seven pages — which is a result about the *list* rather than about any page.** Two were a face
nobody ships and where §9.8.1 puts the answer, two were one word on a page the size of a postage
stamp, two were hairlines, one was an eight-bit ramp on a stamp fixed sixteen sessions earlier.
**The top of the ranking is populations now rather than defects**, and the one new defect in it
came from a synthetic ladder rather than from a reference.

**And the ninth was a correction rather than a finding, which is why it is here.** Two of the
eight above quoted an ink table that was **half** ours and `hayro`'s and whole for the three C
references, because the method file's own command averaged an alpha channel in — a defect
session 161 found, fixed and wrote down in two places, neither of them the file a session reads
when it goes hunting. Both ADRs carry the correction, the recipe is repaired, and there is a new
closed form beside it: the same page at eight times the resolution, which is what says *which*
renderer is measuring area. ADR 0163.

**And the eighth was a gate rather than a page.** `jp2k-resetprob.pdf` sat first on the ranking
at 5.03 and its name is a JPEG 2000 coding option; checking that hypothesis meant decoding every
`JPXDecode` stream in the corpus against ISO/IEC 15444-5's reference software, which **ruled the
codec out for that file and found thirteen of the thirty codestreams wrong** — every one of them
on the irreversible 9/7 path, by up to 87 levels of 255. Four codecs reach this tree through
dependencies and only two of them had ever been checked against anything. ADR 0161,
`doc/JPEG2000_FEEDBACK.md`.

**The seventh was the ranking's own first name.** `issue7821.pdf` sat at 5.44 bounds with a
stamp whose rounded box looked like a plausible flat green fill and is a shading pattern in four
other renderers: an annotation's appearance stream is a form XObject, and §8.7.2's rule about
where a pattern's matrix points was applied on the `Do` path and not on the appearance path, so
the axis landed off the page and `/Extend` painted one colour. **§8.7.2's ledger row has now been
wrong twice about the same sentence, once per way of becoming a parent content stream.** ADR
0160.

**And the sixth was found by a comment rather than by a number.** `issue8697.pdf` was on the
text gate's list with a paragraph explaining that its readback was a question about §9.10.2 and
that "both readbacks are defensible" — four true sentences about the readback, none of which
asked why the page was in Greek. The defect was in font *substitution* one stage upstream, and
the gate that could see the symptom had closed the question downstream of it. ADR 0158.

**Two of those are worth repeating here because they are about this file rather than about a
page.** `CONTRADICTED_SUBSTITUTED_FONT`'s comment said two documents drew "the same five kana in
the same places in all four panels" — our panel was white, and the sentence described the
*references'* half of the side-by-side. **A group's comment is a claim about a picture, the
picture is one `Read` away, and no gate can check a comment.** And `ambiguous` is not a measure of
how wrong a page is: `issue13372.pdf` sat at 26.95 bounds inside a verdict that cannot tell a
blank page from a grainy one.

### 3b. The quorra backend, and what a corpus-scale comparison found in it

**A second GPU backend arrived in the hundred-and-eighty-sixth to -eighth sessions**, written
against `doc/RENDER_LIBRARY.md`'s brief in its own tree and adapted here by `render-quorra`; the
window presents through it. It came with eleven cross-backend scenes and four real pages, which
is a better suite than the Vello backend ever had — and trap 12b is about exactly that gap.
**`render-quorra/tests/corpus.rs` closes it** (ADR 0156): every one of the 974 documents' first
pages, both backends handed the *same display list*, so a difference is two rasterisers
disagreeing and a refusal is a hole in the new one. Three ratchets held by name — refused,
differing at the edges (similarity above 0.99), differing in shape — and both renders of every
differing page written to `target/tmp/quorra/<stem>/`.

**The first run was 900 agree, 50 differ, 7 refused, and every finding in it has been answered.**
The three were written up for the library's team in `doc/QUORRA_FEEDBACK.md`, with the command
that reproduces each, and the same document now carries what closed them:

- **§10.7.4's rule was not asked for** — trap 2's shape. `issue4260_reduced.pdf` drew an empty
  box at similarity **0.49** because the rule lives in `pdf_render::collapsed` so that both
  backends inherit it (ADR 0154) and the new one did not ask. It asks now: **0.9938**.
- **The caches never evicted**, which only a long run can see: 533 of 952 pages refused at 4×
  with the 512 MB budget full, and a page refused in the full run passing on its own. Entries
  carry recency now and the device releases down to half its budget after every frame —
  **zero** resource refusals at 4×, where 413 pages agreed before and 918 do now.
- **Six refusal messages named a byte count under the budget they exceeded.** They add up now,
  and the one that replaced the mis-stated limit says what it is.

**And the shape list bought something nobody asked for**: reading it found strokes under an
*anisotropic* transform being given one scalar device width, which is exact for a similarity and
exactly wrong for a shear. Four documents left the list.

**Where it stands: 913 agree, 43 differ, 1 refused** — and 28 of the 43 are the glyph
antialiasing floor, which shrinks as the page grows (17 pages differ at 2×, 16 at 4×).

**Performance, offscreen, readback included** (AMD 890M, RADV, release): at the page's own scale
the CPU backend takes **2.55 s** over 956 pages against quorra's 6.26 s, median page **2.05×**;
at a window's 2× it is 5.21 s against 10.16 s, median **2.87×**; at 4× — comparable for the first
time now that the eviction fix draws 934 of 952 pages — 11.34 s against 24.13 s, median 3.24×.
The totals ratio *improves* with scale while the median *worsens*, and both are true — our CPU cost grows with the pixels
while the median page is dominated by a per-frame floor that does not. Quote the total against
the median and say which. The window is not measured here and this gate is deliberately not the
place to: a presented frame pays no readback.

### 4. Performance

**One fair comparison exists, and a second Rust renderer was tried and is not one.** Every other
renderer here is C; `hayro` is Rust, forbids unsafe, and rasterises on the CPU single-threaded as
we do. `rasterrocket` is Rust too and fails all three of the other conditions — 335 `unsafe` sites,
CUDA and Vulkan, and no way to start the clock inside its process. ADR 0136, and the summary is
below.

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

**`rasterrocket` compared, in the hundred-and-fifty-fourth session, and the whole of it is ADR
0136.** It is an OCR front-end rather than a viewer, so most of what differs is two programs asked
different questions. Three things survive that:

- **It is slower, and its own shape says why.** Measured over 91 of a 98-document sample through
  `hayro-speed --per-document` joined to its CLI's timings, on the pages where its work clears its
  own 7.35 ms process floor: at 72 dpi **4.52× the median** and 859 ms against our 147; at 150 dpi
  **1.70×** and 1118 ms against our 462. The ratio halves because **our page-one time grows with
  the pixels and theirs does not move at all** — `tracemonkey.pdf` costs it 106.4 ms at 72 dpi and
  106.4 ms at 150, against our 18.6 and 34.6. A viewer is asked for the same page at many
  resolutions, so that is the axis the comparison actually measured.
- **It draws no path fills in this build and says nothing.** `alphatrans.pdf` loses three
  rectangles and a shading; a four-object hand-made document that `pdftoppm` marks 3267 sampled
  pixels of comes back blank. Exit 0 both times. Its golden-image harness has an empty case list,
  which is how 1330 passing tests coexist with that. **This is the strongest external evidence
  this project has that the corpus and the oracle are what make a correctness claim mean
  anything.**
- **It is not going in the oracle**, on four grounds in ADR 0136, three of which are traps already
  in this file: a reference that draws nothing votes for nothing (trap 9), its font module says it
  mirrors poppler's `getFTLoadFlags` exactly so on text it is a fourth vote from a reference we
  have (trap 9 again), and it is not in this repository so a gate on it would skip silently.

**What the comparison names for us is parallel rasterisation, and the first number is in.** Session
153 measured a dense text page spending four to six times as long being drawn as being read; their
fixed per-page cost against our resolution-proportional one says the same thing from outside.
`render-cpu` draws single-threaded, `Band` (ADR 0010) already has the geometry, and rayon is
already here. **The cost of the naive form is measured**: page 101's rasterisation re-measured at
**4 993 M** (session 153's 4 990 M reproduces), of which `CpuRasterizer::draw` is 4 104 M, and
inside it `render_cpu::convert::path` 405 M, `Rect::from_points` 218 M and
`RasterPipelineBlitter::new` 164 M are **787 M — 19% of the render — of per-command work that does
not shrink with the band**, and that a strip replay repeats once per strip a command touches.

**And the counter that decides it is written and run: `examples/strip_spans`, ADR 0137.** At eight
strips a command touches **1.01 to 1.13** of them on four pages, so the 19% is multiplied by 1.13
and not by 8 — a **2.5%** penalty, and duplication is not the problem. **Imbalance is, and a prefix
sum removes it.** The worst strip's share of the estimated cost, against a 12.5% ideal:

| page | equal heights | equal cost |
|---|---|---|
| ISO 32000-2 p. 101 | 15.9% | **12.9%** |
| ISO 32000-2 p. 6 | 15.8% | **13.0%** |
| `tracemonkey.pdf` | 22.3% | **12.6%** |
| `bug1721218_reduced.pdf` | **72.0%** | **12.8%** |

Equal heights give the project's worst page a 1.4× ceiling on eight threads; equal cost gives
every page tested within 4% of perfect. **So strips are chosen by cost, and equal heights are not
a simpler first version — they are the version that fails on the page that most needs it.** The
mask worry is settled too: a clip chain touches 1.06 strips of eight on the 3608-chain page, and
the chains that span many are page-wide ones whose masks are band-tall anyway.

**It was then built, and the oracle refused it. ADR 0138.** The driver worked — strips borrowing
disjoint rows of the pixmap, a `MaskCache` each, rayon — and it changed the picture: **four pages
newly contradicted** (`bug1811694`, `dates`, `issue14705`, `issue15597`). The cause was isolated to
one line of geometry. A curve **clipped by a strip's edge is re-parameterised**, so the clipped
curve is not the `f32` control points the unclipped one was, and its edge coverage differs by up to
64 of 255. A probe pins it exactly: split a page where the shape lies wholly inside one piece and
the result is **bit-identical**; split it where the shape crosses and it is not. The same run with
the strip count forced to one — same planner, same skip test, same refactor — is clean at 836
agreeing pages, so **the strips are the cost and not anything else in the change**.

Everything but the planner was reverted, because `CLAUDE.md` forbids shipping a path nobody takes.

**The probe it named was run in the hundred-and-fifty-fifth session, and the parallel rasteriser
ships. ADR 0139.** The answer is a table rather than a yes — filling one shape into a pixmap and
into two pieces of it, an axis-aligned rectangle crossing the cut differs in **0** bytes, an
oblique straight edge in 292–528 (worst 32), a cubic in 2480–2744 (worst 64). So ADR 0138's
proposed rule was **too weak** (a clipped line keeps its geometry and loses its endpoints) and
**too strong** (page 6's page-wide clip is a rectangle, and rectangles survive).
`pdf_render::unsplittable_rows` marks the rows a re-stated segment spans; `strip_boundaries_avoiding`
minimises the worst strip among the rows left, by binary search on that maximum, because ADR
0137's prefix sum snapped to the nearest legal row gives 24.5% against a 12.5% ideal. **Every
oracle verdict, corpus count and text percentage is unchanged**, which is what says the strips are
exact, and `with_strips` plus `strip_parallelism.rs` is the standing guard.

| page, at the scale a laptop window asks for | serial | split | strips |
|---|---|---|---|
| ISO 32000-2 p. 6 at 2× (1192×1684) | 20.8 ms | **7.9 ms** | 16 |
| ISO 32000-2 p. 101 at 2× | 27.0 ms | **10.9 ms** | 16 |
| `tracemonkey.pdf` at 2× | 33.5 ms | **15.8 ms** | 4 |
| `bug1721218_reduced.pdf` | 105 ms | 105 ms | 1 — no legal cut |

**Two things nobody planned were most of the work, and both are traps one level up.** A serial
per-pixel pass bounds a parallel render: `impose_on_medium` was **7.8 ms of a 17 ms page**, all of
it eight integer divisions per transparent pixel, and §11.4.7's isolated page group makes most of a
page transparent — a `[0,0,0,0]` pixel is exactly the medium, so that case is a copy and the pass is
1.7 ms before it is split at all. And a planner on the drawing path is not a planner in an example:
`command_extents` rebuilt every command's clip chain from the leaf, **606 ms** on
`bug1721218_reduced.pdf`, six times that page's whole rasterisation, correct and unmeasured for two
sessions because only an example called it.

**And ADR 0137's touch ratio was right about four pages and wrong as a property of pages.** The
oracle's first parallel run kept every verdict and went **37.0 s → 59.1 s**; five pages held most of
it and `issue12841_reduced.pdf` is their shape — **two commands, each covering the page**, replayed
sixteen times, 105 ms serial against 166 ms split. `pdf_render::replay_ratio` computes that number
per page and `plan_strips` refuses a division costing more than **1.25** of the list, after which the
oracle is **37.0 s, the serial figure exactly**, at +17% processor time. **This is the first change
in the tree where latency and throughput point in opposite directions**, and `CLAUDE.md` ranks them.

**Interpretation, by callgrind on `examples/callgrind_interpret`**: **2 137.7 M** in session 153, of which
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
shortcut nobody has taken is [todo 40](todo/40-mask-chain-crop.md).

**A page turn on the largest document was 380 ms and is 9 ms** (session 141, ADR 0124). §12.3.3's
`section_at` resolved every outline item's destination with `Pages::index_of`, which is a *search*
of the page tree — 988 items over 1023 pages, `O(items × pages)`, on the path of an arrow key.
`Pages::indices` gathers the whole map in one walk. Two more walks went with it: `Query::LinkAt`
6.05 ms → 52 µs and `Query::PageGeometry` 3.06 ms → 832 ns, both asked at pointer speed and both
`Pages::get` looking up the page already on the screen. **The gates cannot see any of this** — no
gate turns a page in a viewer and the specification's own PDF is in none of them — and the two
regression tests are *ratios* against a walk the test performs itself.

**The GPU backend's own question is open and has a plan** (ADR 0128, session 143), **and step 2
of that plan has been executed as a measurement and the answer is no** (ADR 0131, session 146).
Page 6 is 5933 fills of **107 distinct outlines**, and Vello re-flattens all 5933 every frame —
but the outlines are what is shared and the *coverage* is not. `examples/glyph_reuse` counts it:
an exactly-correct coverage cache hits **116 times out of 5933** on that page and **not once** on
`tracemonkey.pdf`, because a glyph's sub-pixel phase is an arbitrary float. The cache pays only
with the phase quantised, which is a positional departure, and **the oracle contradicts a page at
1/8 of a pixel and is clean at 1/16** — measured by applying it. At 1/16 the reuse is 5.0× on
page 6, 1.3× on `tracemonkey.pdf`, worth at most 39% and 11% of those pages' rasterisation before
the cost of a blitter `tiny-skia` does not provide. Refused, with the numbers.

**What the whole of that argument became is a document.** `doc/RENDER_LIBRARY.md` (session 165)
specifies the library this viewer would want from a team writing one: the input model, the API,
the clause-11 obligations an SVG-shaped model cannot express, the failure contract Vello breaks,
and a performance section built on the measurement `doc/gpu.txt` said nobody had taken.
**`examples/frame_split` is that measurement** — fastest of ten offscreen frames on the 890M —
and it reverses the obvious plan. Scene encoding is **1.1 to 1.6 ms and flat across a sixteenfold
range of pixels**, so a retained scene is worth 4% of a frame at 4× and 22% at a thumbnail's size.
What dominates is the per-pixel floor: the same viewport drawn from a list of **one rectangle**
costs 3.48 ms at 596×842, 8.77 at 1191×1684 and 26.73 at 2382×3368, which is **55% to 92% of the
frame before any of the page is drawn**, and most of it scales with bytes at about 1.2 GB/s —
consistent with the readback a windowed host would not pay. Page 6's 5 933 glyph fills cost 2.4 to
3.3 ms on top of that floor and **do not grow with resolution**.

**That changes the case for our own backend rather than removing it.** The atlas was ADR 0128's
headline and a GPU atlas quantises the same phase for the same reuse, so what it buys over Vello
is 1/16-pixel reuse and no more. The other four items — damage rendering, persistent geometry,
progressive rendering, clause 11 conformance — are untouched and are now the whole of the
argument. The rest of the plan stands: stale-frame zoom (perceived latency, host-side, judged
*ugly but acceptable for now* by the owner), a moving window of interpreted pages, then a spike
against Vello and `vello_hybrid`. A whole document cannot be resident: 70 MB of draw records is
affordable, the **4.0 s** to interpret 1023 pages is not, and the startup rule decides it.

**The item that profile found instead was taken the next session and it was the largest single
win this project has had on a text page** (ADR 0132). `calloc` was 18.3% of page 6's
rasterisation, *all* of it under `tiny_skia::Mask::new`, and `glyph_reuse` said why: **303
distinct `ClipId`s of one distinct region.** The producer wraps each of 303 text runs in `q … W n
… Q` with the same rectangle, and `add_clip` gave each an identifier of its own, so both backends
did the per-chain work 303 times. `DisplayList::add_clip` now returns an existing identifier for
an identical region — exact comparison, no tolerance. Rasterisation of the specification's pages
5, 6 and 101: **4.81×, 4.66× and 2.89×**. Interpretation pays +1.22% for hashing every clip. Every
oracle verdict, corpus count and text percentage is unchanged.

**And it dissolved ADR 0127's cliff on the page that motivated it.** Page 6 no longer overflows
Vello's buffers at 1.9008, or at 5.0 — the black page a person reported was, in substantial part,
this tree handing the device 303 layers for one region. The banding stays (Vello's constants are
fixed and another scene can still exceed them) and its witness is now synthetic:
`a_scene_too_large_for_one_pass_is_banded` gives each of page 6's fills a clip nudged so no two
are equal, which is what a producer with per-run rounding emits and what dedup cannot collapse.
**That test failed on its own guard rather than on a pixel**, which is why the change was noticed
at all.

**Rasterisation is now measured too, and on a text page it is the larger half.** Twenty renders
of the specification's own pages through `examples/callgrind_rasterise`: page 6 **3 601 M** and
page 101 **4 990 M** in session 153, against 16 771 M and 14 406 M before the clip deduplication of
session 147. Per page that is ~180 M and ~250 M of rasterisation against ~43 M of interpretation —
**a dense text page spends four to six times as long being drawn as being read**, which is not the
proportion this file's performance section has historically implied.

**Session 162 re-measured both after the strips landed, and the counter says what the clock does
not.** Interpretation is **2 139.4 M**, a repeat of 153's 2 137.7 M. Rasterisation was page 6
4 543 M and page 101 6 691 M — *up* 26% and 34% on the serial 3 598 M and 5 065 M measured in
the same sitting, against a wall clock that halved (ADR 0139's table). Callgrind counts every
thread, so a parallel render's instruction count is the serial one plus the replay and the
planning; the two numbers are measuring different things and both are true. **Quote the clock for
a parallel change and the counter for a serial one**, and say which — a session that reported
only the counter here would report a 34% regression on a change that made the page appear twice
as fast.

**And then the counter said where the overhead was, in one line of `callgrind_annotate
--tree=caller`.** `Path::bounds` was **17.6% of page 101's parallel render**, from 541 300 calls
over twenty renders — 3007 commands × 9 strips × 20 — because the strip driver asks every
command whether it misses the strip and the answer walked forty control points every time. A
`Path` now keeps its own untransformed hull in a `OnceLock` and maps it, which is **exact**
wherever the transform keeps the axes: `a·x + e` is monotone in `x`, so the same control point
attains the same extreme through the same arithmetic. A shear takes the walk.

| | serial | strips, before | strips, after |
|---|---|---|---|
| page 6 | 3 598 M | 4 543 M (+26%) | **4 035 M (+12%)** |
| page 101 | 5 065 M | 6 691 M (+32%) | **5 565 M (+10%)** |

Wall clock at a window's scale went with it: page 6 at 2× is **19.7 → 5.9 ms** and page 101
**33.7 → 10.1**, both 3.3× where ADR 0139 measured 2.6×. Interpretation pays **+0.8%**
(2 139.4 M → 2 156.9 M) for the branch and the `OnceLock`, which is the honest price of the
memo and is written here rather than left out of the comparison.

**Session 175 re-measured all three after ten sessions of change and none of them moved**:
interpretation **2 156.9 M**, the same figure to the digit; rasterisation of the specification's
page 6 **4 031 M** against 163's 4 035 M and page 101 **5 550 M** against 5 565 M. Nine sessions
of panels, extraction, trigger events and a parallel colour conversion cost the drawing path
nothing, which is what the numbers are for — and the one place they *did* move is priced beside
the change that moved it (ADR 0147).

**Session 195 re-measured after the ten from the hundred-and-eighty-sixth, and the total is
+0.41%.** Interpretation **2 184.4 M** against 185's 2 175.5 M; rasterisation of the
specification's page 6 **4 056.6 M** against 4 023.7 M and page 101 **5 566.4 M** against
5 566.0 M — the second is the same figure to four digits after two sessions that changed how
every fill is drawn, which is what says §10.7.4's rule and the tight stroke bound cost the
rasteriser nothing. **The interpretation figure was 2 211.1 M when it was first taken, and the
27 M between the two was one line.** ADR 0157's per-font tally used `BTreeMap::entry`, which
takes the resource name *by value* — an allocation per show string whether or not the font was
already in the map, and a page names three fonts and shows thousands of strings through them.
Stubbing the tally out measured 2 155.7 M, so the counter cost **2.2%**; hoisting it out of the
per-glyph loop and looking up before allocating gave back 27 M of it. **A counter is not free
where its key is a `String`**, and the way to find that out is to remove it and measure rather
than to read the profile.

**Session 185 re-measured after the ten from the hundred-and-seventy-sixth, and interpretation is
the one that moved: 2 156.9 M → 2 175.5 M, +0.86%.** Rasterisation did not — page 6 **4 023.7 M**
and page 101 **5 566.0 M**, both within the repeat noise of 175's figures — which is the right
shape, since nothing in those ten sessions touched the rasteriser. The 18.6 M is three things and
each is priced where it was spent: a type 1 shading's domain is hashed into a clip at every `sh`
and every shading-pattern fill (ADR 0150), a `Tf` now copies its resource name into the text state
so a report can say which font it means (ADR 0152), and every show operation tallies whether a
substituted face drew (ADR 0152). None is on a path a well-formed Latin page takes more than once
per operator, and all three buy a report or a mark that was missing.

**Colour-managing an image in parallel was taken in the hundred-and-seventy-first, and the item
named the wrong loop** (ADR 0147). `image::unpack` is the obvious target and is not the one a
JPEG takes: `zune-jpeg` writes components into the raster and `convert_channels` converts it in
place afterwards, which is where callgrind puts 27.6% of `issue19971.pdf` plus the 26.2% of
`libm` under it. Parallelising `unpack` measured as noise and was reverted; parallelising
`convert_three` **halves the page**, 110 ms → 57 ms of interpretation, at 1 085 M → 1 365 M
instructions. Eight bands rather than one per core: same clock, two thirds of the extra processor
time, because each band allocates a `Conversion` table and one sized to a twenty-fourth of the
image collides no less than one sized to an eighth. The split is exact because the memo is a memo
of a *pure function of one pixel's samples* — which is precisely what ADR 0138's strips were not.

**Still open, each priced and each with a file**: carrying an image *and its sampling intent* to
the backends, which is one `pdf-render` change unblocking three refusals
([todo 24](todo/24-image-sampling-intent.md)); a clip chain as one crop and one intersect on the
corpus's worst page ([todo 40](todo/40-mask-chain-crop.md)); and a decoded-stream cache, measured
at 0.7% of interpretation and deliberately not taken ([todo 41](todo/41-decoded-stream-cache.md)).

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

**Put the binaries where a person can run them, at the end of every round.** The agent builds
into `/home/AI/cargo-target/pdf-viewer/`, which the human's shell never looks at, so the last step
of a round copies what a person would run into the project's own `target/`:

```sh
cargo build --release -p viewer-ui --bin pdf-viewer && cargo build --release -p pdf-sandbox --bins
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer         target/pdf-viewer
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-sandbox-worker target/pdf-sandbox-worker
```

Both, and both beside each other: the worker is a separate executable the viewer spawns for JBIG2
and JPEG 2000, and a viewer that cannot find it refuses those images rather than falling back.
`doc/todo/02-every-round.md` is the rest of what a round does.

**And rebuild before saying anything about speed**: `cargo test` only ever builds the *debug*
binaries. The hundred-and-forty-second session was reported as "still lags" against a binary three
hours and six commits old, one of which was the 40× page-turn fix. A stale executable is a
measurement of the past.

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
oblige a binary to carry. Escape quits. The title bar names how many things on
the page could not be drawn and the things themselves are printed. A click follows §12.5.6.5's
links and performs the eleven §12.6 actions this program can, and on a markup annotation it **opens the popup window §12.5.6.14 gives it** — a card over the page with Table 172's `/T` in its title bar, Table 166's `/C` behind that and `/Contents` under it, closed again by a second click (ADR 0191) — printing every refusal — including
§12.7.6.4's import, which reads an FDF file **beside the open document** and nowhere else. A
locked document is asked for its password at the terminal (§7.6.4.1), three times, with an empty
line to give up. **`pdf-viewer --licences`** prints `/NOTICE` and exits, which is what both
licences covering the compiled-in standard 14 fonts oblige a binary to carry. `--no-sandbox`
decodes JBIG2 and JPEG 2000 in-process — faster by a spawn and a pipe round trip, appropriate for trusted
documents, and it prints what it gave up.

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

## Verify it

**Nothing here runs in a fresh clone until the specifications are unpacked**, which is one command
and is above.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
cargo test -p conformance -- --nocapture   # 4033 citations, 416 quotations, 193 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild another
# package's binaries. Build it first or the numbers below are somebody else's (trap 10).
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus          -- --ignored --nocapture  # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle          -- --ignored --nocapture  # 1794 pages, ~32 s
cargo test --release -p pdf-model --test text_extraction -- --ignored --nocapture  # ~30 s
cargo test --release -p pdf-model --test dates           -- --ignored --nocapture  # 1545 dates
cargo test --release -p pdf-model --test xmp             -- --ignored --nocapture  # 319 packets
cargo test --release -p pdf-model --test jpeg2000        -- --nocapture            # 30 codestreams
  # against ISO/IEC 15444-5's reference software; needs `opj_decompress` and says so when absent
cargo deny check                           # from the workspace root: fuzz/ is its own workspace
# The two platforms without a confinement, checked the way CI checks them. **`RUSTFLAGS` is not
# optional**: the workspace's lints are `warn` so that a local build stays usable and CI turns them
# into errors, so a cross-target check without it is a different build from the one that gates a
# push. Three dead constants off Linux got through exactly that gap (ADR 0194).
RUSTFLAGS="-D warnings" cargo check --target x86_64-pc-windows-msvc -p pdf-sandbox -p pdf-render --all-targets
RUSTFLAGS="-D warnings" cargo check --target aarch64-apple-darwin  -p pdf-sandbox -p pdf-render --all-targets
RUSTFLAGS="-D warnings" cargo check --target x86_64-pc-windows-msvc -p viewer-ui --bins
  # `--workspace --all-targets` does *not* cross-compile here: `criterion` pulls `alloca`, whose
  # build script needs a C toolchain for the target and this machine has neither MSVC nor macOS's.
  # The CI runners do, which is why the `platforms` job builds the binaries and these two check
  # what a benchmark does not reach.
cargo bench -p pdf-model
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret            # stops at the display list
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo run --release -p pdf-model --example glyph_reuse -- [file.pdf] [page] [scale]  # ADR 0131
cargo run --release -p pdf-model --example strip_spans -- [file.pdf] [page] [scale]  # ADRs 0137, 0139
cargo run --release -p pdf-model --example render_at -- [file.pdf] [page] [scale] [out.png]
  # our own render at any resolution, which is how §3a's step 5b tells a scan-conversion
  # difference from a difference in the shapes themselves
cargo run --release -p render-gpu --example frame_split -- [file.pdf] [page] [scale]
  # where a GPU frame's time goes: encoding, the whole frame, and the same target drawn from a
  # list of one rectangle. doc/RENDER_LIBRARY.md §6.1
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_open [file.pdf]  # §7.5's xref alone, in instructions rather
  # than in a wall clock that moves by 2× between runs of the same binary. ADR 0180
cargo run --release -p pdf-model     --example open_cost -- [file.pdf]
  # where the *launch path's* document half goes: §7.5's xref, the page tree, §12.3.3's outline,
  # §12.8's signatures, each on its own. ADR 0179, doc/todo/42
cargo run --release -p render-quorra --example bring_up  -- [all|vulkan|gl]
  # and where its device half goes: instance, adapter, device — one measurement per process,
  # because a second instance in one process is measured with the loader already warm
cargo build --release -p hayro-compare --bins && \
  cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf   # ~45 min
cargo run --release -p hayro-compare --bin hayro-speed -- --per-document ...  # one line per file,
  # which is how a renderer that is a *program* rather than a crate is joined to the table (ADR 0136)
cd fuzz && cargo +nightly fuzz run lexer         -- -runs=50000   # needs nightly
cd fuzz && cargo +nightly fuzz run cmap          -- -runs=50000   # §9.7's CMap parser
cd fuzz && cargo +nightly fuzz run crypt         -- -runs=50000   # §7.6's algorithms
cd fuzz && cargo +nightly fuzz run variable_text -- -runs=50000   # §12.7.4.3's /DA and layout
cd fuzz && cargo +nightly fuzz run forms_data    -- -runs=50000   # §12.7.8's FDF, §7.9.4's dates
cd fuzz && cargo +nightly fuzz run object        -- -runs=50000   # §7.3's object grammar
cd fuzz && cargo +nightly fuzz run document      -- -runs=50000   # §7.5's file structure
cd fuzz && cargo +nightly fuzz run xmp           -- -runs=50000   # §14.3.2's XMP, the tree's
  # only XML. Its corpus is seeded with all 318 packets the pdf.js documents decode to
cd fuzz && cargo +nightly fuzz run sfnt          -- -runs=50000   # §9.6.3's two glyph-table repairs
  # **seed its corpus with real fonts** — 60 `/FontFile2` streams out of `doc/pdf.js/test/pdfs/`
  # into `fuzz/corpus/sfnt/`. Unseeded it never forms a table directory and tests nothing; seeded
  # it produced two crashers in its first minute (ADR 0175)
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
clock** — and it is the tell for something outside this tree: session 166 saw it at 85.7% on an
unchanged corpus, which was `poppler` being upgraded on this machine and every cached
`pdftoppm` render becoming a new key. Nothing about the verdicts moved. Of that ~30 s, ~23 s is the three external renderers at a 99.7% hit rate; the rest is
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
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3, image decode | Where PDF semantics live. `annotation.rs` is selection and placement (§12.5.5) and knows no subtype; `appearance.rs` constructs what a subtype's clause states, splices under `/NeedAppearances`, and argues the refusals (ADRs 0030, 0032); `icon.rs` beside it is the one module that is pure invention and says so (ADR 0109). `view.rs` is the `ViewState` §12.6.4's actions change — **the precedent the edit log will follow**. `variable_text.rs` is §12.7.4.3 and the one place this tree *writes* a content stream. `image.rs` owns §8.9.6's and §11.6.5.2's masking, with `combine_on_the_finer_grid` the one place two rasters of different sizes are combined rather than refused, its `Decode` one table per component and its `Conversion` an *exact* per-image memo (ADRs 0034, 0035). `page.rs` is §7.7.3 and §14.11.2's five boundaries. `xmp.rs` is §14.3.2's metadata stream and the one module in the tree that parses XML — over `xmlparser`, with four stated budgets, and it is the reason §12.2's `/DisplayDocTitle` is a reading rather than a substitution (ADR 0186). `accessibility.rs`, `uri.rs` and `file_spec.rs` hold no PDF at all. Then one module per clause family: `action.rs`, `forms_data.rs`, `named_page.rs`, `structure.rs`, `article.rs`, `collection.rs`, `measurement.rs`, `thumbnail.rs`, `signature.rs`, `attachment.rs`, `page_label.rs`, `navigation.rs`, `requirements.rs`, `document_part.rs`, `viewer_preferences.rs` |
| `pdf-font` | Glyph outlines via `skrifa`, §9.6.2.2's fourteen and §9.7.5.2's `CMap`s compiled in | Owns both simple-font encoding algorithms (§9.6.5.2, §9.6.5.4 — ADR 0015). `name_keyed.rs` is what a name-keyed program offers a code, and `cff.rs` and `type1.rs` each produce one because §9.6.2.1's NOTE 1 makes them one format's two spellings (ADR 0040). `type1.rs` is the one program kept *parsed*, measured: re-parsing per glyph put 11 ms on `tracemonkey.pdf`. `cmap.rs` is §9.7, where `Code` carries a value *and* a length. `predefined.rs` is §9.7.5.2's 239 registered `CMap`s, deflated one at a time by `build.rs` and inflated on demand, and it is where §9.10.2's third method reads a collection's `-UCS2` table (ADR 0140). `standard.rs` is §9.6.2.2's fourteen font programs as `static` bytes, and it is what stopped `substitute.rs` being the only machine-dependent code in the tree (ADR 0133): the fourteen come from the binary, everything else from the machine with the binary behind it. `collection.rs` is a `ttcf` container in a `/FontFile2` — malformed by Table 127, written by two corpus documents — with the face chosen by the descriptor's own `/FontName` (ADR 0141). `substitute.rs` ranks three sources of a request with an argument — the name, then §9.8.3.2's PANOSE, then Table 121's flags, which producers set carelessly (ADR 0086) |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. Four device decisions live here so the two backends cannot differ: `Image::is_smoothed`, `Image::area_averaged` (a departure from §10.7.4, ADR 0025), `Stroke::device_width` (§8.4.3.2 with §10.7.5, ADR 0028) and `collapsed::split_collapsed_fill` (§10.7.4's "no shape ever disappears", ADR 0154) — the last two ask `thinnest_line` for the same width, which is the point of its being a function. `Command::Group` is the one nested command; `MeshRaster` is §8.7.4.5.5 shared by both backends because neither rasteriser has the primitive and a second copy would drift (ADR 0051). `Transform::max_stretch` is *not* `determinant().abs().sqrt()`: a shear separates the singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** the fallback for a frame the graphics device refuses — no longer the startup path, since page one goes to the device (`CLAUDE.md`, session 273). `blend.rs` is §11.3.5.3's four non-separable modes written here rather than shared, on purpose: sharing them would make the cross-backend scene compare one implementation with itself (ADR 0047). Draws a page on every core since session 155 (ADR 0139) — `encode_in_strips` cuts the target only at rows `pdf_render::unsplittable_rows` permits, so **the picture does not depend on how it was divided**, which is the property `with_strips` exists to let a test check and the reason the oracle's verdicts did not move |
| `render-gpu` | Vello/wgpu backend | Headless by construction. Its own soft-mask readback, because Vello's luminance mask is the SVG formula and no blend mode is a `/TR` |
| `viewer-core` | Toolkit-independent application logic | `Command` in, `Event` out, `Query` → `Answer` beside them (ADRs 0116, 0117). `select.rs` is every choice a selection needs and the standard does not state (ADR 0119); `interact.rs` is what a click does — §12.5.6.5's links and the eleven §12.6 actions; `notes.rs` is what a document says about itself when it opens. `viewer.rs` is the state machine and the one place a render is scheduled; `open.rs` is one document's page, zoom and scroll, and `fitted` there is why a page fitted to a window is not one pixel taller than it; `report.rs` words an `Unsupported` for a person, which is a presentation decision and so not `pdf-model`'s. `tests/headless.rs` is consumer #2 and the proof the crate's first sentence is true |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs`: a window, a keyboard, a GPU with `render-cpu` behind it for a page the device refuses (ADR 0125), and the two decisions a host owns — which files a document may name (§12.7.6.4) and what to do when one asks for a password (§7.6.4.1). Everything else is `viewer-core`'s. `src/chrome.rs` is the third thing a host owns and the newest: **text and panels this program draws for itself**, in a `pdf-render` display list at an identity transform so both backends draw it, and set in `pdf-font`'s compiled-in Helvetica. the sidebar's three tabs are §12.3.3's outline, §8.11.4.3's layers and §7.11.4's files; a native host would use its platform's tree view over the same three queries |
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
a `no render` count is a to-do list of pages nobody has looked at, and it is 18 — one left it in
the hundred-and-seventy-seventh session when a page the file's own cross-reference table had
hidden started rendering (ADR 0148).

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
- **`magick identify` every panel before believing any number, and the flags before that.**
  `pdftoppm` renders the **`/MediaBox`** unless told `-cropbox`; this tree, the oracle and
  `mutool draw` render the **`/CropBox`**. On `freeculture.pdf` the areas differ by 1.378, so a
  ladder taken without the flag put `poppler` at 9.10 against our 12.18 and would have
  manufactured a 34% defect on four pages that agree to 0.03 of 255. This is the twin of
  `-alpha off`, which returns exactly half the ink on a panel that carries an alpha channel;
  **both are a wrong measurement that looks like a finding**, and both now sit in
  `doc/todo/00`'s step 6 where a session reaches for the command.
- **Before trusting a clean fuzz run, ask what fraction of it got past the first branch.** The
  sfnt target ran 50 000 unseeded inputs in under a second and tested nothing: random bytes do
  not form a table directory, so every run left on the first `?`. Seeded with sixty real
  `/FontFile2` streams it produced two crashers inside a minute. A format with a magic number, a
  count and a directory needs a corpus; a content stream or a date does not. ADR 0175.
- **A rewrite driven by untrusted structure is a larger surface than a reader over the same
  bytes.** Both glyph-table repairs had been reviewed and never fuzzed, and both wrote at an
  offset a document supplied. ADR 0175.
- **A count of "marks missed" is a count of something else until you look at what they read
  back as.** 50 codes over 9 documents drew nothing and said nothing; 26 of them were one code
  of `pr12564.pdf`, and `pdftotext` reads that page as `1101#Strayer#Drive` — the code is the
  document's *space*, and having no outline is correct. The exemption that catches an ordinary
  space is "reads back as whitespace", which is blind to a font that reads a space back as `#`.
  `PDFVIEWER_TRACE_MISSING_GLYPH=1` is the trace that settled it in one run.
- **A page-level number cannot clear a mechanism of a defect that is five glyphs wide.** ADR
  0170's session A/B'd its `loca` repair against `issue7074_reduced.pdf` — ink 19.576 with the
  repair on and 19.576 with it off — and concluded the repair did not reach the page. The
  measurement was right and the inference was not: the page is three words of bold nine-point
  text and the defect was five narrow bars, under a tenth of a level. **Point the A/B at the
  quantity the hypothesis is about** — here, which glyph the space's code resolves to — which is
  one assertion rather than one render. ADR 0174.
- **A corpus can hold one document under a dozen names, and the bucket's shape lies until you
  check.** 154 of the ambiguous bucket's 678 were `tracemonkey.pdf` and eleven copies of it with
  annotations added — `pdftotext -f 9 -l 9 | md5sum` is identical across them. One measurement
  settled all 154, and the honest number to report is *one finding*.
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
- **A test corpus has a bibliography, and it is the first step rather than an occasional one.**
  Every pdf.js file is named after the issue that introduced it —
  `issueNNNN…pdf` → `github.com/mozilla/pdf.js/issues/NNNN`, `bugNNNNNNN…pdf` →
  `bugzilla.mozilla.org/show_bug.cgi?id=NNNNNNN` — and the issue says what the file was added to
  prove. It corrected a written conclusion on the first afternoon, and §3a now turns on it.
  **A pair of fixtures with a common stem is an A/B the corpus built for you**: `issue7891_bc0`
  and `issue7891_bc1` differ in `/BC [0 0 0]` against `/BC [1 1 1]` and in nothing else.
  Two cautions. The issue describes **that reader's** defect, which may be one this tree does not
  have — pdf.js's 7891 is *ignoring* `/BC`, which `soft_mask::backdrop` reads and §11.6.5.1's
  outside-the-bounding-box rule is applied for. And an issue is evidence about a *file*, never
  about the clause: principle 5 is not suspended because a bug report is specific.
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
- **A suite of shapes is a suite of shapes.** ADR 0138's equality test failed on three of eight
  cross-backend scenes and all three had a *curve* crossing the cut; a suite of rectangles would
  have passed and let the defect reach the oracle four pages later. Trap 12b asked what *size* a
  suite's scenes are and ADR 0046 what *parameter* they leave at its default — this is the same
  question a third time, about their geometry.
- **Count a suite's *cases*, not its tests.** `rasterrocket` has 1330 passing tests over 93 218
  lines and a golden-image harness whose case list is the comment "CASES is empty until fixture
  PDFs are added" — and it draws no path fill at all, silently, on a document `pdftoppm` renders.
  Ask of any suite: which of them renders the artefact the program exists to produce, and compares
  it to something? ADR 0136.
- **A constant that is right for the hand-built fixture is a landmine when a real file arrives.**
  `incremental_update.rs` replaced "object 1, the catalog", true of the file the test builds
  itself; in `bug900822.pdf` object 1 is the *encryption dictionary*, and the update wrote a
  catalog over it and produced a file no reader could open. Trap 12a's rule, one level up: take
  the identifier from the document, not from the fixture that happened to be first.
- **A test that skips silently is worse than no test.** A missing corpus is a skip; a present
  corpus that lacks what the test needs is a **panic**.
- **A gap measured on both sides is a fact; measured on one side it is an accusation.**
- **Agreement can be a shared *substitute*, and only removing the sharing shows it.** Six oracle
  pages became contradicted the session §9.6.2.2's fourteen font programs were compiled in, and
  none is a defect: `poppler`, `mupdf` and `ghostscript` resolve a non-embedded standard-14 font
  through this machine's fontconfig, so part of our agreement with them had been reading the same
  URW faces off the same disk. **Ask what data a reference reads from *this machine* before
  crediting its agreement.** ADR 0133, and it is trap 9's second shape from the inside.
- **A gate cannot ratchet what has no consumer**, and **fixing an instrument can be worth a
  feature** — one line moved 25 pages into the judged set and showed one drawing nothing.
- **A page can leave the contradicted list without a pixel moving** (the tolerance class comes from
  what *we* drew, so anything improving extraction loosens a bound — take the raster's digest
  before writing "fixed") **and can leave with pixels moving and still be wrong** (`issue20232.pdf`
  agreed once the y flip was fixed and still draws `56` where three references draw `⌀56`).
- **A page can be visibly wrong inside a verdict the gate cannot fail on**, and 45% of the judged
  set lived in `ambiguous` where nothing watched until the hundred-and-seventy-sixth session gave
  it a ratchet (§3a). The standing example was `issue7406.pdf`, which
  drew a JPEG cyan-on-black while its verdict stayed `ambiguous` — **and it is right now**,
  checked in the hundred-and-seventy-fifth by opening the artefact: all five renderers draw the
  same logo and the verdict is still `ambiguous` (mean 5.07 against a bound of 5.00). Nothing
  announced the fix, because nothing was watching then either. **A page in this bucket was
  unwatched in both directions**, so an example of it went stale as quietly as the defect did —
  which is the whole argument for the list the hundred-and-seventy-sixth session put under it.
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
- **"This crate does not have X" is a claim about the crate, and the crate is greppable.**
  §7.6.4.3.2's row said "this crate holds no Annex D table" for a hundred and twenty-nine
  sessions, and `text_string.rs` had held the whole of Table D.3 since the ninety-second — put
  there for §7.9.2.2, a different clause, in the same crate. **A capability recorded as absent is
  worth one `grep` of your own tree before it is believed**, and the two clauses that wanted it
  had no reason to cite each other.
- **A capability recorded as blocked on a decision outlives the decision.** §9.7.5.2's row said
  vendoring the predefined `CMap`s was "a licensing decision rather than a coding one" for a
  hundred and fifty sessions. The decision was taken in the hundred-and-thirtieth and written
  into this file; the *row* never heard, because nothing fires when a stated blocker expires.
  ADR 0140, and it is ADR 0108's regular expression finding its fourth instance — the first
  where the blocker was this project's own.
- **A test that pins a refusal must be rewritten when the refusal ends, and it will not fail
  helpfully.** `a_predefined_cmap_is_refused_by_name` failed with "a predefined CMap this tree
  has no data for must be refused", which reads like a regression and was a success. Its
  replacement asserts what says the `CMap` was consulted — that a *two-byte* code comes back.
  **The same session left the same shape in a ratchet and it went unread for ten.** Session 156
  lifted six documents to 100% of `pdftotext`'s words and left all six in `TEXT_BELOW_FLOOR`, so
  the text gate has been *red since*, with a message beginning "6 document(s) no longer below the
  floor" — and two sessions of "everything re-verified" recorded the summary line the run also
  prints. **When a ratchet fires, read which direction it fired in before believing the word
  `FAILED`**, and after a session that improves a population, prune the list *in the same
  session*: the handover entry said "six fewer" and the constant did not.
- **A wrong diagnosis is a silence with a sentence in front of it.** Two documents were refused
  for "units per em is zero" for eighty sessions; both embed a `/FontFile2` whose stream is
  *short*, and `metrics()` answers zero when it cannot find `head`. The refusal was right and
  the reason was not, so nobody could act on it. **Ask of any report whether its words name a
  cause or an effect** — and the condition for the new one had to be narrowed four times, each
  time by a document that draws (trap 11 again, on a condition rather than on a count).
- **A dependency's error message can name the fix.** `Invalid sfnt version 0x74746366` sat in
  the corpus output for as long as the gate has existed; those four bytes are `ttcf`, so the
  report was saying "this is a font collection" in hexadecimal. Reading it took ninety lines and
  closed two documents. **Convert the number in a refusal you have stopped reading.** ADR 0141.
- **Run the sweeps over the source, not only over the ledger.** The ledger has a gate and the
source does not, and the two-hundred-and-twenty-first session found four claims in `crates/`
false for between forty and two hundred sessions — `pdf-model`'s own crate documentation ("[t]ext
and images are not yet drawn"), `set_dash`'s ("only the 'solid line' case is honoured for now",
the sentence from before ADR 0018), and three of `requirements::unmet`'s arms whose capability
had arrived. **The last had predicted itself**: "a session that builds a layer panel has to come
back and change `OCInteract`". A warning written where the work is does not fire either.

**A retired claim is a string, and strings are greppable.** When a session disproves a sentence
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
  notes finds them in twenty minutes. ADR 0108. **The same regular expression paid again in the
  hundred-and-fifty-first**: §11.3.7.2 said a group's shape "needs §11.4.6", which the
  seventy-first session built — three sessions after the note was written, unnoticed for eighty.
  What §11.4.6 needed turned out not to be that shape at all.
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
  counting the panic. **Second instance, session 161**: our ink measured at exactly half the three
  C renderers' on ten pages running, 2.00 to three significant figures, which is not what hinting
  does. Our renders and `hayro`'s carry an alpha channel and the three C ones do not, and
  `magick -colorspace Gray` was averaging alpha in as a fourth channel — so **the tell was that
  the two renderers agreeing with us were the two whose output *format* matched ours**. Ask what
  the agreeing group has in common besides the answer.
- **A lesson recorded where it was learned and not where it is *used* has not been recorded.**
  The paragraph above was written in session 161, in this file and in `CONTRADICTED_GLYPH_EDGES`.
  The recipe in `doc/todo/00-ambiguous-bucket.md` — the file a session opens when it goes hunting
  — still carried the broken command, and sessions 197 and 199 followed it and drew the same
  wrong conclusion twice, with the same 2.00 ratio in front of them. Both ADRs carry the
  correction now and the recipe is fixed. **When a habit lands, ask which document a person will
  be holding when they need it.** ADR 0163.
- **Look at the heatmap's shape before opening anything else.** Twelve of the oracle's fourteen
  unexplained pages were diagnosed in two sessions without a debugger: a heatmap that is the
  whole silhouette says colour, one that is glyph outlines says grid-fitting, and the ink table
  then says which. Both are three minutes per page against a list that had not moved in twenty
  sessions.
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
- **A ratio measured on four pages is a fact about four pages.** ADR 0137 counted 1.01–1.13 strips
  touched per command and concluded duplication was not the problem, which is true of those four;
  `issue12841_reduced.pdf` is *two* commands each covering the page, so sixteen strips replay both
  sixteen times. Computing the same ratio per page is one function and is what made the split safe
  to ship. **Ask of any measured constant whether it is a property of the thing or of the sample.**
  ADR 0139.
- **A function only an example calls is a function nobody has measured.** `command_extents` rebuilt
  every command's clip chain from the leaf: 606 ms on one page, six times its whole rasterisation,
  correct and unnoticed for two sessions. **Before moving code onto a path a person waits on, time
  it there.**
- **A priced item names a loop, and the loop it names may not be the one the file takes.** The
  handover priced "colour-managing an image in parallel" for thirty sessions on
  `issue19971.pdf`'s photograph. `image::unpack` is the per-sample conversion, it is obviously
  the loop, and a JPEG does not enter it: `zune-jpeg` writes components into the raster and
  `convert_channels` converts that in place afterwards. Parallelising the obvious one measured as
  noise. **Before optimising a named function, check on the named file that it runs** — one
  `callgrind_annotate` would have said so before the change rather than after. ADR 0147.
- **Ask what a parallel unit's answer depends on before asking how to divide it.** A colour
  conversion is a function of one pixel's samples, so a band boundary changes which conversions
  are *repeated* and never which answer is given, and the split is byte-exact at any band size.
  A rasterisation is not a function of one row's geometry — a curve clipped by a strip's edge is
  re-parameterised — which is the whole of ADR 0138. The two look like the same problem and are
  not.
- **A serial pass over every pixel is what bounds a parallel render**, and it hides inside a
  function whose cost nobody attributed: `impose_on_medium` was 7.8 ms of a 17 ms page, all of it
  eight integer divisions per *transparent* pixel — and §11.4.7's isolated page group makes most of
  a page transparent. Amdahl's law names where to look after any successful division. ADR 0139.

- **A check deferred for cost belongs wherever the cost is already being paid, and nothing tells
  you when that place appears.** Table 45's `/CheckSum` was read and not verified for eighty-three
  sessions, on a reason that was true — "checking would mean inflating every attachment" — and
  that expired the moment one path decoded one stream. The clause names where it belongs:
  "the checksum of the bytes of the **uncompressed** embedded file". **After a session that
  makes something decoded, decompressed or laid out for the first time, re-read the entries whose
  reason for being unread was that nobody had it yet.** ADR 0145, and it is ADR 0108's regular
  expression looking for a different kind of blocker: not "needs §X" but "would cost too much
  here".

- **The three sweeps found a fourth shape in the hundred-and-ninety-first, and it is the
  strongest one yet: a row whose "this program has no ___" was about a *verb*.** §12.8.6 said
  a usage-rights signature grants "features of a PDF processor that are not available by
  default" and that "this program has no feature behind such a gate"; §12.8.2.3 said the same.
  Both were true when written and both stopped being true in the hundred-and-thirty-fifth and
  -sixth sessions, when this program learned to fill in a field and save the file — which are
  exactly the rights Table 258 grants and exactly the changes Table 257's `/P` restricts. And
  the requirement was not a new one: §12.8.2.2.1 has always carried, in a parenthesis, "(These
  changes to the document shall also be prevented if the signature dictionary is referred from
  the DocMDP entry in the permissions dictionary.)" A `shall`, addressed to a processor that
  modifies, unread for fifty-six sessions after this one became one. `ViewState::set_field` now
  refuses at `/P` 1 and permits at 2 and 3, and §12.8.2.3's `should` — remove a UR signature
  the modification exceeds — is named as owed. **After a session that gives the program a verb,
  the rows to re-read are the ones whose reason is about what the program *is*, not only about
  what a clause needs.**

- **Sweep for the reason's *shape*, not for its clause.** Sessions 118 and 122 grepped the
  ledger's notes for "while §X does not exist" and for entries claimed unread. The
  hundred-and-seventy-fourth grepped for a third shape — "this program has no ___", "no panel",
  "which this is not" — over `partial`, `reported` **and** `inapplicable` rows, and found
  §12.6.3 saying "[n]othing raises an event … this crate has no events", which stopped being
  true in the hundred-and-thirty-second when `Command::Pointer` landed. Forty-one sessions. The
  three sweeps are twenty lines of Python apiece and each has paid on its first run.

- **An `inapplicable` row whose reason is "this program has no ___" is a row waiting for a
  session that gives the program one.** §14.3.3 was `inapplicable` because "a viewer with a
  document-properties panel would read it; this one has no panel", and the panel arrived seven
  sessions before anybody re-read the row. That is the second instance after §12.7.4.2's field
  names, and the trigger is ADR 0122's: **after a session that adds a capability, sweep the rows
  whose reason begins "this program has no"** — `inapplicable` as well as `partial`, which the
  earlier sweeps did not cover.

- **Read the whole sentence a feature is built from, and count what the other half is worth
  before deciding.** §12.3.3 says a click makes a processor "jump to a destination **or trigger
  an action**"; the hundred-and-sixty-sixth session built the jump and shipped a `Command`
  variant shaped exactly like half a sentence. Two sessions later the count — 281 corpus outline
  items with an `/A`, 32 of them not a go-to — said the other half was one refactor away, and the
  variant became a path nobody takes and was removed. **A command shaped like half a clause is a
  command that will be replaced**, and the habit is ADR 0110's one level up: where a rule lists
  what it applies to, count them against the code *before* designing the interface.

- **A gate that cannot see a surface is a gate that cannot see a surface.** The corpus
  interprets page one, the oracle rasterises pages it is handed, the text gate reads words and
  the date gate reads strings — not one of them opens a viewer, so every line of chrome this
  project draws is unwatched by all four. `viewer-ui/tests/panel.rs` answers it the only way that
  discriminates: rasterise the panel's own display list with `render-cpu` and *count ink*, then
  delete the glyph drawing and check the count goes to zero. A test that asserted the display
  list held the right number of commands would have passed with every glyph missing.

### Code, bounds and dependencies

- **A gap inside a feature you have implemented does not announce itself.** Every missing
  *subsystem* reports, because whoever decided not to build it wrote the report. **A fast path
  inherits none of the rules of the path it skips.**
- **A "nothing here" is data, and dropping it is not the same as recording it.** §7.5's free
  entries and §7.5.8.3's unknown entry types both say an object number names nothing; both were
  *skipped*, so the question fell through to an older section and the reader resurrected objects
  its own file had deleted. **Ask what a `continue`, a dropped branch or an unmatched arm hands the
  question *to*.** ADR 0100.
- **A refusal is not a repair, and the difference is invisible from inside the function that
  refuses.** `Document::load` would not hand back an object whose header named a different number,
  which is right — returning object 2's bytes under number 3 corrupts the graph silently. What it
  handed the question to was the page-tree walk, which found object 3 was not a `/Type /Page`,
  skipped it, and returned the *next* kid: `issue7229.pdf`'s page one was its page two for the
  project's whole life, with `Pages::len()` answering 2 from `/Count` and `get(1)` answering
  `None`. **Every correct local refusal is a question passed upwards, and the caller may answer it
  by drawing something.** ADR 0148, and it is ADR 0100 one level along.
- **A partial repair can be worse than none.** The first version of that fix recovered *in-use*
  entries one at a time and left the misfiled free entry standing, so the page's image became a
  deletion and the page drew **nothing** where it had drawn the wrong page. A displacement is a
  property of the subsection; repairing half of one is a new file nobody wrote. Ask what class the
  defect belongs to before choosing the granularity of the fix.
- **The archetype is the `d` operator.** Every layer of dashing existed and one line read only the
  *empty* array, so not one dashed line in 974 documents. When a feature looks finished, check the
  operand path from the content stream to the state. **A feature switched off in one place is
  switched off everywhere it is not switched on**, and **a clause whose operators are implemented
  can still be unread** (`J`/`j`/`M` from the first commit; Table 57's `/LC`/`/LJ`/`/ML` for
  twenty-three sessions).
- **A cache that reports a perfect hit rate can still be missing.** `render-cpu`'s mask cache
  answered every one of the 303 lookups page 6 made and built 303 identical page-wide masks,
  because the key was the leaf's `ClipId` — a *name* — and the page states one region. **Instrument
  the count of distinct keys, not the hit rate**: a hit rate is a statement about the lookups you
  made, never about the ones you should have made. ADR 0132, and it is ADR 0115 with the sign
  reversed — that key was too weak, this one too strong, and both ask whether the key is what the
  claim is about.
- **A count of what is *shared* is not a count of what can be *reused*.** 5933 fills of 107
  outlines said a coverage cache would hit 55 times over; the outlines are shared through an
  `Arc` and the coverage is not shared at all, because the sub-pixel phase the count left out is
  what a coverage bitmap depends on. **Ask what the cache's key would have to be before believing
  the count.** ADR 0131.
- **A cache is a claim that two things are the same, and the currency of the claim is the key.**
  The font cache said it in the weakest one available — a resource name, which §7.8.3 scopes to the
  dictionary that defines it — and handed a form `XObject`'s `/F1` the page's glyphs for
  thirty-one sessions. Every other cache keys on object identity. ADR 0115.
- **A display list holding the right commands can still draw nothing, and no report will say so.**
  A type 5 mesh was complete, correct and 180 points from where it belonged. Between "we could not
  build it" and "we drew it" there is a third state only the oracle catches.
- **A representation can forbid a correct answer.** No evenly spaced array of colours can express a
  discontinuity. Ask what a data structure *cannot say*.
- **A file's extension is a claim, and the bytes decide.** PDFium ships the standard 14's Foxit
  faces as `.pfb` and every one of them begins `01 00 04 02`, which is a CFF header and not
  PostScript. Four lines of `xxd` settled what a module comment would have got wrong. ADR 0133.
- **A dependency's refusal can be silent *and* size-dependent.** `tiny-skia` insets the clip by a
  pixel before hairline stroking and returns early when the inset is empty, so a hairline stroke
  into a target under three rows tall draws nothing and reports nothing. Found by a test that had
  passed for a hundred and fifty sessions failing at one of its three scales — trap 12b's question
  ("what *size* is every case in this suite?") arriving from the other direction. ADR 0139.
- **A probe is a suite, and a suite of one shape proves one shape.** ADR 0138 split a page with a
  cubic in it and concluded "a clipped line is the same line"; a quadrilateral took ten minutes and
  said otherwise, which moved the rule and doubled what it permits. ADR 0139.
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
- **A dependency can be doing the thing your architecture forbids.** Trap 6 has said since the
  sixth session that `ColourSpace::to_rgb` is the only place a colour becomes RGB, and
  `zune-jpeg` was converting every four-component codestream to RGB with a formula of its own —
  reachable by any `DeviceCMYK` JPEG, invisible to `colour_paths.rs` because every fixture there
  states its samples as hex rather than as a codestream. **Ask of each dependency which of your
  own invariants it is in a position to break**, and write the fixture in the form the dependency
  actually sees. ADR 0149.
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
| 146 | The glyph coverage cache, counted: what is shared is the outline, not the coverage | 0131 |
| 147 | One clipping region stated 303 times became one: 4.7× on a dense page | 0132 |
| 148 | §9.6.2.2's fourteen font programs compiled in, and the notices that go with them | 0133 |
| 149 | §14.7's structure tree reaches a consumer: `Query::AccessibilityTree` | 0134 |
| 150 | §12.4.4.1's `/Dur`: a state machine with no clock is told the time | 0135 |
| 151 | The ledger re-read against seven sessions of new capability | — |
| 152 | §7.6.4.3.2 step (a): the Annex D table this crate said it did not hold | — |
| 153 | Everything re-verified after ten sessions of change | — |
| 154 | `rasterrocket` measured rather than read; the strips it named, built and refused | 0136, 0137, 0138 |
| 155 | The rows a cut may fall on: the CPU rasteriser draws a page on every core, byte for byte | 0139 |
| 156 | §9.7.5.2's predefined `CMap`s compiled in, and §9.10.2's third method with them | 0140 |
| 157 | A `TrueType` Collection where §9.9 states a font program, and the face the file names | 0141 |
| 158 | "units per em is zero" was a symptom; two font programs are simply short | — |
| 159 | Five `partial` rows swept: one blocker expired twice over and fourteen entries called unread | — |
| 160 | An unexplained contradicted page measured: one `k` operator, and four renderers' profiles | — |
| 161 | Eleven more measured: the unexplained list is 14 → 2, and the instrument had to be checked | — |
| 162 | Everything re-verified after seven sessions of change; the strips' instruction cost priced | — |
| 163 | 541 300 calls to `Path::bounds` become 3 007: the strips' overhead is a third of what it was | — |
| 164 | The window re-run on `Xvfb` after ten sessions, because no gate turns a page | — |
| 165 | What a rendering library would have to be, for a team writing one; the frame split measured | — |
| 166 | The first panel: §12.3.3's outline drawn in the fourteen fonts the clause guarantees | 0142 |
| 167 | The other two: §8.11's layers with their switches, and §7.11.4's files | 0143 |
| 168 | Activate the item rather than its destination, and §12.3.3 closes | 0144 |
| 169 | An embedded file comes out of the document, and its checksum finds a place to be checked | 0145 |
| 170 | A catalog row wrong about eighteen entries, and the two the sweep made worth building | 0146 |
| 171 | A photograph's colours converted on eight threads; the priced item named the wrong loop | 0147 |
| 172 | `/NOTICE` over the page in Courier: the About panel the owner asked for | — |
| 173 | §14.3.3's `/Info` shown, and an `inapplicable` row that decayed when a panel arrived | — |
| 174 | §12.6.3's events raised at last, and Table 167's `ReadOnly` read for the first time | — |
| 175 | Everything re-verified after ten sessions of change | — |
| 176 | The ambiguous bucket gets a ratchet, a ranking and its first diagnosis | — |
| 177 | A subsection filed one object too high: the page one nobody had ever seen | 0148 |
| 178 | A CMYK JPEG converted by its decoder rather than by the one conversion | 0149 |
| 179 | §10.7.5's last sentence decides a page four renderers could not | — |
| 180 | A type 1 shading's domain is a parallelogram, and we painted a square | 0150 |
| 181 | A stencil whose current colour is a pattern: a blank page, reported complete | 0151 |
| 182 | A substitute that draws nothing, and the sentence that said it drew | 0152 |
| 183 | A substitute chosen by coverage: eight blank pages draw | 0153 |
| 184 | §10.7.4 says no shape may disappear, and a page of ruling lines does | — |
| 185 | Everything re-verified after ten sessions of change | — |
| 186 | §10.7.4 says no shape may disappear, and a grid of ruling lines now draws | 0154 |
| 187 | A pattern's cell clipped where it cuts nothing, measured: 15% of the ink | — |
| 188 | And not applied where it cuts nothing — which found the bound it replaced wrong | 0155 |
| 189 | The quorra backend over the whole corpus: a fifth gate, and what it found | 0156 |
| 190 | A page that is all sub-pixel line work, and the pair that antialiases it | — |
| 191 | The three sweeps run again: §12.8.2.2's parenthesis became a `shall` when the program learned to write | — |
| 192 | The 320-page book's cover is one reduced JPEG, and `pdfimages` said so in twenty minutes | — |
| 193 | A font program that draws nothing says so, and the condition was not the one written down | 0157 |
| 194 | A one-bit scan reduced by six: the bucket's fourth entry is one image too | — |
| 195 | Everything re-verified after ten sessions of change; a 2.2% counter given back | — |
| 196 | The corpus feedback answered by the library's team, and re-measured here | — |
| 196 | The corpus feedback answered; the owed work moved into `doc/todo/` and this file halved again | — |
| 197 | A name that ends in the word "Symbol" is not the standard-14 `Symbol`, and §9.6.5.4 says which half of the file wins | 0158 |
| 198 | §12.8.2.3: a save that outgrows a usage rights signature withdraws it — and no corpus document can trip that | 0159 |
| 199 | An annotation's appearance is a form, so §8.7.2 places its patterns in *its* space — a stamp's gradient was flat | 0160 |
| 200 | A JPEG 2000 decoder nobody had ever checked: 13 of 30 corpus codestreams disagree with the reference software | 0161 |
| 201 | §12.3.2.1's other two items: a destination's magnification and its place on the page, owed for sixty-nine sessions after the window arrived | 0162 |
| 202 | The ink recipe was halving our own page, two sessions ran on it, and the fix for it was written in the wrong file in 161 | 0163 |
| 203 | Four pages off the ranking with the repaired instrument: three reductions against their own high-resolution limit, and an `scn` the file broke | — |
| 204 | A page turn is six events in one order, and Table 198's two had been read since session 77 with no caller | 0164 |
| 205 | A widget's border lost a fifth of its ink to a `/BBox` clip lying on its own edge; the miter bound was the thing in the way | 0165 |
| 206 | A radial shading is not a two-point conical gradient, and all three backends inherit the same wrong one | — |
| 207 | A miter over the limit is a *bevel*, not a long miter — the second bound in the same function, and a comb field's separators | 0165 |
| 208 | Six more of the ranking measured against the closed form; `render_at` asks *our* renderer the same question | — |
| 209 | A link border and a sub-pixel one on the same page; the second sweep run clean, which is a result | — |
| 210 | Sizing the radial fix found trap 2 in the middle of it | — |
| 211 | A 112-unit border on a 150×20 rectangle, and §12.5.4's one sentence settles it against `poppler` | — |
| 212 | A page whose ink everybody agrees on is one for the heatmap, not the ink table | — |
| 213 | Everything re-verified; two recipes that had stopped working; the miter arithmetic costs −0.01% | 0165 |
| 214 | Ctrl + wheel zooms about the pointer, which needed a page point the scroll cannot express; and a field's two names, one of which a `shall` asks for | 0166, 0167 |
| 215 | The ambiguous ranking above 1.6 emptied in one sitting — seven pages, four groups, and the only defect in them found by a synthetic ladder | — |
| 216 | A fourth sweep — grep the *string* a correction retired — and a page where the high-resolution limit is the thing that is wrong | — |
| 217 | §12.5.3's `NoZoom` and `NoRotate`, blocked for 217 sessions on a reason that was two reasons | 0168 |
| 218 | A stencil through a tiling pattern — and then the page was still blank, because every tile in the tree was measured from the wrong place | 0169 |
| 219 | Three more off the ranking, one of them a reference that drew nothing; and the fourth sweep's second catch | — |
| 220 | §12.5.6.4's own sentence about the view, reachable three sessions after the flag it names | — |
| 221 | The four sweeps run over the *source* instead of the ledger: four claims false for between forty and two hundred sessions | — |
| 222 | A `loca` that goes backwards, and half a sentence missing in silence — the eleventh defect the ambiguous bucket has produced | — |
| 223 | And the repair: a `glyf` rebuilt in glyph order, because an entry is self-describing | 0170 |
| 224 | The repair checked against the other five documents it could have touched, and a page where the spaces are marks | — |
| 225 | Three more where ours is nearest the geometry — and one where the two ladders do not converge, which is the rule working | — |
| 226 | One text field, four renderers, four answers — and one of them draws the string backwards | — |
| 227 | Three more, one of them the case where step 6 has one ladder and not two | — |
| 228 | A page where all five renderers paint one to seven levels more than the geometry, and ours the least | — |
| 229 | Why half the ambiguous bucket is ambiguous — and it is not the reason this file gave for it | — |
| 230 | Everything re-verified after seventeen rounds of change: five fuzzers, `deny`, the six gates and the window | — |
| 231 | The ranking's top page, and §10.7.4's own last sentence is the answer to it | — |
| 232 | A radial shading is not a gradient: §8.7.4.5.4's greatest *admissible* root, on all three backends | 0171 |
| 233 | A quarter of the ambiguous bucket was one paper under twelve names — and `pdftoppm` renders the wrong box | — |
| 234 | Two pages where the *reference* is alone, and both corrected a ledger row rather than a pixel | — |
| 235 | Fourteen pages of the standard 14, where §9.6.2.2 states the names and not one outline | — |
| 236 | Two `shall`s that cannot both hold: a strike-out follows the text it strikes out | 0172 |
| 237 | A blurred word nobody drew: §8.6.8's restriction reached inside a soft mask's group | 0173 |
| 238 | A page that states its own answer, and two rows whose neighbour's correction never reached them | — |
| 239 | An empty glyph stays empty — the `loca` repair had been handing the space a real glyph | 0174 |
| 240 | The whole bucket swept for missing content, and it is empty: −0.84 to +0.42 of 255 | — |
| 241 | A sixth fuzz target, over the two glyph-table repairs, and two crashers in its first minute | 0175 |
| 242 | The contradicted list's older unexplained page, settled by two ladders rather than a debugger | — |
| 243 | And the last one: `CONTRADICTED_UNEXPLAINED` is empty, from 42 | — |
| 244 | Codes that reach no glyph, counted at last: 109 over 14 documents, two thirds in two | — |
| 245 | And half of the count was a space the font reads back as `#`: the real silence is 24 codes | — |
| 246 | The one document left in that silence, diagnosed to an `/Encoding` naming a content stream | — |
| 247 | And the font named, which split it in two | — |
| 248 | Both halves traced to the end of every route the standard states: the subsets do not hold the glyphs | — |
| 249 | Everything re-verified after eighteen rounds: eight fuzzers, `deny`, the six gates and the window | — |
| 250 | Twenty rounds' worth of prose brought up to date, and what the twenty had in common | — |
| 251 | A selection costs a compositor layer a quad: reproduced, measured to the byte, not fixed | — |
| 252 | And it costs one now, whatever is selected: 268 quads in 16.5 ms, from a refusal at 63 | 0176 |
| 253 | The pointer was on a *link*: §12.5.5's appearances and §12.5.6.19's `/H` reached no other subtype | 0177 |
| 254 | The same sweep the other way — every `pub fn` nobody calls — and §8.11.4.3's `/ListMode` was one | 0178 |
| 255 | A thread followed to the *bead*: Table 163's `/R` composes Table 149's `/FitR` | — |
| 256 | A todo file's own claim was stale: both witnesses of the per-character fallback already draw | — |
| 257 | All ten of Table 197's events raised: `/Fo` and `/Bl` wanted a *press*, not a vocabulary | — |
| 258 | `/Helv` is not an arbitrary name: fourteen `/DA` abbreviations, and a no-break space | — |
| 259 | The ambiguous ranking's head: one level of green over a third of a page, and ours is the arithmetic | — |
| 260 | The next two off the ranking: an image reduced by four, and one line of ideographs | — |
| 261 | §12.3.4's thumbnail panel: a fifth tab, and `/PageMode /UseThumbs` names something at last | — |
| 262 | Both long books diagnosed as populations — 489 undiagnosed to **136** — and one page the band caught | — |
| 263 | `TAMReview.pdf` as a third population, and `calrgb.pdf` as §10.3.1's own sentence | — |
| 264 | The tail's new head: four pages, the tightest three-way ink agreement the bucket has produced | — |
| 265 | A page this tree drew nothing on, found by the sweep at −1.78: §12.5.6.4's point, not a rectangle | — |
| 266 | The sweep corrected three ways and run over all 787: twenty names past −1, every one explained | — |
| 267 | Six icons a clause only recommends, because their names name *objects*; `Stamp`'s do not | — |
| 268 | Everything re-verified after sixteen rounds — and ADR 0126's own recipe had decayed | 0126 |
| 269 | §12.3's parent rows, one round after their member landed; two more off the ambiguous tail | — |
| 270 | The capability sweep over `crates/`: 89 matches, 87 of them true, and both false ones ten rounds old | — |
| 271 | Twenty rounds' prose brought up to date, and the question the twenty had in common | — |
| 272 | quorra took all three of §7's suggestions: a refused present is 6 ms, not 1.008 s | 0176 |
| 273 | Page one goes to the GPU by decision, so bring-up is measured: 43.5 ms, 32.2 of it adapter choice | — |
| 274 | A launch is a timeline: **145 ms** to the first frame, and a 1023-page document opens 33× slower than a 5-page one | 0179 |
| 275 | Bring-up is on quorra's critical path now, and the brief still told it otherwise in three places | — |
| 276 | §7.5.6's rule stated once for a file rather than once per entry: `Document::open` 41% off | 0180 |
| 277 | A form says whether it has signature fields, and Table 225 says to ask: 1.681 ms to 0.017 | 0181 |
| 278 | The five sweeps: a row that contradicted *itself*, a parent stale about three children, a `shall` no host asked | — |
| 279 | Two off the ambiguous tail's head, and both were a *width*: 0.4 of a pixel, and one sentence on a stamp | — |
| 280 | The first frame pays 12 ms the tenth does not, and a 1 s sleep proves it is not the shaders | — |
| 281 | The document opens *beside* the window: nothing the window needs has ever looked at a PDF | 0182 |
| 282 | A `Tf` naming `/Helvetica` with an empty resource dictionary: the standard says the name means something | 0183 |
| 283 | A refused value's report named the wrong half: `Załącznik` is missing a *code*, not a glyph | — |
| 284 | So the invented font names the glyph: `/Differences [1 /aogonek]`, and poppler reads back *Zacznik* | 0184 |
| 285 | Everything re-verified after eleven rounds: eight fuzzers, `deny`, the seven gates, the window | — |
| 286 | Two more off the ambiguous tail, both into groups that already existed: an image and a border | — |
| 287 | §9.6.5.2's last sentence: `.notdef` substituted where an encoding names a glyph the program lacks | — |
| 288 | quorra took both of §8's asks and refused §8.3's knob: bring-up 33–45 ms to 13–19 | 0185 |
| 289 | A 1023-page document and a 5-page one now cost the launch the same: 5 ms of join | — |
| 290 | The sweeps over four rounds' nouns: §12.7.4.3 still said the reason was a glyph | — |
| 291 | Step 7 over all 786 after three rounds of new pixels: four names past −0.7, all diagnosed | — |
| 292 | Everything re-verified after quorra's update — and the round-285 deletion `rm -f` reported had not happened | — |
| 293 | Twenty rounds' prose brought up to date, and the sentence the twenty had in common | — |
| 294 | §14.3.2's XMP, 319 documents' worth: an XML parser taken, fuzzed, and `dc:title` in the title bar at last | 0186 |
| 295 | The sweep over the noun round 294 had just corrected — and §14.3.4, `inapplicable` on two capabilities that arrived a hundred and twenty sessions ago | — |
| 296 | §14.8.2.5's logical order reaches a selection at last: the clause had a reader for 140 sessions and no caller | — |
| 297 | Table 31's `/Tabs`: all five navigation orders, and the key §12.5.1 names bound to them | — |
| 298 | The sixth sweep, which is arithmetic: five parents owing more than their own children, four of them wrong | — |
| 299 | A reference that decoded an image wrongly and had the right amount of ink — the picture, not the metric | — |
| 300 | Four names for one measurement: the ranking pointed at a page whose answer was already written two pages over | — |
| 302 | The other half of §12.5.1: where the focus *is*, so a host can draw a ring round it — and one does | — |
| 303 | A gap between two clusters is still a question for the ladder — and three rounds of counts the handover never received | — |
| 304 | The second sweep, in a "Not read:" list: a flag that constrains typing binds a program that types | — |
| 305 | A page where our 8× lands *between* the two reference limits, 0.003 from one | — |
| 306 | An empty field's border and a page of two ramps: one climbs onto a limit 0.001 wide, the other has none | — |
| 307 | Nine of 255 painted and given back: the sub-pixel line-work group's extreme, and nobody is wrong | — |
| 308 | Two ladders ending 0.70 apart, which is the text tolerance's own premise demonstrated | — |
| 309 | The tightest *ratio* the tail has produced, and it was a page with no limit tight enough to be alone from | — |
| 310 | quorra's coverage lane chosen per frame from the magnification; the page extent taken once, in f64 | — |
| 311 | The fourteen specification documents out of the tree and back in encrypted; a snapshot release, and the two platforms it refuses to ship; the pipeline read for the first time in three days, and both its failures were this tree's; a budget that priced a texture nobody allocated; a JPEG 2000 hypothesis from session 200 confirmed, and the half of the same clause it had been hiding | 0187, 0188, 0189, 0190 |
| 312 | A window the page does not draw: §12.5.6.14's popups, from a capability reason that had expired — and Table 197's `/U` reaching only a release that also followed a link | 0191 |
| 313 | Four names off the ambiguous list for one measurement, and two `partial` rows whose blocker has been a sidebar since session 166 | — |
| 314 | Ten shapes with no size — Table 179's line endings, drawn — and the box a constructed appearance turned out not to have, which had been clipping four annotations away in silence | 0192, 0193 |
| 315 | Three platforms and one confinement: the `compile_error!` that kept macOS and Windows out becomes a sentence the viewer prints, by the owner's decision | 0194 |
