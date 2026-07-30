# ADR 0048 — One profile, seen twice

Status: accepted, 2026-07-31.

## Context

`CONTRADICTED_UNEXPLAINED` held fifty pages: contradicted by two references that agree, with
nothing on the page to explain it. The handover calls it the most valuable list in the
repository, and the way in is the one thing this project keeps getting right — open the
artefact, then *measure* it, because a label this project wrote is still a label.

Four of the fifty had a signature visible before any of them was opened. Comparing every
render against every other, `ours` and `poppler` sat within 0.6 of a level, `mupdf`,
`ghostscript` and `hayro` within 0.6 of each other, and the two groups 3.6 to 10 levels
apart. Two clusters of two is not one page's problem.

## What the pages are

`type4psfunc.pdf` and `postscript_type4_many_outputs.pdf` reach `DeviceCMYK` through a
`/DeviceN` whose alternate it is; `function_based_shading_cmyk.pdf` reaches it directly.

**`postscript_type4_many_outputs.pdf` is a controlled experiment somebody else wrote.** A
200-pixel page holding one axial shading, whose function is
`{ dup dup dup dup dup dup dup dup }` and whose tint transform is
`{ pop pop pop pop pop pop pop pop 0 0 0 }` — so nine `/DeviceN` components all equal `t`,
eight are popped, and the colour is exactly `(t, 0, 0, 0)` for `t` running linearly from 0 at
the left edge to 1 at the right. One CMYK axis, sampled two hundred times, with the function
itself too simple to be the disagreement.

All five renderers agree at both ends — white at `c` = 0, and (0, 174, 239) within a level at
`c` = 1 — and differ only in between. **That is the signature of an interpolation, not of a
formula.** Ours is exactly ADR 0009's multilinear interpolation of the sixteen corners: at
`c` = 0.5 red is 127, which is 255 × (1 − c), and green is 214, which is 255 − c × (255 − 173).

## The finding: their agreement is one ICC profile, and our own evaluator proves it

Trap 9 says two references can agree because they share code or because they share a *gap*.
This is a third way: **they share data.**

`/usr/share/ghostscript/iccprofiles/default_cmyk.icc`, evaluated by `pdf_model::icc` — this
tree's own A2B evaluator, written for `ICCBased` streams and here pointed at a file on this
machine — gives, for the red channel at the nine eighths of `c`:

| | 0 | ⅛ | ¼ | ⅜ | ½ | ⅝ | ¾ | ⅞ | 1 |
|---|---|---|---|---|---|---|---|---|---|
| **our evaluator on `default_cmyk.icc`** | 255 | 219 | 186 | 150 | 112 | 59 | 0 | 0 | 0 |
| `mupdf` | 255 | 220 | 186 | 150 | 110 | 54 | 0 | 0 | 0 |
| `ghostscript` | 254 | 218 | 184 | 148 | 108 | 50 | 0 | 0 | 0 |
| **ours** (sixteen corners) | 254 | 222 | 191 | 159 | 127 | 95 | 63 | 31 | 1 |

So the two references that outvoted us are one CMYK profile run twice. Their agreement is not
evidence about the clause, and — trap 12 — the *tightness* of it is also what shrinks the
relative bound until our difference falls outside it.

**The instrument was already in the tree.** The A2B evaluator exists because §8.6.5.5 needs it
for `ICCBased` streams; pointing it at another program's profile took one throwaway example and
answered in one run what reading two projects' source would have taken an afternoon to
suggest and could not have proved.

## Decision

**The four pages become `CONTRADICTED_DEVICE_CMYK_CONVERSION`, and are not fixed.**

Principle 5 forbids the fix. ISO 32000-2 states no destination for `DeviceCMYK`: §8.6.4.4 says
only "concentrations of process colourants", §10.4.2.1 ranks §10.3's ICC route above §10.4.2's
"crude approximations", and §10.3.2 licenses a processor to supply a profile for a device
space — which is what `default_cmyk.icc` is, somebody else's choice of press. Adopting it
because it would move four pages into agreement is curve-fitting with a licence attached, and
the measurement that supports the sixteen corners is corpus-wide rather than four pages: ADR
0042 tried §10.4.2.5's formula and the gate went from 802 agreeing to 800.

What *would* change these pages is a document asking for a profile — a `/DefaultCMYK`
(§8.6.5.6) or an output intent's `/DestOutputProfile` (§14.11.5), both of which already
outrank the table. None of these three files has one.

## The spec track: §7.5 read as a family, thirteen rows

Clause 7's remaining unread rows were all §7.5, and two of them mattered.

**§7.5.2's second half was not implemented.** "[B]yte offsets shall be calculated from the
PERCENT SIGN (25h)", with NOTE 1 permitting arbitrary bytes before the header. A file with junk
in front of `%PDF-` therefore has a *correct* cross-reference table whose every offset is short
by the junk's length — and this reader fell through to scanning the whole file for `N G obj`
headers instead. `read_from_startxref` now adds the header's position to `startxref`, `/Prev`,
`/XRefStm` and every entry, which is zero for a file whose header is at zero. One corpus
document has junk before its header, `issue6069.pdf`, which says so in its own page text.

The test is written as a comparison — the same file with and without junk — because a reader
that ignores the rule still *opens* it by scanning, so a plain "it opens" assertion passes
either way. `was_recovered()` is what separates them, and the test was confirmed to fail with
the rule removed.

**Table 15's `/Size` rule is a departure, and now it has a number.** "Any object in a
cross-reference section whose number is greater than this value shall be ignored and defined to
be missing by a PDF reader." Enforcing it, temporarily, takes the corpus gate's *no page one*
count from **11 to 77**: 66 documents lose their page tree to a `/Size` their producer
understated, and 68 of 974 write at least one entry beyond it. The rule protects nothing here —
a number beyond `/Size` is looked up like any other and fails like any other if its offset is
wrong — and it costs 66 documents. Recorded on the row with the measurement, not with an
opinion.

Three more rows record something satisfied by construction rather than by a rule, which is
worth as much: §7.5.7's "strings occurring anywhere in an object stream shall not be separately
encrypted" is true because `expand_object_stream` parses out of already-decrypted data and
there is no path to a second decryption; §7.5.6's "most recent copy of each object" is one line
in `XrefTable::add`; and §7.5.8.4's precedence for a hybrid file's `/XRefStm` is produced by the
order of two loops rather than by a comparison.

## Consequences

- `CONTRADICTED_UNEXPLAINED` is 46, down from 50, and none of the four was a defect.
- Clause 7 has no `unreviewed` row left: 138 of 138.
- Trap 9 gains a third shape — **shared data** — and the method that found it: point this
  tree's own evaluator at the other program's data.
