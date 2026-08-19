# 597 — The border a default draws

2026-08-19. Both tracks, as `doc/todo/02` §1 requires.

## Demand-driven: `doc/todo/33`'s last drawing item, §12.5.4's border on a free text annotation

**Drawn, where it had been refused since the four-hundred-and-first session.** ADR 0432 has the
argument; `doc/todo/33` §1d and the §12.5.4 and §12.5.6.6 ledger rows have the result.

What the clauses state, read verbatim from `doc/md/`:

- §12.5.4 gives the shape whole — "An annotation may optionally be surrounded by a border when
  displayed or printed. If present, the border shall be drawn completely inside the annotation
  rectangle", and "If neither the Border nor the BS entry is present, the border shall be drawn as
  a solid line with a width of 1 point".
- Table 177 gives this subtype the entry that carries it, a `/BS` for "the annotation's border",
  and its `/RD` row puts the mark on the inner rectangle.
- Table 168's `/S` styles it. §12.5.4 names the four subtypes whose `/BS` is width-and-dash-only —
  line, square, circle, ink — and free text is not one of them.
- **The colour is stated nowhere, and this was re-checked rather than assumed.** Every sentence in
  ISO 32000-2 relating a border to a colour was extracted and read: a link's `/C`, a widget's
  `/BC`, a collection card's, a tagged element's `BorderColor`. For an annotation's border in
  general the standard states none, so black is §8.4.1 Table 51's initial value and is written down
  as a choice.

**The population, printed before it was trusted (trap 11).** `examples/free_text_census` gained
four counters over the annotations an appearance is *constructed* for:

- 73 free text annotations in 27 of 964 openable documents;
- 67 carry an `/AP` `/N` stream, where §12.5.2 has a reader ignore `/Border` and `/BS` outright;
- of the six left, **all six state `/Border` and four state a width of zero**; **none** states
  neither entry;
- so the border report's whole population was **two annotations in one document**, and that
  document is `poppler-395-0-fuzzed.pdf`.

`doc/todo/33` and the ledger row both said the refusal "fires on every annotation". It did not, and
that sentence is what the round retracted: it was a claim about producers, it had an instrument,
and nobody had run it for a hundred sessions.

**Additive or substitutive (ADR 0106).** No entry claims this colour, so painting in the initial
one substitutes for nothing — additive, and drawn. A cloudy `/BE` *does* state a different shape,
so it stays a whole refusal, which §12.5.4 gives this subtype in as many words and which nothing
here had been reading.

**Pixels moved on exactly one page, and it was looked at.** `poppler-395-0-fuzzed.pdf` page 1,
rendered at 2× with the border call disabled and again with it live: the diff is the two
rectangles and nothing else (0.32% of pixels), each drawn inside its `/Rect`, the text unmoved. The
page is a signature panel and reads as one now. Its two reports are gone; the annotation's other
two — an undecodable content stream and a `/DA` naming a font the `/DR` does not define — remain.

`doc/todo/00` step 7's ink sweep re-run over all 786 ambiguous pages from the artefacts on disk:
20 at or past −1, head `issue12418_reduced.pdf` −19.4, `issue4722.pdf` −13.8,
`issue15977_reduced.pdf` −12.9. The moved page is invisible to it — its oracle verdict is *not
comparable*, all three references having failed on the fuzzed file — which is the same shape as the
four-hundred-and-fifth's entry in that file.

## Spec-driven: a table of counts in `doc/ledger-and-claims.md`

Session 593 recorded a status table whose `rows` column disagreed with the gate; the gate says
`implemented` 433, `partial` 225, `reported` 17, `inapplicable` 79, `writer-side` 8,
`out-of-scope` 113 against a table reading 410, 244, 18, 82, 8, 113. Fixed the way ADR 0281 says:
**the column is gone** and the command that prints it is named. What the table keeps is what no
command can produce — what each status means and what its population has cost.

Two more instances of the shape in the same file, found while there:

- "`partial` and `reported` are **262** rows between them" — arithmetic on the table above it, the
  exact construction `tools/state.sh` refuses.
- "All **823** subclauses of the eight technical clauses have been read" — two sentences above the
  paragraph that already names `--bin ledger` as where counts come from.

One discrepancy left standing deliberately: the paragraph on `writer-side` says "7 rows were
re-read in the hundred-and-thirty-seventh session … six stay and one moved", beside a gate that
prints 8. That is a record of a past round's work rather than a claim about the present
population, so it is a history sentence and not a stale count. A later round may disagree.

## Errata

`spec-errata emit` run over `doc/ISO_32000-2_sponsored_EC3.pdf` before writing, per `doc/todo/02`
§4. Clause 12.5 carries sixteen accepted or completed annotations; none touches a border's colour.
The one that bears on this family is **Issue #287**, which sharpens Table 166's "the Border entry
is ignored" to "shall be ignored" — already recorded at `appearance::border_width` and in the
§12.5.4 row, and the same precedence either way.

## Gates

Whole of `doc/todo/02` §2, this being a round that moves a pixel. `clippy --workspace
--all-targets` was run **after** the final edit, per §2's own rule and session 596's note.
