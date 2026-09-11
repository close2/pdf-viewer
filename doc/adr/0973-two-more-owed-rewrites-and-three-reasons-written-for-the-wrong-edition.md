# 0973 — Two more owed rewrites, a clause read one sentence too early, and three reasons written for the wrong edition

Session 966. Status: **accepted**. It **continues ADR 0965** over what
`doc/pdf-a-mitigations.md` §13.3 had left, moves **two** more rows out of `decision.rs`'s
`REFUSED_BY_NAME` — one of them a row §13.3 had already *given up on* — and reports a walk of the
decision table for justifications that cite ISO 32000-2 at a target whose base standard is
ISO 32000-1:2008.

Context: `crates/pdf-transform/src/archive/{decision,prepare,rewrite,sites}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §4.1, §4.2, §13.3 and §13.3.1,
ISO 19005-2 6.2.3, 6.2.4.4, 6.2.6, 6.2.11.4.2 and 6.3.3, ISO 19005-4 6.2.3 and 6.2.4.4,
ISO 32000-1:2008 7.3.8.1, 7.7.3.4, 8.6.5.8, 9.8.1 and Table 164.

## 1. `graphics/one-destination-profile-per-output-intents-array`

ISO 19005-2 6.2.3 and ISO 19005-4 6.2.3 require every entry of an `OutputIntents` array that
states a `DestOutputProfile` to state **the same indirect object**, and both parts explain in the
same breath where several entries come from: a file conforming to ISO 19005 and to PDF/X or PDF/E
at the same time carries one output intent per standard.

That note is also what makes the row buildable, because such a file usually carries the *same*
profile twice. So the answer is a **proof rather than a reading**: both profiles are decoded, and
the entries are pointed at one object only where every one of them holds the same bytes under the
same stream dictionary — `/Length`, `/Filter` and `/DecodeParms` set aside as §7.3.8.2's
description of how the bytes are *carried*, and everything else, §8.6.5.5's `/N` and `/Alternate`
included, required to agree. When that holds, the object that goes carried a copy of the object
that stays, every entry still refers its colours to the profile it already referred them to, and
nothing has been decided. `Rewrite::SharedDestinationProfile` is therefore `Mechanical`.

**The half that is not built is the half the catalogue always called a `discard`.** Two entries
naming profiles that are *not* the same bytes name two destinations; the clause admits one; the
loser's statement of which press or display this file was prepared for is thrown away, and nothing
in the file says which its producer meant. That needs §4.1's `winner` from a configuration, and
until there is one the row is refused with a sentence that names the half that is done.

A third shape is refused as well, and it is worth writing down because it looks like a bug and is
not: an entry whose `DestOutputProfile` is *not* an indirect reference. §7.3.8.1 requires every
stream to be an indirect object — and ISO 32000-1:2008, 7.3.8.1 states the same sentence, so it
binds a part 2 target too — so such a value is not the ICC profile stream 6.2.3 requires, there is
no object for the other entries to share, and what the entry was meant to name is not in the file.

### Why this one reads the array rather than the findings

`sites.rs`'s standing rule is that `pdf_archive`'s findings decide which objects a rewrite touches,
because the validator is the reading of ISO 19005 and a second reading made in the converter is a
second chance to differ. This preparation reads the catalog's `OutputIntents` array instead, and
the distinction is worth keeping: the requirement *is about that array*, which is one key of the
catalog. §13.3.1's warning — the one that stopped the duplicate-profile rows — is about a site that
can only be found by walking the content streams again to see which colour space a page selected.
Reading a key of the catalog is not that. The findings still decide whether the preparation runs at
all, which is the rule that keeps a conforming document unread.

## 2. `graphics/spot-colourants-appear-in-the-colorants-dictionary`, and a clause read one sentence too early

This row was in §13.3's *owed* list, then moved out of it in session 962 with an argument that was
correct as far as it went: ISO 19005 6.2.4.4 requires an entry in the `Colorants` dictionary for
every spot colourant a `DeviceN` uses; the entry is a §8.6.6.4 `Separation`, whose tint transform
takes **one** input where the `DeviceN`'s takes *N*; and §7.10 gives a PDF function no way to call
another, so a one-input transform derived from an N-input one can only be a **sample** of the
producer's. An archive carrying a sample as though it were a definition is a loss wearing a
mechanical's clothes, and refusing it was right.

**What both readings missed is the next paragraph of the same subclause.** 6.2.4.4 goes on to
require that every `Separation` array in a single file naming the same colourant — expressly
including the arrays written inside a `Colorants` dictionary — state the same alternate space and the
same tint transform, with the comparison made on the PDF objects rather than on what using them
computes. So for any file that states a `Separation` for the colourant anywhere, the entry that
requirement admits is **that array and no other**. There is nothing to derive, nothing to sample
and nothing to choose: the producer's own definition of the ink is what the clause demands and what
gets written.

`Rewrite::SpotColorantEntry` does exactly that and nothing else. A colourant the file never defines
on its own keeps the refusal, with the sampling sentence; a colourant the file defines *twice,
differently*, keeps a refusal of its own, because the file is already failing
`graphics/separations-of-one-name-agree` and there is no producer's definition to copy.

Two things the building needed that the catalogue did not mention:

- **The site is the object the colour space is written in, which may be an array object.**
  §13.3.1's first lesson, met again: `pdf_archive` reports a `DeviceN` at the object it is written
  in, and a colour space is as often its own object as it is a value inside a page's resource
  dictionary. `Rewriter::rewrite` had arms for a dictionary and a stream and carried every other
  object unchanged, so this is the first rewrite in the verb that reaches an object that is *an
  array*. Every other one writes a dictionary entry, and that is now said where the arm is.
- **The placement is proved before it is promised.** The preparation runs the same placement the
  rewriter will run, on a copy, and refuses the document unless every colourant it named was
  placed. A space whose `/Attributes` or `/Colorants` is an indirect object of its own is not the
  object any finding named, so it is left alone — and without the proof the conversion would have
  written a file that still failed the clause it claimed to answer.

## 3. The base-standard audit: three reasons written for the wrong edition

**PDF/A-2 delegates to ISO 32000-1:2008 by name and PDF/A-4 to ISO 32000-2.** Session 961 found two
*validator* rows enforcing an ISO 32000-2 sentence against PDF/A-2 documents; this session walked
the converter's decision table for the same class of error — a row justified by a `§` clause while
binding a part 2 target. `Clauses::both` and `Clauses::only_two` are the rows that bind one; the
justifications are the `*_REINTERPRETS` constants, the `REMEDIES` comments and each `Rewrite`'s own
doc.

**No row writes a wrong file.** That is the audit's headline and it is worth stating before the
three findings, because the finding is about the *reason*, which is the thing this project keeps.

- **`fonts/charset-lists-every-glyph-in-the-program` and `fonts/cidset-lists-every-cid-in-the-program`
  bind a part 2 target and nothing else**, and `Rewrite::DescriptorSetRemoved`'s whole argument for
  *removing* the entry rather than recomputing it was §9.8.1's Table 122 deprecating both keys in
  PDF 2.0. ISO 32000-1:2008 deprecates neither. Removal is still right and for a reason that
  edition does state: its Table 122 and its Table 124 make both entries optional and give each the same
  meaning when absent — a subset is then indicated by the subset tag in `/FontName` and by nothing
  else — so a descriptor stating neither is a conforming descriptor of the edition PDF/A-2 adheres
  to. The doc now leads with that and keeps the deprecation as the reason removal stays right at a
  later target. This is the sharpest of the three: the *only* stated ground for the route taken was
  a sentence the only target it binds never sees.
- **`annotations/appearance-dictionary-present` binds a part 2 target and nothing else**, and
  `APPEARANCE_REINTERPRETS` told the reader that "ISO 32000-2 Table 166 requires a writer to include
  an appearance dictionary, so nothing here is invented". ISO 32000-1:2008's Table 164 makes `/AP`
  plainly optional; the writer obligation is ISO 32000-2's addition. The requirement that asks for
  the dictionary at a part 2 target is ISO 19005-2 6.3.3's own — and the *contents* of the
  appearance, which is where "nothing is invented" actually earns its keep, come from each
  subtype's clause, which both editions state alike. The sentence now says both, which it has to,
  because the same constant also serves
  `annotations/appearance-dictionary-present-from-base-standard`, and *that* row is part 4's and is
  Table 166 exactly.
- **`graphics/rendering-intent-entries-name-one-of-four` binds a part 2 target and nothing else**,
  and `RENDERING_INTENT_REINTERPRETS` rested on §8.6.5.8. Here the base standard agrees word for
  word — ISO 32000-1:2008, 8.6.5.8 has a conforming reader use `RelativeColorimetric` for a name it
  does not recognise, where ISO 32000-2 says the same of a PDF processor — so only the citation was
  for the wrong edition. The sentence now names the edition the target actually adheres to.

**One row that looked like a fourth and is not.** `forms/no-needs-rendering` binds both parts and
its justification leans on §7.7.2's Table 29 *deprecating* `/NeedsRendering`, which ISO 32000-1:2008
does not. The deprecation is decoration there: the load-bearing half is the entry's default of
`false`, which ISO 32000-1's Table 28 states in the same words, and the other half is that
`forms/no-xfa-key` binds both parts too, so no file this rewrite reaches has an XFA form for any
reader to regenerate. Checked and left alone — which is what makes the audit worth writing down
rather than assuming.

**The model the other rows should follow already exists in the tree**:
`sites::cid_to_gid_maps` refuses at a part 4 target *in code*, because ISO 32000-1's table gives
`/CIDToGIDMap` a default of `Identity` and ISO 32000-2's does not — the same question asked in the
other direction, and answered where it changes behaviour rather than only where it changes prose.

### One finding the audit turned up that is not about editions

`§7.7.3.3` appears eight times across `sites.rs` and `rewrite.rs` as the clause that makes
`/Resources` inheritable. In **both** editions 7.7.3.3 is *Page objects* and the inheritance rule is
7.7.3.4. The citations are corrected. Nothing behaved differently; a reader checking the claim would
have been sent to the wrong subclause eight times.

### And one open question, recorded rather than answered

`fonts/symbolic-truetype-states-no-encoding` and
`fonts/non-symbolic-truetype-uses-a-standard-encoding` bind **both** parts, and session 962 made
them mechanical on a proof: the font is loaded as the file states it and again as it would be
written, and every code the content streams showed has to reach the same glyph both ways. The proof
is empirical and therefore edition-independent *in form* — but it is made through `pdf_font`, which
implements one edition's reading of the TrueType encoding algorithm, and the two editions renumbered
and rewrote it (ISO 32000-1's 9.6.6.4 against ISO 32000-2's 9.6.5.4). So the proof says "no code
moves **under this program's reading**", and for a part 2 target the reading that binds is
ISO 32000-1's. Whether the two readings can disagree for a font this rewrite would touch is a
question for whoever owns `pdf_font`, not for this round; it is written here so that it is a
question rather than an assumption.

## Consequences

- Two rows leave `REFUSED_BY_NAME`; the census still reports **no** `Unconsidered` requirement at
  any of the six targets, which is the ratchet those rows had to stay inside.
- §13.3's tally is now seventeen of twenty-two waiting on code, four waiting on a decision, one
  still waiting on code — and the spot-colourant row is the standing example of a claim in *the
  other* direction decaying: "this is a choice, not unwritten work" is a claim about the standard
  too, and it lasted four rounds.
- The output-intent group's single refusal sentence is three sentences, one per row, because the
  shared one hid which of the three was buildable.

## What a later round should take

- `fonts/vertical-metrics-agree-with-the-program`, the last of §13.3's twenty-two, whose rewrite is
  `pdf_font::restate`'s and not this crate's.
- The §7.10.2 sampled-slice route for a spot colourant the file defines nowhere but inside the
  `DeviceN` that uses it — exact where the other axes' zero encodes to an integer sample index, and
  an approximation otherwise, which is the line to hold.
- The same base-standard walk over `pdf-archive`'s own table, which session 961 began and did not
  finish.
