# ADR 0401 — A quotation checker cannot see a citation, and two of them came from another edition

Status: accepted.
Session: the five-hundred-and-sixty-sixth, answering four clause corrections another
implementation's developers sent back against `doc/HAYRO_ISSUES_FOR_QUORRA.md`.

## 1. What this decides

Four things, and the third is the reusable one.

1. **All four of quorra's corrections are accepted, each after reading the clause here first.**
   Two of the four were misreadings on our side; two were a wrong clause number over a reading
   that was right. `doc/QUORRA_HAYRO_MAP_ANSWER.md` is the reply with the sentences under the
   clause numbers, and `doc/HAYRO_ISSUES_FOR_QUORRA.md` carries every correction *in place*, with
   what it changed, rather than quietly acquiring the right citation.
2. **A fifth was found while checking theirs, and it is the same failure as their fourth: a
   sentence quoted from ISO 32000-1 and attributed to ISO 32000-2.** Two of five is a shape.
3. **`tools/conformance/quotations` has three populations it cannot report, and this round is the
   first time all three were exhibited at once.** §3 is the whole of it, and it is why the sweep
   ran clean over a document with five wrong citations in it.
4. **§8.5.3.2's parenthesis "(specified by a trailing m operator)" is read here as a gloss and not
   as a restriction**, and §4 says on what evidence — because the whole of the second correction
   rests on that word and neither side had said so.

## 2. What the two misreadings were, briefly

The reply document has them in full; what belongs here is the pair that is *not* the same mistake,
because the difference decided what else had to be checked.

- **§8.9.6.4 for a mask on a grid of its own.** §8.9.6.4 is *Colour key masking* — `/Mask` in its
  array form, a test on the base image's own samples, with no second grid to differ from. The
  sentence wanted is §8.9.6.3's. One key, two clauses, and Table 87 names both in one sentence,
  which is what makes the slip easy.
- **§8.4.3.3 for a leading degenerate `MoveTo`.** We reasoned from "caps go on both ends of open
  subpaths" to a dot under round or square caps. §8.5.3.2's last sentence says no output, with no
  cap condition, and a specific rule about the same mark beats a general one.

Both of those are *readings*, and a wrong reading can be in the code. The other two —
§8.7.4.3 for a shading's coordinate space, and 32000-1's `/Interpolate` wording — were correct
readings under a wrong number, which can be in a doc comment.

So both kinds were grepped across `crates/`, the ledger and every other document. **The tree was
right in all four places.** The ledger's §8.5.3.2 row has said "a subpath that is only a trailing
`m` is no output under any cap" since the twenty-fourth session; `pdf-model`'s `MaskEntry` splits
§8.9.6.3 from §8.9.6.4 at the line Table 87 draws; every §8.7.4.3 citation in `crates/` is Table
77's, which is what §8.7.4.3 decides; and `pdf-render`'s `paint.rs` carries the EC3 `/Interpolate`
wording including the hint sentence verbatim. The error was in the hand-over document alone.

## 3. The three things the quotation sweep cannot report

`tools/conformance/quotations` (ADR 0249, made a program in the four-hundred-and-nineteenth
session) reads every blockquote and quoted span in `crates/`, in `doc/conformance/ledger.toml`
**and in every Markdown document under `doc/`** — these hand-over documents included — and judges
each against the conversions in `doc/md/`. It reports a quotation that matches a specification for
at least `MIN_MATCH` words and *then* diverges.

The five wrong citations in one document are, between them, one instance of each of three
populations, and only one of the three is visible:

| population | example here | what the sweep does |
|---|---|---|
| a **near-miss** — matches, then diverges | `/Interpolate`: "shall be performed by a conforming reader" against "should be performed by a PDF processor" | **reports it**, "matched 9 of 16 words, then diverged" |
| a **foreign sentence** — shares almost nothing | §10.7.4: "a conforming reader may need to make a determination about whether the pixel is painted" | counts it in "sharing too little with any of them to be a quotation of one"; prints nothing |
| a **wrong clause number over a right quotation** | §8.9.6.4 for §8.9.6.3's sentence; §8.7.4.3 for §8.7.2's rule | cannot see it at all — it checks what the words are, never what they are attributed to |

Three consequences, and they are the decision:

- **The instrument is most sensitive where the error is smallest.** The closer a misquotation is to
  the standard, the louder the sweep is about it; a sentence lifted wholesale from somewhere else
  is quieter than one word out of place. That is the opposite of the ordering a reader would
  assume, and assuming it is how a document with five wrong citations passes a sweep that reads it.
