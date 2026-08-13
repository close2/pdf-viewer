# ADR 0323 — Three verdicts for the interactive surface, each its own shape

Status: accepted, 2026-08-14. The design `doc/todo/05` owed; no instrument is built here.

## Context

The corpus and the oracle judge a raster, and the work has moved off the raster: a drag that
selects text, a field filled and saved, a tree a screen reader walks. Each is a capability
`doc/HANDOVER.md` claims and no independent judge has looked at, at corpus scale — and the
defects such a judge would catch are exactly the kind that lived for seventy-five sessions in
`user_space_at` because no gate clicks (ADR 0118).

`doc/todo/05` names three candidate instruments and the questions the design must settle: the
verdict's shape per candidate, the denominator per candidate, what each reference is being
asked (trap 3), and where each runs. The founding principle is ADR 0005's — two implementations
sharing no code agreeing about an answer is evidence — and the founding obligation is the one
`Tolerance` met: **a bound is measured reference-against-reference before it judges us**, never
invented.

## The evidence, measured this round

The tolerance question for selection geometry was measured the way the raster bounds were:
page 1 of every pdf.js corpus document through two positional extractors sharing no code —
`pdftotext -bbox` (poppler) and `mutool draw -F stext` (mupdf) — with words matched wherever
the same text occurs exactly once in both extractions. Throwaway scripts, not committed; the
invocations are recorded here so the instrument round can reproduce them.

```sh
pdftotext -bbox -cropbox -f 1 -l 1 -q <doc> out.html   # -cropbox: see finding 1
mutool draw -q -F stext -o out.xml <doc> 1
```

### Finding 1: trap 3 reprised in the text domain, three mechanisms deep

**The first run manufactured a false spread, for exactly the reason trap 3 records against
`pdftoppm`.** Invoked naïvely, 41 documents disagreed about the page's very size, with whole
pages of words displaced by hundreds of points:

- **`pdftotext -bbox` works in the MediaBox; `mutool` in the CropBox.** `pdftotext` has a
  `-cropbox` flag and the default is the other one. Passing it took the page-size mismatches
  41 → 8. (`annotation_hidden_noview.pdf`: MediaBox 612×792, CropBox 341×130 — poppler
  reported the former, mupdf the latter.)
- **mupdf scales coordinates by Table 31's `/UserUnit`; poppler does not.**
  `bug1947248_text.pdf` states `/UserUnit 3` and mupdf reports a 1836×2376 page over poppler's
  612×792.
- **mupdf applies the page's `/Rotate` to its text space; `pdftotext -bbox` does not.**
  `hello_world_rotated.pdf` (`/Rotate 90`): poppler 612×792, mupdf 792×612 — every box in one
  frame is the other's rotated.

Each is a different answer to "which space are these coordinates in", and none is a
disagreement about where a word is. **The instrument must therefore audit the frame before
comparing a box**: page size equal within the harness's own one-pixel-per-axis rule, `/UserUnit`
and `/Rotate` normalised explicitly, and a document failing the audit refused *by name* rather
than judged.

### Finding 2: horizontal geometry is comparable and essentially exact

Over the clean population — 301 documents and 13 130 matched word pairs whose page boxes agree
and whose frames carry no gross shift — the two references' **horizontal** word-box edges
agree to:

| absolute Δ in points | median | p90 | p99 | max |
|---|---|---|---|---|
| horizontal edges (x0, x1) | 0.0000 | 0.0002 | 0.14 | 2.52 |

Only 4 of 311 documents show a median horizontal shift over 1 pt. Two independent
implementations of §9.4.4's glyph-positioning arithmetic land the pen within a ten-thousandth
of a point of each other: **the horizontal extent of a word is a specification quantity, and a
tight bound is honest.**

### Finding 3: the vertical extent of a word box is each library's convention

The same pairs, vertically:

| absolute Δ in points | median | p90 | p99 | max |
|---|---|---|---|---|
| vertical edges (y0, y1) | 1.08 | 2.79 | 7.06 | 97.5 |
| vertical **centre** | 0.59 | 1.23 | 2.79 | 6.39 |

