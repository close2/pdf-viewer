# 1721 — `DeviceCMYK` is read where a colour is painted

Session 1721, 2026-10-23. Batch 111, slot 2 (clause). ADRs 2276, 2277; no `Q`. §§8.6.5.6,
11.7.2, 11.7.5.3, 14.11.5. This session worked under `scratchpad/r1721/`.

**Controls first.** Four independent controls (`tests/icc_paint_context/cmyk_redefinition.rs`)
on three presses whose black ink the output intent, an `ICCBased` group and a nested group absorb
differently; expected pixels are each press's encoded corner for `0 1 0 1`. On HEAD `3e08a117`'s
production code two fail on ten pixels: every selection — `k`, `cs`/`sc`, an `Indexed` entry, a
`Separation`'s alternate and an uncoloured pattern — resolved to the intent on the page and
inherited into the group reads the intent's press, and selected in one group and painted in a
nested one reads the outer's. Two pass as they must: the presses differ, and a `/DefaultCMYK`
naming the intent's own profile stays the default in the group. The brief's premise held for `k`
and was wider than it said: the `Indexed`, tint-alternate and pattern routes were stale too.

**Change.** `SolidSource::Cmyk` is the family however the selection read it, and paints as
`device_cmyk_at` reads it there; a whole space and an uncoloured pattern's tint keep the reading
they held, replaced by `read_where_painted` (`content/colour.rs`, `content/pattern.rs`,
`pattern/tiling.rs`). The family is told by allocation (ADR 2277). §11.7.5.3 `partial` →
`departed` on ADR 1242's bypass alone; §11.7.5 and §11.7 `partial` → `implemented`, their last
leaf settled; §11.7.2's note says where the family is read.

**Gates.** HEAD controls in an export: 39 passed, 2 failed (exit 101); candidate 41 passed,
exit 0; four mutations alone each exit 101 (ADR 2277). `cargo fmt --check`, warnings-denied
clippy `-p pdf-model --all-targets`, rustdoc `-D warnings --document-private-items`: exit 0.
`cargo nextest run -p pdf-model` on the export: 2192 passed, 21 skipped, exit 0. `cargo test -p
conformance`: exit 101 on siblings' gates only (`names`, `records`, `teaching`, §10.7.4's ADR
2281); the ledger, frontier, `evidence` and `adr_status` tests pass on my rows. On the export,
`raster_golden` 2 passed, exit 0, and the six arms by page against HEAD's six arms: 0 of 5799
digests differ, each arm exit 0, 1684 s hold. One `--clock` hold: six cost pairs, 12 sides exit 0
(ADR 2277), then `t2-turn_path` exit 0, 33 figures banded, 33 judged, 0 outside.

**Witness.** HEAD and candidate pictures of the `k` and nested-`Separation` controls, rendered and
looked at: the inherited patch matches the cold one on the candidate, and HEAD's is the intent's
lighter pink and, nested, a grey.

**Remains.** A damaged `ICCBased` resource whose alternate is the family, selected a second time
in one group and painted in a nested group of another profile (ADR 2277).
