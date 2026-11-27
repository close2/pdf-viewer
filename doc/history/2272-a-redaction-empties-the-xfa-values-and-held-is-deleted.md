# 2272 — A redaction empties the XFA values, Annex Q's routes are mended, and `HELD` is deleted

Lane 3, review 2271's fix groups 1 and 2 and the census's last nine held sites. ADRs 3789–3792.

- **B04** (ADR 3789): `redact` now takes a removed field's value out of the XFA resource:
  `pdf_model::xfa::redacted` finds the `datasets` data value by chapter 4's binding
  (`Datasets::bound`, shared with `value`), the `form` packet's field by the PDF name as a SOM
  expression, and the template's default, and empties each (node kept, so later same-named siblings
  keep their index); where any place is uncertain — dynamic form, unreadable resource, unfollowed
  mapping, unnamed field, unfollowed binding, a data value another kept field binds, an unreachable
  same-named `form` field — `/XFA` (and `/NeedsRendering`) goes whole and the report says why.
  `redaction_leaves_no_trace` carries the value in all three packets and gains a whole-removal case.
- **B01–B03** (ADR 3790): the `gs` `/Font` route walks Type 3 glyphs; the visited set is keyed by
  route (`Within::Page` / `Within::MaskGroup`); a tiling cell starts from the state its parent stream
  began in. Each has a survey test that fails without it. **Optional content** (ADR 3791): an element
  in an off group stays in Q.2's population — no code change.
- **The nine sites** (ADR 3792): new class `model/tiling-pattern-entry-unusable` (Table 74's `/BBox`,
  `/Resources`), mapped to section 5.1; a soft mask group's `/Resources` of another type now logs
  `form-resources-not-a-dictionary`, as its comment already claimed; five marked. `HELD` deleted: the
  census fails naming each silent line.
- 2271-B01..B04 out of `expected-failures-current.json`, `fixed` in 2271's `findings.json`; ledger
  notes of Q.2, Q.3, Q.5 and K.2 restated, statuses unchanged.
- Left: `transparency_group`'s `?` reads a malformed `/Group` as none, silently (a shape the census
  cannot see); 2271's D01 and D02.

**Census.** With the pack, on the pre-rebase tree `1bb8294c6`, against 2269's log: the twelve
summary lines and every refusal line are 2269's — no document moves. veraPDF 1.30.3 over 5503
outputs: 40 not compliant, 12 unfinished, the log line for line 2269's. `tools/state.sh --round
2272 launch`: 6 documents, 36 figures banded, 0 outside. No per-object walk on the launch path
changed (a tiling pattern's read and a mask group's are once per object), so no instruction count.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p
conformance`, rebased on `12be4d733`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5880 of 5883 — the census
and the content-class gate found this round's own new sites, fixed, and this record before its
gates were written; re-run of `pdf-model` and `conformance` after the fix 0; rustdoc of
`pdf-model`, `pdf-archive`, `pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0.
