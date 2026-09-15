# 1067 — A bound prints beside its figure, whatever shape the bound has

The instruments slot, continuing 1061. ADR 1075 left three things standing; all three are done
here (ADR 1081). **Three ceilings are named populations now.** `MAX_LOCKED` (10),
`MAX_UNREADABLE_ENCRYPTION` (1) and `MAX_PAGELESS` (5) become `LOCKED`, `UNREADABLE_ENCRYPTION`
and `PAGELESS`, each name carrying its reason: for the ten, where its password comes from —
eight from the pdf.js issue the file is
named after, `issue21579.pdf`'s `pässwört` (ADR 0820), and `encrypted-attachment.pdf`, there by
ADR 1040's reading of §7.6.6 rather than by a failure; for the five, round 1058's diagnosis of each,
clause by clause. `gate_ratchet::population` prints the same line as `ceiling` — the count beside
the length of the list — so 1061's table stays whole, then holds the two sets equal **both ways**.

**Calibrated on the gate, not in a unit test** (trap 13). One name in `LOCKED` swapped for one that
does not exist: the line still reads `10, ceiling 10, slack 0` — the count is blind — and the gate
fails with `joined ["print_protection.pdf"], left ["print_protection_calibration.pdf"]`. One deleted
from `PAGELESS`: `5, ceiling 4`, `joined ["Brotli-Prototype-FileA.pdf"], left []`. Both restored;
the gate is green on the restored tree.

**Both band gates print every figure beside its band on every run.** `gate_ratchet::band` prints
`figure, band, distance to the nearer edge (share of the band)` and asserts nothing — a band's
verdict has conditions that belong to the gate. ADR 1075 deferred this as needing "the round that
can run that gate's display"; that premise was false, `launch_path` is headless under `--release`.
**It paid on the first run.** Three `open_kinstructions` figures sit 10–15% from the **high** edge
of bands whose measured spread is 0.011% — `xfa_filled_imm1344e.pdf` has 3.7 thousand instructions
of headroom on 1816 — so the next round that adds a little to an open fails for a change nobody
would call a regression. No band moved: that needs the measurement, not the discovery. The other
edge of it: eight `fixed-documents.toml` rows print 0% forever, because a page pinned
`ink = 0.0 .. 1.0` is honestly blank and ink has nowhere below zero to go — a wall, not a warning
(trap 39), said in `band`'s doc comment.

**`tools/state.sh ratchets`** runs every gate carrying a bound and keeps the table: 149 lines, 48
one-sided bounds and 101 bands. Population derived (every tracked test file calling
`gate_ratchet::`, trap 25), invocation derived from `doc/todo/02` §2 down to the profile and the
`--ignored` — `--include-ignored` was the first attempt and was wrong, because `save_round_trip.rs`
holds an ignored corpus walk and an unignored check over one document in one temporary directory
and libtest ran them in parallel threads. Composed, so not in `all`. **`ratchets.rs`'s own floor
moves 10 → 8 with its reason above it**: a named population is a `&[&str]`, which
`integer_constant` deliberately does not count, so three gates getting stronger took three off it.
**Shared worktree**: four gates in one section run died on a neighbour's `pdf-model` mid-edit and
were re-run when it compiled; gates and exit codes are in the report.
