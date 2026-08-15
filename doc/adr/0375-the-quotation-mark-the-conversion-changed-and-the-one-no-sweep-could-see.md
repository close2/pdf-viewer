# ADR 0375 — The quotation mark the conversion changed, and the one no sweep could see

Status: accepted, 2026-08-15 (session 540).

## Context

`doc/todo/48` had three things left on it, and its own header named them: §8.9.5.4, §14.8.6.3's
enclosure requirement, and the ledger's single-quoted spans. The first two are clauses this tree
records as owed; the third is a hole in the instrument that decides whether anything the tree
writes about the standard is true.

They turned out to be one subject. `doc/HANDOVER.md`'s rule — *when a gate accuses the standard of
a gap, suspect the conversion first* — is usually advice about a **finding**. Here it is advice
about the **instrument**: two of the three are answered by `pdftotext -layout` over the PDF in
`doc/`, and what it answers is that a quotation mark is not something `doc/md/` preserves.

## §8.9.5.4 — the conversion is faithful, and the reason the rewrite was declined is not

`doc/md/`'s §8.9.5.4 is the PDF's §8.9.5.4, word for word; there is nothing lost here and the
`![Image]` in the middle of it is an annotation icon rather than dropped text. What is unfaithful
is this tree's record of the *erratum*.

Errata Collection 3's Issue #79 is five marks on pages 279 and 280 of the sponsored copy, and
applied in the order they sit on the page they produce this:

- a) "If the base image contains an OC entry that specifies that the content is not visible, then
  nothing shall be shown."
- b) unamended: a visible base image is rendered.
- c) "Otherwise if the PDF is being printed and any of the Alternates entries has DefaultForPrinting
  set to true, then that alternate image shall be printed."
- d) "Otherwise, the list of alternates specified by the base image Alternates entry is examined,
  and the first alternate containing an OC entry specifying that its content is visible shall be
  shown (Alternates that have no OC entry shall not be shown.) Furthermore if the image dictionary
  that forms the value of the Image key of the selected alternate contains an OC entry, then that
  OC in the image dictionary shall not be examined."
- e) "If steps c and d above do not identify an alternate to be rendered then the base image shall
  be rendered."

The four-hundred-and-seventeenth session declined to implement this, on the ground that "the
amended step a) ends 'then nothing shall be shown', which reads as terminal and would leave the
amended d) unreachable for a hidden base, so a rewrite trades one contradiction for another"
(ADR 0253, and the sentence was copied into the ledger row, the doc comment and `doc/todo/48`).

**The premise is true and the conclusion does not follow.** a) is terminal, and d) *is* unreachable
for a hidden base image — because a) and b) between them dispose of every base image that states an
`/OC` at all, and c) and d) open with the word "Otherwise". They belong to a base image that states
no `/OC`, which the 2020 algorithm had nothing to say about. Read that way the five steps are
total, disjoint and reachable; the four they replace were not, which is why step c) contradicted
itself and why this tree carried a documented choice between two readings of it for a hundred and
twenty sessions.

**Decision: implement the amended algorithm.** `Interpreter::alternate_image` is step d) and
nothing else; `xobject.rs` carries a), b) and e); c) addresses printing and this device is a
screen, which is why `/DefaultForPrinting` is still read by nothing. §8.9.5.4 is `implemented`.

What changes on a page: a hidden base image no longer draws an alternate, a base image with no
`/OC` now may, an alternate with no `/OC` of its own is never shown, and the alternate's own image
`/OC` is no longer consulted. **Nothing in the corpus moves** — the corpus gate prints the same
974/64 before and after, and no corpus document states `/Alternates` — so the six fixtures beside
the code are the whole evidence, which is what a clause no file exercises is entitled to.

The general lesson is `CLAUDE.md`'s own and is worth more than the clause: a refusal recorded with
its reason decays exactly as a ledger row does, and the reason is the part to re-read.

## §14.8.6.3 — the conversion changes the standard's quotation marks, in this very clause

