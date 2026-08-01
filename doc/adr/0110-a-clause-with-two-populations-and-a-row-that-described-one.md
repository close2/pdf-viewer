# ADR 0110 — A clause with two populations, and a row that described one

Status: accepted, 2026-08-01.

## Context

ADR 0109's lesson was to read a `reported` row's clause for what it *obliges*. Applying it to
the remaining 34 rows one by one, §11.7.4.4's note said this:

> This tree emits a `Fill` and a `Stroke`, so the band they share composites twice. That is the
> same gap §11.6.2 owns and is reported on the same condition … Implementing it is §11.4.6's
> knockout groups, which is where the fix belongs.

Both sentences are false, and each in a different way. §11.4.6's knockout groups were drawn in
the seventy-first session; §11.6.2's fill-and-stroke pair became one of them in the
seventy-second. The row had gone on describing the world of session seventy for forty-nine
sessions — which is session 118's "a reason that expired" wearing the shape session 115 found,
a note understating the code.

That is not the finding. The finding is what the row's *first* sentence hides.

## The finding: the clause names two populations

> These include the B , B\* , b , and b\* operators (see 8.5.3, "Path-painting operators") **and
> the painting of glyphs with text rendering mode 2 or 6** (9.3.6, "Text rendering mode").

The path operators are one population and glyphs shown in mode 2 or 6 are the other. The
seventy-second session implemented the first, under §11.6.2, whose own scope is paths. The
second was **neither implemented nor reported**: `show_text` emitted a `Fill` and a `Stroke` per
glyph and nothing anywhere said so. A `reported` row was covering a silence.

The obvious defence — that §9.3.8's text knockout already wraps a text object in a knockout
group — is refuted by the clause itself, in a note written for exactly this confusion:

> NOTE 1 In the case of showing text with the combined filling and stroking text rendering
> modes, this behaviour is independent of the text knockout parameter in the graphics state
> (see 9.3.8, "Text knockout").

§9.3.8's group is built only where `/TK` is true *and* two glyphs of the object overlap. A
single translucent outlined glyph satisfies neither, and got the double border NOTE 2 exists to
prevent:

> NOTE 2 The purpose of these rules is to avoid having a non-opaque stroke composite with the
> result of the fill in the region of overlap, which would produce a double border effect that
> is usually undesirable.

## The decision

A glyph shown in mode 2 or 6, under a paint that composites, with both parts marking the page,
becomes one `Command::Group` with `knockout` set, alpha 1.0, no mask and the Normal blend
mode — the same construction the `B` operator has had since the seventy-second session, and the
same three conditions, because they come from the same two clauses.

**Where §9.3.8's own group encloses the text object, it holds the glyphs' parts flat.** One
knockout group inside another is not something either backend can state — `knockout_is_drawable`
rejects an element that is itself a group — and it does not have to be: in a knockout group
every element composites with the initial backdrop, so at each point the topmost element wins,
and nesting cannot change which element that is. The whole-object group therefore computes both
clauses at once. That is why the ranges are *recorded* as glyphs are drawn and the group is
chosen at `ET`: which of the two exists is one decision, and taking it in one place is what
keeps them from being built on top of each other.

The report, where a pair cannot be drawn as a knockout, is one per text object rather than one
per glyph. A line of outlined display type would otherwise name the same gap a hundred times.

## Consequences, measured

**No corpus page reaches this**, and that is measured rather than inferred: every page of all
974 documents was interpreted either side of the change and the display lists hold the same 15
fill-and-stroke knockout groups and the same 161 groups in total. No document in the corpus
shows a glyph in mode 2 or 6 under a paint that composites. All four gates are unmoved — 840
agreeing and 65 contradicted, 90 incomplete, 97.9% of `pdftotext`'s words, 1545 dates.

Four tests, of which **three were confirmed to fail when the condition is disabled**. The
fourth is a guard rather than a witness: it asserts that the enclosing §9.3.8 group holds four
flat commands rather than two nested groups, and it passes under the old code too, because what
it protects is a future change rather than this one.

`reported` falls **34 → 33**, `partial` rises 241 → 242; tests 885 → 889.

`show_text` crossed the cognitive-complexity gate on the way, and the split it forced is worth
keeping: one glyph's drawing is now `show_program_glyph`, and Table 104's three operations plus
the two knockout questions are `GlyphPainting::read` — five answers about the *paint*, computed
once per show string, where four of them had been recomputed per glyph.

## What this says about the audit

Session 120 read a `reported` row's clause for its modal verb. This one read it for its
**scope**, and the two findings are the same shape: a row is a sentence about a clause, and a
clause can say more than one thing in one sentence. **When a row names the operators a rule
applies to, check that it named all of them** — a list with an "and" in it is a list a summary
will shorten.
