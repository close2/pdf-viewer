# 2279 — The unrestated route over a machine's face is the loss `machine-face`

Session 1722, the archive slot of batch one hundred and eleven. Status: implemented. Context: ADRs
0270, 2162, 2262; `doc/todo/72`'s owed machine-face entry.

## What was owed

ADR 2162 made embedding the shipped face over a face of this machine's the loss `machine-face` on
the described route and the restatement route: the file states no face (§9.5's NOTE 5 leaves it to
the processor), so nothing the file says is lost, and what is lost is what this viewer's window
drew on this machine. The third route — a font whose own descriptor states no program and whose
dictionary needs no restating — read the drawn face off the font as loaded and kept only the
shipped-face answer, so a TrueType font whose name `substitute::find` matched an installed face had
the shipped face embedded over the machine's as a statement, unasked. ADR 2262 named that gap.

## Decision

**The third route names the loss as the other two do.** `described::drawn_by_the_viewer`'s answer
is taken whole: a machine's face is `Loss::MachineFace`, which `as-if-printed` discards and
`as-shown` stops on, and the shipped face fitted to the widths is `substitute-stretch` as before.
That route has already refused a shown code the shipped face cannot draw. An operator's `--font`
still incurs neither; it is the operator's statement.

**What is lost, looked at.** `issue8697.pdf`'s heading is a TrueType font drawn here from the
machine's own Liberation Sans, a later version than the one this program ships. Source against the
`as-if-printed` output, page 1: the same words at the same positions, unchanged to the eye, and
8 591 pixels differing at 288 dpi, up to 255 levels on glyph edges — the machine's outlines against
the shipped ones. That output is pixel for pixel the one HEAD wrote; what changed is that its
report and `xmpMM:History` now say it lost the machine's face.

**Tests that named a machine's face by accident say so.** Five fixtures named `/LiberationSans` or
`/Tahoma-Bold`, which this machine matches; they converted under no authorisation only because the
route dropped the loss. They now authorise the face losses, the one asserting `Stated` asserts
`machine-face` where the viewer draws a machine's face, and a new fixture holds the route both ways
(skipped, saying so, on a machine without the face).

## The census, against HEAD, by document

Real corpora (pdf.js, `doc/corpora`) at 2b and 4, both exports' binaries, the walk run alone;
converted / conforming / refused / unreadable, HEAD → this change with ADR 2278 beside it:

| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| pdf.js | 757/2/197/11, unchanged | 723/0/233/11, unchanged | 745/2/209/11 → 737/2/217/11 | 712/0/244/11 → 704/0/252/11 |
| `doc/corpora` | 341/6/127/16 → 343/6/125/16 | 333/0/142/16 → 336/0/139/16 | 301/6/168/16 → 302/6/167/16 | 289/0/186/16 → 290/0/185/16 |

**Under `as-if-printed` no document is newly refused**; the four it newly converts, two at 2b and
three at 4, are ADR 2278's. **Under `as-shown` eight pdf.js documents are newly refused at each part**, each by
`machine-face` withheld alone (`bug1671312_ArialNarrow`, `issue14438`, `issue15262`, `issue4934`,
`issue7454`, `issue8697`, `issue9291`, `non-embedded-NuptialScript`), and the veraPDF corpus adds
two at each part (`6-2-11-4-1-t01-fail-a` and `-c`, `6-2-10-4-1-t01-fail-a` and `-c`). In
`doc/corpora` no document is newly refused: seven already stopped on a font loss and now name
`machine-face` beside it, and one converts by ADR 2278. Every one is a document that converted
under `as-shown` by drawing other shapes than its window did and saying nothing.
