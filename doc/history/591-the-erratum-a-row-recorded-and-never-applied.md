# 591 — The erratum a row recorded and never applied

Session 590 found the §14.8.4.7.2 ledger row naming Errata Collection 3's Issue #437 since session
418 and then quoting the sentence that erratum struck out, two sentences later. **A row that
records an erratum is not a row that has applied it** — a whole failure shape, and nothing in this
tree looked for it, because a place that names the erratum looks maximally diligent. It has a
command now, and its first run found the same shape one clause family over.

Date: 2026-08-18.
ADR: [0426](../adr/0426-the-erratum-a-row-recorded-and-never-applied.md).

## The sweep

`cargo run --release -p spec-errata -- applied doc/*.pdf`, two seconds. In the sidecar rather than
under `conformance` because ADR 0252's dependency rule is a rule: the errata are read out of
fourteen PDFs, and nothing this project generates may become what the gate checks the standard
against.

Its discriminator is that **nothing is inferred** — the erratum is named as data, by the writer, in
the place itself, and the `StrikeOut` and the `Caret` supply both sides of the comparison. Three
unit tests establish that it discriminates; the first plants the §14.8.4.7.2 defect session 590
fixed. Planted into the real `ledger.toml` it comes back on the read-first list with the erratum's
own replacement underneath, and the unmarked count moves by exactly one.

## What it took to make the marker honest

At 200 characters backwards the first run put 72 hits on the read-first list; at 400 either side it
put 26, and every one of the 46 that moved was a correction. This project writes a correction in
both orders — the retirement sentence in front of the quotation in `standard.rs`, behind it in
`appearance.rs` — and the phrase list is deliberately *not*
`conformance::blockers::HISTORY`'s, whose `said` and `this row` would have marked the one defect the
sweep exists for as noise.

## The other half: the twelfth sweep's comparison was blind twice over

`squeezed` kept square brackets, so `CLAUDE.md`'s own `"[e]ncloses"` spelling of an altered first
letter made a passage unfindable; and it kept dash shapes, so a quotation carrying a table caption
could not match the conversion. It is `conformance::prose::folded` now — one comparison in the
crate rather than two. Thirteen new landings, read one by one, three of them defects.

## Files

- `tools/spec-errata/src/applied.rs` — the sweep. `src/lib.rs` — `Note::change` and `squeezed`.
  `src/main.rs` — the subcommand.
- `tools/conformance/src/prose.rs` — `blocks`, with `quotations` one step on top of it.
- `crates/pdf-font/src/loading.rs`, `crates/pdf-model/tests/composite_fonts.rs` — §9.10.3's two
  blockquotes.
- `doc/conformance/ledger.toml` — §9.6.2.1, §9.6.2.2 (twice), §9.10.3.
- `doc/errata-read.md`, `doc/todo/48`, `doc/todo/01`, `doc/todo/02`, `doc/ledger-and-claims.md`.

## Gates

The whole of `doc/todo/02` §2. Every one green; the numbers are `tools/state.sh`'s to print.
