# ADR 0249 — The quotations nobody was checking

Status: accepted, 2026-08-09 (session 413).

## Context

`CLAUDE.md` principle 5 states the rule this decision is about, and states it without an exception:

> **Quotation marks mean verbatim.** A load-bearing normative sentence goes in as a rustdoc
> blockquote, exact, under its clause number, so that the conformance checker can verify it against
> `doc/md/`. Anything less than verbatim is prose *without* quotation marks: paraphrase is fine and
> often clearer, paraphrase that claims to be a quote is not.

`tools/conformance` enforces it — `every_quotation_is_the_standards_own_words` reads every rustdoc
blockquote in `crates/` and finds it in the clause cited above it, **567 of them at this commit**,
and the module that does the comparison says in its own doc comment why it was built: "of five
quotations sampled by hand, three were paraphrases wearing quotation marks."

**It has never read `doc/conformance/ledger.toml`.** The ledger's notes quote the standard
constantly — 977 double-quoted spans of four words or more at this commit — and nothing has ever
compared one of them with anything. The four-hundred-and-twelfth session found the first casualty by
hand: a note quoting Table 227 bit 1 in single quotes, with wording the standard does not use, and
recorded the gap as a question for the next round:

> the conformance checker only verifies rustdoc blockquotes, not quoted prose inside ledger notes.
> That is a gap in the gate itself: an eleventh sweep, or a checker change, could find every ledger
> note that quotes the standard without being verified.

This decides which of the two, and it decides it on the numbers rather than on the principle,
because the principle points at the gate and the numbers point away from it.

## The measurement

The sweep was built and run over all 875 rows. `doc/todo/01` has the run; the shape of it is:

| | spans |
|---|---|
| double-quoted spans of ≥ 4 words in a ledger note | **977** |
| found verbatim in some document under `doc/md/` | 560 |
| found in none of them | **417** |

**417 misses, and a gate cannot be built on that number.** Almost all of them are quotations of
something that is not the standard, and the ledger has no syntax that says which is which. A row
quotes:

- **its own retired wording**, which is the whole of `doc/todo/01`'s fourth sweep — "this row said
  *X*" — and *X* is by construction a sentence nobody but this project ever wrote;
- **`CLAUDE.md`**, `QUORRA_FEEDBACK.md`, `RENDER_LIBRARY.md` and the four other standards in
  `doc/md/` that are not ISO 32000-2;
- **a report this program prints**, a test's name, an identifier, a corpus document's own text;
- **another implementation**, quoted as evidence about our reading.

A rustdoc blockquote is unambiguous — `> ` under a clause number means *this is the standard's* —
and a pair of `"` in a TOML string means nothing at all.

## Decision

**It is a sweep, and the checker is unchanged.** `doc/todo/01` gains an eleventh sweep, run under
`doc/todo/02` §4 with the other ten.

**What makes it usable is a discriminator rather than a filter**, and that is this ADR's
contribution to the method: a quotation this project invented shares almost no words with the
standard, and a *misquotation* shares most of them. So the sweep reports only the misses whose
longest matching prefix is at least five words **and** at least half the quotation:

| | spans |
|---|---|
| misses | 417 |
| of those, matching the standard for ≥ 5 words and ≥ half, then diverging | **12** |
| of those twelve, defects | **6** |

Six defects in twelve suspects is a better ratio than the ninth sweep's 18 in 94, and the ninth
sweep is the one this project decided not to gate. The remaining six are all one shape and it is
worth naming, because it is about the *instrument* rather than about the rows: the Markdown
conversion of the PDF breaks a word across a line — `text-tospeech`, `hierarch y`, `T h`,
`implementationdependent` — so a quotation that is exactly right cannot be found. `quote::normalise`
does not repair those either, which is why two blockquotes written in this very round failed the
gate until they were shortened.

## What the first run found

- **Three rows quote Table 112's `/Differences` as "an array of character codes and glyph names".**
  The standard's word is **character** names, and the difference is not cosmetic in a font clause:
  §9.6.5.1's array maps a code to a *name*, and whether that name is a glyph's or a character's is
  the distinction the whole of §9.6.5.4 turns on. §9.6.5, §9.6.5.1 and §12.7.4.3 all carried it.
- **§8.4.4 quotes §10.7.2 as "a PDF processor may ignore this parameter"**, a sentence ISO 32000-2
  does not contain. §10.7.2's own row, one screen away, carries the real one — "PDF processors may
  choose to ignore any flatness tolerance specified within a PDF file." Two rows, one permission,
  and the one that invented a sentence is the one nobody re-read: `doc/todo/01`'s seventh shape.
- **§8.3.2.4 drops a word from the middle of a quotation**: the standard says the pattern matrix
  maps pattern space to "the default **(initial)** coordinate space of the page".
- **§7.9.3 elides without saying so.** The standard's sentence carries a cross-reference in the
  middle; the row's quotation runs the two halves together. An elision is honest and often necessary
  — `quote::occurs_in` implements exactly that — and it has to be marked, or a quotation can join
  two clauses' worth of text into a sentence the standard does not contain.

## And a defect the sweep found on the way, which is about the file rather than about a claim

Seventeen rows carried **72 double-escaped quotation marks** — `\\\"` in the TOML, which
`toml_subset` decodes to a literal backslash followed by a quote, so 36 quotations rendered with
stray backslashes in their prose. Fourteen of the seventeen are the §8.4 family, written in one
sitting; the block is the ninth sweep's own signature applied to punctuation. `cargo run -p
conformance --bin ledger` round-trips the file unchanged after the repair, which is how the fix was
checked rather than by reading it.

## Consequences

- **The gate's coverage is now stated rather than assumed.** 567 quotations are verified and 977 are
  not, and this file is where a future round finds that number instead of rediscovering it.
- **A gate stays possible and its price is written down.** It needs a syntax in the ledger that
  marks a quotation's source — the thing `> ` does for a doc comment — and migrating 417 spans onto
  it. That is a round of its own and it should not be paid for a defect rate of six.
- **The conversion's broken words are now a known limit of the instrument**, for the checker as much
  as for the sweep: a load-bearing sentence that happens to contain one cannot be quoted whole, and
  the honest response is a shorter quotation rather than a repaired copy of the standard.
