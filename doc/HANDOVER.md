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
| [`doc/verify.md`](verify.md) | every instrument that is not the round's gate sequence: `deny`, the twelve fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI | you have a reason to run one |
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
draws them in its own colour; a person can **fill in a form field** — in the window since the three-hundred-and-forty-ninth session, where the host keeps the *point* it clicked and never the text, so §12.7.5.3's truncation is read back rather than predicted (ADR 0201), **with a caret since the three-hundred-and-seventy-first** that says where the next character goes and moves with the arrow keys, so correcting the middle of a value is no longer deleting back to it (ADR 0211) — undo it and redo it; a click on a markup annotation **opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about activation and was owed to a capability this program had had for a hundred and eighty sessions (ADR 0191); and the
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

**Every number in this table was printed by the gate beside it in the three-hundred-and-ninety-eighth
session**, which ran the whole of `doc/todo/02-every-round.md` §2 and read each figure off the
output. Nothing in it is arithmetic performed here — that has been wrong twice — and nothing in it
is carried forward from a previous round.

**The three-hundred-and-ninety-ninth ran the whole sequence again and every line reproduced
character for character except the test count**, which its own seven tests moved. That is the
strongest thing it has to say about a change to the correctness oracle's rasteriser that halved the
corpus's worst page (ADR 0236).

| gate | what it printed | where |
|---|---|---|
| tests | `1398 tests run: 1398 passed, 9 skipped`, and `cargo test --workspace --doc` **1 passed** beside it, so `cargo test --workspace` reports **1399**. `clippy --workspace --all-targets` silent under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`; `fmt --all --check` clean | `cargo nextest run --workspace`, **28.9 s** |
| corpus (974 pdf.js documents, page one) | `974 documents in 4.3s: 0 unopenable, 8 locked, 2 encrypted beyond us, 5 pageless, **65 incomplete**, 0 slow` | `tests/corpus.rs`, **4.3 s** |
| oracle (1794 pages vs poppler, mupdf, ghostscript) | `1794 pages (1693 we call complete, 101 incomplete)`; **904 agree / 862 of them complete**, **69 contradicted / 67 complete**, **786 ambiguous / 753 complete**, our geometry 1/0, reference geometry 2/2, not comparable 14/9, no render 18/0 — and **the undiagnosed ambiguous list printed empty**, which is the ratchet holding. The two pages that became *complete* in the four-hundredth session both landed ambiguous and both have a group with a two-ladder diagnosis (ADR 0237) | `tests/oracle.rs`, **47.2 s** |
| text (vs `pdftotext`, same 974) | `overall 99.2% (24043/24243 words), 25 below 90%`, with 24 skipped and 62 incomplete and not gated | `tests/text_extraction.rs`, **30.1 s** |
| **quorra vs the CPU oracle** (974 documents, page one, same display list) | `957 pages compared in 33.5s: **912 agree, 36 differ, 9 refused, 17 not comparable**` — one refusal is `bug1721218_reduced.pdf`, whose coverage outgrows a 16384×16384 scratch image; **four are §11.4.6's stated shape**, which needs two Porter-Duff operators quorra's `Compose` does not have (ADR 0234, `QUORRA_FEEDBACK.md` section 14); **four more are §11.4.4's non-isolated group**, whose buffer has to start as a copy of the page where `GroupSpec` opens one on transparency (ADR 0237, section 16). All eight used to *agree* about a picture both backends drew wrongly | `render-quorra/tests/corpus.rs`, **33.5 s** |
| dates | `1545 date strings in 974 documents: **1514 conform** to §7.9.4 (97.99%), 31 do not, over 22 distinct strings` | `tests/dates.rs`, **0.9 s** |
| **§14.3.2's XMP** (same 974) | `319 documents carry §14.3.2's stream: **318 read, 1 refused**, 3191 properties between them, 106 state dc:title` — the refusal is a fuzzed file whose stream does not decode at all | `tests/xmp.rs`, **0.4 s** |
| **JPEG 2000 vs ISO/IEC 15444-5's reference software** | 30 corpus codestreams: **14 byte-identical, 13 differing, 3 not comparable**, and no remaining difference exceeds one level. `doc/JPEG2000_FEEDBACK.md` §§7–8 has the two defects behind that | `tests/jpeg2000.rs`, **13.8 s** |
| conformance | **5492 citations**, all naming clauses the standard has; **538 quotations**, all verbatim; **210** distinct tables cited by this tree and **249** named in the ledger's notes; **875 ledger rows** (400 implemented, 252 partial, 19 reported, 83 inapplicable, 8 writer-side, 113 out-of-scope) | `cargo test -p conformance`, **2.0 s** |
| **the round itself** | **not measured as one span this round**, and the honest number is what the gates themselves printed: **154 s** of test execution summed from the ten lines above (25.7 + 4.2 + 46.9 + 30.1 + 34.1 + 0.6 + 0.3 + 9.9 + 2.0), with each gate's incremental build on top and each run separately rather than back to back. `doc/todo/02` records **268 s** for §2 *and* §5's binaries together, from 608 s until the three-hundred-and-eighty-fifth measured every step (ADR 0222); the three-hundred-and-ninety-seventh read 287 s off file timestamps for §2 alone | ADR 0222, `doc/todo/43` |

