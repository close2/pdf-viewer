# 912 — A second consumer for a damaged dictionary, and the door that stays shut for the other 327

Date: 2026-09-04.
ADRs: [0866](../adr/0866-the-second-consumer-of-a-damaged-dictionary-is-decided-by-its-own-clause.md),
[0867](../adr/0867-what-else-names-a-damaged-dictionary-and-why-the-door-stays-shut-for-each.md).
Touched: `crates/pdf-model/src/type3.rs`, `crates/pdf-model/src/content/font.rs`,
`crates/pdf-model/tests/damaged_char_procs.rs` (new),
`crates/pdf-model/examples/damaged_dictionary_consumers.rs` (new),
`crates/pdf-syntax/src/document.rs` (a claim that went stale),
`crates/pdf-model/tests/damaged_page_dictionaries.rs` (the same claim),
`doc/conformance/ledger.toml` (§7.3.7, §9.6.4), `doc/checks/fixed-documents.toml`,
`doc/traps/parsers-and-streams.md`, `doc/todo/03-more-corpora.md` §47, two ADRs, this file; and the
merge commit before this round's own. **One document in 90 535 changes what it draws, and no page
of `doc/pdf.js` does**, which is why every raster gate's figures are the merged tree's.

## The merge

**`round-907` (2451a301) is on `main` as `9b302ce3`**, `--no-ff`, on top of round 908. It contains
round 904, which `main` already had, so what arrived is one round's work rather than two: §11.5.3's
colorimetric branch at four components, carried by §11.4.7's pair of rasters moved inside the mask,
with `Press::luminance` sampling the profile's own `A2B` under one level of 255 everywhere.

**One conflict, and it is the shape a pin file has.** `doc/checks/fixed-documents.toml`: both sides
appended a `[[document]]` block at the end — round 907's `4605565.pdf` and round 908's
`cairo-85141-0.zip-3.pdf` — so the resolution keeps both in session order and the file goes from 69
rows to 71. `doc/conformance/ledger.toml` auto-merged and was checked by asking each side's diff
which rows it moved rather than by reading the merged file: `main`'s §7.3.7 and §7.3.10 from round
908 and the branch's §11.5.3 are the only three, and they are 2990 lines apart. Nothing else
overlapped, because `main` has not touched `doc/todo/23`, `doc/state-of-play.md` or
`doc/QUORRA_FEEDBACK.md` since the branch left.

**The thing the merge has to say out loud** — round 907 said it and it is repeated here because it
is a ranking the owner has been asked for, not a defect: of the 41 crawl pages that four-component
branch moves, **38 move away from what poppler, mupdf and ghostscript agree on** and 3 toward it, by
0.77, 0.84 and 1.47 levels of 255 in the mean. That is ADR 0797's `issue21346.pdf` disagreement one
component count over, it is one reading of one sentence applied to one, three and four components
alike, and nothing here changes it.

**`round-904` and `round-907` both stay open**, checked rather than assumed: `git merge-base
--is-ancestor` says rounds 867, 906, 909 and 911 all carry `round-904` as an ancestor, and
`round-907` is not yet merged anywhere else.

## The consumer round 908 named

ADR 0784 built a door for §7.3.7's dictionary that stops part-way — `Document::get` still answers
§7.3.10's null, `Document::damaged_dictionary` answers the entries that were whole to a caller that
asks **by name** — and gave it one consumer, `Pages`' recovery. Round 908 stopped the parser walking
out of an object (ADR 0858) and left forty Type 3 glyph procedures behind that door, in writing.

**The question is not whether a prefix may be taken.** ADR 0784 settled that: it may, it is a
*choice* rather than a derivation, and §7.3.7's "unordered even though an arbitrary order may be
imposed upon them when written in a file" is why. The question is **what the consumer's own clause
does with the entries that are missing**, and §9.6.4's step b) answers it outright:

> If the name is not present as a key in CharProcs , no glyph shall be painted.

So the residue of a subset is an **omission the standard defines** — not a default this reader
chose, and not a substitute mark. Two more things follow from the same table and are what make the
result legible: Table 110's `/Widths` is in the font dictionary, which is whole, so **no advance
moves**; and §9.6.5.3 with Table 110's `/Encoding` cell make `/Differences` "the complete character
encoding for this font", also whole, so **what is lost is a list** rather than an estimate.

ADR 0866 states that as a three-part test — the clause states the residue as an omission, the loss
can be named from a whole dictionary, and `Document::get` was asked first and said null — and ADR
0867 applies it to everything else.

## Round 896's counter-example, reckoned with

ADR 0836 refuses a damaged **font program**, and the refusal stands. The test's first condition is
the whole difference: a font program that loads draws through §9.6.5.4, whose routes run out and
whose closing permission lets a processor "supply a mapping of its choosing" — on
`issue13316_reduced.pdf` that tier draws **A C E F** where `pdftoppm` draws five CJK glyphs,
reporting nothing, which is *other marks in the producer's places* (ADR 0106, ADR 0459). A
`/CharProcs` prefix has no such tier and cannot acquire one, because step b) closes the case before
a fallback could apply and §9.6.5.3's NOTE adds that "Type 3 fonts do not support the concept of a
default glyph name". **The counter-example is not overruled; it is the other side of the same
test**, and a round wanting the font program would still owe §9.6.5.4 an answer.

