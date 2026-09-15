# 1105 — A word for a decided departure

Contract: A63. The owner answered `doc/questions/Q63` on 2026-09-14 with "Add `departed`" —
a status for a clause every requirement of which is executed except one sentence decided
against with its cost recorded. This round built it.

## What was done

- **`Status::Departed`** in `tools/conformance/src/ledger.rs`: `as_str` "departed", `owes`
  false (a fifth settled status), `all` now `[Self; 9]`. The vocabulary doc comment and
  `SETTLED` in `counts.rs` follow. ADR 1119 for the definition and the aggregate rule.
- **The aggregate rule is ADR 1035 section 5's**, inherited unchanged: a `departed` row owes
  nothing, so `Ledger::is_aggregate` lets a heading above one settle. Three unit tests pin it —
  the plant, its control, and a heading over a `departed` leaf.
- **`check` refuses a `departed` row whose note names no ADR** (`names_an_adr`): "with its cost
  recorded" is a claim about a document. Calibrated by a plant (trap 13) — stripping every
  `ADR NNNN` from §12.11.6's note fired ``departed needs a note naming the ADR``; restored, green.
- **`tools/state.sh` counts `departed` as its own figure** — the conformance filter gains the
  word, never folded into `implemented` or `partial`.
- **Fourteen rows moved** `partial` → `departed`, each with its first sentence naming the
  departure and the deciding ADR: §7.4.2 (0036), §7.4.8 (0036), §7.5.5 (1035 §3), §7.10.2
  (0098), §8.5.3.3.1 (1060), §8.6.5.7 (0272), §10.4.2.5 (0263), §10.7.5 (0028), §12.4.4 (0230),
  §12.4.4.1 (0230), §12.5.2 (0304), §12.5.5 (0030), §12.5.6.19 (0239), §12.11.6 (0460).
- **§12.5.2's `/Lang` finding:** its note carried a retired session-1040 sentence claiming
  `/AF` and `/Lang` had no reader; `structure::annotation_languages` (in the row's own `code`,
  tested) has read it since session 1051, so the requirement is owned and the row is a pure
  departure. Deleted the stale sentence rather than leave the row looking ownerless.
- Headers: `ledger.toml`'s and `bin/ledger`'s generated preamble, `doc/PLAN.md` §5a's status
  list and prose, one line each to `doc/HANDOVER.md` and `doc/state-of-play.md`.

## Counts and gates

`implemented` 558 unchanged; `partial` 123 → 109; `departed` 0 → 14; owing 95 → 81 of 120.

`cargo test -p conformance` green (259 lib incl. 3 new, plus the ledger gate). `bash -n
tools/state.sh` and every status-counting subcommand exit 0. `foreign_corpus` and the
quotation gate's failures on `image_masks.rs` / `preserve.rs` are sibling rounds' live edits,
not this round's — see the report.
