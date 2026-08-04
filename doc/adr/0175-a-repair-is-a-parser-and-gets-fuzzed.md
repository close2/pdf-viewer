# ADR 0175 — A repair is a parser over untrusted bytes, and gets fuzzed like one

Status: accepted, two-hundred-and-forty-first session.

## Context

`CLAUDE.md` principle 3: "Fuzzing from the first parser commit. Every crasher found becomes a
permanent regression test." Five targets covered the lexer, the `CMap` parser, §7.6's crypto,
§12.7.4.3's variable text and §12.7.8's FDF.

The two glyph-table repairs — ADR 0170's `repaired_loca_order` and its predecessor
`repaired_loca_format` — had none, and the two-hundred-and-thirty-ninth session changed one of
them. They are a parser by every test that matters: they walk a table directory, a `loca` and a
`glyf` taken from a stream a document supplied, and then **rewrite an sfnt** from what they
found. A rewrite driven by untrusted structure is a larger surface than a reader over the same
bytes, not a smaller one, and it had been reviewed and never fuzzed.

## Decision

**A sixth target, `fuzz/fuzz_targets/sfnt.rs`, over `pdf_font::repaired_font_program`** — a new
public function that applies both repairs in the order `LoadedFont::load` applies them, which is
the door the target needed. Three properties:

- the repair **terminates and never panics**, over any bytes;
- a repaired program is **still an sfnt with the same table tags** — the repair rewrites two
  tables and copies the rest, so a lost tag is a font the caller cannot load for a reason the
  repair invented;
- the repair is **idempotent**. Its own output must be a table it considers well formed. A
  rewrite that keeps finding work to do is one that is losing information every pass.

**Seed the corpus with real fonts.** Sixty `/FontFile2` streams out of `doc/pdf.js/test/pdfs/`.
Unseeded, the target ran 50 000 inputs in under a second and tested nothing: random bytes do not
form a table directory, so every run left on the first `?`. Seeded, it produced **two crashers
inside a minute**, and both were the third and second properties rather than the first — a font
that panics was never the likely defect here.

## What it found

**A table beginning inside the table directory.** The directory's `head` entry pointed at the
directory itself, so `repaired_loca_format`'s two-byte write to `indexToLocFormat` landed on
another entry's *tag* — the repair damaged the directory it was reading. Refused now: no table
may begin before `12 + 16 × numTables`.

**A tag naming two tables.** `sfnt_tables` builds a map and so keeps the *last* entry with a
given tag; `rewritten_sfnt` finds the *first* by scanning. With a duplicate they disagree, so the
repair patched one entry and read the other — and `repaired_font_program` found work to do on its
own output for ever. Refused now: one tag, one table.

Both refusals are the same answer the tree already gives elsewhere — a font that contradicts
itself structurally is not one this can reason about, and `skrifa`'s own answer for it stands.
Neither can reach a well-formed font, and no corpus document changes: corpus, oracle, text
extraction and the cross-backend gate are all unmoved.

The crashers are 30 KB of mutated font and are **not** checked in; what is checked in is
`a_directory_that_overlaps_or_repeats_itself_is_refused`, which builds the smallest font that
reaches each and asserts the refusal — with a third assertion that the *unmodified* fixture is
still repairable, without which the test would pass for want of a repair to make.

## Consequences

Clean at **1 000 000 runs** after the two fixes.

**The lesson is about seeding rather than about fonts.** An unseeded target over a format with a
magic number, a table count and a directory is a target that measures how quickly a `?` returns.
The five existing targets take formats where random bytes are plausible input — a content stream,
a `CMap`, a date — and needed no corpus to be useful; this one needed sixty files. **Before
trusting a clean fuzz run, ask what fraction of it got past the first branch.**
