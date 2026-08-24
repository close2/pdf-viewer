# ADR 0601 — Two accepted errata that cannot both be applied

Status: accepted, 2026-08-25. Session the seven-hundred-and-twentieth, beside ADR 0600 and found by
the same run of `spec-errata emit`. Amends §12.4.2 in the ledger and one doc comment in
`crates/pdf-model/src/page_label.rs`; adds one section to `doc/errata-read.md`. **No code changes
and no status moves.** Extends ADR 0567's rule that a round runs `emit` before it writes.

## 1. Where this came from

`doc/todo/02` §4 asks a round to run `spec-errata emit` over the document it is about before it
writes. The clause was §12.4.4 and the pages `emit` covers are the outline's, so the run took in
§12.4.2 and §12.4.3 as well. **Against §12.4.4 it found nothing new** — Issues #36 and #75, both
already recorded — and that negative is what the round wanted. Two pages earlier it found a pair
this collection had never named.

## 2. The sentence, and the two amendments

Table 161's `/S` row, as published:

> A Uppercase letters (A to Z for the first 26 pages, AA to ZZ for the next 26, and so on)

- **Issue #432**, `Review`/`Accepted`, 2024-06-17, is a `StrikeOut` over `ZZ` with a `Caret` saying
  `AZ`, and the same pair over `zz` saying `az`. Amended, the sentence reads *AA to **AZ** for the
  next 26* — an **odometer**, where the twenty-eighth page is `AB`.
- **Issue #593**, `Review`/`Accepted`, 2026-05-21, is a `Caret` with no strike, inserting *AAA to
  ZZZ for the next 26, AAAA to ZZZZ for the next 26,* after that same clause, and its lowercase
  twin. That is the **repeat**, where the twenty-eighth page is `BB`.

Both are `Accepted`. They are mutually exclusive: under #432 the enumeration carries, and #593's
addition is then false by three orders of magnitude — an odometer runs `AAA` to `ZZZ` in 17 576
labels, not 26.

**The placement is arithmetic**, and `doc/errata-read.md` has it: #432's strike rectangle matches
`pdftotext -bbox`'s box for the word `ZZ` on physical page 474 to six decimal places, and #593's
caret sits on the same line just past the `26,` that ends the clause #432 rewrites.

## 3. The decision, which is not to decide

**The code is unchanged**, and the reason is neither erratum's authority.

The published sentence carries its own count. *AA to ZZ* is 26 labels if and only if the letter
repeats; read as an odometer it is 676, and the sentence would contradict the number in its own
clause. `page_label::letters` has produced `A…Z`, `AA…ZZ`, `AAA…` since it was written, on exactly
that arithmetic, with `letters_repeat_rather_than_carrying` pinning `BB` at 28 against base 26's
`AB`. #593 states that reading outright, which is a stronger form of the same answer — a clause that
says a thing beats one that implies it (`CLAUDE.md` principle 5, and ADR 0502's miter limit is the
standing instance). #432 denies it.

**What is written down is the disagreement.** A future edition will apply one of the two, and if it
applies #432 this reader is wrong about every alphabetic page label past the twenty-sixth. That is a
claim about the specification with a known way to decay, which is the thing `CLAUDE.md` principle 5
asks to be recorded *as* a claim rather than settled by silence. §12.4.2's ledger row carries it,
`letters`'s doc comment carries it, and `doc/errata-read.md` carries the geometry that places both.

## 4. What this says about the instrument

`spec-errata check` could have printed neither. #593 is a `Caret` with **no** `StrikeOut` — the
first of that command's three blindnesses — and #432's strike is one word, under the four-word
floor, which is the second. This is the fourth consecutive round in which `emit` found what `check`
structurally cannot, and the first in which the two errata it found contradict **each other**
rather than the tree.

**A collection of errata is not a corrected standard.** Every round here has treated an accepted
erratum as settling a question. That works while the collection is consistent, and this is the first
place found where it is not — so the rule that comes out of it is: **an erratum is evidence about
the standard, in exactly the way another renderer is evidence about our reading.** Where two
disagree, the published clause and its own arithmetic decide, and the disagreement is recorded.
