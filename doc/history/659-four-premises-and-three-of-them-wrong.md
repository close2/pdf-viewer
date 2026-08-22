# 659 — Four premises, and three of them wrong

Ninth merge round, four branches, **no conflicts** — the second clean four-way merge running. What
distinguishes this batch is that three of its four rounds were sent to do something the briefing had
described incorrectly, and each found the error by reading the clause rather than by building what it
was asked for.

## What was merged

`round-655`, `round-656`, `round-657`, `round-658`, branched from `78c1d8a9`. Eight files were
touched by more than one branch, none collided.

## The sequence, whole, on a quiet machine (load 0.74)

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `cargo check --manifest-path
fuzz/Cargo.toml --bins` — all silent · `nextest` **2405 passed, 17 skipped** · doctests, conformance
(163 + 5 + 1) · corpus **974 documents, 68 incomplete** · oracle **908 agrees, 65 contradicted, 786
ambiguous** · `render-quorra` **933 agree, 22 differ** · `fixed_documents` **40 checked, 0 absent** ·
accessibility census **1336 with no place** · selection, text, dates, XMP, JPEG 2000 · `cargo deny`
all four ok.

**The ledger moved for the first time in five merge rounds**: 224 `partial` and 76 `inapplicable`,
where it has read 222/78 since session 637. Both are 658's — §14.8.3.3 and §14.8.5.4.5, which had
claimed the clause could not reach this program.

## Three premises, and what each turned out to be

**655 was told the `scn` resolves three clauses at one moment. It resolves none of them there.**
§11.6.7 puts a shading pattern's black point, intent and smoothness at *the beginning of the content
stream holding the `scn`* — "shall not inherit the current values of the graphics state parameters at
the time it is evaluated" — §11.7.2 puts the compositing target at the group the mark is in, and
§11.7.5.2 puts the transfer function at the mark. Three clauses, three different moments, and the
briefing had all three in one place.

Its finding is better than the correction: **`Interpreter::base` — §8.7.2's pattern matrix, scoped at
the four ways of becoming a parent since session 52 — *is* §11.6.7's first named parameter**, and the
other two had never been connected to it. Trap 5's shape lifted to a clause: *where one rule governs a
set of parameters, implementing it for one is the failure that reports nothing.*

**658 was told a computed extent would be "ours" against Table 379's "theirs".** It is not: §14.8.3.3
is a `shall` — "Two enclosing rectangles shall be associated with each BLSE and ILSE … The content
rectangle shall be derived from the shape of the enclosed content" — and §14.8.5.4.5 states the
derivation per structure type, two of whose five cases are marks rather than layout. So the question
trap 5 was supposed to settle did not arise: it is **additive**, replacing an *error* on
`org.a11y.atspi.Component` with a rectangle, and where the two disagree the marks win — which
**reverses half of ADR 0301 on ADR 0301's own argument**.

It also corrected the briefing's citation: §14.7.4.2 is the namespace dictionary; marked-content
sequences are §14.7.5.2.

**656 was told to use 651's method of choosing a group. That method was spent**, so it asked the
question one level down — *how many of a group's own members does its note actually measure?* Thirteen
of fourteen measure all of theirs; one measured two of five and described the other three from their
**dictionaries**, which is trap 9's fourth shape on the clause family ADR 0456 already records costing
six rounds.

**Only 657's premise held**, and it delivered four defects against it.

## Trap 9's sixth mechanism: a shared external standard

656's is the batch's most durable finding. On five `DeviceCMYK` shadings, at 125 sample points, there
are two camps: ours↔`poppler` within 4 levels, `mupdf`↔`ghostscript`↔`hayro` within 6, **48 and 51
across the divide**. And ADR 0009's sixteen ink corners over the closed-form CMYK are within **one
level of 255 of our own raster everywhere** — a statement about this tree with no renderer in it.

`ghostscript` reads Artifex's CMYK SWOP profile off disk; the identical 187 484 bytes
(`md5 fd199526f0a7e0bceb294a777cd84252`) are embedded **verbatim** in `libmupdf.so`. That is the
shared-data shape. But **`hayro` shares neither** — `objdump -p` gives `libgcc_s`, `libm`, `libc` and
no colour library, and it carries its own 8 464-byte CC0 profile — and sits with them anyway, because
Artifex's descriptor says SWOP and CGATS TR 001 is SWOP's characterisation data. **Two independently
authored files, one printing condition.** Not shared code, data, defaults, wider code or coincidence:
a shared external *standard*, which no dependency graph and no digest comparison can find.

By-catch: `poppler` fails the document's own six-square invariant by one row of 600 pixels, and the
sentence licensing all four assumptions is **§10.3.2's NOTE about the CIE-based *source*** where the
code and the ledger cited §10.3.1's NOTE about the *destination* — one subclause off, both carrying
the phrase "assumptions made by the PDF processor software".

## 657, and the habit it earned

Four defects, of which two are worth restating. **§9.7's row cited "§9.7.5.1's remainder", a string
`git log -S` finds in exactly one commit — the same commit whose own message records moving §9.7.5.1
to `implemented`.** A self-contradiction committed in one act, standing 228 sessions. And **§10.7 said
smoothness is ignored by permission**, while Table 57's `/SM` has decided a shading's colour-function
sampling since session 74 — §8.4.5 carried the identical claim and session 565 retired it *there*
without grepping the tree, leaving ninety-two sessions in the surviving home.

Its habit is now in `doc/todo/01`: **run the sweeps before your edit as well as after, and account for
every moved number.** Its own first correction was entirely true and moved `--bin unread` from 69
rows/182 keys to 70/185, by repeating three entry names a neighbouring row already carried. *A level
is one integer from a program that does not know what you are trying to say — a round can talk itself
past a hit, not past 185→182.*

And it re-derived a price under the rule this project adopted last batch, and **the price stood**:
§10.7.5's grid-fitting departure, where the two layers built since were checked and *snap nothing*, so
they do not shorten the work — they make it a different work.

## Two lessons that belonged to no round, committed separately (`92dc36ad`)

- **A negative claim decays when the population grows**, which is a fifth decay shape and unlike the
  others in that nothing in the tree changes. 655's witness: "no corpus document writes one" of a
  pattern's `/ExtGState` was true when written and measured over corpora with no crawl in them —
  `doc/pdf.js` holds 38 Type 2 patterns and **zero** state one, the crawl holds 1504 and **42** do.
- **The scratchpad directory is shared between parallel rounds**, not per-session; 656 had its gate
  log overwritten mid-run. It sits beside the `git stash` warning because it is the same shape.

## Owed

- **1021 elements whose sequences marked nothing** — no clause derives a rectangle from no marks, and
  that is recorded as an answer with its count as the instrument.
- **A form XObject's sequences share the page's `/MCID` numbering** (§14.7.5.2 permits it), which
  could misattribute both a rectangle *and* the text range keyed the same way since ADR 0134.
  **Unmeasured; no corpus document checked.**
- **§10.5's remaining half**: the shading object carried in `PatternPaint::Shading` so colours rebuild
  at the paint — with 655's warning that the rebuild must keep reading `PatternInitial`, or it trades
  one departure for another.
- **The owner's session** for `tmp/pi.pdf`, and **a push**: nothing since the fuzz repair has faced CI.
