# 0757 — A bound the printed page could not state, and a step that filters the selector

Status: accepted.
Context: ISO 32000-2 §7.4.3's three "shall never occur" conditions, `crates/pdf-syntax/src/filter.rs`,
and the errata selection rule's nineteenth use — the first under `doc/todo/01`'s step 6 (ADRs 0753,
0749, 0746, 0743, and 0343 for the two voices a filter refuses in).

## The defect: an exponent that is missing from every instrument this project has

§7.4.3 closes with three conditions, and this is how ISO 32000-2 prints the first:

> - The value represented by a group of 5 characters is greater than 232 - 1.

The superscript is gone. It is gone from the ISO PDF's text layer, so `pdftotext` prints "232 - 1";
it is gone from `doc/md/`, which is the conversion every quotation gate in this tree verifies
against. **So a quotation of the published sentence would have matched perfectly, and the check that
exists to catch a wrong quotation would have agreed with itself.** `spec-errata check` has the same
blindness for the same reason — it compares this tree's quotations against *struck* passages, and
nothing here quoted this bullet at all.

Errata Collection 3's Issue #98 strikes those characters and writes *2^32 - 1*, with the 32
superscripted. It is the whole of the erratum, and it restores the only thing about the sentence
that made it a requirement.

The gap it exposes is not marginal. Five base-85 digits reach 85⁵ − 1 = 4 437 053 124 where four
bytes hold 4 294 967 295, so **about 3% of the five-character groups the alphabet admits name no
four bytes at all** — and `push_ascii85_group` accumulated a group in a `u32` with
`saturating_mul`, which decodes every one of those as four `0xFF` bytes and reports nothing. The
third condition, "[a] final partial group contains only one character", was answered the same way
from the other side: the character was dropped and the stream called whole. Only the second, a `z`
inside a group, was refused, and by accident of the alphabet test rather than by decision, since `z`
is above `u`.

## What the row said

§7.4.3's ledger row said:

> All three of the clause's "shall never occur" conditions are refused rather than guessed at: a
> character outside the range fails the stream, a `z` inside a group fails it because `z` is outside
> `!`..=`u`, and a final group of one character yields nothing.

Three things are wrong with that sentence and they compound. It **claims three and enumerates two**,
because "a character outside the range" is the sentence *above* the bullet list rather than one of
its members. It **calls silence a refusal**, since "yields nothing" is exactly what an unenforced
condition looks like. And the member it drops is the one member the page could not print. That is
the eighteenth sweep's overstating shape (ADR 0475) with the unread requirement sitting inside the
gap between the count and the enumeration — and no sweep could print it, because the row's own list
is prose and its cardinal agrees with the standard's.

## The decision

Both conditions are enforced, and each route keeps the voice ADR 0343 and ADR 0587 gave it:

- `ascii85`, the buffered route, returns `FilterRefusal::Corrupt` for the whole stream. Every
  consumer whose prefix is not a shorter thing of the same kind takes it — a cross-reference stream,
  a font program, an ICC profile — and a prefix of a table is not a shorter table.
- `filter::Ascii85`, the same clause driven a window at a time, returns
  `Standing::Damaged(Damage::Corrupt)` over the groups it has already handed to a lexer. It is only
  ever run over §7.8.2's "sequence of instructions", where a prefix *is* one.

The accumulator is a `u64` and the test is `u32::try_from`, which is not a style choice: an
accumulator that saturates inside the type it is being tested against cannot answer whether the sum
left the type.

**The padded final group needs no separate bound, and that is arithmetic rather than an assumption.**
The clause's own encoder appends 4 − *n* zero bytes and writes *n* + 1 characters; the decoder pads
the missing digits with 84. For *n* = 1, 2 and 3 the padding adds at most 614 124, 7 224 and 84 to a
value the encoder derived from at most 0xFF000000, 0xFFFF0000 and 0xFFFFFF00 respectively, so a
legitimate partial group cannot be pushed over 2^32 − 1. One test applies to full and partial groups
alike.

