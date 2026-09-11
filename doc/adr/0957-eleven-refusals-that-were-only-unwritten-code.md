# 0957 — Eleven refusals that were only unwritten code, and the fifth kind of answer

Session 957. Status: **accepted**. It **extends ADR 0947**'s three stages with seven rewrites and
one routing rule, **applies ADR 0948**'s `Stated` class to four of them, and **moves eleven rows**
out of `decision.rs`'s `REFUSED_BY_NAME` — which is the ratchet ADR 0955 built running in the
direction it was built for.

## What was refused, and why it was not a refusal

`doc/pdf-a-mitigations.md` section 13.3, written in the session before this one, found a class it
called **owed, not optional**: twenty-two requirements the converter refused although the right
answer *loses nothing*. They were waiting on code rather than on a decision, which is the one thing
a refusal must never mean for long — a user told "not yet" comes back tomorrow, and the sentence is
only honest while somebody is coming.

Eleven of the twenty-two are built here. The other eleven are named in that section and each is
where its own entry left it; two of them wait on something outside this crate (a JP2 box writer,
the font reader's vertical metrics), two on a proof per font and per code, and one on a rewrite
this converter does not yet offer at all.

## The shape they share, and the file that holds it

Every one of the eleven has the same shape, and it is worth naming because it is what made them a
batch rather than eleven separate arguments:

> the standard states exactly what the corrected value is, and the only open question is **which
> objects** it goes into.

So the new code splits along that seam. `crates/pdf-transform/src/archive/sites.rs` answers the
second question — which objects, read off `pdf_archive`'s own findings — and `rewrite.rs` writes
the value. Nothing in either re-reads ISO 19005: which dictionary is a graphics state and which
annotation is exempt is the validator's judgement, and a second reading made in the converter is
exactly the drift `CLAUDE.md` principle 5 is about.

### Four are `Stated` and three are `Mechanical`, and the split is ADR 0948's

- **`Stated`** — a rendering intent restated as `RelativeColorimetric` (§8.6.5.8 says a processor
  does that anyway), a `/BM` array restated as `Normal` (§8.4.1's Table 57 says a reader takes
  that from it), an appearance subdictionary collapsed to the stream its own `/AS` selects
  (§12.5.2's Table 166 says that is the applicable one), and `/CIDToGIDMap` written as `Identity`
  at a part 2 target. Each loses nothing and each all the same changes what the file *asserts*, so
  the sentence saying what travels inside the decision.
- **`Mechanical`** — a page given the `/Resources` §7.7.3.3's inheritance already put in force, an
  `/Order` array completed with groups the file already lists, and a `/CharSet` or `/CIDSet`
  removed. Nothing is asserted that was not asserted before.

### One is the serializer's and needed no rewrite at all

`implementation-limits/indirect-object-count` joins `WRITER_EMITS`. The writer carries only what
the converted document reaches, so a file over ISO 19005-2's limit *through accumulated orphans* —
a twenty-year-old form revised two hundred times — converts with nothing lost and nobody asked to
authorise anything. A file genuinely over the limit is still refused, by the output's own verdict,
which is ADR 0947's third stage doing the job it exists for.

## The fifth kind of answer: `Answer::AsUnderlying`

`signatures/signature-widgets-meet-the-annotation-rules` is not a rule. It is ISO 19005-2 section
6.4.3 asking the annotation flag and appearance rules *again*, of a signature field's widget — and
every one of those rules is already a row of `REMEDIES`. The converter refused it because the
decision was taken per requirement identifier, so a document whose every actual failure had an
answer was told there was none.

The answer is a fifth `Answer` variant naming the rows the compound asks again, and a rule for
combining their decisions: **the most constraining wins**, in the order a conversion is
constrained — a refusal, then an unauthorised loss, then an authorised one, then a statement, then
a rewrite that loses nothing. That is what the conversion is going to do anyway; what changes is
that the row a reader is looking at says it.

Where **none** of the named rules failed, the compound is refused with a sentence of its own, and
the sentence is a real fact rather than a hedge: `pdf_archive` walks the annotation rules over the
pages and this one over the interactive form's field tree, so a widget the form names and no page's
`/Annots` array holds is one this conversion never sees.

## Three things the catalogue was wrong about, found by building it

The corrections are in `doc/pdf-a-mitigations.md` section 13.3.1 as well, because that is where a
reader of the catalogue will look. They are recorded here because two of them would have produced
a *wrong file* rather than a missing one.

1. **The site of a failure is the object a dictionary is written *in*, not the dictionary.** A
   graphics state written directly inside a page's resource dictionary is reported at the page's
   object number. A rewrite that edited the named dictionary alone would have written nothing and
   reported that it had. Both the blend-mode and the rendering-intent rewrites descend the object
   they are given, to the depth `pdf_archive`'s own walk uses.
2. **`/Intent` is two keys with one spelling.** §8.9.5.1's Table 87 gives an image `XObject` a
   rendering intent; §8.11.2.3 gives an optional content group an `/Intent` of `View` or `Design`.
   Taking the catalogue at its word would have turned a layer's intent into
   `RelativeColorimetric`. The rewrite is guarded by `/Subtype /Image`, which is the validator's
   own test, and there is a test for the layer.
3. **`/CharSet` is deprecated too**, not only `/CIDSet` — §9.8.1's Table 122 deprecates both in
   PDF 2.0. The catalogue offered *recompute* and *remove* as equally lossless and left the choice
   open; the base standard closes it, because the entry a conforming file wants is no entry.

## What it cost and what it bought

The census before: 373 `not-built-yet` target-requirement pairs, and a `remedy` column of 27 at
PDF/A-2b. After: 325 and 36. The corpus sweep moved less, and that is the expected shape rather
than a disappointment — `CLAUDE.md`'s two denominators are two questions, and several of these
eleven have no witness in the veraPDF corpus at all. Six more documents convert at PDF/A-2b and two
more at PDF/A-4, in both columns of the sweep; no conforming count fell, and every document that
conformed before still conforms after its conversion.

## What it does not do

- **No shape the standard leaves unanswered is guessed at.** A bare blend-mode name, an appearance
  subdictionary with no `/AS`, a form `XObject` with no resources, a `/CIDToGIDMap` at a part 4
  target: each keeps its refusal, and each refusal's sentence now says which half of its rule is
  answered and which is not. That precision is the load-bearing part — a sentence that said "not
  built" about a requirement half of which *is* built would be the stale-list failure in prose.
- **No configuration entry is created for any of these.** section 13.3's own argument, and it
  holds: offering an operator a `discard` to get past an unwritten afternoon's work trades a
  permanent loss for a missing one. They were owed, and eleven of them are paid.
