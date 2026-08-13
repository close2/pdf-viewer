# An instrument for the interactive surface

Status: **design owed, nothing built.** This file is the thinking; no gate exists and none is to
be built before the design settles what it measures.
Priority: 05 — standing band, because it is an instrument like 00 and 01 rather than a feature
Corpus: the denominator question is half of the design, below
Clauses: §9.10 (extraction), §12.7 (forms), §7.5.6 (the incremental update a save appends),
§14.7–§14.9 (the tree a screen reader walks), §12.3.2/§12.6 (what a click does)
Code: none yet; the existing halves are named below

## Why

The 455–484 block summary says it in one sentence: **the gates measure a raster and the work has
moved off the raster.** The corpus and the oracle held 1794 verdicts steady through thirty rounds
of change — which is what a regression instrument is for, and is also the proof that they cannot
*rank* what those rounds built. A drag that selects text, a field filled and saved, a click that
follows a destination, a tree a screen reader walks: each is a capability `doc/HANDOVER.md` claims
and no independent judge has ever looked at, at corpus scale. Trap 1 — the metrics lie, look at
the page — has no analogue here yet, and the defects it would catch are exactly the kind that
lived for seventy-five sessions in `user_space_at` because *no gate clicks* (ADR 0118).

The principle to build on is the oracle's own (ADR 0005): **two implementations sharing no code
agreeing about an answer is evidence.** The design question is what the "answer" is for each
capability, because a raster has one obvious comparable and a selection does not.

## Three candidates, in the order of how much of each already exists

### 1. Selection and extraction geometry

The *text* half is the half that exists: `doc/todo/02` §2's `text_extraction` line is two gates,
one of them against Apache PDFBox's frozen `PDFTextStripper` output, and `pdf-retrieve` asserts
its answer is `Interpretation::text` byte for byte (ADR 0257). What has no judge is the
**geometry**: the quads a drag turns into `Query`-answered shapes, and the reading order that
decides what lies between two points. `pdftotext -bbox` emits every word's box from poppler's own
layout, sharing no code with this tree — which is the same shape as the raster oracle, with a
tolerance question of its own (two correct extractors legitimately disagree about a box edge the
way two rasterisers disagree about a glyph edge; the bound has to be measured
reference-against-reference first, the way `Tolerance` was, before it can judge us).

### 2. The save path, round-tripped

The one kind of writing `CLAUDE.md` permits is §7.5.6's incremental update, and it has a property
no other capability has: **the file itself is the answer.** A round-trip gate needs no tolerance
at all —

- the producer's bytes are byte-for-byte a prefix of the saved file (a property of the
  construction, assertable directly);
- this tree re-opens its own save and reads back what was added;
- **poppler and mupdf open the saved file and see it too** — the annotation where it was put, the
  field holding its value — which is the independent half, and the half that would have caught a
  malformed update this tree happens to be able to re-read (trap 8's "a measurement taken with
  the instrument under test").

Denominator: every corpus document the editor can open, edit and save, which is a population
`ViewState::save` already decides one document at a time.

### 3. The accessibility tree

`viewer-accessibility` is walked off a real AT-SPI bus today (ADR 0214), one document at a time by
hand. No reference implementation puts a comparable tree on that bus, so **this one is honestly a
ratchet rather than an oracle**: corpus-scale counts — documents with structure, elements reached,
`/Alt` carried, headers resolved, pages that answer at all (`doc/todo/31` names the empty answer
every page but the first currently gets) — printed by `tools/state.sh`, ratcheted the way every
other count is. Saying plainly that it is a ratchet is the design: a count that cannot fall is
weaker than a judge that disagrees, and pretending otherwise is how a corpus becomes a
specification.

## What the design round must settle before any code

- **The verdict's shape per candidate**: exact (the save prefix), bounded (a box distance whose
  bound is measured reference-against-reference first), or a ratchet (the tree) — and it should
  say so per candidate rather than force one shape onto all three.
- **The denominator per candidate**, stated the way the two-questions table states it: which
  documents are in, and what a refusal costs the judged set (trap 11 — a page that reports is a
  page the oracle stops judging, and the same arithmetic will apply here).
- **Where it runs**: a `tools/state.sh` section per instrument, gated in `doc/todo/02` §2 once
  its numbers hold, never before.
- **What each reference is being asked** (trap 3): `pdftotext -bbox` answers "where is the word",
  not "what would a drag select between these two points" — the second is composed from the
  first, and the composition is ours, which is exactly the kind of gap that manufactured false
  contradictions in the raster oracle's first run.

One round for the design, ending in an ADR; one round per instrument after that, in the order
above, because it is also the order of how much already exists.