## Calibration

Per trap 13, above the commit that makes the change and in both directions for each condition:

- the accumulator put back to a saturating `u32` — the buffered route answers `None` where
  `a_base85_group_above_four_bytes_is_an_impossible_combination` wants `Some(Corrupt)`;
- the bound moved one power down to `i32::MAX` — the same test's **control** fails, and `s8W-!`, the
  base-85 spelling of 4 294 967 295, stops decoding;
- the one-character refusal removed — `a_final_base85_group_of_one_character_is_refused` fails on
  the buffered assertion;
- the one-character refusal widened to every partial final group —
  `a_raw_deflate_stream_behind_an_armour_still_falls_back` and
  `every_pumpable_chain_agrees_with_the_whole_decode` both go red, which is the clause's own encoding
  failing.

Both test operands are derived rather than chosen, which is what makes them a test of the bound
rather than of a number: `s8W-!` is 4 294 967 295 in base 85, and `uuuuu` is 84 × (85⁴ + 85³ + 85² +
85 + 1) = 4 437 053 124.

No gate could have seen this and the corpus cannot rank it: a group above the bound is a malformed
file, and the standard describes valid ones. That is `CLAUDE.md`'s two denominators exactly — the
coverage question answered by reading the clause, with the corpus gates present only to say that
nothing already on this disk trips the new refusal.

## The other half: what step 6's first run says about step 6

`doc/todo/01`'s step 6 was written by ADR 0753 on a measurement — of the then-54 unread issues, seven
land only on `out-of-scope` rows, all seven are clause 13's, and three of those are the whole
population's requirement-level substitutions, which is the tie-break's first preference. **On its
first run the step did what it was written to do, and the demonstration is exact.** The field offers
exactly one requirement-level substitution: Issue #58, turning a `/Asset` cell's *is also referenced*
into *may also be referenced*, on §13.7.2.3.4's `out-of-scope` row. Ranked together it is the head.
Counted apart, the tier is empty and the head falls to the next.

**And the run found what the step still owes.** The head it produced, Issue #679, strikes the type
`number` out of Table 223's argument list — and Table 223 belongs to §12.6.4.18, whose row carries
the same clause-13 exclusion. `emit` files the pair under §12.7.2, because §12.7 opens on that page,
which is ADR 0712's placement rule doing exactly what step 3 already warns it does. Step 6
reads the row `emit` names, so an excluded issue landed in the ranked column and took the head.

The repair is a caution rather than a rebuild, and it is now in the recipe: check the head's own
table or figure against the clause that captions it in `doc/md/` before reading it. One grep. A
filter would be worse — ADR 0750 already established that the placement rule is one clause out often
enough that filtering on it recreates the blindness it is meant to close.

**What this says about the rule's shape is worth more than the instance.** Three ranking units have
now gone flat and the tie-break has chosen four heads running. Step 6 is not a fourth count and does
not try to be: it is a *guard on where the selector may point*, which is the first amendment this
rule has gained that improves the tie-break instead of replacing it. The pattern to expect from here
is filters on the selector.

## Consequences

- §7.4.3's row is `implemented` and now says which conditions are refused, in which voice, and why
  the first of them was unreadable.
- Two impossible base-85 combinations that were decoded silently are refused. A stream carrying one
  was, until now, four `0xFF` bytes per group with nothing said — principle 3's shape and trap 5's.
- The recipe's step 6 carries the filing caveat, and `doc/errata-read.md` carries the six verdicts.
- **A quotation gate cannot see a defect its own conversion shares**, and this is the sharpest
  instance this project has: `doc/md/`, `pdftotext` and the tree would all have agreed on "232 - 1".
  Where a clause states a *magnitude*, the erratum collection is a better instrument than either
  quotation gate, and `spec-errata emit` over the clause is the only thing that would have found it.
