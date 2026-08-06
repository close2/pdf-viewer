# ADR 0206 — The annexes the instrument could not name

Status: accepted, 2026-08-06 (session 360).

## Context

ADR 0205 read the ledger's `inapplicable` rows for the first time, at the project owner's
instruction, and closed with one of three re-reads still owed: `CLAUDE.md`'s **closed exclusion
list**. This round took that one, starting with the entry that looked most settled:

> **XFA** — deprecated by ISO 32000-2 itself and specified outside it.

The first half is true. **The second half is false**, and the standard's own table of contents says
so: `Annex K (normative) XFA forms`, pages 944 onward, inside ISO 32000-2. What is documented
separately is the XFA *template* architecture — the schema language — not the annex that binds it
into PDF.

Checking that one sentence turned out to be the small end of something larger. **The standard has
seventeen annexes and eight of them are normative** — D, E, F, I, K, L, O and Q — and the ledger had
a row for none of them. `CLAUDE.md`'s scope section is written in clause numbers because that is how
the standard's body is organised, and its closed exclusion list says nothing about an annex. So the
annexes were in scope from the beginning, and nothing was looking at them.

**The instrument could not have looked.** `ClauseNumber` is a `Vec<u16>`; `"K.2".parse()` failed,
and its own test asserted that failure — `assert!("A.1".parse::<ClauseNumber>().is_err())`. A
citation to `§K.2` was reported as malformed, a quotation from Annex Q could not be checked against
the text it quotes, and a row for Annex O could not be written. The silence was total rather than
partial, which is why nine sessions of ledger sweeps never brushed against it.

## Decision

### The number type learns the letter

`ClauseNumber` becomes `{ annex: Option<char>, components: Vec<u16> }`. `annex` is first in
declaration order so that the derived ordering puts every numbered clause before every annex, which
is the order the standard prints them in. `clause()` returns `Option<u16>` — an annex belongs to no
numbered clause and a sentinel would have said it belonged to clause zero.

Two artefacts of `doc/md/` had to be read rather than assumed, and both are in the conversion, not
in the standard:

- **An annex's title page is set after the subclauses that open it.** `## Annex K (normative) XFA
  forms` sits *between* `## K.2` and the rest of K.2's text. The span rule already ended a span at
  any heading that is not a descendant of the current one, which would have hidden half of K.2 below
  its own title. A heading that is an *ancestor* of the current number no longer ends its span.
- **`## Annex O` carries no title**; the next heading is `## (normative) Fragment identifiers`. An
  annex heading with an empty title takes the following unnumbered heading's text.

### The ledger's population grows by 52

`NORMATIVE_ANNEXES: [char; 8]` beside `TECHNICAL_CLAUSES: 7..=14`, and one `covered()` iterator
feeding both the generator and the gate. Unlike a clause, an annex gets a row for **its own letter**
as well as its subclauses: Annex L is one normative table with nothing numbered under it, so
excluding the top level the way `§8`'s is excluded would have left a normative annex with no row —
the state the whole population was already in.

The nine informative annexes stay out. They say *informative* on their own title lines and state no
requirement.

### What the 52 rows say

| annex | rows | status |
|---|---|---|
| **D** character sets and encodings | 7 | `implemented` ×5, `reported` (MacExpert), `partial` at the top |
| **E** extending PDF | 3 | `partial`, `writer-side` |
| **F** linearised PDF | 23 | `out-of-scope`, exclusion `writer-side` |
| **I** versions and compatibility | 4 | `partial` — **the version number is not read** |
| **K** XFA | 3 | `out-of-scope`, exclusion `xfa` |
| **L** structure element nesting | 1 | `writer-side` |
| **O** fragment identifiers | 5 | **`silent`** |
| **Q** determining transparency | 6 | `inapplicable` |

Two of those are findings rather than bookkeeping.

**Annex O is `silent`, and it is the ledger's first silence no file can trigger.** Eleven parameters
— `page`, `nameddest`, `structelem`, `comment`, `ef`, `zoom`, `view`, `viewrect`, `highlight`,
`search`, `fdf` — each a `shall` on "the PDF processor" when a document is opened through a URI.
Nine of the eleven name a mechanism this tree already has; what is missing is the sentence that
joins a fragment to them, and `Command::Open` has nowhere to put one. **A document cannot contain a
fragment identifier** — it arrives with the request — so the corpus and the oracle are blind to this
by construction, and no amount of running them would ever have found it. That is `CLAUDE.md`'s two
denominators at their sharpest: coverage answers a question robustness cannot see. `doc/todo/39`.

**Annex I.2 is `partial` for a `should` nobody had read.** "If a PDF processor opens a PDF file with
a version number newer than the version that it supports … it should warn the user." This tree
locates `%PDF-` to fix the byte offsets and throws the digits away, and does not read the catalog's
`/Version` either. The §12.11 half of the same annex *is* met — `requirements::unmet` names what it
cannot process — which is why the row is `partial` rather than `silent`.

### And the exclusion that started it, restated

Annex K stays excluded, on a better argument than the one it had. §K.1 grants it outright: "a PDF
processor may choose to not implement this feature". And §K.2 says what makes declining it *safe*,
which is worth having read rather than assumed — in a conforming hybrid file "[t]he other entries in
the interactive form dictionary shall be consistent with the information in the XFA resource" and
"[t]he XFA field values shall be consistent with the corresponding V entries of the PDF field
objects". So the AcroForm this tree reads **is** the form, by derivation from the standard rather
than by the usual observation that ignoring XFA tends to work.

## Consequences

- **875 ledger rows, up from 823.** `implemented` 397, `partial` 237, `reported` 30, **`silent` 5**,
  `inapplicable` 85, `writer-side` 8, `out-of-scope` 113.
- **`silent` is five, and it stays five until Annex O is built.** ADR 0204 set the precedent one
  round after finding §10.5: a finding is worth more visible than hidden, and the alternative here
  would have been to call eleven `shall`s `inapplicable` because no host has asked yet.
- **`CLAUDE.md` gains an entry it should always have had** and loses half a sentence that was
  wrong. The scope list now names the normative annexes, and the XFA exclusion rests on §K.1's
  permission.
- **A citation to `§K.2` or `§Q.3` now checks**, like any other, and a quotation from an annex is
  verified against the annex's own text.
- **The re-read the owner asked for is finished.** Clause 10's `inapplicable` rows (ADR 0204), the
  rest of them and the 87 `out-of-scope` rows (ADR 0205), and the closed exclusion list (this one).
  Between them they moved seven rows, corrected two sentences of `CLAUDE.md`, and added a
  population of 52 that no instrument in this project could previously address.
