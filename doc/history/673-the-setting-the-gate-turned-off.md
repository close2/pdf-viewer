# Session 673 — The setting the gate turned off

2026-08-22. Branch `round-673`, a parallel round. ADR 0498.

## What was asked

Take quorra's glyph-phase-carry fix (`doc/QUORRA_GLYPH_PHASE_CARRY.md`, their ADR 0073) and close
the hole that let it hide here: the 974-page instrument both projects reach for first ran with the
setting the defect lived in turned **off**.

## What was done

- **Pin moved `cad50156` → `97ad95ac`.** Nine commits, exactly one touching `src/` — verified off
  `git log --stat` over the range, not off the release note. No API change, no line adapted.
- **`tests/corpus.rs` runs at the shipped glyph quantum** and gained
  `PDFVIEWER_QUORRA_GLYPH_QUANTUM`, the fourth knob. `Settings` now carries the four knobs as one
  value, which is also what kept the gate function under `clippy::too_many_lines`.
- **`tests/real_pages.rs`'s two sets of constants re-derived and both forced against the old pin**,
  and the quantum gate prints its comparisons now.
- **`examples/lane_diff.rs`'s comment corrected** — it said it used "the gate's own options", which
  is how both trees came to believe its numbers were the quantum's.
- **`QUORRA_FEEDBACK.md` §37** answers their report; §31 gained the caveat that actually applies.
- **`doc/QUORRA_UPGRADE.md`** has the release section with the four-lane matrix.
- **`doc/todo/02` §2** has the quantum line and what it costs.

## What was measured

Every figure is in ADR 0498 and in the two quorra documents; the logs are `tmp/r673-*.log` in the
worktree. Their §2 reproduced to the page in both arms; three of their claims checked rather than
inherited (no regression, the two settings agreeing after the fix, one commit touching `src/`); the
three lanes they did not run measured here, where the movement is larger than the one they report.

## Gates

The full §2 sequence plus `cargo deny`, and §5's binaries into this worktree's `target/`. The
oracle ran at load average 12 on a desktop with a parallel round building beside it and was green;
its reference-render cache reports a 0% hit rate because a parallel round has its own target
directory and therefore its own cache — trap 10a's tell, with a cause that is not the corpus
moving.

## Owed

- Nothing from this round is half-done. What is *open* is quorra's, and named in ADR 0498 §7: the
  two questions `QUORRA_FEEDBACK.md` §31 asks are still open, and their §33 answer has not arrived.
- Whoever merges owns `doc/todo/02` §2 for the merged result, and the main tree's `target/` still
  holds the pre-bump binaries.