The poppler/mupdf word-box **height ratio** is median 1.29, p90 1.78 — on `alphatrans.pdf`'s
12 pt Helvetica, poppler's box is 11.10 pt tall and mupdf's 8.75 pt, same glyphs, same baseline.
100 of 311 documents carry a median vertical shift over 1 pt that scales with the font size.
The standard states no word box at all; each extractor derives one from ascent and descent
conventions of its own choosing, and ours (Table 122's `/Ascent`/`/Descent`, ADR 0118) is a
third. **Vertical edges are convention against convention and are excluded from any verdict.**
The vertical *centre* is comparable: relative to the word's own height it agrees to median
0.088, p90 0.152, p99 0.455 — so "centres within half the word's height" admits the references'
own spread the way 0.90 admits the text pages' structural noise.

### Finding 4: reading order is a question the references answer differently

Over documents with at least five matched words, the two references emit the matched words in
the **same order on only 143 of 222**. That is not a defect population: `pdftotext` emits its
own layout order and `mutool stext` the content stream's, so the disagreement measures the
question being asked twice, differently — trap 3 again, one level up. **Order therefore cannot
be part of the geometry verdict.** Order already has its instrument: the frozen PDFBox gate
compares both of PDFBox's orders (ADR 0259), and §14.8.2.5's logical order is the document's
own statement, not a reference's opinion.

### Finding 5 (denominators, from the same pass)

Word segmentation is also the extractor's own: the matched-unique fraction is median 1.00 but
p10 0.29, so the instrument compares only words both references emit — the same decision the
text gates made in stripping whitespace — and prints the matched fraction per document.
`pdfinfo` calls 78 of the 974 tagged (its predicate is `/MarkInfo`'s `Marked`), where this
tree's census counts 89 stating a `/StructTreeRoot` — two predicates, both worth printing,
their difference a free cross-check list.

## Decision

Three instruments, **three different verdict shapes** — bounded, exact, ratchet — because the
capabilities have three different kinds of comparable answer. Forcing one shape onto all three
is how a corpus becomes a specification.

### Instrument 1 — selection and extraction geometry: a **bounded** verdict, split by axis

**What it judges.** The word boxes this tree's text layer produces (`Interpretation::text_layer`,
one `Placed` quad per code, page space) against `pdftotext -bbox -cropbox` word boxes, matched
by unique text, page 1 per document — the same population measured above.

**The verdict, from the measurement.** Per matched word: horizontal edges within a bound set
just above the references' own p99 (0.14 pt measured; **0.5 pt** admits it with the same
headroom `Tolerance`'s bounds carry), and vertical **centres** within **half the word's
height** (references' own relative p99: 0.455). Vertical edges deliberately unjudged, on
Finding 3. Per document: the fraction of matched words in bound, floored and ratcheted the way
the text gates' 0.90 word floor is, with failing documents named.

**Denominator.** Documents that pass the frame audit and match at least one unique word. Every
exclusion printed by reason — frame mismatch, no words, no unique matches — because a document
the instrument cannot judge is a document off the judged set, and that arithmetic (trap 11)
applies to this gate exactly as to the oracle.

**What the reference is asked** (trap 3, explicitly): *"where is this word, in CropBox points,
unrotated, un-UserUnit-scaled"* — and nothing else. Not order (Finding 4), not segmentation
(Finding 5), not what a drag selects.

**The drag is composed, and the composition is ours — so it is judged end-to-end with the
reference supplying only what it can answer.** The second half drives the real boundary in
`viewer-core`'s headless harness: take a reference's word box, map it to device pixels
(`Command::Resize` to the page's point size at scale 1, so the mapping is the y-flip alone —
trap 12a's exact hazard, which is the point), send `Command::Pointer` press → drag → release
across the box, and `Query::Selection` must answer text containing that word. The verdict is
**exact** given the box. This is the check that catches `user_space_at`'s mirror on its first
run, because the box comes from outside the code under test — the rule trap 12a already
states: when a test needs a point, take it from the document, not from the code.

**Two properties need no reference at all** and belong beside it: `Query::Offset` of the point
`Query::Caret` answers for an offset must return that offset (the pair is documented as
inverse), and `Selection::All`'s text agrees with `Interpretation::text` — the same
byte-for-byte discipline `pdf-retrieve`'s own test holds its default answer to (ADR 0257),
asserted here for the selection path.

**Surfaces exercised**: `Command::Open/Resize/GoTo/Pointer/Select`, `Query::Selection`,
`Query::LogicalSelection` (where the document states an order — judged against the document,
not a reference), `Query::Caret`/`Offset`/`FieldSelection` (self-inverse only: no reference
emits §12.7.4.3's field layout, and none is invented for it).

### Instrument 2 — the save path, round-tripped: an **exact** verdict

**What it judges.** §7.5.6's incremental update at corpus scale: one synthetic edit per
document, saved, and three assertions with no tolerance anywhere:

1. **The prefix property.** The producer's bytes are byte-for-byte a prefix of the saved file —
   a property of the construction, asserted directly.
2. **Our own readback.** This tree re-opens its save and reads the edit back — necessary, and
   insufficient alone: a measurement taken with the instrument under test (trap 8).
3. **The references see it too.** poppler and mupdf open the saved file and report the edit —
   the annotation present, the field holding its value — which is the half that catches a
   malformed update this tree happens to be able to re-read.

**The strongest independent half reuses the oracle whole**: render the *saved* document through
the existing verdict machinery, so three references draw the appearance stream this tree wrote
and the consensus judges it. Nothing new is designed for that — the cache key already admits
any bytes (invocation + renderer version + SHA-256, trap 10a). Two prices, named now:
reference renders of saved files fill a cache the corpus never warms, so the oracle half runs
over a bounded sample (every document with a field, plus a fixed sample of the rest) rather
than all 974; and §7.6.3.2's fresh per-string initialisation vector makes an encrypted
document's save non-deterministic by construction (ADR 0129), so encrypted documents are
excluded from the *cached-render* half only — assertions 1–3 still bind them.

**The edit.** `Edit::FreeText` at a fixed rectangle for every document — the one edit that
needs no selection and no field, so the denominator is widest — and `Edit::SetField` besides,
wherever `Query::Fields` answers with a text field, because §12.7.4.3's appearance construction
and §12.7.5.3's truncation are the hard half. (`pdfinfo` counts 155 of the 974 carrying an
AcroForm.)

**Denominator.** Every corpus document that opens — without a password or with the corpus's
known ones — and accepts the edit. A refusal is counted, printed by reason, and **the refusal
count is itself ratcheted**: under the default `Restrict(On)`, a document whose Table 22 flags
forbid modification is *refused by policy*, which is the policy working and must stay legible
as itself rather than shrinking the judged set in silence (trap 11).

**What each reference is asked**: *"open this file and say what is in it"* — not to render a
verdict on our geometry (that is the oracle half's job, with its measured bounds). Table 231
bit 14's password rule is already the precedent for what this instrument exists to catch: the
save path wrote a person's password into the file for as long as nobody independent read the
file back (ADR 0247).

**Surfaces exercised**: `Command::Edit` (all four), `Undo`/`Redo` (an undone edit must not be
in the save), `Command::Save`, `Event::Saved`, `Event::Dirty`/`Query::Dirty`,
`Command::Restrict` (the refusal population), `Query::Fields`/`FieldAt` (finding the field).

### Instrument 3 — the accessibility tree: a **ratchet**, and honestly one

**No reference implementation puts a comparable tree on AT-SPI, so there is no oracle here and
this design does not pretend one.** A count that cannot fall is weaker than a judge that
disagrees; stating that plainly is the design.

**The counts**, chosen so that each names a defect class rather than decorating a total:

1. **Pages that answer at all.** `doc/todo/31` records that `walk`'s 8192-element bound makes
   every page but the first of a large tagged document answer *empty* — so the first count is
   documents whose every page with structure answers non-empty, and its first baseline makes
   that known defect corpus-visible before anything else is counted.
2. Documents with structure, by stated predicate — `/StructTreeRoot` present — with poppler's
   `/MarkInfo` count printed beside it (78 against our census's 89; the difference is a list
   worth reading once, and the two predicates are both facts).
3. Elements reached, `/Alt` carried, headers resolved (the censuses `doc/todo/31` already
   names, promoted from examples to printed counts).
4. Untagged documents answering the honest single node, which is a decision (ADR 0214) and
   must not decay into an invented reading order without the count moving.

**Denominator.** All 974, with the tagged subpopulation stated by predicate. Ratcheted in both
directions the way the oracle's lists are: a fall fails, a rise is examined as a new report.

**Surfaces exercised**: `Query::AccessibilityTree`, `Query::PageLabel`,
`Answer::Accessibility` — the whole of what a screen reader reaches, minus the bus itself
(`viewer-accessibility`'s AT-SPI walk stays the by-hand verification it is; `Xvfb` and a bus
are not build dependencies, and a silently-skipping gate is worse than none).

## Where each runs

- **Instrument 1's geometry half**: a corpus-scale `--ignored` test in `pdf-model`'s
  `text_extraction` binary, beside the two gates already there — deliberately, because §2's
  line runs every ignored test in that binary, so a gate lands in the sequence with no new
  line, and anything in that binary that is *not* a gate must decline by itself (ADR 0282's
  rule). Reference output cached under `pdfref`'s cache rules (the invocation is the key, so
  `-cropbox` is in the key — trap 10a).
- **Instrument 1's drag half and the self-inverse properties**: `viewer-core`'s headless
  tests, ordinary and un-ignored, over a fixture subset small enough to run every round.
- **Instrument 2**: a corpus-scale `--ignored` test of its own in `pdf-model` (the sweep is
  `ViewState` against every document; the *boundary* path — `Command::Edit` to `Event::Saved`
  — is asserted per edit kind in the headless harness, once, not 974 times); its oracle half
  joins the oracle binary as a separate ignored test over the bounded sample.
- **Instrument 3**: a `tools/state.sh` section from the day it exists, printed not stored.
- Each instrument's numbers enter `doc/todo/02` §2 only after they have held across rounds —
  `doc/todo/05`'s own rule, kept.

## Build order, and what each round owes

The order of how much already exists, which `doc/todo/05` stated and this design confirms:

1. **Selection geometry.** Build the frame audit first (Finding 1's three mechanisms are the
   audit's checklist), then the matcher, then the two bounds as measured here — re-derived by
   the instrument itself over its real population before they gate, the way
   `the_fixed_bounds_against_the_references_own_spread` re-derives the raster bounds, so the
   bound's derivation lives next to the bound. Then the drag half in the headless harness.
2. **The save round-trip.** Assertions 1–3 corpus-wide; the refusal census under both
   `Restrict` levels printed once (the population that differs is the policy's own count);
   the oracle-over-saved-files sample last, priced when built.
3. **The accessibility ratchet.** Count 1 first, because it is the one that names a known
   defect; the fix for `walk`'s bound (§14.7.5.4's parent tree, `doc/todo/31`) may land in the
   same round or before it, and the count is honest either way.

## Consequences

- Three shapes, stated per instrument, so no future round has to force a tolerance onto a
  byte-exact property or fake an oracle for a ratchet.
- The reference-vs-reference spread for selection geometry is now measured, not invented:
  horizontal p99 0.14 pt, vertical-centre relative p99 0.455 — the numbers the instrument
  round starts from and re-derives.
- Trap 3 has its text-domain entry: MediaBox/CropBox, `/UserUnit`, `/Rotate` — three ways a
  positional extractor answers a different question than it was thought to be asked, all found
  before any instrument existed to be misled by them.
- Reading order is settled *out* of the geometry verdict on measurement (143 of 222), which
  closes off the most tempting false gate this surface offered.
- `doc/todo/05` becomes three build items with their designs settled here.