## The witness, and the four references that do not agree

`corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-0.zip-3.pdf` page 1. Object 76 is `/F16`'s
`/CharProcs`; its bytes stop at `/a112 57` under another stream's compressed data, and its prefix
holds **41 entries** — 39 references to glyph descriptions, `/a112` cut to a bare integer (present
as a key, and not a content stream, so it paints nothing anyway), and **one key lexed out of the
binary damage**, `d:\xff\xf1\xc0…` with a string for a value. The font's `/Encoding` states 49
names, so **39 draw and 10 paint nothing**, and the page renders its Finnish problem sheet with
those ten letters as holes: *L en oha joi 4, Viikko 40* where the producer wrote *Luentoharjoitus 4,
Viikko 40*.

| | ink at 72 dpi | what it does with object 76 |
|---|---|---|
| ours, before | 1.70076 | discards it — `NoCharProcs` |
| **ours, after** | **4.63038** | draws the 39 descriptions the file states |
| `pdftoppm -cropbox` | 1.75734 | discards it |
| `mutool draw` | 1.66218 | discards it |
| `pdfref-hayro` | 1.69921 | discards it |
| `ghostscript` | 8.93729 | draws glyphs for names whose descriptions are **not in the file** |

**The four references do not agree with each other, and the spread is 7.3 levels.** The ten missing
descriptions are physically absent — their bytes are object 78's stream data, checked at offset
20 678 — so whatever ghostscript puts in their place is not the producer's. Three readers discard
an object §7.3.7 nowhere tells them to discard and one supplies marks it nowhere tells it to supply;
Annex C's one recovery sentence is informative and is about the cross-reference table. So this is
principle 5's ordinary case rather than a consensus to move toward: **the disagreement is about a
recovery no clause states, and what a clause does state — step b) — is what this reading follows.**

The manufactured key is ADR 0787's caution arriving, and it costs nothing here for a reason worth
keeping: step b) only ever looks up a name step a) produced, and step a) reads the whole font
dictionary's `/Encoding`. A manufactured key would have to collide with an encoded name **and**
carry a value resolving to a stream. That is stated in ADR 0866 rather than left to be discovered.

## The population, over 90 535 documents

`examples/damaged_dictionary_consumers`, new, asks the question nothing had asked: not *which
objects declare themselves*, which is what `Pages`' recovery needs, but **which damaged objects a
reference out of an object that parses names, and under what key** — because a reference is the
file's own statement of what an object is for, made in bytes the damage did not reach.

| | trackers, `openpreserve`, `doc/pdf.js`, `doc/corpora` | `cc-main-2021-31` |
|---|---|---|
| documents read / opened | 24 591 / 24 407 | 65 944 / 65 720 |
| documents holding a damaged dictionary | 287 | 24 |
| damaged dictionaries in all | 885 | 68 |
| documents where a whole object names one | 97 | 2 |

**328 references over 58 distinct keys, and exactly one of the 328 is a Type 3 font's
`/CharProcs`.** So the change reaches **one document**, and that is stated rather than dressed up:
what makes it worth the round is that it is the decision ADR 0784 deferred, that the test is what
the other 327 are now judged by, and that the page goes from unreadable to readable with every mark
on it the producer's own.

The shape is round 908's finding one level up — 885 damaged dictionaries in 24 591 tracker files
against 68 in 65 944 crawled ones — which is `doc/todo/03` §1's argument about *what a file had to
survive to be in the corpus at all*.

**Trap 13 was paid before the number was believed**: the census was run against the defect first,
over `batch5/cairo` alone, and printed the witness.

ADR 0867 is why the door stays shut for the other 327, key family by key family: the page tree
(`/Parent`, `/Pages`, `/Kids`, `/Pg` — §7.7.3.4 makes the missing entries *inherited or defaulted*),
font descriptors (Table 122's entries are what a substitute is built from), streams (`/Contents`,
`/Length` — the door parses no stream data at all), encodings (Table 112's base-encoding fallback
draws *another* glyph, which is ADR 0106's archetype), `/CIDSystemInfo` (§9.7.3 names a collection),
functions (Table 38's `/Domain` changes every value rather than removing one), outlines (a lost
`/Next` costs **other objects'** rows), and resource dictionaries (§7.8.3 states no outcome itself —
it is one question per Table 34 entry).

## What was built

`Type3Font::read` asks a new `char_procs`, which takes the prefix only where the entry is a
reference, `Document::get` answered null, and the file states a readable prefix for it.
`CharProcsDamage` counts the encoding's names against the prefix and lists the undescribed ones;
`CharProcsDamage::detail` is the report.

**The report fires once per page and not once per `Tf`**, which took the font cache's two levels
apart. `cached_font`'s first level is the interpretation's own map and is silent; its second is
`FontCache`, which outlives the page — so a font kept from page one and served to page two would
have drawn part of a `/CharProcs` **in silence**, which is trap 5's own failure. Serving from that
level now notes and inserts into the first.

`tests/damaged_char_procs.rs` is five files differing in one thing, on
`damaged_page_dictionaries.rs`'s discipline. The one that carries the decision is
`the_prefix_draws_what_a_whole_dictionary_of_the_same_entries_draws`: if the two differ, something
the damage took is being drawn as *something else*; if they agree, the residue is step b)'s "no
glyph shall be painted" and nothing more. The discriminating fixture is the witness's own shape — a
key present whose value the damage reached — which is why the report counts **descriptions** rather
than keys.

