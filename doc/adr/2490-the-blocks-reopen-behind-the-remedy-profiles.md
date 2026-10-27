# 2490 — The blocks reopen behind the remedy profiles

Status: proposed, with `doc/questions/Q694` for the three items the owner decides. Session 1828,
the review slot of batch one hundred and twenty-three. Context: `doc/questions/A681` item 1
("and yes open the blocks. we have remedy profiles / baselines"), the list it is decided over,
`doc/pdfa-blocking-issues.md` (every figure below is that census's, and its
`rank.py` re-counts them), ADRs 0816 (the converter edits no producer's content stream), 1014 and
1120 (the near side of the authoring line), 2114 section 4 and 2130 (the press), 2394 (the
photographed page and the owner's gate on it), 2442 and 2477 (the rankings that left the blocks
out), review 1815 section 1.2.

## What changed

ADRs 2442 and 2477 ranked the refusals while leaving out four populations by rule: a shipped
profile's `stop`, the fence rows, `image-smoothing` and the press profile's part 4 block. A681
overrules that: if the window can show the document, a profile or baseline will usually find a way
to archive it, and the genuine exceptions are stated. The census under `as-if-printed` counts 289
documents refused or declined at 2b and 292 at part 4; all but 24 fall under one of nineteen
remedy families. This ADR names, for each block, what reopening it needs, and the order in which
batch 124 builds them. It builds nothing.

## The decision

Every reopening keeps CLAUDE.md's test — does the operation invent marks? — and ADR 0816 as to
bytes: no remedy below edits a byte of a producer's content stream. Each names its loss and the
profile it falls under.

| block | remedy | loss | profile | needs |
|---|---|---|---|---|
| `image-smoothing` stop | R02: `/Interpolate` written false | `image-smoothing` | both `discard` | the profiles' entries; decided here |
| `as-shown` font stops | R09 (a) embed the machine face the window drew, where its permission allows; (b) scale the shipped face's outlines to the stated widths (ADR 0358); (c) until then authorise both by name | `machine-face`, `substitute-stretch` | `as-shown` | (c) the profile's entries; (a), (b) builds; decided here |
| fence: a resource named and not held | R04: the resources the reader used written into the form; an empty resource of its category for a name nothing defines | none | both | builds; decided here |
| fence: `.notdef` named | R03a: the program given a glyph for that code whose outline is its own `.notdef` outline | none visible | both | builds; Q694 item 3 asks whether the owner prefers R03 |
| fence: a TrueType route ISO 19005 does not admit | R11: the program's `cmap` and the encoding rewritten so the admitted route reaches the glyph this reader drew | none visible | both | Q694 item 3 (A48's line) |
| fence: an operator, name, string, box or width the target refuses | R03: ADR 2394's photographed page for the page holding the fault | `text-as-image` | `as-if-printed`; `as-shown` keeps ADR 2394's gate | Q694 item 1 (A570's gate) |
| an unembedded font no route carries | R08: the same photograph for any page showing it | `text-as-image` | `as-if-printed` | Q694 item 1 |
| press block, `DeviceCMYK` beside an RGB intent (part 4) | R05 (a) the shipped press profile as the default; (b) this reader's assumed press written as an ICC CMYK table | `press-assumed` (new) | (a) `as-if-printed`, (b) `as-shown` | Q694 item 2 (ADR 2114 section 4); (b) also an ICC table writer |
| press statements kept | R17: `halftone-name`, overprint mode 1 discarded as `as-shown` already does (ADR 1965) | both named today | `as-if-printed` | the profile's entries; decided here |
| a part the source cannot meet | R06b: "2b, or 4 where 2b refuses" as one request, the part written said | none | both | an option of the verb; decided here |
| a header no version this tree reads | R06a: the target's header written (§7.5.2), the source's header bytes in `xmpMM:History` | none | both | decided here |
| declined as a source | R01: the structure the reader recovered written, the photograph its fallback | `recovered-structure` (new) | both | decided here; it reopens the declining sentence |
| a filter no reader decodes | R07: the stream treated as the reader treated it, absent | `undecodable-stream` (new) | both | decided here |
| a DeviceN colourant with no Separation | R12: its Separation derived from the producer's DeviceN space, the tint transform restricted to that colourant | none for a PostScript calculator function | both | decided here for the exact case; a sampled or exponential function stays refused until its loss is named |
| an embedded file PDF/A-2 will not take | R15: appended as pages under ADR 1014's permission; at 4f kept as it is | `attachment-as-pages` (new) | both | inside ADR 1014; builds |

R02 is decided here because the owner's words open the stops and no earlier answer is changed by
it: ISO 32000-2 §8.9.5.3 says of the entry "However, this is only a hint, and a PDF processor may
ignore it", so a print need not have carried it, and under `as-shown` the loss is real (this reader
honours the hint, so a magnified image is drawn nearest-neighbour in the output) and is named. The
rows that change what an earlier answer of the owner's allowed — R03 and R08 (A570's photograph
gate), R05 (ADR 2114 section 4), R11 and R03a (A48's line) — are asked in `doc/questions/Q694`,
with its recommendation; until it is answered the tree refuses them as it does today.

The four new loss words are `press-assumed`, `recovered-structure`, `undecodable-stream` and
`attachment-as-pages`; each enters both shipped profiles with the action the table gives, and each
is reported per document and in the file's own `xmpMM:History`, as every loss is.

## The build order, for batch 124

By documents freed under `as-if-printed`, each figure given the remedies above it (2b / part 4):
R02 41/39, R08 27/30, R03 30/29, R06b 26/1, R05 —/25, R03a 23/23, R11 23/20, R04 22/22; then R01
18/18, R10 15/15, R06a 7/7, R12 7/7, R07 7/7, R14 6/5, R17 4/4, R15 3/5, R13 3/3, R16 3/3, R18 —/3.
Under `as-shown`, R09 comes first (81 at 2b, 79 at 4) and the rest follow in nearly the same order.
A row that waits for Q694 is passed over, not held: batch 124 builds R02, R06b, R03a, R04 and R01
in that order while Q694 is open (R03a unless item 3's answer prefers the photograph), and an
answered item takes its rank's place.

## What this does not reopen

- The exceptions of the list's section 7: a dynamic XFA form (4 documents — CLAUDE.md's exclusion,
  §K.1's permission), a document the reader draws no page of (5), and an embedded office file at
  2b or plain part 4 (answered at 4f). Of the 27 unreadable, 20 want the password (an input, not
  an exception), 5 use §7.6.5's public-key handler (in scope and owed) and 2 are not PDF files.
- `doc/questions/Q687`: R08's photograph states no widths as the producer's, so it does not depend
  on Q687's answer, and nothing here builds what Q687 asks about.
- The converter's own defects (15 documents at 2b, 17 at 4, the list's section 8): diagnosed by
  batch 124's rounds, not chosen as remedies.

## Consequences

- Batch 124's archive slots take the order above; each remedy's round measures what it frees,
  since a refusal can hide another behind it and "frees" is an upper bound.
- The status moves to accepted when the owner has read the list and answered Q694; each remedy's
  round records what it built in an ADR of its own, and the list is re-counted, not edited.
