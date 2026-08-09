# ADR 0254 — Three populations of quotation, and only one of them had a gate

Status: accepted, 2026-08-09 (session 418).

## Context

ADR 0252 found that the sponsored copies of ISO 32000-2 record Errata Collection 3 as review markup
applied to nothing, so `doc/md/` — the conversion the conformance gate verifies **6087 citations and
575 quotations** against — presents retired sentences as the standard's current words. ADR 0253 read
the 79 struck passages the first instrument named, found two clauses this tree implemented
differently, and corrected the instrument: comparing with the spaces taken out took the count from
79 to **151**. It left two things: 55 newly visible passages nobody had read, and the observation
that `ledger.toml`'s **977 quoted spans** are checked by nothing at all — two of them had been found
stale by hand, two out of two attempts.

Both are done here. The reading is in `doc/errata-read.md`; this file is the decision the sweep
needed and what it cost to make.

## The reading: 54 passages, three findings

`check` names 151 lines and **120 distinct passages**. Sixty-six carried a verdict already; the
other **54** are read now, one line apiece. Two of them are worth naming here because they are about
the *instrument* rather than about a clause:

**A struck passage that `doc/md/` still carries is not always a retired one.** §7.5.4's Issue #113
strikes "[e]ach cross-reference subsection shall contain entries for a contiguous range of object
numbers", and the conversion carries that sentence **twice on one line** — because the standard
prints it twice and the erratum is a de-duplication. `xref.rs::realigned` rests on the rule and
keeps it. Nothing in an annotation says which of two identical copies it covers, so this class of
false positive cannot be removed from `still_in_conversion` and is documented on it instead.

**The check's two questions have different populations, and neither contains the other.** Three of
the errata acted on this round — §7.11.4.1's #481, §12.7.5.2.2's #386, §14.8.2.2.2's #484 — are
*not* among the 151, because `doc/md/` spells those passages differently enough that a containment
test misses them. They were found by the other half of the check, "a quotation in this tree overlaps
struck text". A round that reads only the first list reads the smaller of two lists.

The finding with code behind it is **§7.8.3 and §9.6.4, Issue #128**. The 2020 clause named two
places for a Type 3 glyph description's resources — the font dictionary, then the page — and
`Type3Font::resources` implemented exactly that. EC3 replaces §9.6.4's step d) with a pointer to
§7.8.3 and gives §7.8.3 a four-step search whose **first** step is "the stream dictionary of that
glyph description content stream". That step was missing, so a glyph stream stating its own
`/Resources` was read against somebody else's dictionary, and the failure is silent: a resource name
that resolves to nothing draws nothing and reports nothing. It is implemented, with a test built so
that only the new step can answer it.

## Decision — the sweep goes in the sidecar, in three shapes, and `conformance` still knows nothing

**`spec-errata` learns to read the ledger and to read prose; `conformance` learns nothing.** ADR
0252's argument is unchanged and is the reason this is not a gate: if the checker read a conversion
this project generated, a defect in our extractor would become a defect in the standard we check
ourselves against. The dependency runs one way, `spec-errata` → `conformance`, and this round used
`conformance::ledger::Ledger` to parse the rows rather than writing a second TOML reader.

**The ledger needs no new syntax, because the erratum supplies the other side of the comparison.**
ADR 0249 declined a gate over the 977 spans and priced the alternative: a syntax marking which
quotations are the standard's, and 417 spans migrated onto it. That price was for the question *is
this span in the standard at all*, where 417 misses are almost all quotations of something else — a
row's own retired wording, `CLAUDE.md`, a report this program prints. **This question is different
and needs none of it**: a span that matches a sentence an erratum struck out is the standard's by
construction, whatever else the ledger quotes. So the sweep is cheap, and its precision comes from
the match rather than from a filter.

**Containment runs both ways now, and that is what made the ledger legible.** A rustdoc blockquote
is usually a whole sentence with the struck passage inside it, which is the case ADR 0252's
`quotation ⊇ struck` was built for. A ledger note quotes the other way — five words lifted out of a
paragraph — and so does half the prose. The shorter side still has to be `MIN_WORDS` long, which is
what keeps a four-word coincidence from being reported as a quotation of a paragraph.

