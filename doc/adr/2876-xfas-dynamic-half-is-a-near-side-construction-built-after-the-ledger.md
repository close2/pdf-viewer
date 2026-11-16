# 2876 — XFA's dynamic half is a near-side construction, built after the ledger

Session 2030. Status: **accepted**, on the owner's answer `doc/questions/A840` (2026-11-13: "agree on 1 and 2"), which
adopts `Q840`'s recommendations 1 and 2. Clauses: ISO 32000-2 §K.1, §K.2, Table 29 (`/NeedsRendering`). Supersedes
the exclusion `doc/questions/A639` kept (ADR 2430) until the price `doc/todo/78` measured (ADR 2837).

## The decision

`CLAUDE.md`'s closed list loses *XFA's dynamic half*, and the authoring line's amendments gain the construction RFC
0011 section 7 worded: **a page laid out by a producer's own XFA template**. Every mark on such a page is one the
template names, its positions are what XFA 3.3's layout and binding rules decide from the template and the
document's own data, nothing else is placed on it, and it is said to be generated.

## Why it is on the near side, and the test it fails

The authoring line asks whether an operation invents marks, read by ADR 1120 as their provenance. A dynamic form's
marks are all the producer's — a `draw`'s text, a field's value and caption, a border, a barcode's data — so the
provenance test holds. ADR 1123's second part does not: no clause of ISO 32000-2 fixes the positions, XFA 3.3
does, and §K.2 names XFA 3.3 as the resource's format. The owner accepted exactly that sentence: a normative
reference's layout algorithm may stand where a clause's position would. The target is the reason — "every PDF
that exists renders as its producer specified" — and the producer of a `/NeedsRendering` file specified its pages
in the template, not in the wrapper page it wrote for processors that decline.

## When, and what binds the build first

The build waits until the ISO 32000-2 ledger rows reach their final states (A840's words) and follows
`doc/todo/78` section 4's order: the merge and page one's launch cost are measured first, because *nothing eager*
still binds and the first round must be able to say page one costs too much. Its first requirement is A840
item 2: at script level `off`, the default, page one is laid out from the template and the data alone and the
window says the form's scripts did not run (`doc/todo/78` section 5). Until it lands §K.1's permission stays
taken out loud and Annex K's rows keep their statuses; their notes now say the construction is accepted and
scheduled rather than excluded.

## What it costs

About 23 to 28 rounds (`doc/todo/78` section 4), a new crate `xfa-layout`, a template-script object model, and
pages with no producer's bytes for the oracle — compared against this program's earlier output, which says
*unchanged* and never *correct* (ADR 1014). The barcodes those pages draw are A840 item 3, ADR 2877.

## Reopened if

A generated page is asked to carry anything its template and data do not state, or to be laid out by any
algorithm but XFA 3.3's — each is a new amendment, argued on its own.