- **The near-miss was reported and unread, for as long as the document has existed.** Removing it
  took the divergence count from 26 to 25, which is how it was confirmed to be that line. The
  sweep's own preamble says "a divergence is a question for a person, not a build failure" — which
  is right, and does not make anybody the person. **No gate is added for this**: turning the sweep
  into a build failure would make its 25 standing questions block every round, and the failure here
  was reading rather than enforcement.
- **The citation half is a different sweep and is not built.** Checking that a clause number
  matches the clause its quotation came from is decidable — `Conversion` already knows which
  heading a matched span falls under — and it is not attempted here, because the population it
  would sweep is 1711 verbatim quotations and the round that builds it should be a round that can
  read its output. Recorded as owed rather than done.

**A cheaper check exists and costs nothing.** ISO 32000-2 §0.3 states that "[s]tarting with ISO
32000-2:2017 (PDF 2.0) the term 'conforming reader' is no longer used", and `pdftotext` over the
whole sponsored EC3 finds the phrase three times: once there, and twice in §6.3.2.1's NOTE
explaining that the subset standards define it and that "the notion of a 'conforming reader' is
not useful for this document". Three occurrences, all three of them *about* the term and none of
them using it. So **a quotation attributed to ISO 32000-2 that contains the words *conforming
reader* is wrong by inspection**, without finding the clause. Both of this round's edition slips contain it. That is a
one-line grep and it is worth more here than a sweep, because it decides the *edition* rather than
the wording — and the two documents this round touched were the only places in the tree where such
a quotation survived.

## 4. The parenthesis the second correction rests on

§8.5.3.2's last paragraph disposes of three shapes in three sentences:

> If a subpath is degenerate (consists of a single-point closed path or of two or more points at
> the same coordinates), the S operator shall paint it only if round line caps have been
> specified, producing a filled circle centred at the single point. If butt or projecting square
> line caps have been specified, S shall produce no output, because the orientation of the caps
> would be indeterminate. … A single-point open subpath (specified by a trailing m operator) shall
> produce no output.

The mark in question — a glyph outline beginning `MoveTo(0,0)` before its real `MoveTo` — is a
single-point **open** subpath in a **leading** position. The clause's *degenerate* is defined by
its own parenthesis as a single-point **closed** path or two-or-more coincident points, so the
cap-dependent rule does not reach it; the last sentence does, and says no output under any cap.

**Unless "(specified by a trailing m operator)" restricts rather than illustrates.** Read as a
restriction, a non-trailing single-point open subpath is governed by no sentence in §8.5.3.2 at
all, §8.4.3.3's general cap rule returns, and the original answer — a dot — comes back with it.
Read as a gloss, the clause is complete.

**The gloss reading is taken, on the sentence two above it**: "This rule shall apply only to
zero-length subpaths of the path being stroked, and not to zero-length dashes in a dash pattern of
a non-degenerate subpath." That sentence classifies by *shape* — zero-length, of the path being
stroked — and says nothing about position, in a paragraph whose whole subject is which shapes make
a mark. The restrictive reading would have the clause create a gap in its own subject matter and
leave it to a general rule three subclauses away; the gloss reading has it name the ordinary way a
one-point subpath arises and dispose of the shape. §8.5.3.3.1's parallel sentence supports this
from the other side: there the position *is* carried by normative words outside the parenthesis
("if the last subpath in the path is a single-point open subpath (specified by a trailing m
operator)"), and §8.5.3.2 has no such words — which is what a gloss looks like when the same
drafter also writes a restriction.

Recorded because the answer is stated as a fact in two documents now and it is one word away from
being the opposite fact, and because it is the kind of claim `CLAUDE.md` says decays: if a future
round finds a clarification or an erratum that makes the parenthesis restrictive, this is what it
has to overturn.

## 5. What was not changed

- **No code.** All four citations were already right in `crates/`. One comment in
  `render-quorra`'s `scene.rs` calls the display list's shading transform "§8.7.4.3's shading
  matrix" where Table 75's `/Matrix` is §8.7.4.1's; it is about which crate anchors the paint
  rather than about the clause, and it is left with the reply document as its record.
- **No ledger row.** All eleven clauses this round read already carry rows, none `unreviewed`, and
  §8.7.2's and §8.7.4.1's already quote the two sentences the third correction is about. A round
  that had to *add* the sentence would have found a hole; finding none is the result.
- **No gate.** §3's second bullet says why.