**And a third population arrived on its own: quotation marks inside ordinary rustdoc prose.**
`CLAUDE.md` binds those exactly as hard as a blockquote — "[q]uotation marks mean verbatim" — and
the gate's scanner walks straight past them, because `> ` is what it looks for. Nobody had counted
them. They are the largest of the three by defect count:

| population | in-clause landings | elsewhere | stale quotations found |
|---|---|---|---|
| rustdoc blockquotes — the one population with a gate | 8 | 10 | **1** |
| rustdoc prose | 11 | 28 | **6** |
| `ledger.toml` notes | 11 | 10 | **4** |

**Four of the eleven are in the "elsewhere" column**, which `check` prints under "a repeated phrase
rather than a finding". `Landing::in_clause` compares the clause a quotation cites against the
clause the *outline* puts the erratum's page in, and a heading that straddles a page break files a
real landing as a coincidence. ADR 0253 found three that way; this round found four more. The bucket
is a sort order and not a verdict, and a round that reads only the first list is choosing to miss
about a third of what the sweep found.

**Prose has to be read a block at a time and that is the whole difficulty.** The first version of
`prose_quotations` was line-at-a-time and found nothing that mattered, because a quotation worth
checking is longer than the 96 columns this tree wraps at: its opening `"` and its closing one are
on different lines, and a line-at-a-time reader sees two unmatched marks. It missed
`attachment.rs`'s `/Subtype` quotation, which had already been found by hand — an instrument that
cannot see what a person found by reading is not an instrument. Doc-comment runs are joined before
the marks are counted, with a blockquote line ending the run rather than being skipped, or two
quotations either side of one would be joined into a span the file does not contain.

Single quotes are **not** collected, deliberately: the ledger uses them for quotations too, and an
apostrophe would make every possessive an opening mark. That gap has a witness — §12.7.5.2.2's
stale quotation is in single quotes and was found through `form.rs`'s copy of the same sentence
rather than by the sweep — and it is recorded rather than closed, because the alternative is a
heuristic over English punctuation.

## Consequences

- **Eleven stale quotations corrected**, six of them in a population nothing had ever looked at.
  Two are the standard-14 `shall`s the four-hundred-and-seventeenth session corrected in three
  places and missed in two more, which is what a sweep is for and what reading three files by hand
  is not.
- **One behaviour changed**, §7.8.3's first search step, and one test holds it. `1498` tests where
  `doc/todo/02` said 1495 and this round added one: the line was two behind, which is the failure
  that file names in its own longest paragraph.
- **Four of session 417's five owed items are settled as documented choices** rather than as code —
  §7.3.10's object-number grammar (a reader's tolerance, now stated), §7.5.6's multi-update version
  reduction (a deliberate omission with its cost written down: correcting it means resolving a
  catalog per `/Prev` section on the open path, for a number Annex I uses to warn), §14.8.4.7.2's
  enclosure reframing, and §14.6.1's Figure 9 in `variable_text.rs`'s `PERMITTED` list. The fifth,
  §8.9.5.4, stays declined for ADR 0253's reason.
- **`doc/md/` is unchanged and so are its 6087 citations and 575 quotations.** Every correction here
  either drops a quotation in favour of prose or keeps the retired words with the erratum stated
  beside them, so the gate verifies exactly as before. That is the same discipline the two rounds
  before this one used, and it is why the sidecar has cost nothing to run.
- **A gate over the ledger is no closer and no further away.** This sweep answers one question about
  977 spans and leaves the other — whether a span is in the standard at all — priced exactly as ADR
  0249 priced it.
- **One silence was found on the way and is not fixed here.** `draw_xobject` returns without a word
  when a `Do` names a resource the dictionary does not define, which is how the Type 3 defect stayed
  invisible: the test above passes `is_complete()` even with the fix reverted. A missing *font* is
  reported (`content.rs` argues for it at length); a missing XObject is not. Changing it moves the
  corpus's incomplete list and belongs to a round that can read that list, not to this one.