**Four claims in the tree went stale the moment the second consumer existed** and were corrected
rather than left: `Document::damaged_dictionary`'s doc comment, `damaged_page_dictionaries.rs`'s
module comment, §7.3.7's ledger note and `doc/traps/parsers-and-streams.md`, each of which said
`Pages`' recovery was *the one consumer*.

## Gates

The whole of `doc/todo/02` §2 on `main`, on the finished tree — the merge and this round's work
together — each walking line under `tools/bounded.sh` (`--tree 8` for a build, `--data 12 --tree 12`
for a walk), one at a time, after checking `ps` for a neighbour's walk. **Every one of its
twenty-six lines exit 0 on its first run**, with no failure to attribute:
`Summary [69.279s] 3199 tests run: 3199 passed (1 slow), 27 skipped`; doctests 0 failed; corpus
**974 documents in 12.5s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete,
0 slow**; oracle **1945 pages in 46.4s (1841 complete, 104 incomplete)**, 979 agrees, **61
contradicted**, 836 ambiguous, 47 not comparable, with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**; selection census
**1000/1011 words (98.91%) over 453 documents**; accessibility census green over **102 853
elements**, 57 116 a caret can move through; dates **1514 of 1545 (97.99%)**; XMP **318 of 319
read**; JPEG 2000 green; quorra **958 pages compared in 33.1s: 929 agree, 22 differ, 7 refused, 16
not comparable**; fixed documents **71 checked, 0 absent, 71 rows**; the transform gate **186.9
pages/s over a floor of 40**; the six transform walks green over 974 documents each (`foreign` 203
of 974 at stride 8); conformance **875 subclauses, 14 036 citations, 1247 quotations verbatim**.

The sequence ran with the code, the ledger, the pin file and `doc/todo/03` in place and the two
ADRs and this file not yet written, so **the core, the two `fuzz/` lines and the conformance gate
were re-run after the last document landed** — §2's rule that a number belongs to the round that
ran the gate last. Both runs green, and the test count is the same 3199 either way.

**It ran once rather than twice, and that is a deviation worth naming.** `doc/todo/02` §2 asks a
merge for its own sequence; this run is of `main` *with* the merge in it, so the merge's own
obligation — that green in a worktree establishes nothing about `main` — is discharged, and what is
given up is only the ability to attribute a failure between the two halves, of which there was
none.

**Two lines failed while the round was being written and both were this round's own doing**, caught
before the sequence: `cargo test -p conformance` on the two new blockquotes of §9.6.4's step b),
which had `` `CharProcs`, `` where `doc/md/` has `CharProcs ,` — the quotation gate reading a
markup convenience as an invention — and on a `§3` written after "ADR 0784", where a `§` means ISO
32000-2. The report string that quoted the same sentence is now **paraphrase without quotation
marks**, which is `CLAUDE.md`'s own rule: paraphrase is fine, paraphrase claiming to be a quote is
not.

`doc/todo/00` step 7 was re-run over the oracle's own artefacts (839 ambiguous pages, 776 with our
raster and a live reference). The head is the standing set — `issue12418_reduced.pdf` −19.447,
`issue4722.pdf` −13.810, `issue15977_reduced.pdf` −12.927, every one of them a page the corpus gate
already reports — and everything at or past −1 is either `[incomplete]` or one of the four names
`oracle.rs` already carries a group note for. Nothing in either end is this round's, which is what a
population of zero `doc/pdf.js` documents predicts.

§5's binaries are not owed: 912 is not a fifth round, and this round's measurements were taken from
`--profile gates` examples built in this tree rather than from a launch number.

## What is left

- **The other 327 references**, each with its clause written down in ADR 0867 and none of them
  taken. The nearest to arguable is `/Resources` and the named-resource keys, which are one question
  per Table 34 entry rather than one question, and which no round has a witness for.
- **`batch5`'s other seventeen trackers**, `pdfminer.six` (123) and `qpdf` (111) the largest, and
  the two reconstruction cases `cairo-101530-0.pdf` and `cairo-101531-0.pdf`.
- **The four-component `/Luminosity` ranking**, unchanged by this round and still the owner's: 38
  crawl pages moving away from three references on one reading of §11.5.3, now one question for one,
  three and four components rather than three.
- **`round-904` and `round-907` stay open**, and four live branches carry the first as an ancestor.