**Two things beyond §2 were run in the three-hundred-and-ninety-eighth and are claimed**: the
`confined_wire` fuzz target, because the round added a decoder to the confined transport —
**13 942 159 runs in 181 s, clean** — and the **window under `Xvfb`**, because the round's whole
point is something a host does with a pointer. A click at the check box's own centre printed
`note: setting the field typeScript to Yes` and `note: this document has unsaved changes`, and a
second click printed `note: setting the field typeScript to Off`; ADR 0126's recipe, with the window
name taken from §12.2's `/DisplayDocTitle` (`PDF Form Example — page 1 of 1`) rather than from the
file.

**Not re-run and therefore not claimed**: the other eleven fuzz targets, `cargo deny` and the two
cross-target checks. The three-hundred-and-ninety-eighth added a
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
Read off that command in the three-hundred-and-ninety-sixth, unmoved from the round before:

| status | rows | |
|---|---|---|
| `implemented` | 400 | every normative requirement in the clause is executed |
| `partial` | 252 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so — **Annex O's five were the last, and they were built in the three-hundred-and-sixty-ninth** |
| `inapplicable` | 83 | a press, a layout engine, a production workflow — **and read at last** (ADR 0205) |
| `out-of-scope` | 113 | principle 5's closed exclusions, which the row names |
| `reported` | 19 | not implemented, detected and named at runtime |
| `writer-side` | 8 | addresses a PDF *generator* |

**`silent` is zero.** There is no requirement in the standard — the eight technical clauses or the
eight normative annexes — that this program fails without saying so. That is a narrow claim:
`partial` and `reported` are 271 rows between them and each names what it owes.

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
| A `/DA` font `/DR` does not define **and cannot be spelled** (Arabic, one document); a composite `/DA`, a list box, `/DS`, `/RV` | 3 | [todo 22](todo/22-variable-text-edges.md) |
| Transparency departures (§11.4.4, §11.6.6) — **§11.5.3's population closed in the three-hundred-and-eighty-third** (ADRs 0217, 0220), **§11.4.6's shape in the three-hundred-and-ninety-seventh** (ADR 0234) and **§11.4.4's non-isolated group in the four-hundredth** (ADR 0237), which found that NOTE 4's second accumulator is divided out again by the composite that follows it. What stands is one raster format — a painted group's blending space, all `/DeviceCMYK` — plus §11.4.6's knockout where the elements blend. Reports with no corpus member sit inside the closed ones, `/AIS` among them | 6 | [todo 23](todo/23-transparency-departures.md) |
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

### 0. The UI boundary — built, with two consumers on it and one panel on top

**[`doc/ui-boundary.md`](ui-boundary.md)** — the vocabulary (`Command`, `Event`, `Query` →
`Answer`), why each message exists, the five rules, the three pixel tiers, the text layer, the edit
log, and what is still owed. ADRs 0116 to 0121. What is left of it is *hosts*:
[30](todo/30-a-native-host.md) a native host and then `viewer-ffi`,
[31](todo/31-accessibility-host.md) the four edges the AccessKit bridge does not cover,
[32](todo/32-presentation-player.md) the presentation player's remaining five styles, and
[33](todo/33-annotation-editing.md) free text.

### 1. Third-party data: shipped, and the record of what was read

**[`doc/third-party-data.md`](third-party-data.md)** — the four data sets, the source examined for
each, the terms, what `/NOTICE` owes and what the GPL trap in `poppler-data`'s second half is; and
every dependency decision since, including the one in the three-hundred-and-ninety-second that came
out **no** (ADR 0229).

### 2. The ledger, and where a false claim can still hide

**[`doc/ledger-and-claims.md`](ledger-and-claims.md)**, and the reading task itself is
[todo 01](todo/01-ledger-partial-rows.md).

### 3. What the corpus still names

**[`doc/oracle-and-corpus.md`](oracle-and-corpus.md)** — the 67 contradicted pages grouped with
their diagnoses, the 70 incomplete documents split by report kind, and the two cautions the
contradicted list earned.

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
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
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
`cargo deny`, the two cross-target checks under `-D warnings`, the twelve fuzz targets and which
need a seeded corpus, the callgrind counters, the census and ladder examples, and the AT-SPI recipe.

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
  deliberately. `cargo-deny` is in the agent's `~/.cargo/bin`.
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
