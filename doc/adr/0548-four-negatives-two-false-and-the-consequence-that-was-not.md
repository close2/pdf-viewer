# ADR 0548 — Four negatives, two of them false, and a consequence that is not a structure

Status: accepted.

Four of `doc/todo/01`'s owed "no corpus document …" sentences re-derived over both populations.
**Two are false, two hold**, and the two false ones do not fail the same way: one leaves a
requirement reachable that the ledger called `implemented`, and the other leaves a *sharper*
sentence standing that the row never said. A fifth erratum in as many rounds arrived with them.

## The queue, run rather than quoted

`doc/todo/01`'s own script over `doc/conformance/ledger.toml`, on this worktree before the edit,
printed **30 done and 16 owed** over a population of 46 — which is what the briefing said and what
running it confirmed. After this round it prints **34 and 12**, over the same 46. The population is
checked as well as the counts, because a correction written in this project's house style —
`[n]o corpus document …` — stops the script's regular expression matching and silently takes a row
out of the sweep instead of across it (ADR 0523's first draft did that to two rows).

**Of the twelve left, ten are not measurement.** Four are not a claim about a corpus at all
(§7.5.6, §8.9.3, §10.7.4, §11.6.2) and six are the sweep's own noise — three corrections quoting
the negative they retired, three the grep's sentence boundary landing inside a clause number.
**Two are real and both are the expensive kind**: §9.7.6.2's codespace range and §11.6.7's tiling
pattern paint, each needing an instrument that is not a name census.

## The instrument this round owed: a census of operator *shapes*

`examples/witness_census`'s third column already searches every stream's decoded data, so anything
whose witness is a **token** is in reach without an interpreter (ADR 0523). What is not in reach is
anything whose witness is an *order*: a segment operator that has no move before it, a `q` and a `Q`
with a `Tm` between them. `examples/operator_shape_census` is that instrument. It lexes a page's
`/Contents` and every form `XObject` its resources reach, tracks two small state machines, and takes
the three scopes ADR 0523 gave the other four censuses plus a `--pages=` bound, because §8.5.2.1's
sentence is about a first page and §9.4.2's is about any page.

Three things it does deliberately:

- **Inline images are skipped through `pdf_model::inline_image::scan` rather than lexed.** Image
  data is bytes and bytes lex into keywords. A planted fixture whose samples spell `l 1 1 l` and
  `q … Tm … Q` scores zero on every column, and a second fixture with `0 0 l S` *after* the `EI`
  scores one — so the skip is checked in both directions rather than assumed.
- **What it does not walk is printed as a count**: tiling pattern cells, Type 3 glyph procedures and
  annotation appearance streams, 37 685 of them across the crawl's first pages. A zero over a
  population an instrument cannot see is this project's standing false-zero shape, so the size of
  the unseen part is under every run.
- **`h` is counted apart from `l`, `c`, `v` and `y`.** They are one sentence in the standard and two
  different costs in this tree, which is the whole finding below.

## §8.5.2.1 — false, and the row's status was wrong because of it

The row said, and had said since the twenty-fourth session, that the clause's own sentence — *if the
current point is undefined, an error shall be generated* — was **reachable by no corpus first page**,
while the row's status read `implemented`.

It is reachable on both populations. One curated first page does it: `issue6342.pdf`, a pdf.js test
file whose form `XObject` the document itself titles *Form XObject with errors*, writes a `c` after
an `f` with no `m` between. Five crawled first pages carry 133 such operators between them, and over
the curated corpora's first hundred pages apiece it is twelve documents and 5010.

**And the consequence splits where the clause does not.** A segment with no current point reaches
`tiny_skia::PathBuilder`, which injects a move to the origin before it, so the page gets a line from
the corner of user space that nothing asked for. A close with no current point reaches
`content::path::close_subpath`, which pushes nothing onto a path with no command to close — so the
standard's error and this tree's silence draw the same page. That is 890 `h` on sixteen further
crawled first pages costing nothing, beside 133 segments costing a mark.

So the row is `partial` now, with the refusal named as what is owed. **The neighbouring sentence in
§8.5.3.1 is the shape of the answer and not a licence to borrow it**: Errata Collection 3's Issue
#549 turns *generate an error* into *be ignored* there, and left this one alone — and the same census
gives that amended sentence its population, 38 047 painting operators on an undefined path over 459
crawled first pages, which nothing had counted either.

## §9.4.2 — false in its outer half, true in an inner half nobody had stated

The row said 13 of 974 documents put a `q` or a `Q` inside a text object and **not one moves Tm
between the two**, so Issue #368's rule — that `q` and `Q` push and pop Tm and Tlm inside a text
object — moves no page.

The second clause is false. `NegativeFontSize.pdf`, in `doc/corpora/pdf-differences` and therefore
in the oracle's own population, writes four `q` … `Tm` … `Tj` … `Q` pairs strictly inside text
objects on its first page; a crawled first page writes a fifth.

**But a restore only reaches a mark if something is drawn from it**, and Table 106 says the operands
of a `Tm` *shall not be concatenated onto the current text matrix, but shall replace it*. A producer
that closes a save inside a text object positions afresh immediately afterwards, so the restored
matrix is overwritten before a glyph sees it. The census counts that as its own column, and it is
**zero on every well-formed page of both populations**. The two pages where it fires are a
`govdocs1-error-pdfs` file and a crawled document whose content stream is corrupt; both reach the
rule by lexing damage into a `q`, which was read by hand rather than trusted to the count.

So the row keeps its conclusion on a sentence it never wrote: the rule moves no page a producer
wrote, and `text_state.rs`'s synthetic pair is still the only thing that discriminates. **A negative
can be false in the half a row states and true in the half it means**, which is ADR 0516's rule met
for the fourth and fifth time — and a round that had asked only the outer question would have
reported a defect that is not there.

## §9.7.5.4 and §12.7.5.4 — both hold, on populations fifty-three times the size

- **§9.7.5.4**: no CMap writes `beginrearrangedfont` or `beginusematrix`. Zero of the 65 703 crawled
  documents that open, against zero of 1239 curated, with `endcmap` in **46 028** of them as the
  control that the search reaches a CMap operator at all. Twenty-three minutes of `witness_census`,
  and the status does not move.
- **§12.7.5.4**: every list box in the corpus states an appearance stream and none is in a
  `/NeedAppearances` document. The crawl states two list-box widgets over two documents and both are
  the same. `examples/variable_text_census` took the three-scope selector `doc/todo/01` said it owed;
  its control run reproduces the row's figures — 26 combo boxes with one lacking an `/AP` `/N`, ten
  list boxes over eight documents — to the digit before the crawl run was believed. The combo-box
  count moved (273, of which 14 state no appearance) and no claim moved with it.

## Issue #373, and the second reason `check` is blind

`spec-errata emit` on every clause this round touched found **Issue #373, `Review/Completed`, on
§9.4.2, recorded nowhere in this tree**: Table 106's `T*` row gives the operator as the code
`0 –Tl TD`, and the erratum strikes `TD` for `Td`. Its `/QuadPoints` and `pdftotext -bbox`'s word box
agree to three decimals, so which `TD` it is was established rather than inferred.

The four rounds before this one each found a `Caret` with no `StrikeOut` and concluded that an
erratum which only *adds* is invisible to `check`. **This one strikes and replaces and is invisible
for a different reason**: `check` compares struck passages of four words or more, so an erratum
correcting a single token is below its floor whatever its shape. Both roads lead to the same rule —
`emit` before writing, which `doc/todo/02` §4 already asks for.

It costs no arithmetic: `TD` sets the leading to the negation of its own `ty`, and a `ty` of `-Tl`
leaves the leading where it was. What it corrects is a reader, and it had already corrected this
one — `run.rs` implements `T*` without touching the leading. §9.4.2's ledger row and
`text_state.rs`'s `leading_moves_the_next_line_downwards` both quoted `0 -Tl TD` and now do not.

Two more, recorded so that the next round need not look: **Issue #372** strikes `Tm` for `Tm` and is
typography; **Issue #191** is filed by `emit` under §12.7.5.4 and belongs to §12.7.5.3, giving Table
232's `/MaxLen` a stated floor of zero that `appearance.rs` already enforces.

## The rule this round adds to the recipe

The four before it earned: a false negative need not imply owed work; count the condition rather than
the noun; split the negative; probe a positive as well as a zero; and check the population as well as
the counts. A sixth:

> **A negative about a *structure* and a negative about its *consequence* are different sentences,
> and the instrument must be able to print both.**

§9.4.2's row conflated them and was half wrong; §8.5.2.1's row conflated them and was wrong in the
opposite direction, calling a reachable requirement `implemented` because the shape it feared had no
witness. Neither could be settled by a census that answers *does this shape occur*, because the
question that decides a page is *does anything read what the shape left behind*. Where the two can be
separated by a state machine over the same tokens, separate them: it is one extra column, and here it
turned one false negative into a true sentence and one true-looking status into a `partial`.