The enclosure requirement is real. EC3 (Issues #72 and #719) replaces the MathML sentence with one
that requires the `math` element to "be used to enclose the formula under the Formula structure
element type" and requires "[a]ll MathML structure element types and their attributes" to have the
namespace explicitly defined.

**It is a `shall` on whoever includes the mathematics.** The sentence opens "[w]hen including
mathematics structured as MathML", which addresses a producer, and `CLAUDE.md`'s closed exclusion
covers a clause whose requirements fall on a generator. What a *reader* owes here — that the
namespace identifies the mathematics, that an element in it keeps its own type, that it needs no
`RoleMapNS` — is done. The row says so now instead of carrying an unshaped debt, and what keeps it
`partial` is the one thing a validator would do: report the document that breaks either `shall`.

That reading is not the finding. The finding is three lines above it in the same subclause:

| | the PDF, by `pdftotext -layout` | `doc/md/` |
|---|---|---|
| §14.8.6.1 | `"http://iso.org/pdf2/ssn"` | `"http://iso.org/pdf2/ssn"` |
| §14.8.6.2 | `14.8.6.3, “Other namespaces”` | `14.8.6.3 , ' Other namespaces '` |
| §14.8.6.3 | `“http://www.w3.org/1998/Math/MathML”` | `' http://www.w3.org/1998/Math/MathML '` |

One glyph in the standard, two spellings in the conversion, and a space inserted inside one of
them. A rustdoc blockquote quoting §14.8.6.3 verbatim therefore **fails the gate**, and the gate's
message would say the standard does not contain it.

**Decision: `conformance::quote::normalise` drops every shape of quotation mark**, straight and
curly, single and double — the same category as the `*` and the `` ` `` it already drops. It is
applied to both sides, so it can only make the comparison coarser; what it costs is a quotation
differing from the standard in nothing but its punctuation. The mark's inserted spaces go with it,
because the surrounding whitespace then collapses.

## The ledger's single-quoted spans — and the rule that tells one from an apostrophe

`quoted_spans` collected `"` … `"` and nothing else, in two copies, for the reason `doc/todo/48`
recorded: an apostrophe is the same character as a closing single quote, so a scanner that pairs
every `'` makes an opening mark of every possessive. The population that cost is not small — the
ledger writes 106 single-quoted quotations of the standard, because a note already sits inside a
TOML string where a `"` has to be escaped — and it is where §12.7.5.2.2's stale quotation was found
by hand rather than by any instrument.

**Decision: one rule, in `conformance::quote`, shared by all three populations.** A `'` opens a
span only where nothing, a space or a bracket precedes it, and closes one only where nothing, a
space or ordinary punctuation follows it. `Table 89's`, `don't` and `processors'` fail the first
test; `‘` is unambiguous and needs neither. Three further decisions came out of running it:

- **A double quotation mark ends the search for a closing single one.** §9.4.3 names two
  text-showing operators `'` and `"`, so a note listing them opens a span on an operator, and
  without the bound that span runs to the next apostrophe and swallows every real quotation in
  between. What the bound costs is a single-quoted quotation carrying one of the standard's
  `(see 9.8, "Font descriptors")` cross-references — and it costs it cheaply, because the inner
  span is then collected in its own right and nothing goes unread.
- **A mark that never closes opens nothing.** The old `split('"').skip(1).step_by(2)` took the
  unterminated tail of an odd-numbered file as a quotation; its own doc comment said it did not.
- **A span inside a span belongs to the quotation that encloses it**, so `"a 'sticky note' attached
  to a point"` is one quotation and not two.

## The instrument had to end up able to see all of this

A round that corrects three quotations and teaches the checker nothing has fixed three quotations.
So:

- **The ledger's notes are a population of `--bin quotations`.** The eleventh sweep of
  `doc/todo/01` has been a hand-written script since the four-hundred-and-thirteenth, and a sweep
  whose rule is retyped every round is a sweep whose *level* cannot be compared between rounds —
  ADR 0360's argument about the fifth, one population over. Its first committed run found **three
  defects** in the ledger's own notes: Table 147's `dc:title` **element** quoted as an *entry*
  (§14.3.3's word, one clause away), §12.3.2.2's crop-box sentence closing its parenthesis where
  the standard has a semicolon and a cross-reference, and §14.8.4.2 quoting two of the standard's
  bullets as one sentence with an invented full stop.
- **The report prints how many spans were single-quoted**, so that a population reporting nothing
  is evidence rather than a silence. It is 106 in the ledger and **0** in 537 Markdown documents,
  which is the explanation `doc/todo/48` guessed at: this project writes single-quoted quotations
  where a `"` would need escaping, and essentially nowhere else.
- **`prose::folded` drops hyphens as well as spaces, and folds the fraction slash.** Four of the
  nine divergences the ledger's first run reported were `doc/md/` losing the hyphen of a word it
  broke across a line — `text-tospeech`, `implementationdependent`, `markedcontent` — and one was
  `1 ⁄ 72` set with U+2044 against a note that types `1/72`. All five are the conversion; none is a
  quotation. This is ADR 0253's space-removal repair extended to the same defect one character
  over.

## Consequences

- The gate can verify a quotation of any clause whose own text carries quotation marks. It could
  not before, and the failure would have been reported as the standard's.
- The ledger's notes are checked by a program for the first time, in both marks, every round.
- `spec-errata check`'s ledger landings went from 12 to 20 in the struck-out-of-another-clause
  bucket the moment the single-quoted spans became visible to it; the in-clause bucket is 10 and
  every one of them is a row quoting the wording it retired, which is `doc/todo/01`'s known shape.
- `Namespace::is_standard` has a caller after a hundred and fifteen sessions, and it is not the one
  its doc comment predicted. §14.8.6.2's "[a]n element shall be considered to be in one of these
  namespaces if:" is not only a conformance requirement on a document: it is what decides whether a
  type *name* is §14.8.4's word or a foreign vocabulary's homonym, so `Tree::standard_role` now
  answers `None` for a name that ends outside a standard structure namespace. `Tree::role` still
  answers the name, because that is what the document wrote.
- What the comparison no longer notices: punctuation, apostrophes, and — in the coarse sweep only —
  hyphens. Each is written down where it is applied, with the witness that bought it.
