# 978 — The other document this tree quotes verbatim

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `bb77086d`. Stream: general
improvement — the slot kept open so that a focused push never takes every round. Four sibling
rounds were live in the same working tree throughout (974 in `pdf-font`/`pdf-model`/`render-*`,
975 in `pdf-archive`, 976 in `pdf-transform`, 977 in `tools/conformance/`).

Argued in **ADR 0989**.

## What the round chose, and why over the alternatives

The brief offered `doc/todo/` items in the free crates, the normative annexes `CLAUDE.md` lists as
in scope, an unused dependency in `pdf-syntax`, and the method that has now paid five rounds
running: **the population question, asked somewhere new**.

Two candidates were eliminated by a number before the third was taken.

- **The normative annexes.** `awk` over `doc/conformance/ledger.toml` prints a status for every
  row whose clause begins with a letter: 7 for D, 3 for E, 23 for F, 4 for I, 3 for K, 1 for L,
  5 for O, 6 for Q — **52 rows, none `unreviewed`, none `silent`**. Annex O was settled by session
  973. So "the annexes have no rows" was false and "the annexes have unread rows" was false; what
  was left was whether a row's *reason* still held, which is where the round went next.
- **The `reported` and `silent` rows.** The same `awk`: **17 `reported`, 0 `silent`**, and every
  one of the seventeen is a public-key handler, a network action, a media engine or a revocation
  check — each owned by a `doc/todo/` item and most of them in a sibling's crate this round.
  Nothing there was both free and unclaimed.

The third candidate was the one that paid. Reading the 23 Annex F rows to check their reason, all
23 turned out to quote `CLAUDE.md` saying something `CLAUDE.md` does not say, and one of the three
things that retired sentence excluded — §7.5.7's object-stream packing — is implemented **in the
crate whose module header repeats the same exclusion**. That is the population question one
document over: `--bin quotations` reads every quotation in this tree against the specification
Markdown, so the tree's *other* verbatim source, its own governing document, had no reader at all.

## What changed

- `tools/governing-quotations.py` — the sweep, reporting rather than failing, with `doc/adr/`,
  `doc/history/` and `doc/rfc/` printed apart as records. `tools/state.sh governing` runs it and
  `doc/verify.md` catalogues it.
- `doc/conformance/ledger.toml` — the 23 Annex F rows (`F`, `F.1`, `F.2`, `F.3`, `F.3.1`–`F.3.11`,
  `F.4`, `F.4.1`–`F.4.7`) now quote `CLAUDE.md`'s current exclusion, verbatim. Status unchanged:
  `out-of-scope`, `exclusion = "writer-side"`. The aggregate row and §7.5.6's row record the
  correction.
- `crates/pdf-syntax/src/write.rs` — the header names both of this tree's writers instead of
  denying the second, and keeps the sentence it had wrong in view.
- `crates/pdf-syntax/Cargo.toml` — `pdf-spec` removed. Declared in the crate's first commit
  (`f26bbcb1`), named by no file in the crate, ever.
- `doc/HAYRO_MERGE.md` — one misquotation of `CLAUDE.md` replaced by principle 3's own sentence.

## What is left

Four misquotations the sweep prints and this round could not touch, because four siblings hold the
files: `crates/pdf-font/examples/type1_encoding_census.rs`, `crates/pdf-model/src/restriction.rs`,
`doc/todo/01-ledger-partial-rows.md` (twice) and `tools/conformance/src/unread.rs`. ADR 0989 §5
tabulates what each says against what `CLAUDE.md` says. None is a behaviour defect.

The sweep is a Python script under `tools/` rather than a `conformance --bin`, which is where it
belongs — `tools/conformance/` was session 977's for the whole of this round. Promoting it is a
line of work for whoever next owns that crate.
