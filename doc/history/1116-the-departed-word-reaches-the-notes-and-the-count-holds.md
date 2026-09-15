# 1116 — The departed word reaches the notes, and the count holds

Contract: verify `departed` (ADR 1119, 14 rows) is complete and coherent after the
double-launch that landed it. Found four note residues and one stale count; the checker and
`tools/state.sh` already agreed.

## What was measured

- **The four navigational documents.** `doc/PLAN.md` §5a, `doc/HANDOVER.md` and
  `doc/state-of-play.md` each name `departed` where they enumerate statuses; `doc/crate-map.md`
  enumerates none (one row per crate), so it owes nothing.
- **The count and the aggregate rule.** `tools/state.sh ledger`/`conformance` print
  `departed 14` as its own figure; `bash -n` clean; every status subcommand exits 0. Planted
  §12.11.6 `departed`→`partial` (trap 13): the live checker moved unsettled 119→120 and owing
  80→81, so a `departed` leaf and the heading above it were excluded from debt. Restored to 14.

## What was fixed (as *what is*, ADR 1023)

- **Four departed notes carried retired debt words** the 1105 move appended past rather than
  swept: §10.7.5 ("the debt this row is `partial` for" → "the requirement this row departs
  from"), §8.6.5.7 ("still owes" → "departs from"), §12.4.4 ("the debt this row is `partial`
  for" → "the departure this row records"), §12.5.6.19 (dropped the "kept this row `partial`"
  clause). No departed note now says `partial`/`still owed` in its body; all 14 lead with
  **Departed** and name a deciding ADR.
- **`doc/state-of-play.md`** called `departed` a "sixth word" — wrong (9 statuses, 5 settled)
  and a counted fact besides; removed the number.

## The two candidates, re-read against A63's shape

Both **stay `partial`** — neither owes only a decided departure. §11.7.2 carries two reported
residuals (a four-component group with no profile, a press past `MAX_PRESSES`) and a declined
`should` — not one sentence. §12.7.8.3.3 keeps `/Rename true`'s refusal (ADR 1070) *and* `/F`'s
external-file refusal, the latter an undecided capability. A row with any undecided debt is not
a pure departure (ADR 1119).

## Gates

`cargo test -p conformance`: lib 259, conformance 7, documents/questions ok; records fails only
on a sibling's live-edited record, not this round's. `bash -n tools/state.sh` exit 0; the eleven
status subcommands exit 0; `ledger.toml` clean of `\uXXXX`.
