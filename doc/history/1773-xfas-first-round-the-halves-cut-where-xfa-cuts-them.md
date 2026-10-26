# 1773 — XFA's first round: the halves cut where XFA 3.3 cuts them, and the amendment asked by name

Session 1773, a clause round in batch 117; 2026-10-26. RFC 0011; ADR 2380; ADR 2381 unused; Q639.

**Contract.** Slot 3: `A633`'s first XFA round — the XFA 3.3 text converted into `doc/md/`, RFC 0011
placing a static form's behaviours apart from a dynamic form's layout and asking for the authoring
amendment by name, the corpora's hybrid and `/NeedsRendering` files censused, Annex K's notes pointed
at the RFC. No status moves.

**Premises.** *The conversion is to be made* — **did not hold**: `doc/md/XFA-3_3.md` exists (1 584 page
markers) from a PDF whose SHA-256 is the owner's `doc/XFA-3_3.pdf`'s, by `tools/spec-md.py`, unchanged
since; not re-made, and the *Grammar Excluded from XFAF* table (pages 272–273) checked against
`pdftotext -layout`. *Four hybrids and three `/NeedsRendering` documents* — **half held**: 8 of 1 470
files mutool opens state `/XFA`, 4 hybrids and 4 asking to be regenerated (the fourth is the corpora's
MOZILLA file RFC 0009 counted); the K.1 note's 7 and 3 come from another population and stand.
*The halves separate at `layout` and `presence`* — **half held**: the cut is `occur`, `subformSet`
and Table 29's flag (pages 72, 333); `layout` is one row of the XFAF table, and `presence` belongs to
both halves. The XFAF table is not a presence test: all four hybrids state `layout="tb"` and page `w`/`h`.

**Found.** `ViewState::save` writes `/V` and never the `datasets` packet, so a filled, saved hybrid
form breaks §K.2's writer rule "The XFA field values shall be consistent with the corresponding V
entries of the PDF field objects" — the static half's first round (ADR 2380). The hybrids hold no
calculation, validation or format and one script (`xfa.host.print`, a Tier 2 call); the behaviour
— 2 243 JavaScript and 10 FormCalc scripts, 363 validations — is in the two dynamic government forms.
`doc/md`'s §K.2 drops "these rules:" from its writer sentence; the PDF has it.

**Archive, and next.** `doc/specifications.zip` need not carry XFA 3.3 until a test derives a value
from it (no gate opens it). The next XFA round is RFC 0011 section 13 item 2: template, data, `datasets`.

**Files.** New: `doc/rfc/0011-xfa-a-static-forms-behaviours-and-a-dynamic-forms-layout.md`, ADR 2380,
`doc/questions/Q639-may-a-page-be-laid-out-by-a-producers-own-xfa-template.md`, this record. Edited:
`doc/rfc/README.md` (one row), `doc/conformance/ledger.toml` (the K, K.1, K.2 notes, `-U0`). I worked
in `scratchpad/r1773/`.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 101, 462 passed, 4 red, none mine —
the quotation gate on the sibling's `pdf-model/src/media/deprecated.rs:45`, two on §14.8.5.4.4's
sibling edit, and record 1776's 42 lines. `--bin quotations`: exit 0, none of my files diverging.
`tools/state.sh --round 1773 docs`: exit 101, rustdoc links in `pdf-model` and `viewer-core`, crates
siblings are editing and I did not touch. Census walk under the bound: 42 s. No crate, no tier 1.
