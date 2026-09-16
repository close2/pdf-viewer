# 1140 — The remaining frontier, mapped by why each row is not done

2026-09-16. Direction slot. Files: `doc/todo/65-the-remaining-frontier.md` (new),
`doc/HANDOVER.md` (one index row), this record. No code, no pixel, no ledger row moved — a map,
not a decision, so no ADR. Five siblings mid-flight in the shared tree.

## Measurement (trap 8)

Read every `partial` and `reported` note once — the reason is the note's residue, never the
corpus's. `cargo run -q -p conformance --bin ledger`: 102 `partial`, 11 `reported` = 113 rows.
Classified each into the six briefed buckets; two shapes fell outside them and are named.

## The shape

- **Aggregate** (34 rows) — a parent that states no debt of its own and flips when a child does
  (the §7.6, §11.x, §12.x family heads). Not independently actionable; the largest group, and the
  reason a raw `partial` count overstates the owed frontier.
- **Host-UI-blocked** (20) — printing, collections, URI-open, measurement tools, the restriction
  levels: a surface principle 3 defers. **The largest actionable bucket.**
- **Hard rendering/architecture** (18) — the shape channel, the second-raster transfer/knockout
  pair, the patch-mesh tessellation tolerance, the non-affine colour routes.
- **Not-owed** (17) — a documented choice, an exclusion, a deprecation, or a standard-gap; the
  bucket principle 5 says decays, flagged to re-read first.
- **External-dependency-blocked** (9) — brainpool/Ed448 packages, `hayro` decoder limits, ISO
  18619 / ISO/IEC 15444-2 / ISO 19444-1 unheld.
- **Genuinely buildable now** (7) — §8.10.2, §8.9.5.1, §7.5.6's version chain, the deprecated
  catalog/name-tree entries; plus §12.5.6.23 / §8.6.6.5 / §12.7.4.3 as feature-depth extensions.
- **Owner-question-blocked** (5) — the public-key handler family (§7.6.5.x, §7.6.6), gated on
  `doc/questions/Q66`, which round 1139 raised in this same batch (a `doc/stack.md` decision).

## The headline

Of 113 open rows, ~10 are a normal round's to take today; the rest wait on a host surface, a
dependency, cross-round architecture, the owner's Q66, or are not owed. The coverage frontier has
narrowed to blockers, not to missing reads.

## Gates

`cargo test -p conformance`, `tools/state.sh ledger questions`, `tools/batch.sh check` — in the report.
