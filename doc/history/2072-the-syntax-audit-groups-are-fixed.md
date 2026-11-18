# 2072 — The syntax audit groups are fixed

Session 2072, an audit-fix round in the gwg worktree: review 2038's groups G001 to G008 (`fixes.md`), 34 findings,
and the conformance checks the merged audit files fail. ADRs 2958 to 2963.

**Fixed, B.** B54: `Document::decode_parms` resolves each indirect value of a parameter dictionary (§7.3.10). B07:
`/CF`'s `Identity` key is skipped before it is read (Table 20). B57: a public-key `/StrF` filter other than
`/StmF`'s keys the strings from its own seed and lists (ADR 2959). B21: a header past the 1024-byte window is looked
for before a rebuild (ADR 2960). B09 and B10: −0 equals +0 at the stitching edge and only a zero-width interval is
degenerate (ADR 2963). B45: the Type 4 tokenizer splits on Table 1, NUL included. B11: a non-number `/Decode`
element is an inconsistent entry (Table 87). B55: a linearised file states at least 1.2, Issue #331 (ADR 2958).

**Fixed, D.** D18: Algorithm 2.B runs to its own bound of 287 repetitions (ADR 2961). D16: reading `/Perms` over a
disagreeing `/P` is called a recovery policy (ADR 2962). D08, D28, D42: the hex-string doc, §14.4's note (D42 the
audit had already corrected).

**T, by plant.** Each of the twenty T findings' tests was run against a plant of its defect; nineteen audit tests
were killed. T141's survived (the `endstream` check recovered from framing by `/DL`), so
`filters::dl_at_an_offset_where_endstream_appears_still_does_not_frame_the_stream` is new and is killed. T108's
`remaining` weak `/ID` assertion is strengthened and killed by a plant. No owner audit file was edited for a finding;
two had a comment changed for the conformance gates (below).

**Conformance owed.** `crypt.rs` writes "RFC 4013 section 3"; `calculator_functions_spec_audit.rs` "PLRM 3 Table
3.1"; `public_key_witnesses.rs` names the shard's path, not a round; `records.rs` holds
`2038-specification-evidence-audit.md` by name (`GATES_HELD`). §14.9.2.2's row is G017's.

**Ledger.** §7.5.2, §7.6.4.3.4, §14.4 and §F.3.2 notes restated, statuses unchanged (implemented).

**Gates.** Over pdf-syntax, pdf-colour, pdf-model, pdf-transform, conformance: fmt `--check` exit 1, on
`pdf-model/tests/named_property_actualtext_spec_audit.rs` alone (on main, G017's file, left); clippy `-D warnings
--all-targets` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a` exit 100, 4700 run, 4650 passed, 50
failed: the expected list's 60 for these crates less the 11 fixed, plus
`named_page_geometry_spec_audit::apref_page_rotation_…_widget_pixels`, which fails in round 2070's own broad.log
and is missing from the list; rustdoc exit 0; conformance after `git add -N` in the report. Fuzz not owed.

**Trap.** A plant reversed by moving a saved copy back restores an mtime older than the build, and cargo then keeps
the planted build: restore by writing the file.

**Left.** `xref::sections` and `walk_revisions` still take the window's header (ADR 2960 section 3).
