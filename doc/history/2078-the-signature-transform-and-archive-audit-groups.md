# 2078 — The signature, transform and archive audit groups G079 to G091

Session 2078, an audit-fix round in `pdf-viewer-gwg`. Groups G079 to G091 of
`doc/reviews/2038-specification-audit/fixes.md`: 24 findings (9 B, 11 T, 4 D).

- **Fixed 23, open 1, argued 0, owner 0.** Each line of `fixes.md` names its ADR and tests.
- ADR 2994 (amends 1461): the Annex Q survey reports pages a bound hid from the transparency
  rows rather than meeting them; Table 166's `/CA` counts only without an appearance stream; a
  soft mask's group does not itself make a page transparent.
- ADR 2995 (amends 1796, 0159): a form value is written on the terminal field's `/T`, not a
  shared ancestor's `/V`; a usage-rights save asks `/Annots /Create` and `/Modify` too.
- ADR 2996 (amends 1049, 1096): a document timestamp's field, widget, `/Fields`, `/Annots` and
  `/SigFlags` registration is Table 257's validation material; a field renamed after signing is
  in a Table 259 selection under either name.
- ADR 2997 (amends 1270): Annex K packets are string-and-stream pairs or the resource is refused;
  `apply_protected` refuses protection for verbs that write no protected file (archive among them).
- ADR 2998 (amends 2034): a threshold halftone carrying a transfer function is not replaced under
  a screen authorisation unless the transfer row removes the function.
- ADR 2999: a structure element with no content goes where its parent goes (Table 369's `WP`).
- New tests: `pdf-model` `signed_field_lock_shapes` (three genuinely signed witnesses, All,
  Exclude, `/P`, from the audit's generator), `pdf-transform` `date_grammar` (§7.9.4 read without
  `Date::parse`), `pdf-archive` `soft_mask_population`, and the archive protection refusal.
- B46 stays `open`: the 256-component merge is in `pdf-model`'s `file_spec.rs`, G049's file, held
  by round 2077.
- Ledger notes moved: 12.8.2.2.2, 12.8.2.3, Q.2; no status changed.
- 12 manifest failures pass and left `expected-failures-current.json`; none appeared.

**Gates.** Over `-p pdf-model -p pdf-signature -p pdf-transform -p pdf-archive`: fmt exit 0;
clippy `-D warnings --all-targets` exit 0; nextest `--lib --bins --tests` exit 100, 5104 run, 10
failed, exactly the manifest's remaining 10 for these crates, none new; rustdoc `-D warnings
--document-private-items` exit 0; conformance's `conformance` and `records` exit 0 with this
paragraph written. No public type changed, so no fuzz check is owed. The fast disk's quota was
reached once mid-run and the gates were re-run.

Trap: a full per-user quota on the fast disk shows as a linker bus error ("mold: failed to write
to an output file"), not as a compile error; `quota -s` first, then the round's own
`debug/incremental`, which gates run without.
